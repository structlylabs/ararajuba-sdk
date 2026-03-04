//! UI message and part types — domain types for chat UIs.
//!
//! These types represent the structured content of chat messages as rendered
//! in frontend UIs. They mirror the TS SDK's `UIMessage` / `UIPart` types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A unique content part ID within a UI message.
pub type PartId = String;

/// Role of a UI message sender.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UIMessageRole {
    User,
    Assistant,
    System,
}

impl std::fmt::Display for UIMessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::Assistant => write!(f, "assistant"),
            Self::System => write!(f, "system"),
        }
    }
}

/// A UI message with typed content parts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIMessage {
    /// Unique message identifier.
    pub id: String,
    /// Role of the message sender.
    pub role: UIMessageRole,
    /// Ordered content parts.
    pub parts: Vec<UIPart>,
    /// Optional metadata attached to the message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    /// Creation timestamp (ISO 8601).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

/// A content part in a UI message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum UIPart {
    /// Plain text content.
    #[serde(rename = "text")]
    Text(TextUIPart),
    /// Reasoning / chain-of-thought content.
    #[serde(rename = "reasoning")]
    Reasoning(ReasoningUIPart),
    /// A tool invocation (with input and optional output).
    #[serde(rename = "tool-invocation")]
    Tool(ToolUIPart),
    /// A dynamic tool invocation (provider-managed tool).
    #[serde(rename = "dynamic-tool-invocation")]
    DynamicTool(DynamicToolUIPart),
    /// A file attachment.
    #[serde(rename = "file")]
    File(FileUIPart),
    /// Arbitrary structured data.
    #[serde(rename = "data")]
    Data(DataUIPart),
    /// A source URL reference.
    #[serde(rename = "source-url")]
    SourceUrl(SourceUrlUIPart),
    /// A source document reference.
    #[serde(rename = "source-document")]
    SourceDocument(SourceDocumentUIPart),
    /// A step boundary marker.
    #[serde(rename = "step-start")]
    StepStart(StepStartUIPart),
}

/// Text content part.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextUIPart {
    /// Content part ID.
    pub id: PartId,
    /// The text content.
    pub text: String,
}

/// Reasoning / thinking content part.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningUIPart {
    /// Content part ID.
    pub id: PartId,
    /// The reasoning text.
    pub reasoning: String,
    /// Optional details (e.g., token count).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Tool invocation part — represents a tool call and its result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUIPart {
    /// Content part ID.
    pub id: PartId,
    /// Tool call ID from the model.
    pub tool_call_id: String,
    /// Name of the tool.
    pub tool_name: String,
    /// Tool input arguments.
    pub input: serde_json::Value,
    /// Tool invocation state.
    pub state: ToolInvocationState,
    /// Tool output (populated when state is `Result`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
}

/// State of a tool invocation in the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolInvocationState {
    /// Tool call received, not yet executed.
    Call,
    /// Tool call received, partial result available.
    PartialCall,
    /// Tool execution complete.
    Result,
}

/// Dynamic tool invocation part.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicToolUIPart {
    /// Content part ID.
    pub id: PartId,
    /// Tool call ID.
    pub tool_call_id: String,
    /// Tool name.
    pub tool_name: String,
    /// Raw input.
    pub input: serde_json::Value,
    /// Invocation state.
    pub state: ToolInvocationState,
    /// Output (when complete).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
}

/// File content part.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileUIPart {
    /// Content part ID.
    pub id: PartId,
    /// MIME type of the file.
    pub media_type: String,
    /// File data — either a URL string or base64-encoded data.
    pub data: String,
    /// Optional filename.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
}

/// Arbitrary data part.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataUIPart {
    /// Content part ID.
    pub id: PartId,
    /// The data payload.
    pub data: serde_json::Value,
}

/// Source URL reference part.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceUrlUIPart {
    /// Content part ID.
    pub id: PartId,
    /// The source URL.
    pub url: String,
    /// Optional title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Optional provider ID (e.g., "web-search").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
}

/// Source document reference part.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceDocumentUIPart {
    /// Content part ID.
    pub id: PartId,
    /// Document ID or title.
    pub document_id: String,
    /// Document title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Optional provider ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
}

/// Step boundary marker — indicates start of a new generation step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepStartUIPart {
    /// Content part ID.
    pub id: PartId,
}

// ── Predicates ──────────────────────────────────────────────────────────────

/// Check if a UI part is a text part.
pub fn is_text_ui_part(part: &UIPart) -> bool {
    matches!(part, UIPart::Text(_))
}

/// Check if a UI part is a reasoning part.
pub fn is_reasoning_ui_part(part: &UIPart) -> bool {
    matches!(part, UIPart::Reasoning(_))
}

