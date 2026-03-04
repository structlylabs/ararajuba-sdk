//! Google Image model (Imagen) — implements `ImageModel`.
//!
//! Endpoint: `POST /v1beta/models/{model}:predict`
//! Models: imagen-4.0-generate-001, imagen-4.0-ultra-generate-001, imagen-4.0-fast-generate-001

use crate::error::parse_google_error;
use crate::image::google_image_options::GoogleImageOptions;
use async_trait::async_trait;
use ararajuba_provider::errors::Error;
use ararajuba_provider::image_model::v4::{
    ImageCallOptions, ImageGenerateResult, ImageResponseMetadata,
};
use ararajuba_provider::image_model::v4::image_model_v4::ImageModelV4;
use ararajuba_provider_utils::http::post_to_api::{post_json_to_api, PostJsonOptions};
use ararajuba_provider_utils::http::response_handler::{
    create_json_error_response_handler, create_json_response_handler,
};
use futures::future::BoxFuture;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

/// Configuration for the Google image model.
#[derive(Clone)]
pub struct GoogleImageConfig {
    /// Provider identifier (e.g. "google.generative-ai.image").
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

/// A Google Image model (Imagen).
pub struct GoogleImageModel {
    model_id: String,
    config: GoogleImageConfig,
}

impl GoogleImageModel {
    pub fn new(model_id: String, config: GoogleImageConfig) -> Self {
        Self { model_id, config }
    }

    /// Extract Google-specific options from provider_options.
    fn extract_options(options: &ImageCallOptions) -> GoogleImageOptions {
        options
            .provider_options
            .get("google")
            .and_then(|obj| {
                let val = serde_json::to_value(obj).ok()?;
                serde_json::from_value::<GoogleImageOptions>(val).ok()
            })
            .unwrap_or_default()
    }
}

#[async_trait]
impl ImageModelV4 for GoogleImageModel {
    fn provider(&self) -> &str {
        &self.config.provider
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }

    fn max_images_per_call(&self) -> Option<usize> {
        Some(4)
    }

    async fn do_generate(
        &self,
        options: &ImageCallOptions,
    ) -> Result<ImageGenerateResult, Error> {
            let google_opts = Self::extract_options(options);
            let prompt = options.prompt.as_deref().unwrap_or("");

            // Number of images: provider option > call option > default 1
            let n = google_opts.number_of_images.unwrap_or(options.n);

            // Build the request body per the Google Imagen API.
            let instances = json!({ "prompt": prompt });
            // Some Imagen models also accept image input; add if provided
            // (not yet implemented — text-to-image only for now).

            let mut parameters = serde_json::Map::new();
            parameters.insert("sampleCount".to_string(), json!(n));

            // Aspect ratio: provider option > call option
            if let Some(ref ar) = google_opts.aspect_ratio.as_ref().or(options.aspect_ratio.as_ref())
            {
                parameters.insert("aspectRatio".to_string(), json!(ar));
            }

            if let Some(ref size) = google_opts.image_size {
                parameters.insert("outputOptions".to_string(), json!({
                    "mimeType": "image/png",
                    "compressionQuality": 100,
                }));
                // Map known sizes to pixel dimensions
                let pixels = match size.as_str() {
                    "512" => "512x512",
                    "1K" => "1024x1024",
                    "2K" => "2048x2048",
                    "4K" => "4096x4096",
                    other => other,
                };
                parameters.insert("sampleImageSize".to_string(), json!(pixels));
            }

            if let Some(ref negative_prompt) = google_opts.negative_prompt {
                parameters.insert("negativePrompt".to_string(), json!(negative_prompt));
            }

            if let Some(ref person_gen) = google_opts.person_generation {
                parameters.insert(
                    "personGeneration".to_string(),
                    json!(person_gen),
                );
            }

            if let Some(seed) = options.seed {
                parameters.insert("seed".to_string(), json!(seed));
            }

            let body = json!({
                "instances": [instances],
                "parameters": parameters,
            });

            let url = format!("{}/models/{}:predict", self.config.base_url, self.model_id);

            let response_handler = create_json_response_handler(|v: Value| Ok(v));
            let error_handler = create_json_error_response_handler(parse_google_error);

            let raw = post_json_to_api(PostJsonOptions {
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

            // Parse predictions — each prediction has a "bytesBase64Encoded" field.
            let predictions = raw
                .get("predictions")
                .and_then(|v| v.as_array())
                .ok_or_else(|| Error::Other {
                    message: "No 'predictions' array in Google image response".into(),
                })?;

            let images: Vec<String> = predictions
                .iter()
                .filter_map(|pred| {
                    pred.get("bytesBase64Encoded")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
                .collect();

            if images.is_empty() {
                return Err(Error::Other {
                    message: "No images returned in Google predictions".into(),
                });
            }

            Ok(ImageGenerateResult {
                images,
                warnings: vec![],
                provider_metadata: None,
                response: ImageResponseMetadata {
                    timestamp: chrono::Utc::now(),
                    model_id: self.model_id.clone(),
                    headers: None,
                },
                usage: None,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_google_image_model_metadata() {
        let config = GoogleImageConfig {
            provider: "google.generative-ai.image".into(),
            base_url: "https://generativelanguage.googleapis.com/v1beta".into(),
            headers: HashMap::new(),
            fetch: None,
        };
        let model = GoogleImageModel::new("imagen-4.0-generate-001".into(), config);
        assert_eq!(model.provider(), "google.generative-ai.image");
        assert_eq!(model.model_id(), "imagen-4.0-generate-001");
        assert_eq!(model.max_images_per_call(), Some(4));
    }
}
