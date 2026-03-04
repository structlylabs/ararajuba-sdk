//! Convert between UI messages and language model messages.

use super::types::{
    FileUIPart, ToolInvocationState, UIMessage, UIMessageRole, UIPart,
};
use crate::error::Error;
use ararajuba_provider::language_model::v4::content_part::{
    DataContent, FilePart, TextPart, ToolCallPart, ToolResultOutput, ToolResultPart,
};
use ararajuba_provider::language_model::v4::prompt::{
    AssistantContentPart, Message, ToolContentPart, UserContentPart,
};

/// Convert a list of UI messages to language model messages.
///
/// Maps each `UIMessage` to the corresponding `Message` variant based on role.
/// Tool UI parts in assistant messages are converted to `ToolCall` parts,
/// and completed tool results are emitted as separate `Tool` messages.
pub fn convert_to_model_messages(ui_messages: &[UIMessage]) -> Result<Vec<Message>, Error> {
    let mut messages = Vec::new();

    for msg in ui_messages {
        match msg.role {
            UIMessageRole::System => {
                let text = msg
                    .parts
                    .iter()
                    .filter_map(|p| match p {
                        UIPart::Text(t) => Some(t.text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");
                messages.push(Message::System {
                    content: text,
                    provider_options: None,
                });
            }
            UIMessageRole::User => {
                let mut content = Vec::new();
                for part in &msg.parts {
                    match part {
                        UIPart::Text(t) => {
                            content.push(UserContentPart::Text(TextPart {
                                text: t.text.clone(),
                                provider_options: None,
                            }));
                        }
                        UIPart::File(f) => {
                            content.push(UserContentPart::File(FilePart {
                                media_type: f.media_type.clone(),
                                data: DataContent::Text(f.data.clone()),
                                filename: f.filename.clone(),
                                provider_options: None,
                            }));
                        }
                        _ => {
                            // Other part types in user messages are ignored.
                        }
                    }
                }
                if !content.is_empty() {
                    messages.push(Message::User {
                        content,
                        provider_options: None,
                    });
                }
            }
            UIMessageRole::Assistant => {
                let mut assistant_content = Vec::new();
                let mut tool_results: Vec<ToolContentPart> = Vec::new();

                for part in &msg.parts {
                    match part {
                        UIPart::Text(t) => {
                            assistant_content.push(AssistantContentPart::Text(TextPart {
                                text: t.text.clone(),
                                provider_options: None,
                            }));
                        }
                        UIPart::Tool(t) => {
                            // Add tool call to assistant message.
                            assistant_content.push(AssistantContentPart::ToolCall(ToolCallPart {
                                tool_call_id: t.tool_call_id.clone(),
                                tool_name: t.tool_name.clone(),
                                input: t.input.clone(),
                                provider_executed: None,
                                provider_options: None,
                            }));

                            // If tool has a result, add to tool results.
                            if t.state == ToolInvocationState::Result {
                                if let Some(output) = &t.output {
                                    tool_results.push(ToolContentPart::ToolResult(ToolResultPart {
                                        tool_call_id: t.tool_call_id.clone(),
                                        tool_name: t.tool_name.clone(),
                                        output: ToolResultOutput::Json { value: output.clone() },
                                        provider_options: None,
                                    }));
                                }
                            }
                        }
                        UIPart::File(f) => {
                            assistant_content.push(AssistantContentPart::File(FilePart {
                                media_type: f.media_type.clone(),
                                data: DataContent::Text(f.data.clone()),
                                filename: f.filename.clone(),
                                provider_options: None,
                            }));
                        }
                        _ => {}
                    }
                }

                if !assistant_content.is_empty() {
                    messages.push(Message::Assistant {
                        content: assistant_content,
                        provider_options: None,
                    });
                }

                // Emit tool result messages after the assistant message.
                if !tool_results.is_empty() {
                    messages.push(Message::Tool {
                        content: tool_results,
                        provider_options: None,
                    });
                }
            }
        }
    }

    Ok(messages)
}

/// Convert a list of file paths/URLs to FileUIPart instances.
///
/// Each entry should be a tuple of `(media_type, data, optional_filename)`.
pub fn convert_file_list_to_file_ui_parts(
    files: &[(String, String, Option<String>)],
) -> Vec<UIPart> {
    files
        .iter()
        .enumerate()
        .map(|(i, (media_type, data, filename))| {
            UIPart::File(FileUIPart {
                id: format!("file-{}", i + 1),
                media_type: media_type.clone(),
                data: data.clone(),
                filename: filename.clone(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::types::{TextUIPart, ToolUIPart};

    #[test]
    fn test_convert_system_message() {
        let msgs = vec![UIMessage {
            id: "m1".into(),
            role: UIMessageRole::System,
            parts: vec![UIPart::Text(TextUIPart {
                id: "t1".into(),
                text: "You are a helpful assistant.".into(),
            })],
            metadata: None,
            created_at: None,
        }];
        let result = convert_to_model_messages(&msgs).unwrap();
        assert_eq!(result.len(), 1);
        match &result[0] {
            Message::System { content, .. } => {
                assert_eq!(content, "You are a helpful assistant.");
            }
            _ => panic!("Expected System message"),
        }
    }

    #[test]
    fn test_convert_user_message() {
        let msgs = vec![UIMessage {
            id: "m1".into(),
            role: UIMessageRole::User,
            parts: vec![UIPart::Text(TextUIPart {
                id: "t1".into(),
                text: "Hello!".into(),
            })],
            metadata: None,
            created_at: None,
        }];
        let result = convert_to_model_messages(&msgs).unwrap();
        assert_eq!(result.len(), 1);
        assert!(matches!(&result[0], Message::User { .. }));
    }

    #[test]
    fn test_convert_assistant_with_tool() {
        let msgs = vec![UIMessage {
            id: "m1".into(),
            role: UIMessageRole::Assistant,
            parts: vec![
                UIPart::Text(TextUIPart {
                    id: "t1".into(),
                    text: "Let me search.".into(),
                }),
                UIPart::Tool(ToolUIPart {
                    id: "tool1".into(),
                    tool_call_id: "tc1".into(),
                    tool_name: "search".into(),
                    input: serde_json::json!({"q": "rust"}),
                    state: ToolInvocationState::Result,
                    output: Some(serde_json::json!({"results": []})),
                }),
            ],
            metadata: None,
            created_at: None,
        }];
        let result = convert_to_model_messages(&msgs).unwrap();
        // Should produce: Assistant message + Tool message
        assert_eq!(result.len(), 2);
        assert!(matches!(&result[0], Message::Assistant { .. }));
        assert!(matches!(&result[1], Message::Tool { .. }));
    }

    #[test]
    fn test_convert_file_list() {
        let files = vec![
            ("image/png".into(), "base64data".into(), Some("photo.png".into())),
        ];
        let parts = convert_file_list_to_file_ui_parts(&files);
        assert_eq!(parts.len(), 1);
        assert!(matches!(&parts[0], UIPart::File(f) if f.media_type == "image/png"));
    }
}
