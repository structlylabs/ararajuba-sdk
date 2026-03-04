//! OpenAI provider factory.

use crate::chat::OpenAIChatLanguageModel;
use crate::embedding::OpenAIEmbeddingModel;
use crate::image::OpenAIImageModel;
use crate::responses::openai_responses_language_model::{
    OpenAIResponsesConfig, OpenAIResponsesLanguageModel,
};
use crate::speech::OpenAISpeechModel;
use crate::transcription::OpenAITranscriptionModel;
use ararajuba_provider::embedding_model::v4::embedding_model_v4::EmbeddingModelV4;
use ararajuba_provider::image_model::v4::image_model_v4::ImageModelV4;
use ararajuba_provider::language_model::v4::language_model_v4::LanguageModelV4;
use ararajuba_provider::provider::Provider;
use ararajuba_provider::speech_model::v4::speech_model_v4::SpeechModelV4;
use ararajuba_provider::transcription_model::v4::transcription_model_v4::TranscriptionModelV4;
use ararajuba_openai_compatible::{ChatModelConfig, EmbeddingModelConfig};
use std::collections::HashMap;

/// Default OpenAI base URL.
const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";

/// Settings for creating an OpenAI provider.
#[derive(Clone, Default)]
pub struct OpenAISettings {
    /// OpenAI API key. Falls back to `OPENAI_API_KEY` env var.
    pub api_key: Option<String>,
    /// Custom base URL. Falls back to `OPENAI_BASE_URL` env var.
    pub base_url: Option<String>,
    /// OpenAI organization ID (→ `OpenAI-Organization` header).
    pub organization: Option<String>,
    /// OpenAI project ID (→ `OpenAI-Project` header).
    pub project: Option<String>,
    /// Additional headers.
    pub headers: Option<HashMap<String, String>>,
}

/// An OpenAI provider.
pub struct OpenAIProvider {
    name: String,
    base_url: String,
    headers: HashMap<String, String>,
}

impl OpenAIProvider {
    fn new(settings: OpenAISettings) -> Self {
        let api_key = settings
            .api_key
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .unwrap_or_default();

        let base_url = settings
            .base_url
            .or_else(|| std::env::var("OPENAI_BASE_URL").ok())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), format!("Bearer {api_key}"));

        if let Some(org) = &settings.organization {
            headers.insert("OpenAI-Organization".to_string(), org.clone());
        }
        if let Some(proj) = &settings.project {
            headers.insert("OpenAI-Project".to_string(), proj.clone());
        }

        if let Some(extra) = settings.headers {
            headers.extend(extra);
        }

        Self {
            name: "openai".to_string(),
            base_url: base_url.trim_end_matches('/').to_string(),
            headers,
        }
    }

    /// Create an OpenAI Responses API language model by model ID.
    ///
    /// The Responses API is an alternative to Chat Completions with features
    /// like conversation continuation, built-in tools, and structured output.
    pub fn responses(&self, model_id: &str) -> Box<dyn LanguageModelV4> {
        let config = OpenAIResponsesConfig {
            provider: format!("{}.responses", self.name),
            url: format!("{}/responses", self.base_url),
            headers: self.headers.clone(),
            fetch: None,
        };
        Box::new(OpenAIResponsesLanguageModel::new(
            model_id.to_string(),
            config,
        ))
    }
}

impl Provider for OpenAIProvider {
    fn language_model_v4(&self, model_id: &str) -> Option<Box<dyn LanguageModelV4>> {
        let config = ChatModelConfig {
            provider: format!("{}.chat", self.name),
            url: format!("{}/chat/completions", self.base_url),
            headers: self.headers.clone(),
            include_usage: true,
            supports_structured_outputs: true,
            fetch: None,
        };
        Some(Box::new(OpenAIChatLanguageModel::new(
            model_id.to_string(),
            config,
        )))
    }

    fn embedding_model_v4(&self, model_id: &str) -> Option<Box<dyn EmbeddingModelV4>> {
        let config = EmbeddingModelConfig {
            provider: format!("{}.embedding", self.name),
            url: format!("{}/embeddings", self.base_url),
            headers: self.headers.clone(),
            fetch: None,
        };
        Some(Box::new(OpenAIEmbeddingModel::new(
            model_id.to_string(),
            config,
        )))
    }

    fn image_model_v4(&self, model_id: &str) -> Option<Box<dyn ImageModelV4>> {
        Some(Box::new(OpenAIImageModel::new(
            model_id.to_string(),
            format!("{}.image", self.name),
            &self.base_url,
            self.headers.clone(),
        )))
    }

    fn speech_model_v4(&self, model_id: &str) -> Option<Box<dyn SpeechModelV4>> {
        Some(Box::new(OpenAISpeechModel::new(
            model_id.to_string(),
            format!("{}.speech", self.name),
            &self.base_url,
            self.headers.clone(),
        )))
    }

    fn transcription_model_v4(&self, model_id: &str) -> Option<Box<dyn TranscriptionModelV4>> {
        Some(Box::new(OpenAITranscriptionModel::new(
            model_id.to_string(),
            format!("{}.transcription", self.name),
            &self.base_url,
            self.headers.clone(),
        )))
    }
}

/// Create an OpenAI provider with the given settings.
pub fn create_openai(settings: OpenAISettings) -> OpenAIProvider {
    OpenAIProvider::new(settings)
}

/// Create an OpenAI provider with default settings (API key from env).
pub fn openai(model_id: &str) -> Box<dyn LanguageModelV4> {
    let provider = create_openai(OpenAISettings::default());
    provider.language_model_v4(model_id).expect("language model")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_openai_language_model() {
        let provider = create_openai(OpenAISettings {
            api_key: Some("sk-test".into()),
            ..Default::default()
        });
        let model = provider.language_model_v4("gpt-4o").unwrap();
        assert_eq!(model.provider(), "openai.chat");
        assert_eq!(model.model_id(), "gpt-4o");
    }

    #[test]
    fn test_create_openai_embedding_model() {
        let provider = create_openai(OpenAISettings {
            api_key: Some("sk-test".into()),
            ..Default::default()
        });
        let model = provider.embedding_model_v4("text-embedding-3-small").unwrap();
        assert_eq!(model.provider(), "openai.embedding");
        assert_eq!(model.model_id(), "text-embedding-3-small");
    }

    #[test]
    fn test_custom_base_url() {
        let provider = create_openai(OpenAISettings {
            api_key: Some("sk-test".into()),
            base_url: Some("https://custom.api.com/v1".into()),
            ..Default::default()
        });
        assert_eq!(provider.base_url, "https://custom.api.com/v1");
    }
}
