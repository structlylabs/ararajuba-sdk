//! `ModelRegistry` — lookup providers and models by string IDs like `"openai:gpt-4o"`.

use crate::error::Error;
use ararajuba_provider::embedding_model::v4::embedding_model_v4::EmbeddingModelV4;
use ararajuba_provider::image_model::v4::image_model_v4::ImageModelV4;
use ararajuba_provider::language_model::v4::language_model_v4::LanguageModelV4;
use ararajuba_provider::provider::Provider;
use ararajuba_provider::reranking_model::v4::reranking_model_v4::RerankingModelV4;
use ararajuba_provider::speech_model::v4::speech_model_v4::SpeechModelV4;
use ararajuba_provider::transcription_model::v4::transcription_model_v4::TranscriptionModelV4;
use ararajuba_provider::video_model::v4::video_model_v4::VideoModelV4;
use std::collections::HashMap;

/// A registry that resolves model instances by string IDs in the format
/// `"provider_prefix:model_id"`.
///
/// # Example
/// ```ignore
/// let mut registry = ModelRegistry::new();
/// registry.register("openai", openai_provider);
/// let model = registry.language_model("openai:gpt-4o")?;
/// ```
pub struct ModelRegistry {
    providers: HashMap<String, Box<dyn Provider>>,
    separator: char,
}