/// Check if a UI part is a tool invocation part.
pub fn is_tool_ui_part(part: &UIPart) -> bool {
    matches!(part, UIPart::Tool(_))
}

/// Check if a UI part is a dynamic tool invocation part.
pub fn is_dynamic_tool_ui_part(part: &UIPart) -> bool {
    matches!(part, UIPart::DynamicTool(_))
}

/// Check if a UI part is a file part.
pub fn is_file_ui_part(part: &UIPart) -> bool {
    matches!(part, UIPart::File(_))
}

/// Check if a UI part is a data part.
pub fn is_data_ui_part(part: &UIPart) -> bool {
    matches!(part, UIPart::Data(_))
}

/// Check if a UI part is a source URL part.
pub fn is_source_url_ui_part(part: &UIPart) -> bool {
    matches!(part, UIPart::SourceUrl(_))
}

/// Check if a UI part is a source document part.
pub fn is_source_document_ui_part(part: &UIPart) -> bool {
    matches!(part, UIPart::SourceDocument(_))
}

/// Check if a UI part is a step start part.
pub fn is_step_start_ui_part(part: &UIPart) -> bool {
    matches!(part, UIPart::StepStart(_))
}

/// Get the tool name from a UI part, if it is a tool or dynamic tool part.
pub fn get_tool_name(part: &UIPart) -> Option<&str> {
    match part {
        UIPart::Tool(t) => Some(&t.tool_name),
        UIPart::DynamicTool(t) => Some(&t.tool_name),
        _ => None,
    }
}

/// Extract all text from a UIMessage by concatenating text parts.
pub fn get_text_from_ui_message(message: &UIMessage) -> String {
    message
        .parts
        .iter()
        .filter_map(|part| match part {
            UIPart::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_text_part() -> UIPart {
        UIPart::Text(TextUIPart {
            id: "t1".into(),
            text: "Hello".into(),
        })
    }

    fn sample_tool_part() -> UIPart {
        UIPart::Tool(ToolUIPart {
            id: "tool1".into(),
            tool_call_id: "tc1".into(),
            tool_name: "search".into(),
            input: serde_json::json!({"q": "rust"}),
            state: ToolInvocationState::Result,
            output: Some(serde_json::json!({"results": []})),
        })
    }

    fn sample_reasoning_part() -> UIPart {
        UIPart::Reasoning(ReasoningUIPart {
            id: "r1".into(),
            reasoning: "Let me think...".into(),
            details: None,
        })
    }

    #[test]
    fn test_is_text_ui_part() {
        assert!(is_text_ui_part(&sample_text_part()));
        assert!(!is_text_ui_part(&sample_tool_part()));
    }

    #[test]
    fn test_is_reasoning_ui_part() {
        assert!(is_reasoning_ui_part(&sample_reasoning_part()));
        assert!(!is_reasoning_ui_part(&sample_text_part()));
    }

    #[test]
    fn test_is_tool_ui_part() {
        assert!(is_tool_ui_part(&sample_tool_part()));
        assert!(!is_tool_ui_part(&sample_text_part()));
    }

    #[test]
    fn test_get_tool_name() {
        assert_eq!(get_tool_name(&sample_tool_part()), Some("search"));
        assert_eq!(get_tool_name(&sample_text_part()), None);
    }

    #[test]
    fn test_get_text_from_message() {
        let msg = UIMessage {
            id: "msg1".into(),
            role: UIMessageRole::Assistant,
            parts: vec![
                sample_text_part(),
                sample_tool_part(),
                UIPart::Text(TextUIPart {
                    id: "t2".into(),
                    text: " World".into(),
                }),
            ],
            metadata: None,
            created_at: None,
        };
        assert_eq!(get_text_from_ui_message(&msg), "Hello World");
    }

    #[test]
    fn test_ui_message_serialization() {
        let msg = UIMessage {
            id: "msg1".into(),
            role: UIMessageRole::Assistant,
            parts: vec![sample_text_part()],
            metadata: None,
            created_at: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"assistant\""));
        assert!(json.contains("\"type\":\"text\""));

        let parsed: UIMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "msg1");
        assert_eq!(parsed.parts.len(), 1);
    }

    #[test]
    fn test_tool_invocation_state_serialization() {
        let json = serde_json::to_string(&ToolInvocationState::PartialCall).unwrap();
        assert_eq!(json, "\"partial-call\"");

        let parsed: ToolInvocationState = serde_json::from_str("\"result\"").unwrap();
        assert_eq!(parsed, ToolInvocationState::Result);
    }

    #[test]
    fn test_ui_message_role_display() {
        assert_eq!(UIMessageRole::User.to_string(), "user");
        assert_eq!(UIMessageRole::Assistant.to_string(), "assistant");
        assert_eq!(UIMessageRole::System.to_string(), "system");
    }
}
