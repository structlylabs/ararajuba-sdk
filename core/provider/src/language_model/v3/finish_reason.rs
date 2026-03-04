//! Finish reason for language model generation.

use serde::{Deserialize, Serialize};

/// Why the model stopped generating.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinishReason {
    /// The unified finish reason category.
    pub unified: UnifiedFinishReason,
    /// The raw finish reason string from the provider, if any.
    pub raw: Option<String>,
}

/// Unified finish reason categories.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnifiedFinishReason {
    Stop,
    Length,
    ContentFilter,
    ToolCalls,
    Error,
    Other,
}
