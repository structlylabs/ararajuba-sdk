//! OpenAI-specific provider options for chat models.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// OpenAI-specific provider options for chat models.
///
/// These are extracted from `provider_options["openai"]` in `CallOptions`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenAIChatOptions {
    /// Logit bias for specific tokens (-100..100).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<HashMap<String, f64>>,

    /// Whether to return logprobs, and how many top logprobs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<LogprobsOption>,

    /// Whether to allow parallel tool calls.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,

    /// End-user identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,

    /// Reasoning effort level for reasoning models.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,

    /// Maximum completion tokens (for reasoning models).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u64>,

    /// Whether to store the output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<bool>,

    /// Request metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    /// Prediction parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prediction: Option<Value>,

    /// Service tier: "auto", "flex", "priority", "default".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,

    /// Whether to use strict JSON schema (default: true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict_json_schema: Option<bool>,

    /// Text verbosity: "low", "medium", "high".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_verbosity: Option<String>,

    /// Prompt cache key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_key: Option<String>,

    /// Prompt cache retention: "in_memory", "24h".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_retention: Option<String>,

    /// Safety identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_identifier: Option<String>,

    /// Override system message mode: "system", "developer", "remove".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_message_mode: Option<String>,

    /// Force reasoning model behavior.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_reasoning: Option<bool>,
}

/// Logprobs option: can be a boolean or a number (top_logprobs count).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LogprobsOption {
    Enabled(bool),
    TopN(u32),
}

/// Parse OpenAI options from the provider_options map.
pub fn parse_openai_options(
    provider_options: Option<&ararajuba_provider::shared::ProviderOptions>,
    provider_name: &str,
) -> OpenAIChatOptions {
    provider_options
        .and_then(|po| po.get(provider_name))
        .and_then(|obj| {
            let value = serde_json::to_value(obj).ok()?;
            serde_json::from_value(value).ok()
        })
        .unwrap_or_default()
}
