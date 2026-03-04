//! Google Generative AI embedding model.

use async_trait::async_trait;
use ararajuba_provider::embedding_model::v4::call_options::EmbeddingCallOptions;
use ararajuba_provider::embedding_model::v4::embedding_model_v4::EmbeddingModelV4;
use ararajuba_provider::embedding_model::v4::result::{
    EmbeddingResult, EmbeddingResponseMetadata,
};
use ararajuba_provider::errors::Error;
use ararajuba_provider_utils::http::post_to_api::{post_json_to_api, PostJsonOptions};
use ararajuba_provider_utils::http::response_handler::{
    create_json_error_response_handler, create_json_response_handler,
};
use crate::error::parse_google_error;
use futures::future::BoxFuture;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

/// Configuration for the Google Generative AI embedding model.
#[derive(Clone)]
pub struct GoogleEmbeddingConfig {
    pub provider: String,
    pub base_url: String,
    pub headers: HashMap<String, String>,
    pub fetch: Option<
        Arc<
            dyn Fn(reqwest::Request) -> BoxFuture<'static, reqwest::Result<reqwest::Response>>
                + Send
                + Sync,
        >,
    >,
}

/// A Google Generative AI embedding model.
pub struct GoogleEmbeddingModel {
    model_id: String,
    config: GoogleEmbeddingConfig,
}

impl GoogleEmbeddingModel {
    pub fn new(model_id: String, config: GoogleEmbeddingConfig) -> Self {
        Self { model_id, config }
    }

    fn model_path(&self) -> String {
        if self.model_id.contains('/') {
            self.model_id.clone()
        } else {
            format!("models/{}", self.model_id)
        }
    }
}

#[async_trait]
impl EmbeddingModelV4 for GoogleEmbeddingModel {
    fn provider(&self) -> &str {
        &self.config.provider
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }

    fn max_embeddings_per_call(&self) -> Option<usize> {
        Some(2048)
    }

    fn supports_parallel_calls(&self) -> bool {
        true
    }

    async fn do_embed(
        &self,
        options: &EmbeddingCallOptions,
    ) -> Result<EmbeddingResult, Error> {
            let model = format!("models/{}", self.model_id);

            // Use batch embed endpoint
            let requests: Vec<Value> = options.values
                .iter()
                .map(|text| {
                    json!({
                        "model": model,
                        "content": {
                            "role": "user",
                            "parts": [{ "text": text }],
                        }
                    })
                })
                .collect();

            let body = json!({
                "requests": requests,
            });

            let url = format!(
                "{}/{}:batchEmbedContents",
                self.config.base_url,
                self.model_path()
            );

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

            // Parse embeddings
            let embeddings_arr = raw
                .get("embeddings")
                .and_then(|v| v.as_array())
                .ok_or_else(|| Error::InvalidResponseData {
                    message: "Missing embeddings array".into(),
                })?;

            let mut embeddings = Vec::with_capacity(embeddings_arr.len());
            for emb in embeddings_arr {
                let values = emb
                    .get("values")
                    .and_then(|v| v.as_array())
                    .unwrap_or(&Vec::new())
                    .iter()
                    .filter_map(|v| v.as_f64())
                    .collect::<Vec<f64>>();
                embeddings.push(values);
            }

            Ok(EmbeddingResult {
                embeddings,
                usage: None,
                provider_metadata: None,
                response: Some(EmbeddingResponseMetadata {
                    headers: None,
                    body: Some(raw),
                }),
                warnings: Vec::new(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_path() {
        let model = GoogleEmbeddingModel::new(
            "gemini-embedding-001".into(),
            GoogleEmbeddingConfig {
                provider: "google.embedding".into(),
                base_url: "https://example.com/v1beta".into(),
                headers: HashMap::new(),
                fetch: None,
            },
        );
        assert_eq!(model.model_path(), "models/gemini-embedding-001");
    }

    #[test]
    fn test_custom_model_path() {
        let model = GoogleEmbeddingModel::new(
            "publishers/google/models/gemini-embedding-001".into(),
            GoogleEmbeddingConfig {
                provider: "google.embedding".into(),
                base_url: "https://example.com/v1beta".into(),
                headers: HashMap::new(),
                fetch: None,
            },
        );
        // Contains '/', use as-is
        assert_eq!(model.model_path(), "publishers/google/models/gemini-embedding-001");
    }
}
