//! Convert SDK prompt messages to Anthropic Messages API format.
//!
//! Key differences from OpenAI:
//! - System messages are extracted as a top-level `system` field.
//! - Tool results are merged into user blocks (strictly alternating user/assistant).
//! - Last assistant text is trimmed (Anthropic disallows trailing whitespace).
//! - Files use `image`/`document` types, not `image_url`.

use ararajuba_provider::language_model::v4::content_part::{DataContent, ToolResultOutput};
use ararajuba_provider::language_model::v4::prompt::{
    AssistantContentPart, Message, ToolContentPart, UserContentPart,
};
use serde_json::{json, Value};

/// Result of converting SDK messages to Anthropic format.
pub struct AnthropicPrompt {
    /// Top-level system content (extracted from System messages).
    pub system: Option<Vec<Value>>,
    /// Alternating user/assistant messages.
    pub messages: Vec<Value>,
}

/// Convert SDK prompt messages to Anthropic format.
pub fn convert_to_anthropic_messages_prompt(messages: &[Message]) -> AnthropicPrompt {
    let mut system_parts: Vec<Value> = Vec::new();
    let mut result_messages: Vec<Value> = Vec::new();

    // Group messages into blocks: System is extracted; Tool merges into User.
    let mut i = 0;
    while i < messages.len() {
        match &messages[i] {
            Message::System { content, .. } => {
                system_parts.push(json!({ "type": "text", "text": content }));
                i += 1;
            }
            Message::User { .. } | Message::Tool { .. } => {
                // Collect consecutive user + tool messages into one user block
                let mut content_parts: Vec<Value> = Vec::new();
                while i < messages.len() {
                    match &messages[i] {
                        Message::User { content, .. } => {
                            for part in content {
                                content_parts.push(convert_user_part(part));
                            }
                            i += 1;
                        }
                        Message::Tool { content, .. } => {
                            for part in content {
                                content_parts.push(convert_tool_part(part));
                            }
                            i += 1;
                        }
                        _ => break,
                    }
                }
                result_messages.push(json!({
                    "role": "user",
                    "content": content_parts,
                }));
            }
            Message::Assistant { .. } => {
                let mut content_parts: Vec<Value> = Vec::new();
                // Collect consecutive assistant messages
                while i < messages.len() {
                    if let Message::Assistant { content, .. } = &messages[i] {
                        for part in content {
                            content_parts.push(convert_assistant_part(part));
                        }
                        i += 1;
                    } else {
                        break;
                    }
                }
                // Trim trailing whitespace from last text part
                trim_last_text_part(&mut content_parts);

                result_messages.push(json!({
                    "role": "assistant",
                    "content": content_parts,
                }));
            }
        }
    }

    AnthropicPrompt {
        system: if system_parts.is_empty() {
            None
        } else {
            Some(system_parts)
        },
        messages: result_messages,
    }
}

fn convert_user_part(part: &UserContentPart) -> Value {
    match part {
        UserContentPart::Text(text_part) => {
            json!({ "type": "text", "text": text_part.text })
        }
        UserContentPart::File(file_part) => {
            let media_type = &file_part.media_type;
            if media_type.starts_with("image/") {
                let source = data_content_to_source(&file_part.data, media_type);
                json!({ "type": "image", "source": source })
            } else if media_type == "application/pdf" {
                let source = data_content_to_source(&file_part.data, media_type);
                json!({ "type": "document", "source": source })
            } else if media_type == "text/plain" {
                let text = data_content_to_string(&file_part.data);
                json!({
                    "type": "document",
                    "source": {
                        "type": "text",
                        "media_type": "text/plain",
                        "data": text,
                    }
                })
            } else {
                // Fallback: base64 document
                let source = data_content_to_source(&file_part.data, media_type);
                json!({ "type": "document", "source": source })
            }
        }
    }
}

fn convert_tool_part(part: &ToolContentPart) -> Value {
    match part {
        ToolContentPart::ToolResult(tr) => {
            let is_error = matches!(&tr.output, ToolResultOutput::ErrorText { .. } | ToolResultOutput::ErrorJson { .. });
            let content = tool_result_output_to_string(&tr.output);
            let mut obj = json!({
                "type": "tool_result",
                "tool_use_id": tr.tool_call_id,
                "content": content,
            });
            if is_error {
                obj["is_error"] = json!(true);
            }
            obj
        }
        ToolContentPart::ToolApprovalResponse(tar) => {
            // Tool approval responses aren't directly supported in Anthropic format;
            // map to a tool_result with the decision
            json!({
                "type": "tool_result",
                "tool_use_id": tar.approval_id,
                "content": if tar.approved { "approved" } else { "denied" },
            })
        }
    }
}

fn convert_assistant_part(part: &AssistantContentPart) -> Value {
    match part {
        AssistantContentPart::Text(text_part) => {
            json!({ "type": "text", "text": text_part.text })
        }
        AssistantContentPart::Reasoning(r) => {
            // Check for signature in provider_options
            let signature = r.provider_options.as_ref()
                .and_then(|po| po.get("anthropic"))
                .and_then(|a| a.get("signature"))
                .and_then(|v| v.as_str().or_else(|| v.as_object().and_then(|_| None)));

            if let Some(sig) = signature {
                json!({
                    "type": "thinking",
                    "thinking": r.text,
                    "signature": sig,
                })
            } else {
                json!({
                    "type": "thinking",
                    "thinking": r.text,
                    "signature": "",
                })
            }
        }
        AssistantContentPart::ToolCall(tc) => {
            json!({
                "type": "tool_use",
                "id": tc.tool_call_id,
                "name": tc.tool_name,
                "input": tc.input,
            })
        }
        AssistantContentPart::File(f) => {
            let source = data_content_to_source(&f.data, &f.media_type);
            json!({ "type": "image", "source": source })
        }
        AssistantContentPart::ToolResult(_) => {
            // Tool results in assistant messages are uncommon; skip
            json!({ "type": "text", "text": "" })
        }
    }
}

