//! Stream part types — events emitted during streaming generation.

use super::content::Content;
use super::finish_reason::FinishReason;
use super::generate_result::ResponseMetadata;
use super::usage::Usage;
use crate::shared::{ProviderMetadata, Warning};
use serde::{Deserialize, Serialize};

/// A single event in a streaming language model response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StreamPart {
    // --- Text blocks ---
    #[serde(rename = "text-start")]
    TextStart {
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        provider_metadata: Option<ProviderMetadata>,
    },
    #[serde(rename = "text-delta")]
    TextDelta {
        id: String,
        delta: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        provider_metadata: Option<ProviderMetadata>,
    },
    #[serde(rename = "text-end")]
    TextEnd {
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        provider_metadata: Option<ProviderMetadata>,
    },

    // --- Reasoning blocks ---
    #[serde(rename = "reasoning-start")]
    ReasoningStart {
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        provider_metadata: Option<ProviderMetadata>,
    },
    #[serde(rename = "reasoning-delta")]
    ReasoningDelta {
        id: String,
        delta: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        provider_metadata: Option<ProviderMetadata>,
    },
    #[serde(rename = "reasoning-end")]
    ReasoningEnd {
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        provider_metadata: Option<ProviderMetadata>,
    },

    // --- Tool input streaming ---
    #[serde(rename = "tool-input-start")]
    ToolInputStart {
        id: String,
        tool_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        provider_metadata: Option<ProviderMetadata>,
        #[serde(skip_serializing_if = "Option::is_none")]
        provider_executed: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        dynamic: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
    },
    #[serde(rename = "tool-input-delta")]
    ToolInputDelta {
        id: String,
        delta: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        provider_metadata: Option<ProviderMetadata>,
    },
    #[serde(rename = "tool-input-end")]
    ToolInputEnd {
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        provider_metadata: Option<ProviderMetadata>,
    },

    // --- Inline content types ---
    #[serde(rename = "tool-call")]
    ToolCall {
        tool_call_id: String,
        tool_name: String,
        input: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        provider_executed: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        dynamic: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        provider_metadata: Option<ProviderMetadata>,
    },
    #[serde(rename = "tool-result")]
    ToolResult {
        tool_call_id: String,
        tool_name: String,
        result: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        provider_metadata: Option<ProviderMetadata>,
    },
    #[serde(rename = "tool-approval-request")]
    ToolApprovalRequest {
        approval_id: String,
        tool_call_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        provider_metadata: Option<ProviderMetadata>,
    },
    #[serde(rename = "file")]
    File {
        media_type: String,
        data: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        provider_metadata: Option<ProviderMetadata>,
    },
    #[serde(rename = "source")]
    Source(Content),

    // --- Lifecycle events ---
    #[serde(rename = "stream-start")]
    StreamStart { warnings: Vec<Warning> },
    #[serde(rename = "response-metadata")]
    ResponseMetadata(ResponseMetadata),
    #[serde(rename = "finish")]
    Finish {
        usage: Usage,
        finish_reason: FinishReason,
        #[serde(skip_serializing_if = "Option::is_none")]
        provider_metadata: Option<ProviderMetadata>,
    },
    #[serde(rename = "raw")]
    Raw { raw_value: serde_json::Value },
    #[serde(rename = "error")]
    Error { error: String },
}
