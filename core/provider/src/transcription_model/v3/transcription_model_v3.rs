//! The TranscriptionModel trait and associated types.

use crate::errors::Error;
use crate::shared::{Headers, ProviderMetadata, ProviderOptions, Warning};
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};

/// Options passed to `do_transcribe` for transcription models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionCallOptions {
    /// The audio data to transcribe.
    pub audio: TranscriptionAudioInput,
    /// Language hint (BCP-47 code, e.g. "en").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Prompt/context to guide the transcription.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// Provider-specific options.
    pub provider_options: ProviderOptions,
    /// Additional headers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Headers>,
}

/// Audio input for transcription.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TranscriptionAudioInput {
    /// Base64-encoded audio data.
    #[serde(rename = "base64")]
    Base64 { data: String, media_type: String },
    /// URL pointing to an audio file.
    #[serde(rename = "url")]
    Url { url: String },
}

/// Result of a transcription call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    /// The transcribed text.
    pub text: String,
    /// Segments with timestamps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segments: Option<Vec<TranscriptionSegment>>,
    /// Detected or provided language.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Duration of the audio in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<f64>,
    /// Warnings.
    pub warnings: Vec<Warning>,
    /// Provider-specific metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_metadata: Option<ProviderMetadata>,
    /// Response metadata.
    pub response: TranscriptionResponseMetadata,
}

/// A segment of transcribed text with timing information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionSegment {
    /// The transcribed text for this segment.
    pub text: String,
    /// Start time in seconds.
    pub start: f64,
    /// End time in seconds.
    pub end: f64,
}

/// Response metadata for transcription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResponseMetadata {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub model_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Headers>,
}

/// Trait for audio transcription models (speech-to-text).
pub trait TranscriptionModel: Send + Sync {
    fn specification_version(&self) -> &'static str {
        "v3"
    }

    fn provider(&self) -> &str;
    fn model_id(&self) -> &str;

    /// Transcribe audio to text.
    fn do_transcribe<'a>(
        &'a self,
        options: &'a TranscriptionCallOptions,
    ) -> BoxFuture<'a, Result<TranscriptionResult, Error>>;
}
