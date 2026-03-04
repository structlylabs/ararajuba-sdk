//! Finish reason mapping: OpenAI raw → unified.

use ararajuba_provider::language_model::v4::finish_reason::{FinishReason, UnifiedFinishReason};

/// Map an OpenAI-compatible finish reason string to the unified enum.
pub fn map_openai_compatible_finish_reason(raw: Option<&str>) -> FinishReason {
    let raw_str = raw.unwrap_or("");
    let unified = match raw_str {
        "stop" => UnifiedFinishReason::Stop,
        "length" => UnifiedFinishReason::Length,
        "content_filter" => UnifiedFinishReason::ContentFilter,
        "function_call" | "tool_calls" => UnifiedFinishReason::ToolCalls,
        _ => UnifiedFinishReason::Other,
    };
    FinishReason {
        unified,
        raw: raw.map(|s| s.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_finish_reason_mapping() {
        assert_eq!(map_openai_compatible_finish_reason(Some("stop")).unified, UnifiedFinishReason::Stop);
        assert_eq!(map_openai_compatible_finish_reason(Some("length")).unified, UnifiedFinishReason::Length);
        assert_eq!(map_openai_compatible_finish_reason(Some("content_filter")).unified, UnifiedFinishReason::ContentFilter);
        assert_eq!(map_openai_compatible_finish_reason(Some("tool_calls")).unified, UnifiedFinishReason::ToolCalls);
        assert_eq!(map_openai_compatible_finish_reason(Some("function_call")).unified, UnifiedFinishReason::ToolCalls);
        assert_eq!(map_openai_compatible_finish_reason(Some("unknown")).unified, UnifiedFinishReason::Other);
        assert_eq!(map_openai_compatible_finish_reason(None).unified, UnifiedFinishReason::Other);
    }
}