impl ModelRegistry {
    /// Create an empty registry with the default separator (`:`).
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            separator: ':',
        }
    }

    /// Create a registry with a custom separator.
    pub fn with_separator(separator: char) -> Self {
        Self {
            providers: HashMap::new(),
            separator,
        }
    }

    /// Register a provider under a given prefix.
    pub fn register(&mut self, prefix: &str, provider: Box<dyn Provider>) {
        self.providers.insert(prefix.to_string(), provider);
    }

    /// Resolve a language model from a string ID like `"openai:gpt-4o"`.
    pub fn language_model(&self, id: &str) -> Result<Box<dyn LanguageModelV4>, Error> {
        let (prefix, model_id) = self.split_id(id)?;
        let provider = self.get_provider(prefix)?;
        provider.language_model_v4(model_id).ok_or_else(|| Error::InvalidArgument {
            message: format!("Provider '{prefix}' has no language model '{model_id}'"),
        })
    }

    /// Resolve an embedding model from a string ID.
    pub fn embedding_model(&self, id: &str) -> Result<Box<dyn EmbeddingModelV4>, Error> {
        let (prefix, model_id) = self.split_id(id)?;
        let provider = self.get_provider(prefix)?;
        provider.embedding_model_v4(model_id).ok_or_else(|| Error::InvalidArgument {
            message: format!("Provider '{prefix}' has no embedding model '{model_id}'"),
        })
    }

    /// Resolve an image model from a string ID.
    pub fn image_model(&self, id: &str) -> Result<Box<dyn ImageModelV4>, Error> {
        let (prefix, model_id) = self.split_id(id)?;
        let provider = self.get_provider(prefix)?;
        provider.image_model_v4(model_id).ok_or_else(|| Error::InvalidArgument {
            message: format!("Provider '{prefix}' has no image model '{model_id}'"),
        })
    }

    /// Resolve a speech model from a string ID.
    pub fn speech_model(&self, id: &str) -> Result<Box<dyn SpeechModelV4>, Error> {
        let (prefix, model_id) = self.split_id(id)?;
        let provider = self.get_provider(prefix)?;
        provider.speech_model_v4(model_id).ok_or_else(|| Error::InvalidArgument {
            message: format!("Provider '{prefix}' has no speech model '{model_id}'"),
        })
    }

    /// Resolve a transcription model from a string ID.
    pub fn transcription_model(&self, id: &str) -> Result<Box<dyn TranscriptionModelV4>, Error> {
        let (prefix, model_id) = self.split_id(id)?;
        let provider = self.get_provider(prefix)?;
        provider
            .transcription_model_v4(model_id)
            .ok_or_else(|| Error::InvalidArgument {
                message: format!("Provider '{prefix}' has no transcription model '{model_id}'"),
            })
    }

    /// Resolve a reranking model from a string ID.
    pub fn reranking_model(&self, id: &str) -> Result<Box<dyn RerankingModelV4>, Error> {
        let (prefix, model_id) = self.split_id(id)?;
        let provider = self.get_provider(prefix)?;
        provider
            .reranking_model_v4(model_id)
            .ok_or_else(|| Error::InvalidArgument {
                message: format!("Provider '{prefix}' has no reranking model '{model_id}'"),
            })
    }

    /// Resolve a video model from a string ID.
    pub fn video_model(&self, id: &str) -> Result<Box<dyn VideoModelV4>, Error> {
        let (prefix, model_id) = self.split_id(id)?;
        let provider = self.get_provider(prefix)?;
        provider.video_model_v4(model_id).ok_or_else(|| Error::InvalidArgument {
            message: format!("Provider '{prefix}' has no video model '{model_id}'"),
        })
    }

    /// Split a model ID on the separator.
    fn split_id<'a>(&self, id: &'a str) -> Result<(&'a str, &'a str), Error> {
        id.split_once(self.separator)
            .ok_or_else(|| Error::InvalidArgument {
                message: format!(
                    "Invalid model ID '{id}': expected format 'provider{sep}model-name'",
                    sep = self.separator
                ),
            })
    }

    /// Look up a provider by prefix.
    fn get_provider(&self, prefix: &str) -> Result<&dyn Provider, Error> {
        self.providers
            .get(prefix)
            .map(|p| p.as_ref())
            .ok_or_else(|| Error::NoSuchProvider {
                provider_id: prefix.to_string(),
            })
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// A custom provider that resolves models from explicit maps, with an optional
/// fallback provider.
///
/// Useful when you want to alias model IDs or override specific models without
/// implementing a full `Provider`.
pub struct CustomProvider {
    language_models: HashMap<String, Box<dyn Fn() -> Box<dyn LanguageModelV4> + Send + Sync>>,
    embedding_models: HashMap<String, Box<dyn Fn() -> Box<dyn EmbeddingModelV4> + Send + Sync>>,
    image_models: HashMap<String, Box<dyn Fn() -> Box<dyn ImageModelV4> + Send + Sync>>,
    speech_models: HashMap<String, Box<dyn Fn() -> Box<dyn SpeechModelV4> + Send + Sync>>,
    transcription_models:
        HashMap<String, Box<dyn Fn() -> Box<dyn TranscriptionModelV4> + Send + Sync>>,
    reranking_models: HashMap<String, Box<dyn Fn() -> Box<dyn RerankingModelV4> + Send + Sync>>,
    video_models: HashMap<String, Box<dyn Fn() -> Box<dyn VideoModelV4> + Send + Sync>>,
    fallback: Option<Box<dyn Provider>>,
}

impl CustomProvider {
    pub fn new() -> Self {
        Self {
            language_models: HashMap::new(),
            embedding_models: HashMap::new(),
            image_models: HashMap::new(),
            speech_models: HashMap::new(),
            transcription_models: HashMap::new(),
            reranking_models: HashMap::new(),
            video_models: HashMap::new(),
            fallback: None,
        }
    }

    /// Set a fallback provider for model IDs not found in the explicit maps.
    pub fn with_fallback(mut self, provider: Box<dyn Provider>) -> Self {
        self.fallback = Some(provider);
        self
    }

    /// Register a language model factory for a given model ID.
    pub fn add_language_model(
        mut self,
        id: &str,
        factory: impl Fn() -> Box<dyn LanguageModelV4> + Send + Sync + 'static,
    ) -> Self {
        self.language_models.insert(id.to_string(), Box::new(factory));
        self
    }

    /// Register an embedding model factory for a given model ID.
    pub fn add_embedding_model(
        mut self,
        id: &str,
        factory: impl Fn() -> Box<dyn EmbeddingModelV4> + Send + Sync + 'static,
    ) -> Self {
        self.embedding_models
            .insert(id.to_string(), Box::new(factory));
        self
    }

    /// Register an image model factory for a given model ID.
    pub fn add_image_model(
        mut self,
        id: &str,
        factory: impl Fn() -> Box<dyn ImageModelV4> + Send + Sync + 'static,
    ) -> Self {
        self.image_models.insert(id.to_string(), Box::new(factory));
        self
    }

    /// Register a speech model factory for a given model ID.
    pub fn add_speech_model(
        mut self,
        id: &str,
        factory: impl Fn() -> Box<dyn SpeechModelV4> + Send + Sync + 'static,
    ) -> Self {
        self.speech_models.insert(id.to_string(), Box::new(factory));
        self
    }

    /// Register a transcription model factory for a given model ID.
    pub fn add_transcription_model(
        mut self,
        id: &str,
        factory: impl Fn() -> Box<dyn TranscriptionModelV4> + Send + Sync + 'static,
    ) -> Self {
        self.transcription_models
            .insert(id.to_string(), Box::new(factory));
        self
    }

    /// Register a reranking model factory for a given model ID.
    pub fn add_reranking_model(
        mut self,
        id: &str,
        factory: impl Fn() -> Box<dyn RerankingModelV4> + Send + Sync + 'static,
    ) -> Self {
        self.reranking_models
            .insert(id.to_string(), Box::new(factory));
        self
    }

    /// Register a video model factory for a given model ID.
    pub fn add_video_model(
        mut self,
        id: &str,
        factory: impl Fn() -> Box<dyn VideoModelV4> + Send + Sync + 'static,
    ) -> Self {
        self.video_models.insert(id.to_string(), Box::new(factory));
        self
    }
}

impl Default for CustomProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl Provider for CustomProvider {
    fn language_model_v4(&self, model_id: &str) -> Option<Box<dyn LanguageModelV4>> {
        if let Some(factory) = self.language_models.get(model_id) {
            Some(factory())
        } else {
            self.fallback.as_ref()?.language_model_v4(model_id)
        }
    }

    fn embedding_model_v4(&self, model_id: &str) -> Option<Box<dyn EmbeddingModelV4>> {
        if let Some(factory) = self.embedding_models.get(model_id) {
            Some(factory())
        } else {
            self.fallback.as_ref()?.embedding_model_v4(model_id)
        }
    }

    fn image_model_v4(&self, model_id: &str) -> Option<Box<dyn ImageModelV4>> {
        if let Some(factory) = self.image_models.get(model_id) {
            Some(factory())
        } else {
            self.fallback.as_ref()?.image_model_v4(model_id)
        }
    }

    fn speech_model_v4(&self, model_id: &str) -> Option<Box<dyn SpeechModelV4>> {
        if let Some(factory) = self.speech_models.get(model_id) {
            Some(factory())
        } else {
            self.fallback.as_ref()?.speech_model_v4(model_id)
        }
    }

    fn transcription_model_v4(&self, model_id: &str) -> Option<Box<dyn TranscriptionModelV4>> {
        if let Some(factory) = self.transcription_models.get(model_id) {
            Some(factory())
        } else {
            self.fallback.as_ref()?.transcription_model_v4(model_id)
        }
    }

    fn reranking_model_v4(&self, model_id: &str) -> Option<Box<dyn RerankingModelV4>> {
        if let Some(factory) = self.reranking_models.get(model_id) {
            Some(factory())
        } else {
            self.fallback.as_ref()?.reranking_model_v4(model_id)
        }
    }

    fn video_model_v4(&self, model_id: &str) -> Option<Box<dyn VideoModelV4>> {
        if let Some(factory) = self.video_models.get(model_id) {
            Some(factory())
        } else {
            self.fallback.as_ref()?.video_model_v4(model_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A mock provider for testing
    struct MockProvider;

    impl Provider for MockProvider {
        fn language_model_v4(&self, _model_id: &str) -> Option<Box<dyn LanguageModelV4>> {
            // In a real test we'd return a mock model, but for registry tests
            // we just need to check the routing logic works.
            None
        }
    }

    #[test]
    fn test_split_id_valid() {
        let registry = ModelRegistry::new();
        let (prefix, model) = registry.split_id("openai:gpt-4o").unwrap();
        assert_eq!(prefix, "openai");
        assert_eq!(model, "gpt-4o");
    }

    #[test]
    fn test_split_id_with_multiple_colons() {
        let registry = ModelRegistry::new();
        let (prefix, model) = registry.split_id("azure:deployment:gpt-4").unwrap();
        assert_eq!(prefix, "azure");
        assert_eq!(model, "deployment:gpt-4");
    }

    #[test]
    fn test_split_id_no_separator() {
        let registry = ModelRegistry::new();
        assert!(registry.split_id("invalid").is_err());
    }

    #[test]
    fn test_unknown_provider_returns_error() {
        let registry = ModelRegistry::new();
        let result = registry.language_model("unknown:model");
        assert!(result.is_err());
        match result.err().unwrap() {
            Error::NoSuchProvider { provider_id } => {
                assert_eq!(provider_id, "unknown");
            }
            other => panic!("Expected NoSuchProvider, got {other:?}"),
        }
    }

    #[test]
    fn test_provider_returns_none_becomes_error() {
        let mut registry = ModelRegistry::new();
        registry.register("mock", Box::new(MockProvider));
        let result = registry.language_model("mock:nonexistent");
        assert!(result.is_err());
        match result.err().unwrap() {
            Error::InvalidArgument { message } => {
                assert!(message.contains("no language model"));
            }
            other => panic!("Expected InvalidArgument, got {other:?}"),
        }
    }

    #[test]
    fn test_custom_separator() {
        let registry = ModelRegistry::with_separator('/');
        let (prefix, model) = registry.split_id("openai/gpt-4o").unwrap();
        assert_eq!(prefix, "openai");
        assert_eq!(model, "gpt-4o");
    }
}
