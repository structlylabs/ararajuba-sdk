//! Google-specific provider options for chat models.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Google-specific provider options for chat (Generative AI) models.
///
/// Pass under `provider_options["google"]` in `CallOptions`.
///
/// # Example
/// ```json
/// {
///     "thinkingConfig": { "thinkingBudget": 8192 },
///     "cachedContent": "cachedContents/abc123"
/// }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleChatOptions {
    /// Thinking / reasoning configuration.
    ///
    /// Example: `{ "thinkingBudget": 8192 }`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_config: Option<Value>,

    /// Cached content resource name for context caching.
    ///
    /// Example: `"cachedContents/abc123"`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_content: Option<String>,

    /// Safety settings for content filtering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_settings: Option<Value>,

    /// Response modalities (e.g., `["TEXT", "IMAGE"]`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_modalities: Option<Vec<String>>,
}

/// Parse Google options from the provider_options map.
pub fn parse_google_options(
    provider_options: Option<&ararajuba_provider::shared::ProviderOptions>,
) -> GoogleChatOptions {
    provider_options
        .and_then(|po| po.get("google"))
        .and_then(|obj| {
            let value = serde_json::to_value(obj).ok()?;
            serde_json::from_value(value).ok()
        })
        .unwrap_or_default()
}
