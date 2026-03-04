//! Anthropic provider factory.

use crate::chat::chat_model::{AnthropicChatConfig, AnthropicMessagesLanguageModel};
use ararajuba_provider::language_model::v4::language_model_v4::LanguageModelV4;
use ararajuba_provider::provider::Provider;
use std::collections::HashMap;

/// Default Anthropic API base URL.
const DEFAULT_BASE_URL: &str = "https://api.anthropic.com/v1";

/// Default Anthropic API version.
const DEFAULT_API_VERSION: &str = "2023-06-01";

/// Settings for creating an Anthropic provider.
#[derive(Clone, Default)]
pub struct AnthropicSettings {
    /// Anthropic API key. Falls back to `ANTHROPIC_API_KEY` env var.
    pub api_key: Option<String>,
    /// Custom base URL. Falls back to `ANTHROPIC_BASE_URL` env var.
    pub base_url: Option<String>,
    /// Anthropic API version (default: "2023-06-01").
    pub api_version: Option<String>,
    /// Additional headers.
    pub headers: Option<HashMap<String, String>>,
}

/// An Anthropic provider.
pub struct AnthropicProvider {
    name: String,
    base_url: String,
    headers: HashMap<String, String>,
}

impl AnthropicProvider {
    fn new(settings: AnthropicSettings) -> Self {
        let api_key = settings
            .api_key
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
            .unwrap_or_default();

        let base_url = settings
            .base_url
            .or_else(|| std::env::var("ANTHROPIC_BASE_URL").ok())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

        let api_version = settings
            .api_version
            .unwrap_or_else(|| DEFAULT_API_VERSION.to_string());

        let mut headers = HashMap::new();
        // Anthropic uses x-api-key header (not Bearer)
        headers.insert("x-api-key".to_string(), api_key);
        headers.insert("anthropic-version".to_string(), api_version);
        headers.insert("content-type".to_string(), "application/json".to_string());

        if let Some(extra) = settings.headers {
            headers.extend(extra);
        }

        Self {
            name: "anthropic".to_string(),
            base_url: base_url.trim_end_matches('/').to_string(),
            headers,
        }
    }
}

impl Provider for AnthropicProvider {
    fn language_model_v4(&self, model_id: &str) -> Option<Box<dyn LanguageModelV4>> {
        let config = AnthropicChatConfig {
            provider: format!("{}.chat", self.name),
            url: format!("{}/messages", self.base_url),
            headers: self.headers.clone(),
            fetch: None,
        };
        Some(Box::new(AnthropicMessagesLanguageModel::new(
            model_id.to_string(),
            config,
        )))
    }
}

/// Create an Anthropic provider with the given settings.
pub fn create_anthropic(settings: AnthropicSettings) -> AnthropicProvider {
    AnthropicProvider::new(settings)
}

/// Create an Anthropic provider with default settings and get a language model.
pub fn anthropic(model_id: &str) -> Box<dyn LanguageModelV4> {
    let provider = create_anthropic(AnthropicSettings::default());
    provider.language_model_v4(model_id).expect("language model")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_anthropic_language_model() {
        let provider = create_anthropic(AnthropicSettings {
            api_key: Some("sk-ant-test".into()),
            ..Default::default()
        });
        let model = provider.language_model_v4("claude-sonnet-4-20250514");
        assert!(model.is_some());

        let model = model.unwrap();
        assert_eq!(model.model_id(), "claude-sonnet-4-20250514");
        assert_eq!(model.provider(), "anthropic.chat");
    }

    #[test]
    fn test_anthropic_no_embedding_model() {
        let provider = create_anthropic(AnthropicSettings {
            api_key: Some("sk-ant-test".into()),
            ..Default::default()
        });
        let model = provider.embedding_model_v4("some-model");
        assert!(model.is_none());
    }

    #[test]
    fn test_anthropic_custom_base_url() {
        let provider = create_anthropic(AnthropicSettings {
            api_key: Some("key".into()),
            base_url: Some("https://custom.api.com".into()),
            ..Default::default()
        });
        assert_eq!(provider.base_url, "https://custom.api.com");
    }

    #[test]
    fn test_anthropic_headers() {
        let provider = create_anthropic(AnthropicSettings {
            api_key: Some("sk-ant-xxx".into()),
            api_version: Some("2024-01-01".into()),
            ..Default::default()
        });
        assert_eq!(
            provider.headers.get("x-api-key"),
            Some(&"sk-ant-xxx".to_string())
        );
        assert_eq!(
            provider.headers.get("anthropic-version"),
            Some(&"2024-01-01".to_string())
        );
    }
}
