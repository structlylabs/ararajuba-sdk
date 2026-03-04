//! OpenAI embedding model — thin wrapper around the compatible base.
//!
//! OpenAI-specific options (`dimensions`, `user`) should be passed via
//! `provider_options` under the `"openai"` key.  The compatible base merges
//! them into the request body automatically.

use async_trait::async_trait;
use ararajuba_provider::embedding_model::v4::call_options::EmbeddingCallOptions;
use ararajuba_provider::embedding_model::v4::embedding_model_v4::EmbeddingModelV4;
use ararajuba_provider::embedding_model::v4::result::EmbeddingResult;
use ararajuba_provider::errors::Error;
use ararajuba_openai_compatible::{EmbeddingModelConfig, OpenAICompatibleEmbeddingModel};

/// OpenAI-specific provider options for embedding models.
///
/// Place under `provider_options["openai"]` as JSON:
/// ```json
/// { "dimensions": 512, "user": "user-abc" }
/// ```
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct OpenAIEmbeddingOptions {
    /// Custom dimensions for the embedding (text-embedding-3-* only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<u32>,
    /// End-user identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

/// An OpenAI embedding model.
pub struct OpenAIEmbeddingModel {
    inner: OpenAICompatibleEmbeddingModel,
}

impl OpenAIEmbeddingModel {
    pub fn new(model_id: String, config: EmbeddingModelConfig) -> Self {
        Self {
            inner: OpenAICompatibleEmbeddingModel::new(model_id, config, Some(2048)),
        }
    }
}

#[async_trait]
impl EmbeddingModelV4 for OpenAIEmbeddingModel {
    fn provider(&self) -> &str {
        self.inner.provider()
    }

    fn model_id(&self) -> &str {
        self.inner.model_id()
    }

    fn max_embeddings_per_call(&self) -> Option<usize> {
        self.inner.max_embeddings_per_call()
    }

    fn supports_parallel_calls(&self) -> bool {
        true
    }

    async fn do_embed(
        &self,
        options: &EmbeddingCallOptions,
    ) -> Result<EmbeddingResult, Error> {
        // The compatible base merges provider_options into the request body,
        // so dimensions and user flow through automatically.
        self.inner.do_embed(options).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_openai_embedding_model_metadata() {
        let config = EmbeddingModelConfig {
            provider: "openai.embedding".into(),
            url: "https://api.openai.com/v1/embeddings".into(),
            headers: HashMap::new(),
            fetch: None,
        };
        let model = OpenAIEmbeddingModel::new("text-embedding-3-small".into(), config);
        assert_eq!(model.provider(), "openai.embedding");
        assert_eq!(model.model_id(), "text-embedding-3-small");
        assert_eq!(model.max_embeddings_per_call(), Some(2048));
    }
}
