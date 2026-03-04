//! DeepSeek-specific provider options for chat models.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// DeepSeek-specific provider options for chat models.
///
/// Pass under `provider_options["deepseek"]` in `CallOptions`.
///
/// # Example
/// ```json
/// {
///     "thinking": { "type": "enabled", "budget_tokens": 8192 }
/// }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeepSeekChatOptions {
    /// Thinking / extended reasoning configuration.
    ///
    /// Example: `{ "type": "enabled", "budget_tokens": 8192 }`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<Value>,
}

/// Parse DeepSeek options from the provider_options map.
pub fn parse_deepseek_options(
    provider_options: Option<&ararajuba_provider::shared::ProviderOptions>,
) -> DeepSeekChatOptions {
    provider_options
        .and_then(|po| po.get("deepseek"))
        .and_then(|obj| {
            let value = serde_json::to_value(obj).ok()?;
            serde_json::from_value(value).ok()
        })
        .unwrap_or_default()
}