/// Trim trailing whitespace from the last text block (Anthropic requirement).
fn trim_last_text_part(parts: &mut Vec<Value>) {
    if let Some(last) = parts.last_mut() {
        if last.get("type").and_then(|v| v.as_str()) == Some("text") {
            if let Some(text) = last.get("text").and_then(|v| v.as_str()) {
                let trimmed = text.trim_end().to_string();
                last["text"] = json!(trimmed);
            }
        }
    }
}

fn data_content_to_source(data: &DataContent, media_type: &str) -> Value {
    match data {
        DataContent::Text(text) => {
            // If it looks like a URL, use url source
            if text.starts_with("http://") || text.starts_with("https://") {
                json!({
                    "type": "url",
                    "url": text,
                })
            } else {
                // Assume base64
                json!({
                    "type": "base64",
                    "media_type": media_type,
                    "data": text,
                })
            }
        }
        DataContent::Bytes(bytes) => {
            use base64::Engine;
            let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
            json!({
                "type": "base64",
                "media_type": media_type,
                "data": encoded,
            })
        }
    }
}

fn data_content_to_string(data: &DataContent) -> String {
    match data {
        DataContent::Text(t) => t.clone(),
        DataContent::Bytes(b) => String::from_utf8_lossy(b).to_string(),
    }
}

fn tool_result_output_to_string(output: &ToolResultOutput) -> String {
    match output {
        ToolResultOutput::Text { value } => value.clone(),
        ToolResultOutput::Json { value } => serde_json::to_string(value).unwrap_or_default(),
        ToolResultOutput::ExecutionDenied { reason } => {
            reason.clone().unwrap_or_else(|| "Tool execution denied.".into())
        }
        ToolResultOutput::ErrorText { value } => value.clone(),
        ToolResultOutput::ErrorJson { value } => serde_json::to_string(value).unwrap_or_default(),
        ToolResultOutput::Content { value } => serde_json::to_string(value).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ararajuba_provider::language_model::v4::content_part::{TextPart, ToolCallPart, ToolResultPart};
    use ararajuba_provider::language_model::v4::prompt::AssistantContentPart;

    #[test]
    fn test_system_extracted_to_top_level() {
        let messages = vec![
            Message::System {
                content: "You are helpful".into(),
                provider_options: None,
            },
            Message::User {
                content: vec![UserContentPart::Text(TextPart {
                    text: "Hello".into(),
                    provider_options: None,
                })],
                provider_options: None,
            },
        ];
        let result = convert_to_anthropic_messages_prompt(&messages);
        assert!(result.system.is_some());
        let sys = result.system.unwrap();
        assert_eq!(sys.len(), 1);
        assert_eq!(sys[0]["text"], "You are helpful");
        assert_eq!(result.messages.len(), 1);
        assert_eq!(result.messages[0]["role"], "user");
    }

    #[test]
    fn test_tool_messages_merged_into_user() {
        let messages = vec![
            Message::Assistant {
                content: vec![AssistantContentPart::ToolCall(ToolCallPart {
                    tool_call_id: "tc1".into(),
                    tool_name: "get_weather".into(),
                    input: json!({"city": "London"}),
                    provider_executed: None,
                    provider_options: None,
                })],
                provider_options: None,
            },
            Message::Tool {
                content: vec![ToolContentPart::ToolResult(ToolResultPart {
                    tool_call_id: "tc1".into(),
                    tool_name: "get_weather".into(),
                    output: ToolResultOutput::Text {
                        value: "Sunny, 22°C".into(),
                    },
                    provider_options: None,
                })],
                provider_options: None,
            },
        ];
        let result = convert_to_anthropic_messages_prompt(&messages);
        assert_eq!(result.messages.len(), 2);
        assert_eq!(result.messages[0]["role"], "assistant");
        assert_eq!(result.messages[1]["role"], "user");
        let user_content = result.messages[1]["content"].as_array().unwrap();
        assert_eq!(user_content[0]["type"], "tool_result");
        assert_eq!(user_content[0]["tool_use_id"], "tc1");
    }

    #[test]
    fn test_assistant_text_trimmed() {
        let messages = vec![Message::Assistant {
            content: vec![AssistantContentPart::Text(TextPart {
                text: "Hello   ".into(),
                provider_options: None,
            })],
            provider_options: None,
        }];
        let result = convert_to_anthropic_messages_prompt(&messages);
        let content = result.messages[0]["content"].as_array().unwrap();
        assert_eq!(content[0]["text"], "Hello");
    }

    #[test]
    fn test_user_image() {
        let messages = vec![Message::User {
            content: vec![UserContentPart::File(
                ararajuba_provider::language_model::v4::content_part::FilePart {
                    filename: None,
                    media_type: "image/png".into(),
                    data: DataContent::Text("aGVsbG8=".into()),
                    provider_options: None,
                },
            )],
            provider_options: None,
        }];
        let result = convert_to_anthropic_messages_prompt(&messages);
        let content = result.messages[0]["content"].as_array().unwrap();
        assert_eq!(content[0]["type"], "image");
        assert_eq!(content[0]["source"]["type"], "base64");
    }
}
