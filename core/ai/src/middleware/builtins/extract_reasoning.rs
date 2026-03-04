//! `extract_reasoning_middleware` — strips `<thinking>` tags from model output
//! and populates the `reasoning` content field.

use crate::middleware::wrap_language_model::LanguageModelMiddleware;
use ararajuba_provider::language_model::v4::content::Content;
use ararajuba_provider::language_model::v4::generate_result::GenerateResult;
use std::sync::Arc;

/// Options for the extract reasoning middleware.
pub struct ExtractReasoningOptions {
    /// The tag name to look for (default: "thinking").
    pub tag_name: String,
    /// The separator between the start/end markers.
    pub start_tag: String,
    /// The end tag.
    pub end_tag: String,
}

impl Default for ExtractReasoningOptions {
    fn default() -> Self {
        Self {
            tag_name: "thinking".to_string(),
            start_tag: "<thinking>".to_string(),
            end_tag: "</thinking>".to_string(),
        }
    }
}

/// Creates middleware that strips `<thinking>...</thinking>` tags from text
/// content and re-emits the extracted content as `Content::Reasoning`.
pub fn extract_reasoning_middleware(
    options: Option<ExtractReasoningOptions>,
) -> LanguageModelMiddleware {
    let opts = Arc::new(options.unwrap_or_default());

    let opts_for_gen = Arc::clone(&opts);
    LanguageModelMiddleware {
        wrap_generate: Some(Box::new(move |do_generate, _do_stream, params, _model_ref| {
            let opts = Arc::clone(&opts_for_gen);
            Box::pin(async move {
                let mut result: GenerateResult = do_generate(params).await?;
                result.content = extract_reasoning_from_content(&result.content, &opts);
                Ok(result)
            })
        })),
        ..LanguageModelMiddleware::default()
    }
}

/// Extract reasoning from text content parts.
fn extract_reasoning_from_content(
    content: &[Content],
    opts: &ExtractReasoningOptions,
) -> Vec<Content> {
    let mut new_content = Vec::new();

    for part in content {
        match part {
            Content::Text { text, provider_metadata } => {
                let (clean_text, reasoning) = extract_tags(text, &opts.start_tag, &opts.end_tag);

                if let Some(reasoning_text) = reasoning {
                    new_content.push(Content::Reasoning {
                        text: reasoning_text,
                        provider_metadata: provider_metadata.clone(),
                    });
                }

                if !clean_text.trim().is_empty() {
                    new_content.push(Content::Text {
                        text: clean_text,
                        provider_metadata: provider_metadata.clone(),
                    });
                }
            }
            other => new_content.push(other.clone()),
        }
    }

    new_content
}

/// Extract text between start/end tags, returning (remaining_text, extracted_text).
fn extract_tags(text: &str, start_tag: &str, end_tag: &str) -> (String, Option<String>) {
    let mut remaining = String::new();
    let mut extracted = String::new();
    let mut search_from = 0;
    let mut found_any = false;

    while let Some(start_idx) = text[search_from..].find(start_tag) {
        let abs_start = search_from + start_idx;
        remaining.push_str(&text[search_from..abs_start]);

        let content_start = abs_start + start_tag.len();
        if let Some(end_idx) = text[content_start..].find(end_tag) {
            let abs_end = content_start + end_idx;
            if found_any {
                extracted.push('\n');
            }
            extracted.push_str(&text[content_start..abs_end]);
            found_any = true;
            search_from = abs_end + end_tag.len();
        } else {
            // No closing tag found — treat the rest as reasoning.
            if found_any {
                extracted.push('\n');
            }
            extracted.push_str(&text[content_start..]);
            found_any = true;
            search_from = text.len();
            break;
        }
    }

    remaining.push_str(&text[search_from..]);

    if found_any {
        (remaining, Some(extracted))
    } else {
        (text.to_string(), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_basic_thinking() {
        let (remaining, reasoning) = extract_tags(
            "Hello <thinking>I need to think</thinking> world",
            "<thinking>",
            "</thinking>",
        );
        assert_eq!(remaining, "Hello  world");
        assert_eq!(reasoning, Some("I need to think".to_string()));
    }

    #[test]
    fn test_extract_no_tags() {
        let (remaining, reasoning) = extract_tags("Hello world", "<thinking>", "</thinking>");
        assert_eq!(remaining, "Hello world");
        assert_eq!(reasoning, None);
    }

    #[test]
    fn test_extract_multiple_tags() {
        let (remaining, reasoning) = extract_tags(
            "<thinking>first</thinking>mid<thinking>second</thinking>end",
            "<thinking>",
            "</thinking>",
        );
        assert_eq!(remaining, "midend");
        assert_eq!(reasoning, Some("first\nsecond".to_string()));
    }
}
