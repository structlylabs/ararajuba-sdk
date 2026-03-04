//! Google Generative AI (Gemini) provider factory.

use crate::chat::chat_model::{GoogleChatConfig, GoogleGenerativeAILanguageModel};
use crate::embedding::embedding_model::{GoogleEmbeddingConfig, GoogleEmbeddingModel};
use crate::image::google_image_model::{GoogleImageConfig, GoogleImageModel};
use crate::video::google_video_model::{GoogleVideoConfig, GoogleVideoModel};
use ararajuba_provider::embedding_model::v4::embedding_model_v4::EmbeddingModelV4;
use ararajuba_provider::image_model::v4::image_model_v4::ImageModelV4;
use ararajuba_provider::language_model::v4::language_model_v4::LanguageModelV4;
use ararajuba_provider::provider::Provider;
use ararajuba_provider::video_model::v4::video_model_v4::VideoModelV4;
use std::collections::HashMap;

/// Default Google Generative AI base URL.
const DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Settings for creating a Google Generative AI provider.
#[derive(Clone, Default)]
pub struct GoogleSettings {
    /// Google API key. Falls back to `GOOGLE_GENERATIVE_AI_API_KEY` env var.
    pub api_key: Option<String>,
    /// Custom base URL.
    pub base_url: Option<String>,
    /// Additional headers.
    pub headers: Option<HashMap<String, String>>,
}

/// A Google Generative AI provider.
pub struct GoogleProvider {
    name: String,
    base_url: String,
    headers: HashMap<String, String>,
}

impl GoogleProvider {
    fn new(settings: GoogleSettings) -> Self {
        let api_key = settings
            .api_key
            .or_else(|| std::env::var("GOOGLE_GENERATIVE_AI_API_KEY").ok())
            .unwrap_or_default();

        let base_url = settings
            .base_url
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

        let mut headers = HashMap::new();
        // Google uses x-goog-api-key header
        headers.insert("x-goog-api-key".to_string(), api_key);
        headers.insert("content-type".to_string(), "application/json".to_string());

        if let Some(extra) = settings.headers {
            headers.extend(extra);
        }

        Self {
            name: "google.generative-ai".to_string(),
            base_url: base_url.trim_end_matches('/').to_string(),
            headers,
        }
    }
}

impl Provider for GoogleProvider {
    fn language_model_v4(&self, model_id: &str) -> Option<Box<dyn LanguageModelV4>> {
        let config = GoogleChatConfig {
            provider: format!("{}.chat", self.name),
            base_url: self.base_url.clone(),
            headers: self.headers.clone(),
            model_id: model_id.to_string(),
            generate_id: None,
            fetch: None,
        };
        Some(Box::new(GoogleGenerativeAILanguageModel::new(
            model_id.to_string(),
            config,
        )))
    }

    fn embedding_model_v4(&self, model_id: &str) -> Option<Box<dyn EmbeddingModelV4>> {
        let config = GoogleEmbeddingConfig {
            provider: format!("{}.embedding", self.name),
            base_url: self.base_url.clone(),
            headers: self.headers.clone(),
            fetch: None,
        };
        Some(Box::new(GoogleEmbeddingModel::new(
            model_id.to_string(),
            config,
        )))
    }

    fn image_model_v4(&self, model_id: &str) -> Option<Box<dyn ImageModelV4>> {
        let config = GoogleImageConfig {
            provider: format!("{}.image", self.name),
            base_url: self.base_url.clone(),
            headers: self.headers.clone(),
            fetch: None,
        };
        Some(Box::new(GoogleImageModel::new(
            model_id.to_string(),
            config,
        )))
    }

    fn video_model_v4(&self, model_id: &str) -> Option<Box<dyn VideoModelV4>> {
        let config = GoogleVideoConfig {
            provider: format!("{}.video", self.name),
            base_url: self.base_url.clone(),
            headers: self.headers.clone(),
            fetch: None,
        };
        Some(Box::new(GoogleVideoModel::new(
            model_id.to_string(),
            config,
        )))
    }
}

/// Create a Google Generative AI provider with the given settings.
pub fn create_google(settings: GoogleSettings) -> GoogleProvider {
    GoogleProvider::new(settings)
}

/// Create a Google Generative AI provider with default settings and get a language model.
pub fn google(model_id: &str) -> Box<dyn LanguageModelV4> {
    let provider = create_google(GoogleSettings::default());
    provider.language_model_v4(model_id).expect("language model")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_google_language_model() {
        let provider = create_google(GoogleSettings {
            api_key: Some("AIza-test".into()),
            ..Default::default()
        });
        let model = provider.language_model_v4("gemini-2.0-flash");
        assert!(model.is_some());

        let model = model.unwrap();
        assert_eq!(model.model_id(), "gemini-2.0-flash");
        assert_eq!(model.provider(), "google.generative-ai.chat");
    }

    #[test]
    fn test_create_google_embedding_model() {
        let provider = create_google(GoogleSettings {
            api_key: Some("AIza-test".into()),
            ..Default::default()
        });
        let model = provider.embedding_model_v4("gemini-embedding-001");
        assert!(model.is_some());
        assert_eq!(model.unwrap().model_id(), "gemini-embedding-001");
    }

    #[test]
    fn test_google_headers() {
        let provider = create_google(GoogleSettings {
            api_key: Some("AIza-xxx".into()),
            ..Default::default()
        });
        assert_eq!(
            provider.headers.get("x-goog-api-key"),
            Some(&"AIza-xxx".to_string())
        );
    }

    #[test]
    fn test_google_custom_base_url() {
        let provider = create_google(GoogleSettings {
            api_key: Some("key".into()),
            base_url: Some("https://custom.googleapis.com/v1".into()),
            ..Default::default()
        });
        assert_eq!(provider.base_url, "https://custom.googleapis.com/v1");
    }
}
