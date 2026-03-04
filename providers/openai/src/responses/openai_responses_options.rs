//! OpenAI Responses API — provider options.

use serde::{Deserialize, Serialize};

/// Provider-specific options for the OpenAI Responses API.
///
/// Pass under `provider_options["openai"]` as JSON.
///
/// ```json
/// {
///     "previous_response_id": "resp_abc123",
///     "store": true,
///     "include": ["reasoning.encrypted_content"],
///     "truncation": "auto",
///     "conversation": "conv_123",
///     "reasoning_summary": "auto",
///     "max_tool_calls": 10
/// }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpenAIResponsesOptions {
    /// Continue a conversation by referencing a previous response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_response_id: Option<String>,

    /// Whether to persist the response in OpenAI storage (default `true`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<bool>,

    /// Additional output fields to include in the response.
    /// E.g. `["reasoning.encrypted_content"]`, `["file_search_call.results"]`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include: Option<Vec<String>>,

    /// Truncation strategy: `"auto"` or `"disabled"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncation: Option<String>,

    /// Conversation ID to append to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation: Option<String>,

    /// Reasoning summary granularity: `"auto"` | `"detailed"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_summary: Option<String>,

    /// Maximum number of tool calls per turn.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tool_calls: Option<u32>,

    /// Logprobs — whether to include log-probabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<bool>,

    /// Top logprobs — number of top logprobs to return per token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<u32>,
}
