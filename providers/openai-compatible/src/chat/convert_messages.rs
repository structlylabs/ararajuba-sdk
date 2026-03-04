//! Convert SDK prompt messages → OpenAI-compatible chat message format.

use base64::Engine;
use ararajuba_provider::language_model::v4::content_part::{
    DataContent, ToolResultOutput,
};
use ararajuba_provider::language_model::v4::prompt::{
    AssistantContentPart, Message, ToolContentPart, UserContentPart,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// ── OpenAI-compatible message types ────────────────────────────────────────

/// An OpenAI-compatible chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<OpenAIContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OpenAIToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// Content can be either a plain string or an array of content parts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OpenAIContent {
    Text(String),
    Parts(Vec<Value>),
}

/// An OpenAI-compatible tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: OpenAIFunctionCall,
}

/// The function call detail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIFunctionCall {
    pub name: String,
    pub arguments: String,
}

// ── Conversion ────────────────────────────────────────────────────────────

/// Convert SDK `Prompt` (Vec<Message>) into OpenAI-compatible messages.
pub fn convert_to_openai_compatible_chat_messages(
    messages: &[Message],
) -> Vec<OpenAIMessage> {
    let mut result = Vec::new();

    for msg in messages {
        match msg {
            Message::System { content, .. } => {
                result.push(OpenAIMessage {
                    role: "system".into(),
                    content: Some(OpenAIContent::Text(content.clone())),
                    reasoning_content: None,
                    tool_calls: None,
                    tool_call_id: None,
                });
            }
            Message::User { content, .. } => {
                result.push(convert_user_message(content));
            }
            Message::Assistant { content, .. } => {
                result.push(convert_assistant_message(content));
            }
            Message::Tool { content, .. } => {
                // Each tool result becomes a separate message.
                for part in content {
                    match part {
                        ToolContentPart::ToolResult(r) => {
                            let output_str = tool_result_output_to_string(&r.output);
                            result.push(OpenAIMessage {
                                role: "tool".into(),
                                content: Some(OpenAIContent::Text(output_str)),
                                reasoning_content: None,
                                tool_calls: None,
                                tool_call_id: Some(r.tool_call_id.clone()),
                            });
                        }
                        ToolContentPart::ToolApprovalResponse(_) => {
                            // Approval responses are not sent to OpenAI — skip.
                        }
                    }
                }
            }
        }
    }

    result
}

fn convert_user_message(content: &[UserContentPart]) -> OpenAIMessage {
    // Optimisation: single text part → plain string content.
    if content.len() == 1 {
        if let UserContentPart::Text(t) = &content[0] {
            return OpenAIMessage {
                role: "user".into(),
                content: Some(OpenAIContent::Text(t.text.clone())),
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
            };
        }
    }

    let mut parts = Vec::new();
    for part in content {
        match part {
            UserContentPart::Text(t) => {
                parts.push(json!({
                    "type": "text",
                    "text": t.text,
                }));
            }
            UserContentPart::File(f) => {
                let data_url = file_to_data_url(&f.data, &f.media_type);
                if f.media_type.starts_with("image/") {
                    parts.push(json!({
                        "type": "image_url",
                        "image_url": { "url": data_url },
                    }));
                } else if f.media_type.starts_with("audio/") {
                    // Audio uses input_audio format with base64 data directly.
                    let b64 = data_content_to_base64(&f.data);
                    let format = if f.media_type.contains("mp3") || f.media_type.contains("mpeg") {
                        "mp3"
                    } else {
                        "wav"
                    };
                    parts.push(json!({
                        "type": "input_audio",
                        "input_audio": { "data": b64, "format": format },
                    }));
                } else if f.media_type == "application/pdf" {
                    parts.push(json!({
                        "type": "file",
                        "file": {
                            "filename": f.filename.as_deref().unwrap_or("file.pdf"),
                            "file_data": data_url,
                        },
                    }));
                } else if f.media_type.starts_with("text/") {
                    // Text files inline as text.
                    let text = data_content_to_string(&f.data);
                    parts.push(json!({
                        "type": "text",
                        "text": text,
                    }));
                } else {
                    // Fallback: send as file.
                    parts.push(json!({
                        "type": "file",
                        "file": {
                            "filename": f.filename.as_deref().unwrap_or("file"),
                            "file_data": data_url,
                        },
                    }));
                }
            }
        }
    }

    OpenAIMessage {
        role: "user".into(),
        content: Some(OpenAIContent::Parts(parts)),
        reasoning_content: None,
        tool_calls: None,
        tool_call_id: None,
    }
}

