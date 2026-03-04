//! Content part types used in prompts (messages sent to the model).

use crate::shared::ProviderOptions;
use serde::{Deserialize, Serialize};

/// Text content part.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextPart {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_options: Option<ProviderOptions>,
}

/// Reasoning content part.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningPart {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_options: Option<ProviderOptions>,
}

/// File content part.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePart {
    /// Optional filename.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    /// File data: base64 string, URL string, or raw bytes.
    pub data: DataContent,
    /// MIME type of the file.
    pub media_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_options: Option<ProviderOptions>,
}

/// Data content — can be raw bytes, a base64/URL string.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DataContent {
    Text(String),
    Bytes(Vec<u8>),
}

/// Tool call content part.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallPart {
    pub tool_call_id: String,
    pub tool_name: String,
    pub input: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_executed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_options: Option<ProviderOptions>,
}

/// Tool result content part.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultPart {
    pub tool_call_id: String,
    pub tool_name: String,
    pub output: ToolResultOutput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_options: Option<ProviderOptions>,
}

/// Tool approval response content part.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolApprovalResponsePart {
    pub approval_id: String,
    pub approved: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_options: Option<ProviderOptions>,
}

/// Output from a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolResultOutput {
    #[serde(rename = "text")]
    Text { value: String },
    #[serde(rename = "json")]
    Json { value: serde_json::Value },
    #[serde(rename = "execution-denied")]
    ExecutionDenied { reason: Option<String> },
    #[serde(rename = "error-text")]
    ErrorText { value: String },
    #[serde(rename = "error-json")]
    ErrorJson { value: serde_json::Value },
    #[serde(rename = "content")]
    Content { value: Vec<ContentPart> },
}

/// Generic content part used in tool result output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "file-data")]
    FileData {
        data: String,
        media_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        filename: Option<String>,
    },
    #[serde(rename = "file-url")]
    FileUrl { url: String },
    #[serde(rename = "image-data")]
    ImageData { data: String, media_type: String },
    #[serde(rename = "image-url")]
    ImageUrl { url: String },
    #[serde(rename = "custom")]
    Custom {
        #[serde(skip_serializing_if = "Option::is_none")]
        provider_options: Option<ProviderOptions>,
    },
}
