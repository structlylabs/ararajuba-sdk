//! Prompt types: messages sent to the language model.

use super::content_part::{
    FilePart, ReasoningPart, TextPart, ToolApprovalResponsePart, ToolCallPart, ToolResultPart,
};
use crate::shared::ProviderOptions;
use serde::{Deserialize, Serialize};

/// A prompt is a sequence of messages.
pub type Prompt = Vec<Message>;

/// A message in the prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role")]
pub enum Message {
    #[serde(rename = "system")]
    System {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        provider_options: Option<ProviderOptions>,
    },
    #[serde(rename = "user")]
    User {
        content: Vec<UserContentPart>,
        #[serde(skip_serializing_if = "Option::is_none")]
        provider_options: Option<ProviderOptions>,
    },
    #[serde(rename = "assistant")]
    Assistant {
        content: Vec<AssistantContentPart>,
        #[serde(skip_serializing_if = "Option::is_none")]
        provider_options: Option<ProviderOptions>,
    },
    #[serde(rename = "tool")]
    Tool {
        content: Vec<ToolContentPart>,
        #[serde(skip_serializing_if = "Option::is_none")]
        provider_options: Option<ProviderOptions>,
    },
}

/// Content parts allowed in user messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum UserContentPart {
    #[serde(rename = "text")]
    Text(TextPart),
    #[serde(rename = "file")]
    File(FilePart),
}

/// Content parts allowed in assistant messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AssistantContentPart {
    #[serde(rename = "text")]
    Text(TextPart),
    #[serde(rename = "file")]
    File(FilePart),
    #[serde(rename = "reasoning")]
    Reasoning(ReasoningPart),
    #[serde(rename = "tool-call")]
    ToolCall(ToolCallPart),
    #[serde(rename = "tool-result")]
    ToolResult(ToolResultPart),
}

/// Content parts allowed in tool messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolContentPart {
    #[serde(rename = "tool-result")]
    ToolResult(ToolResultPart),
    #[serde(rename = "tool-approval-response")]
    ToolApprovalResponse(ToolApprovalResponsePart),
}
