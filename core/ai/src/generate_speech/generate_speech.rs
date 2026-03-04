//! The `generate_speech` high-level function.

use crate::error::Error;
use crate::types::call_warning::CallWarning;
use ararajuba_provider::speech_model::v4::speech_model_v4::SpeechModelV4;
use ararajuba_provider::speech_model::v4::{
    AudioData, SpeechCallOptions, SpeechGenerateResult, SpeechResponseMetadata,
};
use std::collections::HashMap;

/// Options for `generate_speech()`.
pub struct GenerateSpeechOptions {
    /// The speech model to use.
    pub model: Box<dyn SpeechModelV4>,
    /// The text to synthesize into speech.
    pub text: String,
    /// Voice identifier (provider-specific).
    pub voice: Option<String>,
    /// Output audio format (e.g. "mp3", "opus").
    pub output_format: Option<String>,
    /// Speech speed multiplier (1.0 = normal).
    pub speed: Option<f64>,
    /// Additional instructions for the speech model.
    pub instructions: Option<String>,
    /// Provider-specific options.
    pub provider_options: Option<HashMap<String, HashMap<String, serde_json::Value>>>,
    /// Additional headers.
    pub headers: Option<HashMap<String, String>>,
}

/// Result of `generate_speech()`.
pub struct GenerateSpeechResult {
    /// The generated audio data.
    pub audio: AudioData,
    /// Warnings from the model.
    pub warnings: Vec<CallWarning>,
    /// Response metadata.
    pub response: SpeechResponseMetadata,
}

/// Generate speech audio from text using a speech model.
pub async fn generate_speech(
    options: GenerateSpeechOptions,
) -> Result<GenerateSpeechResult, Error> {
    let _span = tracing::info_span!(
        "generate_speech",
        voice = options.voice.as_deref().unwrap_or("default"),
        output_format = options.output_format.as_deref().unwrap_or("default"),
    )
    .entered();

    let call_options = SpeechCallOptions {
        text: options.text,
        voice: options.voice,
        output_format: options.output_format,
        speed: options.speed,
        instructions: options.instructions,
        provider_options: options.provider_options.unwrap_or_default(),
        headers: options.headers,
    };

    let result: SpeechGenerateResult = options.model.do_generate(&call_options).await?;

    Ok(GenerateSpeechResult {
        audio: result.audio,
        warnings: result.warnings.into_iter().map(CallWarning::from).collect(),
        response: result.response,
    })
}
