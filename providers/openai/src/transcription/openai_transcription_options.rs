//! OpenAI-specific options for audio transcription.

use serde::{Deserialize, Serialize};

/// OpenAI-specific options for audio transcription.
///
/// Extracted from `provider_options["openai.transcription"]`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpenAITranscriptionOptions {
    /// Temperature for the model (0-1, default 0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    /// Response format: "json", "text", "srt", "verbose_json", "vtt"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<String>,

    /// Timestamp granularities: "word", "segment"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_granularities: Option<Vec<String>>,
}

/// Parse OpenAI transcription options from provider_options.
pub fn parse_openai_transcription_options(
    provider_options: &ararajuba_provider::shared::ProviderOptions,
    provider_name: &str,
) -> OpenAITranscriptionOptions {
    provider_options
        .get(provider_name)
        .and_then(|obj| {
            let value = serde_json::to_value(obj).ok()?;
            serde_json::from_value(value).ok()
        })
        .unwrap_or_default()
}
