//! Finish reason types for the high-level SDK.

use serde::{Deserialize, Serialize};

/// Why the model stopped generating — simplified from the provider-level type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FinishReason {
    Stop,
    Length,
    ContentFilter,
    ToolCalls,
    Error,
    Other,
}

impl FinishReason {
    /// Convert from the provider-level unified finish reason.
    pub fn from_provider(
        reason: &ararajuba_provider::language_model::v4::finish_reason::UnifiedFinishReason,
    ) -> Self {
        use ararajuba_provider::language_model::v4::finish_reason::UnifiedFinishReason;
        match reason {
            UnifiedFinishReason::Stop => FinishReason::Stop,
            UnifiedFinishReason::Length => FinishReason::Length,
            UnifiedFinishReason::ContentFilter => FinishReason::ContentFilter,
            UnifiedFinishReason::ToolCalls => FinishReason::ToolCalls,
            UnifiedFinishReason::Error => FinishReason::Error,
            UnifiedFinishReason::Other => FinishReason::Other,
        }
    }
}
