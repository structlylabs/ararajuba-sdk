//! `extract_json_middleware` — extracts JSON from fenced code blocks in text output.

use crate::middleware::wrap_language_model::LanguageModelMiddleware;
use ararajuba_provider::language_model::v4::content::Content;
use ararajuba_provider::language_model::v4::generate_result::GenerateResult;

/// Creates middleware that extracts JSON from fenced code blocks (` ```json ... ``` `)
/// in text output. If the text contains a JSON code block, the text content
/// is replaced with just the extracted JSON.
pub fn extract_json_middleware() -> LanguageModelMiddleware {
    LanguageModelMiddleware {
        wrap_generate: Some(Box::new(|do_generate, _do_stream, params, _model_ref| {
            Box::pin(async move {
                let mut result: GenerateResult = do_generate(params).await?;
                result.content = extract_json_from_content(&result.content);
                Ok(result)
            })
        })),
        ..LanguageModelMiddleware::default()
    }
}

/// Extract JSON from fenced code blocks in text content.
fn extract_json_from_content(content: &[Content]) -> Vec<Content> {
    content
        .iter()
        .map(|part| match part {
            Content::Text {
                text,
                provider_metadata,
            } => {
                if let Some(json) = extract_json_from_code_block(text) {
                    Content::Text {
                        text: json,
                        provider_metadata: provider_metadata.clone(),
                    }
                } else {
                    part.clone()
                }
            }
            other => other.clone(),
        })
        .collect()
}

/// Extract JSON from a markdown fenced code block.
fn extract_json_from_code_block(text: &str) -> Option<String> {
    // Look for ```json ... ``` or ``` ... ```
    let patterns = ["```json\n", "```json\r\n", "```\n", "```\r\n"];

    for pattern in &patterns {
        if let Some(start_idx) = text.find(pattern) {
            let content_start = start_idx + pattern.len();
            if let Some(end_idx) = text[content_start..].find("```") {
                let json_text = text[content_start..content_start + end_idx].trim();
                // Validate it's actual JSON.
                if serde_json::from_str::<serde_json::Value>(json_text).is_ok() {
                    return Some(json_text.to_string());
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_from_code_block() {
        let text = "Here is the result:\n```json\n{\"name\": \"test\"}\n```\nDone.";
        assert_eq!(
            extract_json_from_code_block(text),
            Some("{\"name\": \"test\"}".to_string())
        );
    }

    #[test]
    fn test_no_code_block() {
        assert_eq!(extract_json_from_code_block("Hello world"), None);
    }

    #[test]
    fn test_invalid_json_in_code_block() {
        let text = "```json\nnot json\n```";
        assert_eq!(extract_json_from_code_block(text), None);
    }
}
