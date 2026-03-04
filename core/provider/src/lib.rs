//! # ararajuba-provider
//!
//! Spec/trait layer for AI provider implementations.
//! Mirrors `@ai-sdk/provider` from the Vercel AI SDK.
//!
//! This crate defines the interfaces that providers must implement.
//! Both v3 (legacy) and v4 (current) interfaces are available.

pub mod errors;
pub mod json_value;
pub mod language_model;
pub mod embedding_model;
pub mod image_model;
pub mod speech_model;
pub mod transcription_model;
pub mod reranking_model;
pub mod video_model;
pub mod provider;
pub mod shared;

// ---------------------------------------------------------------------------
// v3 re-exports (deprecated — use the v4 equivalents)
// ---------------------------------------------------------------------------
#[deprecated(since = "0.2.0", note = "Use LanguageModelV4 instead")]
pub use language_model::v3::language_model_v3::LanguageModel;
#[deprecated(since = "0.2.0", note = "Use EmbeddingModelV4 instead")]
pub use embedding_model::v3::embedding_model_v3::EmbeddingModel;
#[deprecated(since = "0.2.0", note = "Use ImageModelV4 instead")]
pub use image_model::v3::image_model_v3::ImageModel;
#[deprecated(since = "0.2.0", note = "Use SpeechModelV4 instead")]
pub use speech_model::v3::speech_model_v3::SpeechModel;
#[deprecated(since = "0.2.0", note = "Use TranscriptionModelV4 instead")]
pub use transcription_model::v3::transcription_model_v3::TranscriptionModel;
#[deprecated(since = "0.2.0", note = "Use RerankingModelV4 instead")]
pub use reranking_model::v3::reranking_model_v3::RerankingModel;
#[deprecated(since = "0.2.0", note = "Use VideoModelV4 instead")]
pub use video_model::v3::video_model_v3::VideoModel;

// ---------------------------------------------------------------------------
// v4 re-exports (current)
// ---------------------------------------------------------------------------
pub use language_model::v4::language_model_v4::LanguageModelV4;
pub use language_model::v4::capabilities;
pub use language_model::v4::compat::V3LanguageModelAdapter;
pub use language_model::v4::stream_result::{
    ContentDelta, ToolCallDelta, MetadataDelta, AbortHandle,
    StreamResult as StreamResultV4,
};
pub use embedding_model::v4::embedding_model_v4::EmbeddingModelV4;
pub use image_model::v4::image_model_v4::ImageModelV4;
pub use speech_model::v4::speech_model_v4::SpeechModelV4;
pub use transcription_model::v4::transcription_model_v4::TranscriptionModelV4;
pub use reranking_model::v4::reranking_model_v4::RerankingModelV4;
pub use video_model::v4::video_model_v4::VideoModelV4;

// ---------------------------------------------------------------------------
// Common re-exports
// ---------------------------------------------------------------------------
pub use errors::Error;
pub use json_value::{JSONArray, JSONObject, JSONValue};
pub use provider::Provider;
pub use shared::{ProviderMetadata, ProviderOptions, Warning};
