//! Google Video model (Veo) — implements `VideoModel`.
//!
//! Endpoint: `POST /v1beta/models/{model}:predictLongRunning`
//! This is an async operation — the initial POST returns an operation name,
//! which is polled until the video is ready.
//!
//! Models: veo-2.0-generate-001, veo-3.0-generate-001, veo-3.0-fast-generate-001, veo-3.1-generate

use crate::error::parse_google_error;
use crate::video::google_video_options::GoogleVideoOptions;
use ararajuba_provider::errors::Error;
use async_trait::async_trait;
use ararajuba_provider::video_model::v4::video_model_v4::VideoModelV4;
use ararajuba_provider::video_model::v4::{
    VideoCallOptions, VideoData, VideoGenerateResult, VideoImageInput,
    VideoResponseMetadata,
};
use ararajuba_provider_utils::http::post_to_api::{post_json_to_api, PostJsonOptions};
use ararajuba_provider_utils::http::response_handler::{
    create_json_error_response_handler, create_json_response_handler,
};
use futures::future::BoxFuture;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

/// Interval between operation polls (in milliseconds).
const POLL_INTERVAL_MS: u64 = 5_000;
/// Maximum number of polling attempts.
const MAX_POLL_ATTEMPTS: u32 = 120; // 10 minutes at 5s intervals

/// Configuration for the Google video model.
#[derive(Clone)]
pub struct GoogleVideoConfig {
    /// Provider identifier (e.g. "google.generative-ai.video").
    pub provider: String,
    /// Base URL (e.g. "https://generativelanguage.googleapis.com/v1beta").
    pub base_url: String,
    /// Headers (must include x-goog-api-key).
    pub headers: HashMap<String, String>,
    /// Custom fetch function (for testing / middleware).
    pub fetch: Option<
        Arc<
            dyn Fn(reqwest::Request) -> BoxFuture<'static, reqwest::Result<reqwest::Response>>
                + Send
                + Sync,
        >,
    >,
}

/// A Google Video model (Veo).
pub struct GoogleVideoModel {
    model_id: String,
    config: GoogleVideoConfig,
}

impl GoogleVideoModel {
    pub fn new(model_id: String, config: GoogleVideoConfig) -> Self {
        Self { model_id, config }
    }

    /// Extract Google-specific options from provider_options.
    fn extract_options(options: &VideoCallOptions) -> GoogleVideoOptions {
        options
            .provider_options
            .get("google")
            .and_then(|obj| {
                let val = serde_json::to_value(obj).ok()?;
                serde_json::from_value::<GoogleVideoOptions>(val).ok()
            })
            .unwrap_or_default()
    }

    /// Poll for a long-running operation result.
    async fn poll_operation(&self, operation_name: &str) -> Result<Value, Error> {
        for _ in 0..MAX_POLL_ATTEMPTS {
            tokio::time::sleep(tokio::time::Duration::from_millis(POLL_INTERVAL_MS)).await;

            // GET the operation status
            let url = format!("{}/{}", self.config.base_url, operation_name);

            let client = reqwest::Client::new();
            let mut req_builder = client.get(&url);
            for (k, v) in &self.config.headers {
                req_builder = req_builder.header(k, v);
            }

            let response = req_builder.send().await.map_err(|e| Error::Other {
                message: format!("Failed to poll operation: {e}"),
            })?;

            if !response.status().is_success() {
                let body_text = response.text().await.unwrap_or_default();
                let body_val: Value = serde_json::from_str(&body_text).unwrap_or(json!({}));
                return Err(parse_google_error(body_val));
            }

            let body: Value = response.json().await.map_err(|e| Error::Other {
                message: format!("Failed to parse poll response: {e}"),
            })?;

            // Check if the operation is done
            if body.get("done").and_then(|v| v.as_bool()).unwrap_or(false) {
                // Check for error in the response
                if let Some(error) = body.get("error") {
                    return Err(parse_google_error(error.clone()));
                }

                return body.get("response").cloned().ok_or_else(|| Error::Other {
                    message: "Operation done but no 'response' field".into(),
                });
            }
        }

        Err(Error::Other {
            message: format!(
                "Video generation timed out after {} seconds",
                (POLL_INTERVAL_MS * MAX_POLL_ATTEMPTS as u64) / 1000
            ),
        })
    }
}

#[async_trait]
impl VideoModelV4 for GoogleVideoModel {
    fn provider(&self) -> &str {
        &self.config.provider
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }

