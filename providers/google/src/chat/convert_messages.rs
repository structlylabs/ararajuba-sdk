//! Convert SDK prompt messages to Google Generative AI (Gemini) format.
//!
//! Key differences from OpenAI/Anthropic:
//! - System messages → top-level `systemInstruction { parts: [{ text }] }`.
//! - Roles: `user` and `model` (no `assistant`/`tool`).
//! - Tool results go as `user` role with `functionResponse` parts.
//! - File parts use `inlineData` (base64) or `fileData` (URL).
//! - Reasoning parts have `thought: true`.
//! - No tool call IDs from API; generated client-side.

use ararajuba_provider::language_model::v4::content_part::{DataContent, ToolResultOutput};
use ararajuba_provider::language_model::v4::prompt::{
    AssistantContentPart, Message, ToolContentPart, UserContentPart,
};
use serde_json::{json, Value};

/// Result of converting SDK messages to Google Gemini format.
pub struct GooglePrompt {
    /// Top-level system instruction.
    pub system_instruction: Option<Value>,
    /// Contents array with user/model roles.
    pub contents: Vec<Value>,
}

/// Convert SDK prompt messages to Google Generative AI format.
pub fn convert_to_google_generative_ai_messages(messages: &[Message]) -> GooglePrompt {
    let mut system_parts: Vec<Value> = Vec::new();
    let mut contents: Vec<Value> = Vec::new();

    for msg in messages {
        match msg {
            Message::System { content, .. } => {
                system_parts.push(json!({ "text": content }));
            }
            Message::User { content, .. } => {
                let parts: Vec<Value> = content.iter().map(convert_user_part).collect();
                contents.push(json!({
                    "role": "user",
                    "parts": parts,
                }));
            }
            Message::Assistant { content, .. } => {
                let parts: Vec<Value> = content.iter().map(convert_assistant_part).collect();
                contents.push(json!({
                    "role": "model",
                    "parts": parts,
                }));
            }
            Message::Tool { content, .. } => {
                // Tool results go as user role with functionResponse parts
                let parts: Vec<Value> = content.iter().map(convert_tool_part).collect();
                contents.push(json!({
                    "role": "user",
                    "parts": parts,
                }));
            }
        }
    }

    GooglePrompt {
        system_instruction: if system_parts.is_empty() {
            None
        } else {
            Some(json!({ "parts": system_parts }))
        },
        contents,
    }
}

fn convert_user_part(part: &UserContentPart) -> Value {
    match part {
        UserContentPart::Text(text_part) => {
            json!({ "text": text_part.text })
        }
        UserContentPart::File(file_part) => {
            convert_file_data(&file_part.data, &file_part.media_type)
        }
    }
}

fn convert_assistant_part(part: &AssistantContentPart) -> Value {
    match part {
        AssistantContentPart::Text(text_part) => {
            json!({ "text": text_part.text })
        }
        AssistantContentPart::Reasoning(r) => {
            // Reasoning parts have thought: true
            let mut obj = json!({
                "text": r.text,
                "thought": true,
            });
            // Include thought signature if available
            if let Some(po) = &r.provider_options {
                if let Some(google_opts) = po.get("google") {
                    if let Some(sig) = google_opts.get("thoughtSignature") {
                        obj["thoughtSignature"] = sig.clone();
                    }
                }
            }
            obj
        }
        AssistantContentPart::ToolCall(tc) => {
            // Google uses functionCall: { name, args }
            json!({
                "functionCall": {
                    "name": tc.tool_name,
                    "args": tc.input,
                }
            })
        }
        AssistantContentPart::File(f) => {
            convert_file_data(&f.data, &f.media_type)
        }
        AssistantContentPart::ToolResult(_) => {
            // Tool results in assistant messages are uncommon
            json!({ "text": "" })
        }
    }
}

fn convert_tool_part(part: &ToolContentPart) -> Value {
    match part {
        ToolContentPart::ToolResult(tr) => {
            let output = tool_result_output_to_value(&tr.output);
            json!({
                "functionResponse": {
                    "name": tr.tool_name,
                    "response": {
                        "name": tr.tool_name,
                        "content": output,
                    }
                }
            })
        }
        ToolContentPart::ToolApprovalResponse(tar) => {
            json!({
                "functionResponse": {
                    "name": tar.approval_id,
                    "response": {
                        "name": tar.approval_id,
                        "content": if tar.approved { "approved" } else { "denied" },
                    }
                }
            })
        }
    }
}