fn convert_assistant_message(content: &[AssistantContentPart]) -> OpenAIMessage {
    let mut text_parts = Vec::new();
    let mut reasoning_parts = Vec::new();
    let mut tool_calls = Vec::new();

    for part in content {
        match part {
            AssistantContentPart::Text(t) => {
                text_parts.push(t.text.clone());
            }
            AssistantContentPart::Reasoning(r) => {
                reasoning_parts.push(r.text.clone());
            }
            AssistantContentPart::ToolCall(tc) => {
                let args_str = if tc.input.is_string() {
                    tc.input.as_str().unwrap_or("").to_string()
                } else {
                    serde_json::to_string(&tc.input).unwrap_or_default()
                };
                tool_calls.push(OpenAIToolCall {
                    id: tc.tool_call_id.clone(),
                    call_type: "function".into(),
                    function: OpenAIFunctionCall {
                        name: tc.tool_name.clone(),
                        arguments: args_str,
                    },
                });
            }
            AssistantContentPart::File(_) | AssistantContentPart::ToolResult(_) => {
                // File / ToolResult in assistant content are not sent as OpenAI messages.
            }
        }
    }

    let content = if text_parts.is_empty() {
        None
    } else {
        Some(OpenAIContent::Text(text_parts.join("")))
    };

    let reasoning_content = if reasoning_parts.is_empty() {
        None
    } else {
        Some(reasoning_parts.join(""))
    };

    let tool_calls_opt = if tool_calls.is_empty() {
        None
    } else {
        Some(tool_calls)
    };

    OpenAIMessage {
        role: "assistant".into(),
        content,
        reasoning_content,
        tool_calls: tool_calls_opt,
        tool_call_id: None,
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn tool_result_output_to_string(output: &ToolResultOutput) -> String {
    match output {
        ToolResultOutput::Text { value } => value.clone(),
        ToolResultOutput::ErrorText { value } => value.clone(),
        ToolResultOutput::ExecutionDenied { reason } => {
            reason
                .as_deref()
                .unwrap_or("Tool execution denied.")
                .to_string()
        }
        ToolResultOutput::Json { value } => serde_json::to_string(value).unwrap_or_default(),
        ToolResultOutput::ErrorJson { value } => serde_json::to_string(value).unwrap_or_default(),
        ToolResultOutput::Content { value } => serde_json::to_string(value).unwrap_or_default(),
    }
}

fn file_to_data_url(data: &DataContent, media_type: &str) -> String {
    match data {
        DataContent::Text(s) => {
            if s.starts_with("data:") || s.starts_with("http://") || s.starts_with("https://") {
                s.clone()
            } else {
                // Assume base64
                format!("data:{media_type};base64,{s}")
            }
        }
        DataContent::Bytes(bytes) => {
            let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
            format!("data:{media_type};base64,{b64}")
        }
    }
}

fn data_content_to_base64(data: &DataContent) -> String {
    match data {
        DataContent::Text(s) => {
            if s.starts_with("data:") {
                // Extract base64 from data URL.
                s.split_once(',').map(|(_, b)| b.to_string()).unwrap_or_default()
            } else {
                s.clone()
            }
        }
        DataContent::Bytes(bytes) => {
            base64::engine::general_purpose::STANDARD.encode(bytes)
        }
    }
}

fn data_content_to_string(data: &DataContent) -> String {
    match data {
        DataContent::Text(s) => s.clone(),
        DataContent::Bytes(bytes) => String::from_utf8_lossy(bytes).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ararajuba_provider::language_model::v4::content_part::TextPart;

    #[test]
    fn test_system_message() {
        let msgs = vec![Message::System {
            content: "You are helpful.".into(),
            provider_options: None,
        }];
        let result = convert_to_openai_compatible_chat_messages(&msgs);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].role, "system");
        match &result[0].content {
            Some(OpenAIContent::Text(t)) => assert_eq!(t, "You are helpful."),
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn test_user_single_text() {
        let msgs = vec![Message::User {
            content: vec![UserContentPart::Text(TextPart {
                text: "Hello!".into(),
                provider_options: None,
            })],
            provider_options: None,
        }];
        let result = convert_to_openai_compatible_chat_messages(&msgs);
        assert_eq!(result[0].role, "user");
        // Single text → plain string, not array
        match &result[0].content {
            Some(OpenAIContent::Text(t)) => assert_eq!(t, "Hello!"),
            _ => panic!("Expected text content, not parts array"),
        }
    }

    #[test]
    fn test_assistant_with_tool_calls() {
        use ararajuba_provider::language_model::v4::content_part::ToolCallPart;
        let msgs = vec![Message::Assistant {
            content: vec![
                AssistantContentPart::Text(TextPart {
                    text: "Let me search.".into(),
                    provider_options: None,
                }),
                AssistantContentPart::ToolCall(ToolCallPart {
                    tool_call_id: "tc1".into(),
                    tool_name: "search".into(),
                    input: serde_json::json!({"q": "rust"}),
                    provider_executed: None,
                    provider_options: None,
                }),
            ],
            provider_options: None,
        }];
        let result = convert_to_openai_compatible_chat_messages(&msgs);
        assert_eq!(result[0].role, "assistant");
        assert!(result[0].tool_calls.is_some());
        let tc = &result[0].tool_calls.as_ref().unwrap()[0];
        assert_eq!(tc.function.name, "search");
        assert!(tc.function.arguments.contains("rust"));
    }

    #[test]
    fn test_tool_result_message() {
        use ararajuba_provider::language_model::v4::content_part::ToolResultPart;
        let msgs = vec![Message::Tool {
            content: vec![ToolContentPart::ToolResult(ToolResultPart {
                tool_call_id: "tc1".into(),
                tool_name: "search".into(),
                output: ToolResultOutput::Text {
                    value: "found 10 results".into(),
                },
                provider_options: None,
            })],
            provider_options: None,
        }];
        let result = convert_to_openai_compatible_chat_messages(&msgs);
        assert_eq!(result[0].role, "tool");
        assert_eq!(result[0].tool_call_id.as_deref(), Some("tc1"));
    }
}