    async fn do_generate(
        &self,
        options: &VideoCallOptions,
    ) -> Result<VideoGenerateResult, Error> {
            let google_opts = Self::extract_options(options);
            let prompt = options.prompt.as_deref().unwrap_or("");

            // Build instances
            let mut instance = serde_json::Map::new();
            instance.insert("prompt".to_string(), json!(prompt));

            // Add image input if provided (image-to-video)
            if let Some(ref image) = options.image {
                match image {
                    VideoImageInput::Base64 {
                        data, media_type, ..
                    } => {
                        instance.insert(
                            "image".to_string(),
                            json!({
                                "bytesBase64Encoded": data,
                                "mimeType": media_type,
                            }),
                        );
                    }
                    VideoImageInput::Url { url } => {
                        instance.insert(
                            "image".to_string(),
                            json!({ "imageUri": url }),
                        );
                    }
                }
            }

            // Build parameters
            let mut parameters = serde_json::Map::new();

            // Aspect ratio: provider option > call option
            if let Some(ref ar) = google_opts
                .aspect_ratio
                .as_ref()
                .or(options.aspect_ratio.as_ref())
            {
                parameters.insert("aspectRatio".to_string(), json!(ar));
            }

            // Duration: provider option > call option
            if let Some(dur) = google_opts
                .duration_seconds
                .map(|d| d as f64)
                .or(options.duration_seconds)
            {
                parameters.insert("durationSeconds".to_string(), json!(dur));
            }

            if let Some(ref person_gen) = google_opts.person_generation {
                parameters.insert("personGeneration".to_string(), json!(person_gen));
            }

            let n = google_opts.number_of_videos.unwrap_or(1);
            parameters.insert("sampleCount".to_string(), json!(n));

            if let Some(seed) = options.seed {
                parameters.insert("seed".to_string(), json!(seed));
            }

            let body = json!({
                "instances": [Value::Object(instance)],
                "parameters": parameters,
            });

            let url = format!(
                "{}/models/{}:predictLongRunning",
                self.config.base_url, self.model_id
            );

            let response_handler = create_json_response_handler(|v: Value| Ok(v));
            let error_handler = create_json_error_response_handler(parse_google_error);

            // Start the long-running operation
            let operation = post_json_to_api(PostJsonOptions {
                url,
                headers: Some(self.config.headers.clone()),
                body,
                successful_response_handler: response_handler,
                failed_response_handler: error_handler,
                fetch: self.config.fetch.clone(),
                retry: None,
                cancellation_token: None,
            })
            .await?;

            // Extract operation name for polling
            let operation_name = operation
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Other {
                    message: "No 'name' field in operation response".into(),
                })?;

            // Poll until done
            let result = self.poll_operation(operation_name).await?;

            // Parse the video from the response
            // Response contains "generateVideoResponse" -> "generatedSamples" array
            let samples = result
                .get("generateVideoResponse")
                .or_else(|| result.get("generatedSamples"))
                .and_then(|v| {
                    // Could be {"generatedSamples": [...]} or directly an array
                    v.get("generatedSamples")
                        .and_then(|s| s.as_array())
                        .or_else(|| v.as_array())
                })
                .ok_or_else(|| Error::Other {
                    message: "No generated samples in video response".into(),
                })?;

            // Return the first video (most common use case)
            let first = samples.first().ok_or_else(|| Error::Other {
                message: "Empty generated samples array".into(),
            })?;

            let video = if let Some(uri) = first.get("video").and_then(|v| v.get("uri")).and_then(|v| v.as_str()) {
                VideoData::Url {
                    url: uri.to_string(),
                }
            } else if let Some(b64) = first
                .get("video")
                .and_then(|v| v.get("bytesBase64Encoded"))
                .and_then(|v| v.as_str())
            {
                VideoData::Base64 {
                    data: b64.to_string(),
                    media_type: "video/mp4".to_string(),
                }
            } else {
                return Err(Error::Other {
                    message: "Cannot extract video data from response".into(),
                });
            };

            Ok(VideoGenerateResult {
                video,
                warnings: vec![],
                provider_metadata: None,
                response: VideoResponseMetadata {
                    timestamp: chrono::Utc::now(),
                    model_id: self.model_id.clone(),
                    headers: None,
                },
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_google_video_model_metadata() {
        let config = GoogleVideoConfig {
            provider: "google.generative-ai.video".into(),
            base_url: "https://generativelanguage.googleapis.com/v1beta".into(),
            headers: HashMap::new(),
            fetch: None,
        };
        let model = GoogleVideoModel::new("veo-2.0-generate-001".into(), config);
        assert_eq!(model.provider(), "google.generative-ai.video");
        assert_eq!(model.model_id(), "veo-2.0-generate-001");
    }
}
