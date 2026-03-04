//! OpenAI-specific options for image generation.

use serde::{Deserialize, Serialize};

/// OpenAI-specific options for image generation.
///
/// Extracted from `provider_options["openai.image"]` in `ImageCallOptions`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpenAIImageOptions {
    /// Image quality: "standard" or "hd" (DALL·E 3 only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,

    /// Image style: "vivid" or "natural" (DALL·E 3 only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,

    /// Response format: "url" or "b64_json"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<String>,

    /// End-user identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,

    /// Output compression level (0-100, gpt-image-1 only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_compression: Option<u32>,

    /// Output format: "png", "jpeg", "webp" (gpt-image-1 only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,

    /// Moderation: "auto" or "low" (gpt-image-1 only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moderation: Option<String>,

    /// Background: "auto", "transparent", "opaque" (gpt-image-1 only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
}

/// Parse OpenAI image options from provider_options.
pub fn parse_openai_image_options(
    provider_options: &ararajuba_provider::shared::ProviderOptions,
    provider_name: &str,
) -> OpenAIImageOptions {
    provider_options
        .get(provider_name)
        .and_then(|obj| {
            let value = serde_json::to_value(obj).ok()?;
            serde_json::from_value(value).ok()
        })
        .unwrap_or_default()
}
