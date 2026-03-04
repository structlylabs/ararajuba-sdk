//! Usage conversion: OpenAI raw → SDK Usage.

use ararajuba_provider::language_model::v4::usage::{InputTokens, OutputTokens, Usage};
use serde_json::Value;

/// Convert raw OpenAI-compatible usage JSON into the SDK `Usage` struct.
pub fn convert_openai_compatible_usage(raw: Option<&Value>) -> Usage {
    let raw = match raw {
        Some(v) if v.is_object() => v,
        _ => return Usage::default(),
    };

    let prompt_tokens = raw.get("prompt_tokens").and_then(|v| v.as_u64());
    let completion_tokens = raw.get("completion_tokens").and_then(|v| v.as_u64());

    let cached_tokens = raw
        .get("prompt_tokens_details")
        .and_then(|d| d.get("cached_tokens"))
        .and_then(|v| v.as_u64());

    let reasoning_tokens = raw
        .get("completion_tokens_details")
        .and_then(|d| d.get("reasoning_tokens"))
        .and_then(|v| v.as_u64());

    Usage {
        input_tokens: InputTokens {
            total: prompt_tokens,
            no_cache: match (prompt_tokens, cached_tokens) {
                (Some(p), Some(c)) => Some(p.saturating_sub(c)),
                _ => None,
            },
            cache_read: cached_tokens,
            cache_write: None,
        },
        output_tokens: OutputTokens {
            total: completion_tokens,
            text: match (completion_tokens, reasoning_tokens) {
                (Some(c), Some(r)) => Some(c.saturating_sub(r)),
                _ => None,
            },
            reasoning: reasoning_tokens,
        },
        raw: Some(raw.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_basic_usage() {
        let raw = json!({
            "prompt_tokens": 100,
            "completion_tokens": 50,
        });
        let usage = convert_openai_compatible_usage(Some(&raw));
        assert_eq!(usage.input_tokens.total, Some(100));
        assert_eq!(usage.output_tokens.total, Some(50));
        assert!(usage.input_tokens.cache_read.is_none());
    }

    #[test]
    fn test_usage_with_details() {
        let raw = json!({
            "prompt_tokens": 100,
            "completion_tokens": 80,
            "prompt_tokens_details": { "cached_tokens": 30 },
            "completion_tokens_details": { "reasoning_tokens": 20 },
        });
        let usage = convert_openai_compatible_usage(Some(&raw));
        assert_eq!(usage.input_tokens.total, Some(100));
        assert_eq!(usage.input_tokens.cache_read, Some(30));
        assert_eq!(usage.input_tokens.no_cache, Some(70));
        assert_eq!(usage.output_tokens.total, Some(80));
        assert_eq!(usage.output_tokens.reasoning, Some(20));
        assert_eq!(usage.output_tokens.text, Some(60));
    }

    #[test]
    fn test_none_usage() {
        let usage = convert_openai_compatible_usage(None);
        assert!(usage.input_tokens.total.is_none());
        assert!(usage.output_tokens.total.is_none());
    }
}
