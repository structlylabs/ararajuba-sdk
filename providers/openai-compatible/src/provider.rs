//! OpenAI-compatible provider factory.
//!
//! Creates a `Provider` that produces chat and embedding models for any
//! OpenAI-compatible API (xAI, Together AI, Fireworks, etc.).

use crate::chat::{ChatModelConfig, OpenAICompatibleChatLanguageModel};
use crate::embedding::{EmbeddingModelConfig, OpenAICompatibleEmbeddingModel};
use ararajuba_provider::embedding_model::v4::embedding_model_v4::EmbeddingModelV4;
use ararajuba_provider::language_model::v4::language_model_v4::LanguageModelV4;
use ararajuba_provider::provider::Provider;
use futures::future::BoxFuture;
use std::collections::HashMap;
use std::sync::Arc;

/// Settings for creating an OpenAI-compatible provider.
#[derive(Clone)]
pub struct OpenAICompatibleSettings {
    /// The provider name (e.g., "xai", "together").
    pub name: String,
    /// Base URL for the API (e.g., "https://api.x.ai/v1").
    pub base_url: String,
    /// Default headers for all requests (typically includes authorization).
    pub headers: HashMap<String, String>,
    /// Whether the API supports structured outputs (json_schema response format).
    pub supports_structured_outputs: bool,
    /// Whether to request usage information in streaming responses.
    pub include_usage: bool,
    /// Maximum embeddings per call (defaults to 2048 if not set).
    pub max_embeddings_per_call: Option<usize>,
    /// Custom fetch function.
    pub fetch: Option<
        Arc<
            dyn Fn(reqwest::Request) -> BoxFuture<'static, reqwest::Result<reqwest::Response>>
                + Send
                + Sync,
        >,
    >,
}

/// An OpenAI-compatible provider.
pub struct OpenAICompatibleProvider {
    settings: OpenAICompatibleSettings,
}

impl OpenAICompatibleProvider {
    pub fn new(settings: OpenAICompatibleSettings) -> Self {
        Self { settings }
    }

    fn chat_url(&self) -> String {
        format!("{}/chat/completions", self.settings.base_url.trim_end_matches('/'))
    }

    fn embeddings_url(&self) -> String {
        format!("{}/embeddings", self.settings.base_url.trim_end_matches('/'))
    }
}

impl Provider for OpenAICompatibleProvider {
    fn language_model_v4(&self, model_id: &str) -> Option<Box<dyn LanguageModelV4>> {
        let config = ChatModelConfig {
            provider: format!("{}.chat", self.settings.name),
            url: self.chat_url(),
            headers: self.settings.headers.clone(),
            include_usage: self.settings.include_usage,
            supports_structured_outputs: self.settings.supports_structured_outputs,
            fetch: self.settings.fetch.clone(),
        };
        Some(Box::new(OpenAICompatibleChatLanguageModel::new(
            model_id.to_string(),
            config,
        )))
    }

    fn embedding_model_v4(&self, model_id: &str) -> Option<Box<dyn EmbeddingModelV4>> {
        let config = EmbeddingModelConfig {
            provider: format!("{}.embedding", self.settings.name),
            url: self.embeddings_url(),
            headers: self.settings.headers.clone(),
            fetch: self.settings.fetch.clone(),
        };
        Some(Box::new(OpenAICompatibleEmbeddingModel::new(
            model_id.to_string(),
            config,
            self.settings.max_embeddings_per_call.or(Some(2048)),
        )))
    }
}

/// Create an OpenAI-compatible provider from settings.
pub fn create_openai_compatible(settings: OpenAICompatibleSettings) -> OpenAICompatibleProvider {
    OpenAICompatibleProvider::new(settings)
}

/// Convenience: create settings with an API key in the Authorization header.
pub fn openai_compatible_settings(
    name: impl Into<String>,
    base_url: impl Into<String>,
    api_key: impl Into<String>,
) -> OpenAICompatibleSettings {
    let mut headers = HashMap::new();
    headers.insert(
        "Authorization".to_string(),
        format!("Bearer {}", api_key.into()),
    );
    OpenAICompatibleSettings {
        name: name.into(),
        base_url: base_url.into(),
        headers,
        supports_structured_outputs: false,
        include_usage: true,
        max_embeddings_per_call: Some(2048),
        fetch: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creates_language_model() {
        let provider = create_openai_compatible(openai_compatible_settings(
            "test",
            "https://api.example.com/v1",
            "sk-test",
        ));
        let model = provider.language_model_v4("gpt-4").unwrap();
        assert_eq!(model.provider(), "test.chat");
        assert_eq!(model.model_id(), "gpt-4");
    }

    #[test]
    fn test_provider_creates_embedding_model() {
        let provider = create_openai_compatible(openai_compatible_settings(
            "test",
            "https://api.example.com/v1",
            "sk-test",
        ));
        let model = provider.embedding_model_v4("text-embedding-3-small").unwrap();
        assert_eq!(model.provider(), "test.embedding");
        assert_eq!(model.model_id(), "text-embedding-3-small");
    }

    #[test]
    fn test_url_construction() {
        let provider = create_openai_compatible(openai_compatible_settings(
            "test",
            "https://api.example.com/v1/",
            "sk-test",
        ));
        assert_eq!(provider.chat_url(), "https://api.example.com/v1/chat/completions");
        assert_eq!(provider.embeddings_url(), "https://api.example.com/v1/embeddings");
    }
}
