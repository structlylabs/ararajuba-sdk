//! OpenAI-specific options for speech generation.

use serde::{Deserialize, Serialize};

/// OpenAI-specific options for text-to-speech.
///
/// Extracted from `provider_options["openai.speech"]` in `SpeechCallOptions`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpenAISpeechOptions {
    /// Response format: "mp3" (default), "opus", "aac", "flac", "wav", "pcm"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<String>,
}

/// Parse OpenAI speech options from provider_options.
pub fn parse_openai_speech_options(
    provider_options: &ararajuba_provider::shared::ProviderOptions,
    provider_name: &str,
) -> OpenAISpeechOptions {
    provider_options
        .get(provider_name)
        .and_then(|obj| {
            let value = serde_json::to_value(obj).ok()?;
            serde_json::from_value(value).ok()
        })
        .unwrap_or_default()
}