fn convert_file_data(data: &DataContent, media_type: &str) -> Value {
    match data {
        DataContent::Text(text) => {
            if text.starts_with("http://") || text.starts_with("https://") || text.starts_with("gs://") {
                json!({
                    "fileData": {
                        "mimeType": media_type,
                        "fileUri": text,
                    }
                })
            } else {
                // base64
                json!({
                    "inlineData": {
                        "mimeType": media_type,
                        "data": text,
                    }
                })
            }
        }
        DataContent::Bytes(bytes) => {
            use base64::Engine;
            let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
            json!({
                "inlineData": {
                    "mimeType": media_type,
                    "data": encoded,
                }
            })
        }
    }
}

fn tool_result_output_to_value(output: &ToolResultOutput) -> Value {
    match output {
        ToolResultOutput::Text { value } => json!(value),
        ToolResultOutput::Json { value } => value.clone(),
        ToolResultOutput::ExecutionDenied { reason } => {
            json!(reason.clone().unwrap_or_else(|| "Tool execution denied.".into()))
        }
        ToolResultOutput::ErrorText { value } => json!({ "error": value }),
        ToolResultOutput::ErrorJson { value } => json!({ "error": value }),
        ToolResultOutput::Content { value } => serde_json::to_value(value).unwrap_or(json!(null)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ararajuba_provider::language_model::v4::content_part::{TextPart, ToolCallPart, ToolResultPart};
    use ararajuba_provider::language_model::v4::prompt::AssistantContentPart;

    #[test]
    fn test_system_as_instruction() {
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
        let result = convert_to_google_generative_ai_messages(&messages);
        assert!(result.system_instruction.is_some());
        let si = result.system_instruction.unwrap();
        assert_eq!(si["parts"][0]["text"], "You are helpful");
        assert_eq!(result.contents.len(), 1);
        assert_eq!(result.contents[0]["role"], "user");
    }

    #[test]
    fn test_assistant_maps_to_model_role() {
        let messages = vec![Message::Assistant {
            content: vec![AssistantContentPart::Text(TextPart {
                text: "Hello!".into(),
                provider_options: None,
            })],
            provider_options: None,
        }];
        let result = convert_to_google_generative_ai_messages(&messages);
        assert_eq!(result.contents[0]["role"], "model");
        assert_eq!(result.contents[0]["parts"][0]["text"], "Hello!");
    }

    #[test]
    fn test_tool_results_as_user_role() {
        let messages = vec![
            Message::Assistant {
                content: vec![AssistantContentPart::ToolCall(ToolCallPart {
                    tool_call_id: "tc1".into(),
                    tool_name: "get_weather".into(),
                    input: serde_json::json!({"city": "London"}),
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
        let result = convert_to_google_generative_ai_messages(&messages);
        assert_eq!(result.contents.len(), 2);
        assert_eq!(result.contents[0]["role"], "model");
        assert_eq!(result.contents[1]["role"], "user");
        let func_resp = &result.contents[1]["parts"][0]["functionResponse"];
        assert_eq!(func_resp["name"], "get_weather");
    }

    #[test]
    fn test_file_url() {
        let messages = vec![Message::User {
            content: vec![UserContentPart::File(
                ararajuba_provider::language_model::v4::content_part::FilePart {
                    filename: None,
                    media_type: "image/png".into(),
                    data: DataContent::Text("https://example.com/image.png".into()),
                    provider_options: None,
                },
            )],
            provider_options: None,
        }];
        let result = convert_to_google_generative_ai_messages(&messages);
        let part = &result.contents[0]["parts"][0];
        assert_eq!(part["fileData"]["fileUri"], "https://example.com/image.png");
    }

    #[test]
    fn test_inline_data() {
        let messages = vec![Message::User {
            content: vec![UserContentPart::File(
                ararajuba_provider::language_model::v4::content_part::FilePart {
                    filename: None,
                    media_type: "image/jpeg".into(),
                    data: DataContent::Text("aGVsbG8=".into()),
                    provider_options: None,
                },
            )],
            provider_options: None,
        }];
        let result = convert_to_google_generative_ai_messages(&messages);
        let part = &result.contents[0]["parts"][0];
        assert_eq!(part["inlineData"]["mimeType"], "image/jpeg");
        assert_eq!(part["inlineData"]["data"], "aGVsbG8=");
    }
}
