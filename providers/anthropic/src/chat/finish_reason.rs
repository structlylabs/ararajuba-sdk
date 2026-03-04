//! Anthropic stop reason → SDK FinishReason mapping.

use ararajuba_provider::language_model::v4::finish_reason::{FinishReason, UnifiedFinishReason};

/// Map an Anthropic stop_reason to SDK FinishReason.
pub fn map_anthropic_stop_reason(
    raw: Option<&str>,
    is_json_response_tool: bool,
) -> FinishReason {
    let unified = match raw {
        Some("end_turn") | Some("stop_sequence") | Some("pause_turn") => {
            UnifiedFinishReason::Stop
        }
        Some("refusal") => UnifiedFinishReason::ContentFilter,
        Some("tool_use") => {
            if is_json_response_tool {
                UnifiedFinishReason::Stop
            } else {
                UnifiedFinishReason::ToolCalls
            }
        }
        Some("max_tokens") | Some("model_context_window_exceeded") => {
            UnifiedFinishReason::Length
        }
        _ => UnifiedFinishReason::Other,
    };
    FinishReason {
        unified,
        raw: raw.map(|s| s.to_string()),
    }
}

/// Convert Anthropic usage to SDK Usage.
pub fn convert_anthropic_usage(
    input_tokens: u64,
    output_tokens: u64,
    cache_creation_tokens: Option<u64>,
    cache_read_tokens: Option<u64>,
) -> ararajuba_provider::language_model::v4::usage::Usage {
    use ararajuba_provider::language_model::v4::usage::{InputTokens, OutputTokens, Usage};

    let cache_write = cache_creation_tokens.unwrap_or(0);
    let cache_read = cache_read_tokens.unwrap_or(0);

    Usage {
        input_tokens: InputTokens {
            total: Some(input_tokens + cache_write + cache_read),
            no_cache: Some(input_tokens),
            cache_read: if cache_read > 0 {
                Some(cache_read)
            } else {
                None
            },
            cache_write: if cache_write > 0 {
                Some(cache_write)
            } else {
                None
            },
        },
        output_tokens: OutputTokens {
            total: Some(output_tokens),
            text: None,
            reasoning: None,
        },
        raw: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stop_reason_mapping() {
        assert_eq!(
            map_anthropic_stop_reason(Some("end_turn"), false).unified,
            UnifiedFinishReason::Stop
        );
        assert_eq!(
            map_anthropic_stop_reason(Some("max_tokens"), false).unified,
            UnifiedFinishReason::Length
        );
        assert_eq!(
            map_anthropic_stop_reason(Some("tool_use"), false).unified,
            UnifiedFinishReason::ToolCalls
        );
        assert_eq!(
            map_anthropic_stop_reason(Some("tool_use"), true).unified,
            UnifiedFinishReason::Stop
        );
        assert_eq!(
            map_anthropic_stop_reason(Some("refusal"), false).unified,
            UnifiedFinishReason::ContentFilter
        );
        assert_eq!(
            map_anthropic_stop_reason(None, false).unified,
            UnifiedFinishReason::Other
        );
    }

    #[test]
    fn test_usage_conversion() {
        let usage = convert_anthropic_usage(100, 50, Some(20), Some(10));
        assert_eq!(usage.input_tokens.total, Some(130)); // 100 + 20 + 10
        assert_eq!(usage.input_tokens.no_cache, Some(100));
        assert_eq!(usage.input_tokens.cache_write, Some(20));
        assert_eq!(usage.input_tokens.cache_read, Some(10));
        assert_eq!(usage.output_tokens.total, Some(50));
    }

    #[test]
    fn test_usage_no_cache() {
        let usage = convert_anthropic_usage(100, 50, None, None);
        assert_eq!(usage.input_tokens.total, Some(100));
        assert!(usage.input_tokens.cache_write.is_none());
        assert!(usage.input_tokens.cache_read.is_none());
    }
}
