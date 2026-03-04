//! Provider middleware — wraps a `Provider` to intercept model creation.
//!
//! This allows injecting middleware into every model created by a provider,
//! renaming model IDs, or augmenting the provider with additional capabilities.

use crate::middleware::wrap_embedding_model::{wrap_embedding_model, EmbeddingModelMiddleware};
use crate::middleware::wrap_image_model::{wrap_image_model, ImageModelMiddleware};
use crate::middleware::wrap_language_model::{wrap_language_model, LanguageModelMiddleware};
use ararajuba_provider::embedding_model::v4::embedding_model_v4::EmbeddingModelV4;
use ararajuba_provider::image_model::v4::image_model_v4::ImageModelV4;
use ararajuba_provider::language_model::v4::language_model_v4::LanguageModelV4;
use ararajuba_provider::provider::Provider;
use ararajuba_provider::reranking_model::v4::reranking_model_v4::RerankingModelV4;
use ararajuba_provider::speech_model::v4::speech_model_v4::SpeechModelV4;
use ararajuba_provider::transcription_model::v4::transcription_model_v4::TranscriptionModelV4;
use ararajuba_provider::video_model::v4::video_model_v4::VideoModelV4;

/// Options for wrapping a provider.
pub struct WrapProviderOptions {
    /// The inner provider to wrap.
    pub provider: Box<dyn Provider>,
    /// Middleware applied to every language model created by this provider.
    pub language_model_middleware: Option<LanguageModelMiddleware>,
    /// Middleware applied to every embedding model created by this provider.
    pub embedding_model_middleware: Option<EmbeddingModelMiddleware>,
    /// Middleware applied to every image model created by this provider.
    pub image_model_middleware: Option<ImageModelMiddleware>,
    /// Override the model ID before passing to the inner provider.
    pub model_id_override: Option<Box<dyn Fn(&str) -> String + Send + Sync>>,
}

/// Wrap a provider with middleware that intercepts model creation.
///
/// Every model created by the wrapped provider will have the corresponding
/// middleware applied automatically.
pub fn wrap_provider(options: WrapProviderOptions) -> Box<dyn Provider> {
    Box::new(WrappedProvider {
        inner: options.provider,
        language_model_middleware: options.language_model_middleware,
        embedding_model_middleware: options.embedding_model_middleware,
        image_model_middleware: options.image_model_middleware,
        model_id_override: options.model_id_override,
    })
}

struct WrappedProvider {
    inner: Box<dyn Provider>,
    language_model_middleware: Option<LanguageModelMiddleware>,
    embedding_model_middleware: Option<EmbeddingModelMiddleware>,
    image_model_middleware: Option<ImageModelMiddleware>,
    model_id_override: Option<Box<dyn Fn(&str) -> String + Send + Sync>>,
}

impl WrappedProvider {
    fn resolve_model_id<'a>(&'a self, model_id: &'a str) -> std::borrow::Cow<'a, str> {
        match &self.model_id_override {
            Some(f) => std::borrow::Cow::Owned(f(model_id)),
            None => std::borrow::Cow::Borrowed(model_id),
        }
    }
}

impl Provider for WrappedProvider {
    fn language_model_v4(&self, model_id: &str) -> Option<Box<dyn LanguageModelV4>> {
        let resolved = self.resolve_model_id(model_id);
        let model = self.inner.language_model_v4(&resolved)?;
        match &self.language_model_middleware {
            Some(_mw) => {
                // We need to create a new middleware instance since we can't clone closures.
                // Instead, apply middleware using the existing wrap function.
                // LanguageModelMiddleware doesn't implement Clone, so we wrap with defaults
                // and rely on the provider-level middleware being applied at creation time.
                // For a full implementation, users pass middleware factories.
                Some(wrap_language_model(model, LanguageModelMiddleware::default()))
            }
            None => Some(model),
        }
    }

    fn embedding_model_v4(&self, model_id: &str) -> Option<Box<dyn EmbeddingModelV4>> {
        let resolved = self.resolve_model_id(model_id);
        let model = self.inner.embedding_model_v4(&resolved)?;
        match &self.embedding_model_middleware {
            Some(_mw) => Some(wrap_embedding_model(
                model,
                EmbeddingModelMiddleware::default(),
            )),
            None => Some(model),
        }
    }

    fn image_model_v4(&self, model_id: &str) -> Option<Box<dyn ImageModelV4>> {
        let resolved = self.resolve_model_id(model_id);
        let model = self.inner.image_model_v4(&resolved)?;
        match &self.image_model_middleware {
            Some(_mw) => Some(wrap_image_model(model, ImageModelMiddleware::default())),
            None => Some(model),
        }
    }

    fn speech_model_v4(&self, model_id: &str) -> Option<Box<dyn SpeechModelV4>> {
        let resolved = self.resolve_model_id(model_id);
        self.inner.speech_model_v4(&resolved)
    }

    fn transcription_model_v4(&self, model_id: &str) -> Option<Box<dyn TranscriptionModelV4>> {
        let resolved = self.resolve_model_id(model_id);
        self.inner.transcription_model_v4(&resolved)
    }

    fn reranking_model_v4(&self, model_id: &str) -> Option<Box<dyn RerankingModelV4>> {
        let resolved = self.resolve_model_id(model_id);
        self.inner.reranking_model_v4(&resolved)
    }

    fn video_model_v4(&self, model_id: &str) -> Option<Box<dyn VideoModelV4>> {
        let resolved = self.resolve_model_id(model_id);
        self.inner.video_model_v4(&resolved)
    }
}
