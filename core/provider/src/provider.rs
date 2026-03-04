//! The Provider trait — factory for obtaining models by ID.
//!
//! Supports both v3 (legacy) and v4 (current) model interfaces.
//! Providers should implement the v4 methods; v3 methods have default
//! implementations that return `None`.

use crate::embedding_model::v3::embedding_model_v3::EmbeddingModel;
use crate::embedding_model::v4::embedding_model_v4::EmbeddingModelV4;
use crate::image_model::v3::image_model_v3::ImageModel;
use crate::image_model::v4::image_model_v4::ImageModelV4;
use crate::language_model::v3::language_model_v3::LanguageModel;
use crate::language_model::v4::language_model_v4::LanguageModelV4;
use crate::reranking_model::v3::reranking_model_v3::RerankingModel;
use crate::reranking_model::v4::reranking_model_v4::RerankingModelV4;
use crate::speech_model::v3::speech_model_v3::SpeechModel;
use crate::speech_model::v4::speech_model_v4::SpeechModelV4;
use crate::transcription_model::v3::transcription_model_v3::TranscriptionModel;
use crate::transcription_model::v4::transcription_model_v4::TranscriptionModelV4;
use crate::video_model::v3::video_model_v3::VideoModel;
use crate::video_model::v4::video_model_v4::VideoModelV4;

/// A provider is a factory that creates model instances by ID.
///
/// Third-party providers implement this trait to register themselves.
/// Only `language_model` is required; all other model types have default
/// implementations that return `None`, indicating the provider does not
/// support that modality.
///
/// Providers can implement either v3 or v4 methods (or both).
/// The v4 methods are the preferred interface going forward.
pub trait Provider: Send + Sync {
    fn specification_version(&self) -> &'static str {
        "v4"
    }

    // -----------------------------------------------------------------------
    // v4 model factories (preferred)
    // -----------------------------------------------------------------------

    /// Create a v4 language model by ID.
    fn language_model_v4(&self, model_id: &str) -> Option<Box<dyn LanguageModelV4>> {
        let _ = model_id;
        None
    }

    /// Create a v4 embedding model by ID.
    fn embedding_model_v4(&self, model_id: &str) -> Option<Box<dyn EmbeddingModelV4>> {
        let _ = model_id;
        None
    }

    /// Create a v4 image model by ID.
    fn image_model_v4(&self, model_id: &str) -> Option<Box<dyn ImageModelV4>> {
        let _ = model_id;
        None
    }

    /// Create a v4 speech model by ID.
    fn speech_model_v4(&self, model_id: &str) -> Option<Box<dyn SpeechModelV4>> {
        let _ = model_id;
        None
    }

    /// Create a v4 transcription model by ID.
    fn transcription_model_v4(&self, model_id: &str) -> Option<Box<dyn TranscriptionModelV4>> {
        let _ = model_id;
        None
    }

    /// Create a v4 reranking model by ID.
    fn reranking_model_v4(&self, model_id: &str) -> Option<Box<dyn RerankingModelV4>> {
        let _ = model_id;
        None
    }

    /// Create a v4 video model by ID.
    fn video_model_v4(&self, model_id: &str) -> Option<Box<dyn VideoModelV4>> {
        let _ = model_id;
        None
    }

    // -----------------------------------------------------------------------
    // v3 model factories (deprecated — use v4 methods instead)
    // -----------------------------------------------------------------------

    /// Create a v3 language model by ID (deprecated).
    #[deprecated(since = "0.2.0", note = "Use language_model_v4() instead")]
    fn language_model(&self, model_id: &str) -> Option<Box<dyn LanguageModel>> {
        let _ = model_id;
        None
    }

    /// Create a v3 embedding model by ID (deprecated).
    #[deprecated(since = "0.2.0", note = "Use embedding_model_v4() instead")]
    fn embedding_model(&self, model_id: &str) -> Option<Box<dyn EmbeddingModel>> {
        let _ = model_id;
        None
    }

    /// Create a v3 image model by ID (deprecated).
    #[deprecated(since = "0.2.0", note = "Use image_model_v4() instead")]
    fn image_model(&self, model_id: &str) -> Option<Box<dyn ImageModel>> {
        let _ = model_id;
        None
    }

    /// Create a v3 speech model by ID (deprecated).
    #[deprecated(since = "0.2.0", note = "Use speech_model_v4() instead")]
    fn speech_model(&self, model_id: &str) -> Option<Box<dyn SpeechModel>> {
        let _ = model_id;
        None
    }

    /// Create a v3 transcription model by ID (deprecated).
    #[deprecated(since = "0.2.0", note = "Use transcription_model_v4() instead")]
    fn transcription_model(&self, model_id: &str) -> Option<Box<dyn TranscriptionModel>> {
        let _ = model_id;
        None
    }

    /// Create a v3 reranking model by ID (deprecated).
    #[deprecated(since = "0.2.0", note = "Use reranking_model_v4() instead")]
    fn reranking_model(&self, model_id: &str) -> Option<Box<dyn RerankingModel>> {
        let _ = model_id;
        None
    }

    /// Create a v3 video model by ID (deprecated).
    #[deprecated(since = "0.2.0", note = "Use video_model_v4() instead")]
    fn video_model(&self, model_id: &str) -> Option<Box<dyn VideoModel>> {
        let _ = model_id;
        None
    }
}
