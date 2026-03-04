//! OpenAI-compatible embedding model — implements `EmbeddingModelV4`.

use crate::error::parse_openai_compatible_error;
use async_trait::async_trait;
use ararajuba_provider::embedding_model::v4::call_options::EmbeddingCallOptions;
use ararajuba_provider::embedding_model::v4::embedding_model_v4::EmbeddingModelV4;
use ararajuba_provider::embedding_model::v4::result::{
    Embedding, EmbeddingResponseMetadata, EmbeddingResult, EmbeddingUsage,
};
use ararajuba_provider::errors::Error;
use ararajuba_provider_utils::http::post_to_api::{post_json_to_api, PostJsonOptions};
use ararajuba_provider_utils::http::response_handler::{
    create_json_error_response_handler, create_json_response_handler,
};
use futures::future::BoxFuture;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

/// Configuration for the embedding model.
#[derive(Clone)]
pub struct EmbeddingModelConfig {
    /// Provider identifier (e.g., "openai.embedding").
    pub provider: String,
    /// Full URL for the embeddings endpoint.
    pub url: String,
    /// Headers to send with each request.
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

/// An OpenAI-compatible embedding model.
pub struct OpenAICompatibleEmbeddingModel {
    model_id: String,
    config: EmbeddingModelConfig,
    max_embeddings_per_call: Option<usize>,
}

impl OpenAICompatibleEmbeddingModel {
    pub fn new(
        model_id: String,
        config: EmbeddingModelConfig,
        max_embeddings_per_call: Option<usize>,
    ) -> Self {
        Self {
            model_id,
            config,
            max_embeddings_per_call,
        }
    }
}

#[async_trait]
impl EmbeddingModelV4 for OpenAICompatibleEmbeddingModel {
    fn provider(&self) -> &str {
        &self.config.provider
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }

    fn max_embeddings_per_call(&self) -> Option<usize> {
        self.max_embeddings_per_call
    }

    fn supports_parallel_calls(&self) -> bool {
        true
    }

    async fn do_embed(
        &self,
        options: &EmbeddingCallOptions,
    ) -> Result<EmbeddingResult, Error> {
            let mut body = json!({
                "model": self.model_id,
                "input": options.values,
                "encoding_format": "float",
            });

            // Inject provider-specific options into the body.
            // Providers set "openai" (or their own key) in provider_options with
            // fields like "dimensions" and "user".
            if let Some(ref po) = options.provider_options {
                if let Some(obj) = body.as_object_mut() {
                    for (_provider_key, opts) in po {
                        for (k, v) in opts {
                            if !v.is_null() {
                                obj.insert(k.clone(), v.clone());
                            }
                        }
                    }
                }
            }

            let response_handler = create_json_response_handler(|v: Value| Ok(v));
            let error_handler = create_json_error_response_handler(parse_openai_compatible_error);

            let raw = post_json_to_api(PostJsonOptions {
                url: self.config.url.clone(),
                headers: Some(self.config.headers.clone()),
                body,
                successful_response_handler: response_handler,
                failed_response_handler: error_handler,
                fetch: self.config.fetch.clone(),
                retry: None,
                cancellation_token: None,
            })
            .await?;

            // Parse embeddings from response
            let data = raw
                .get("data")
                .and_then(|v| v.as_array())
                .ok_or_else(|| Error::Other {
                    message: "No 'data' array in embedding response".into(),
                })?;

            // Sort by index (API may return out of order)
            let mut indexed: Vec<(usize, &Value)> = data
                .iter()
                .filter_map(|item| {
                    let index = item.get("index")?.as_u64()? as usize;
                    Some((index, item))
                })
                .collect();
            indexed.sort_by_key(|(i, _)| *i);

            let embeddings: Vec<Embedding> = indexed
                .iter()
                .map(|(_, item)| {
                    item.get("embedding")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|n| n.as_f64())
                                .collect::<Vec<f64>>()
                        })
                        .unwrap_or_default()
                })
                .collect();

            // Parse usage
            let usage = raw.get("usage").map(|u| {
                let tokens = u
                    .get("prompt_tokens")
                    .or_else(|| u.get("total_tokens"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                EmbeddingUsage { tokens }
            });

            Ok(EmbeddingResult {
                embeddings,
                usage,
                provider_metadata: None,
                response: Some(EmbeddingResponseMetadata {
                    headers: None,
                    body: Some(raw),
                }),
                warnings: vec![],
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_model_metadata() {
        let config = EmbeddingModelConfig {
            provider: "test.embedding".into(),
            url: "https://api.example.com/v1/embeddings".into(),
            headers: HashMap::new(),
            fetch: None,
        };
        let model = OpenAICompatibleEmbeddingModel::new(
            "text-embedding-3-small".into(),
            config,
            Some(2048),
        );
        assert_eq!(model.provider(), "test.embedding");
        assert_eq!(model.model_id(), "text-embedding-3-small");
        assert_eq!(model.max_embeddings_per_call(), Some(2048));
        assert!(model.supports_parallel_calls());
    }
}
