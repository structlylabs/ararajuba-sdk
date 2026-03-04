//! Token usage tracking for language model calls.

use serde::{Deserialize, Serialize};

/// Token usage for a language model call.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    /// Input token breakdown.
    pub input_tokens: InputTokens,
    /// Output token breakdown.
    pub output_tokens: OutputTokens,
    /// Raw usage data from the provider.
    pub raw: Option<serde_json::Value>,
}

/// Input token breakdown.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InputTokens {
    pub total: Option<u64>,
    pub no_cache: Option<u64>,
    pub cache_read: Option<u64>,
    pub cache_write: Option<u64>,
}

/// Output token breakdown.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OutputTokens {
    pub total: Option<u64>,
    pub text: Option<u64>,
    pub reasoning: Option<u64>,
}
