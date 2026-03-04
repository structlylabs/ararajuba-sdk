//! Anthropic-specific provider options for chat models.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Anthropic-specific provider options for chat models.
///
/// Pass under `provider_options["anthropic"]` in `CallOptions`.
///
/// # Example
/// ```json
/// {
///     "thinking": { "type": "enabled", "budget_tokens": 10000 },
///     "cacheControl": { "type": "ephemeral" }
/// }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnthropicChatOptions {
    /// Thinking / extended reasoning configuration.
    ///
    /// Example: `{ "type": "enabled", "budget_tokens": 10000 }`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<Value>,

    /// Cache control configuration.
    ///
    /// Example: `{ "type": "ephemeral" }`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<Value>,
}

/// Parse Anthropic options from the provider_options map.
pub fn parse_anthropic_options(
    provider_options: Option<&ararajuba_provider::shared::ProviderOptions>,
) -> AnthropicChatOptions {
    provider_options
        .and_then(|po| po.get("anthropic"))
        .and_then(|obj| {
            let value = serde_json::to_value(obj).ok()?;
            serde_json::from_value(value).ok()
        })
        .unwrap_or_default()
}
