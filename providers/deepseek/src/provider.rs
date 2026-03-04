//! DeepSeek provider factory.

use crate::chat::chat_model::DeepSeekChatLanguageModel;
use ararajuba_provider::language_model::v4::language_model_v4::LanguageModelV4;
use ararajuba_provider::provider::Provider;
use ararajuba_openai_compatible::ChatModelConfig;
use std::collections::HashMap;

/// Default DeepSeek API base URL.
const DEFAULT_BASE_URL: &str = "https://api.deepseek.com";

/// Settings for creating a DeepSeek provider.
#[derive(Clone, Default)]
pub struct DeepSeekSettings {
    /// DeepSeek API key. Falls back to `DEEPSEEK_API_KEY` env var.
    pub api_key: Option<String>,
    /// Custom base URL.
    pub base_url: Option<String>,
    /// Additional headers.
    pub headers: Option<HashMap<String, String>>,
}

/// A DeepSeek provider.
pub struct DeepSeekProvider {
    name: String,
    base_url: String,
    headers: HashMap<String, String>,
}

impl DeepSeekProvider {
    fn new(settings: DeepSeekSettings) -> Self {
        let api_key = settings
            .api_key
            .or_else(|| std::env::var("DEEPSEEK_API_KEY").ok())
            .unwrap_or_default();

        let base_url = settings
            .base_url
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), format!("Bearer {api_key}"));
        headers.insert("content-type".to_string(), "application/json".to_string());

        if let Some(extra) = settings.headers {
            headers.extend(extra);
        }

        Self {
            name: "deepseek".to_string(),
            base_url: base_url.trim_end_matches('/').to_string(),
            headers,
        }
    }
}

impl Provider for DeepSeekProvider {
    fn language_model_v4(&self, model_id: &str) -> Option<Box<dyn LanguageModelV4>> {
        let config = ChatModelConfig {
            provider: format!("{}.chat", self.name),
            url: format!("{}/chat/completions", self.base_url),
            headers: self.headers.clone(),
            include_usage: true,
            supports_structured_outputs: false,
            fetch: None,
        };
        Some(Box::new(DeepSeekChatLanguageModel::new(
            model_id.to_string(),
            config,
        )))
    }
}

/// Create a DeepSeek provider with the given settings.
pub fn create_deepseek(settings: DeepSeekSettings) -> DeepSeekProvider {
    DeepSeekProvider::new(settings)
}

/// Create a DeepSeek provider with default settings and get a language model.
pub fn deepseek(model_id: &str) -> Box<dyn LanguageModelV4> {
    let provider = create_deepseek(DeepSeekSettings::default());
    provider.language_model_v4(model_id).expect("language model")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_deepseek_language_model() {
        let provider = create_deepseek(DeepSeekSettings {
            api_key: Some("sk-test".into()),
            ..Default::default()
        });
        let model = provider.language_model_v4("deepseek-chat");
        assert!(model.is_some());

        let model = model.unwrap();
        assert_eq!(model.model_id(), "deepseek-chat");
        assert_eq!(model.provider(), "deepseek.chat");
    }

    #[test]
    fn test_deepseek_custom_base_url() {
        let provider = create_deepseek(DeepSeekSettings {
            api_key: Some("key".into()),
            base_url: Some("https://custom.deepseek.com".into()),
            ..Default::default()
        });
        assert_eq!(provider.base_url, "https://custom.deepseek.com");
    }

    #[test]
    fn test_deepseek_headers() {
        let provider = create_deepseek(DeepSeekSettings {
            api_key: Some("sk-xxx".into()),
            ..Default::default()
        });
        assert_eq!(
            provider.headers.get("Authorization"),
            Some(&"Bearer sk-xxx".to_string())
        );
    }
}
