//! OpenAI image generation model — DALL·E 2, DALL·E 3, gpt-image-1.
//!
//! Implements `ImageModel` against the OpenAI Images API.
//!
//! **Endpoint:** `POST /v1/images/generations`
//!
//! **Supported models:** dall-e-2, dall-e-3, gpt-image-1

use crate::image::openai_image_options::parse_openai_image_options;
use async_trait::async_trait;
use ararajuba_provider::errors::Error;
use ararajuba_provider::image_model::v4::{
    ImageCallOptions, ImageGenerateResult, ImageResponseMetadata, ImageUsage,
};
use ararajuba_provider::image_model::v4::image_model_v4::ImageModelV4;
use ararajuba_provider::shared::Warning;
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;

/// An OpenAI image generation model.
pub struct OpenAIImageModel {
    model_id: String,
    provider_name: String,
    url: String,
    headers: HashMap<String, String>,
    client: Client,
}

impl OpenAIImageModel {
    pub fn new(
        model_id: String,
        provider_name: String,
        base_url: &str,
        headers: HashMap<String, String>,
    ) -> Self {
        Self {
            model_id,
            provider_name,
            url: format!("{}/images/generations", base_url.trim_end_matches('/')),
            headers,
            client: Client::new(),
        }
    }
}

#[async_trait]
impl ImageModelV4 for OpenAIImageModel {
    fn provider(&self) -> &str {
        &self.provider_name
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }

    fn max_images_per_call(&self) -> Option<usize> {
        match self.model_id.as_str() {
            "dall-e-3" => Some(1),
            "dall-e-2" => Some(10),
            "gpt-image-1" => Some(1),
            _ => Some(1),
        }
    }

    async fn do_generate(
        &self,
        options: &ImageCallOptions,
    ) -> Result<ImageGenerateResult, Error> {
            let openai_opts = parse_openai_image_options(&options.provider_options, &self.provider_name);
            let mut warnings = Vec::new();

            // Build request body
            let mut body = json!({
                "model": self.model_id,
                "n": options.n,
            });

            if let Some(ref prompt) = options.prompt {
                body["prompt"] = json!(prompt);
            }

            // Size
            if let Some(ref size) = options.size {
                body["size"] = json!(size);
            }

            // Quality (DALL·E 3 specific)
            if let Some(ref quality) = openai_opts.quality {
                body["quality"] = json!(quality);
            }

            // Style (DALL·E 3 specific)
            if let Some(ref style) = openai_opts.style {
                body["style"] = json!(style);
            }

            // Response format
            let response_fmt = openai_opts
                .response_format
                .as_deref()
                .unwrap_or("b64_json");
            body["response_format"] = json!(response_fmt);

            // User
            if let Some(ref user) = openai_opts.user {
                body["user"] = json!(user);
            }

            // gpt-image-1 specific options
            if let Some(compression) = openai_opts.output_compression {
                body["output_compression"] = json!(compression);
            }
            if let Some(ref output_format) = openai_opts.output_format {
                body["output_format"] = json!(output_format);
            }
            if let Some(ref moderation) = openai_opts.moderation {
                body["moderation"] = json!(moderation);
            }
            if let Some(ref background) = openai_opts.background {
                body["background"] = json!(background);
            }

            // Unsupported options
            if options.seed.is_some() {
                warnings.push(Warning::Unsupported {
                    feature: "seed".into(),
                    details: Some("Not supported by OpenAI image generation".into()),
                });
            }
            if options.aspect_ratio.is_some() {
                warnings.push(Warning::Unsupported {
                    feature: "aspect_ratio".into(),
                    details: Some("Use 'size' instead for OpenAI image models".into()),
                });
            }

            // Build request
            let mut request = self.client.post(&self.url);
            for (k, v) in &self.headers {
                request = request.header(k, v);
            }
            if let Some(ref extra_headers) = options.headers {
                for (k, v) in extra_headers {
                    request = request.header(k, v);
                }
            }

            let response = request
                .json(&body)
                .send()
                .await
                .map_err(|e| Error::Http {
                    message: e.to_string(),
                })?;

            let status = response.status();
            let response_text = response
                .text()
                .await
                .map_err(|e| Error::Http {
                    message: e.to_string(),
                })?;

            if !status.is_success() {
                return Err(Error::ApiCallError {
                    message: format!("Image generation failed: {}", status),
                    url: self.url.clone(),
                    status_code: Some(status.as_u16()),
                    response_body: Some(response_text),
                    is_retryable: status.as_u16() == 429 || status.as_u16() >= 500,
                    data: None,
                });
            }

            let raw: Value =
                serde_json::from_str(&response_text).map_err(|e| Error::JsonParse {
                    message: e.to_string(),
                    text: response_text.clone(),
                })?;

            // Parse images from response
            let data = raw
                .get("data")
                .and_then(|d| d.as_array())
                .ok_or_else(|| Error::InvalidResponseData {
                    message: "No 'data' array in image response".into(),
                })?;

            let images: Vec<String> = data
                .iter()
                .filter_map(|item| {
                    if response_fmt == "b64_json" {
                        item.get("b64_json").and_then(|v| v.as_str()).map(|s| s.to_string())
                    } else {
                        item.get("url").and_then(|v| v.as_str()).map(|s| s.to_string())
                    }
                })
                .collect();

            // Parse usage if present (gpt-image-1)
            let usage = raw.get("usage").map(|u| ImageUsage {
                input_tokens: u.get("input_tokens").and_then(|v| v.as_u64()),
                output_tokens: u.get("output_tokens").and_then(|v| v.as_u64()),
                total_tokens: u.get("total_tokens").and_then(|v| v.as_u64()),
            });

            let created = raw
                .get("created")
                .and_then(|v| v.as_i64())
                .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
                .unwrap_or_else(chrono::Utc::now);

            Ok(ImageGenerateResult {
                images,
                warnings,
                provider_metadata: None,
                response: ImageResponseMetadata {
                    timestamp: created,
                    model_id: self.model_id.clone(),
                    headers: None,
                },
                usage,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_image_model_metadata() {
        let model = OpenAIImageModel::new(
            "dall-e-3".into(),
            "openai.image".into(),
            "https://api.openai.com/v1",
            HashMap::new(),
        );
        assert_eq!(model.provider(), "openai.image");
        assert_eq!(model.model_id(), "dall-e-3");
        assert_eq!(model.max_images_per_call(), Some(1));
    }

    #[test]
    fn test_dalle2_max_images() {
        let model = OpenAIImageModel::new(
            "dall-e-2".into(),
            "openai.image".into(),
            "https://api.openai.com/v1",
            HashMap::new(),
        );
        assert_eq!(model.max_images_per_call(), Some(10));
    }
}
