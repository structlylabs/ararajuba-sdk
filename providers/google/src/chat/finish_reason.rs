//! Google Generative AI finish reason mapping.

use ararajuba_provider::language_model::v4::finish_reason::{FinishReason, UnifiedFinishReason};
use ararajuba_provider::language_model::v4::usage::{InputTokens, OutputTokens, Usage};

/// Map a Google Generative AI finish reason to SDK FinishReason.
///
/// Google finish reasons are UPPERCASE strings.
pub fn map_google_finish_reason(
    raw: Option<&str>,
    has_tool_calls: bool,
) -> FinishReason {
    let unified = match raw {
        Some("STOP") => {
            if has_tool_calls {
                UnifiedFinishReason::ToolCalls
            } else {
                UnifiedFinishReason::Stop
            }
        }
        Some("MAX_TOKENS") => UnifiedFinishReason::Length,
        Some("SAFETY") | Some("RECITATION") | Some("IMAGE_SAFETY")
        | Some("BLOCKLIST") | Some("PROHIBITED_CONTENT") | Some("SPII") => {
            UnifiedFinishReason::ContentFilter
        }
        Some("MALFORMED_FUNCTION_CALL") => UnifiedFinishReason::Error,
        _ => UnifiedFinishReason::Other,
    };
    FinishReason {
        unified,
        raw: raw.map(|s| s.to_string()),
    }
}

/// Convert Google usage metadata to SDK Usage.
pub fn convert_google_usage(
    prompt_token_count: u64,
    candidates_token_count: u64,
    cached_content_token_count: Option<u64>,
    thoughts_token_count: Option<u64>,
) -> Usage {
    let cached = cached_content_token_count.unwrap_or(0);

    Usage {
        input_tokens: InputTokens {
            total: Some(prompt_token_count),
            no_cache: if cached > 0 {
                Some(prompt_token_count.saturating_sub(cached))
            } else {
                None
            },
            cache_read: if cached > 0 { Some(cached) } else { None },
            cache_write: None,
        },
        output_tokens: OutputTokens {
            total: Some(candidates_token_count),
            text: None,
            reasoning: thoughts_token_count,
        },
        raw: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_finish_reason_stop() {
        assert_eq!(
            map_google_finish_reason(Some("STOP"), false).unified,
            UnifiedFinishReason::Stop
        );
    }

    #[test]
    fn test_finish_reason_stop_with_tool_calls() {
        assert_eq!(
            map_google_finish_reason(Some("STOP"), true).unified,
            UnifiedFinishReason::ToolCalls
        );
    }

    #[test]
    fn test_finish_reason_safety() {
        assert_eq!(
            map_google_finish_reason(Some("SAFETY"), false).unified,
            UnifiedFinishReason::ContentFilter
        );
        assert_eq!(
            map_google_finish_reason(Some("RECITATION"), false).unified,
            UnifiedFinishReason::ContentFilter
        );
    }

    #[test]
    fn test_finish_reason_max_tokens() {
        assert_eq!(
            map_google_finish_reason(Some("MAX_TOKENS"), false).unified,
            UnifiedFinishReason::Length
        );
    }

    #[test]
    fn test_usage_basic() {
        let usage = convert_google_usage(100, 50, None, None);
        assert_eq!(usage.input_tokens.total, Some(100));
        assert_eq!(usage.output_tokens.total, Some(50));
        assert!(usage.input_tokens.cache_read.is_none());
    }

    #[test]
    fn test_usage_with_cache() {
        let usage = convert_google_usage(100, 50, Some(30), Some(10));
        assert_eq!(usage.input_tokens.total, Some(100));
        assert_eq!(usage.input_tokens.cache_read, Some(30));
        assert_eq!(usage.input_tokens.no_cache, Some(70));
        assert_eq!(usage.output_tokens.reasoning, Some(10));
    }
}
