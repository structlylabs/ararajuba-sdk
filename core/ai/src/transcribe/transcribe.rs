//! The `transcribe` high-level function.

use crate::error::Error;
use crate::types::call_warning::CallWarning;
use ararajuba_provider::transcription_model::v4::transcription_model_v4::TranscriptionModelV4;
use ararajuba_provider::transcription_model::v4::{
    TranscriptionAudioInput, TranscriptionCallOptions, TranscriptionResult,
    TranscriptionResponseMetadata, TranscriptionSegment,
};
use std::collections::HashMap;

/// Options for `transcribe()`.
pub struct TranscribeOptions {
    /// The transcription model to use.
    pub model: Box<dyn TranscriptionModelV4>,
    /// The audio data to transcribe.
    pub audio: TranscriptionAudioInput,
    /// Language hint (BCP-47 code).
    pub language: Option<String>,
    /// Prompt/context to guide the transcription.
    pub prompt: Option<String>,
    /// Provider-specific options.
    pub provider_options: Option<HashMap<String, HashMap<String, serde_json::Value>>>,
    /// Additional headers.
    pub headers: Option<HashMap<String, String>>,
}

/// Result of `transcribe()`.
pub struct TranscribeResult {
    /// The transcribed text.
    pub text: String,
    /// Segments with timestamps.
    pub segments: Option<Vec<TranscriptionSegment>>,
    /// Detected or provided language.
    pub language: Option<String>,
    /// Duration of the audio in seconds.
    pub duration_seconds: Option<f64>,
    /// Warnings from the model.
    pub warnings: Vec<CallWarning>,
    /// Response metadata.
    pub response: TranscriptionResponseMetadata,
}

/// Transcribe audio to text using a transcription model.
pub async fn transcribe(options: TranscribeOptions) -> Result<TranscribeResult, Error> {
    let _span = tracing::info_span!(
        "transcribe",
        language = options.language.as_deref().unwrap_or("auto"),
    )
    .entered();

    let call_options = TranscriptionCallOptions {
        audio: options.audio,
        language: options.language,
        prompt: options.prompt,
        provider_options: options.provider_options.unwrap_or_default(),
        headers: options.headers,
    };

    let result: TranscriptionResult = options.model.do_transcribe(&call_options).await?;

    Ok(TranscribeResult {
        text: result.text,
        segments: result.segments,
        language: result.language,
        duration_seconds: result.duration_seconds,
        warnings: result.warnings.into_iter().map(CallWarning::from).collect(),
        response: result.response,
    })
}
