//! `UIMessageChunk` — typed events for the UI message stream protocol.
//!
//! Each variant corresponds to a server→client event that frontends can
//! consume to build chat UIs. The chunks are serialised as JSON and
//! delivered over Server-Sent Events (SSE).

use serde::{Deserialize, Serialize};

/// A unique content part ID within a UI message.
pub type PartId = String;

/// A chunk in the UI message stream.
///
/// Mirrors the TS SDK's `UIMessageChunk` discriminated union — each variant
/// has a `type` tag when serialised.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum UIMessageChunk {
    // ── Message lifecycle ────────────────────────────────────────────────
    /// Start of a new assistant message.
    #[serde(rename = "start")]
    Start {
        /// The message ID.
        message_id: String,
    },
    /// The assistant message is complete.
    #[serde(rename = "finish")]
    Finish {
        /// The message ID.
        message_id: String,
        /// Overall finish reason.
        finish_reason: String,
    },
    /// The stream was aborted.
    #[serde(rename = "abort")]
    Abort {
        /// The message ID.
        message_id: String,
    },
    /// An error occurred.
    #[serde(rename = "error")]
    Error {
        /// Human-readable error text.
        error: String,
    },

    // ── Step boundaries ─────────────────────────────────────────────────
    /// Start of a model invocation step.
    #[serde(rename = "start-step")]
    StartStep {
        /// The message ID.
        message_id: String,
    },
    /// End of a model invocation step.
    #[serde(rename = "finish-step")]
    FinishStep {
        /// The message ID.
        message_id: String,
        /// Finish reason for this step.
        finish_reason: String,
    },

    // ── Text content ────────────────────────────────────────────────────
    /// Start of a text content part.
    #[serde(rename = "text-start")]
    TextStart {
        /// Content part ID.
        id: PartId,
    },
    /// A text delta.
    #[serde(rename = "text-delta")]
    TextDelta {
        /// Content part ID.
        id: PartId,
        /// The text delta.
        delta: String,
    },
    /// End of a text content part.
    #[serde(rename = "text-end")]
    TextEnd {
        /// Content part ID.
        id: PartId,
    },

    // ── Reasoning content ───────────────────────────────────────────────
    /// Start of a reasoning/thinking content part.
    #[serde(rename = "reasoning-start")]
    ReasoningStart {
        /// Content part ID.
        id: PartId,
    },
    /// A reasoning text delta.
    #[serde(rename = "reasoning-delta")]
    ReasoningDelta {
        /// Content part ID.
        id: PartId,
        /// The reasoning delta.
        delta: String,
    },
    /// End of a reasoning content part.
    #[serde(rename = "reasoning-end")]
    ReasoningEnd {
        /// Content part ID.
        id: PartId,
    },

    // ── Tool calls ──────────────────────────────────────────────────────
    /// Start of a tool call input.
    #[serde(rename = "tool-input-start")]
    ToolInputStart {
        /// Content part ID.
        id: PartId,
        /// Tool call ID.
        tool_call_id: String,
        /// Tool name.
        tool_name: String,
    },
    /// A delta of the tool input (partial JSON).
    #[serde(rename = "tool-input-delta")]
    ToolInputDelta {
        /// Content part ID.
        id: PartId,
        /// Partial input delta.
        delta: String,
    },
    /// Tool call input is fully available.
    #[serde(rename = "tool-input-available")]
    ToolInputAvailable {
        /// Content part ID.
        id: PartId,
        /// Complete input.
        input: serde_json::Value,
    },

    // ── Tool results ────────────────────────────────────────────────────
    /// Tool output is available.
    #[serde(rename = "tool-output-available")]
    ToolOutputAvailable {
        /// Content part ID (matches tool-input-start).
        id: PartId,
        /// The tool output.
        output: serde_json::Value,
    },
    /// Tool execution errored.
    #[serde(rename = "tool-output-error")]
    ToolOutputError {
        /// Content part ID.
        id: PartId,
        /// Error message.
        error: String,
    },
    /// Tool execution was denied (approval flow).
    #[serde(rename = "tool-output-denied")]
    ToolOutputDenied {
        /// Content part ID.
        id: PartId,
        /// Reason for denial.
        reason: Option<String>,
    },

    // ── Tool approval ───────────────────────────────────────────────────
    /// Request for tool execution approval.
    #[serde(rename = "tool-approval-request")]
    ToolApprovalRequest {
        /// Content part ID.
        id: PartId,
        /// Approval request ID.
        approval_id: String,
        /// Tool call ID.
        tool_call_id: String,
        /// Tool name.
        tool_name: String,
        /// Tool input.
        input: serde_json::Value,
    },

    // ── Sources / attachments ───────────────────────────────────────────
    /// A source URL reference.
    #[serde(rename = "source-url")]
    SourceUrl {
        /// Content part ID.
        id: PartId,
        /// The URL.
        url: String,
        /// Optional title.
        title: Option<String>,
    },

    // ── Metadata ────────────────────────────────────────────────────────
    /// Arbitrary metadata for the message.
    #[serde(rename = "message-metadata")]
    MessageMetadata {
        /// The message ID.
        message_id: String,
        /// Metadata key-value pairs.
        metadata: serde_json::Value,
    },
}

impl UIMessageChunk {
    /// Serialise this chunk as a JSON string suitable for an SSE `data:` field.
    pub fn to_sse_data(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Format as a complete SSE event line: `data: <json>\n\n`.
    pub fn to_sse_event(&self) -> Result<String, serde_json::Error> {
        Ok(format!("data: {}\n\n", self.to_sse_data()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_delta_serialization() {
        let chunk = UIMessageChunk::TextDelta {
            id: "part-1".to_string(),
            delta: "Hello".to_string(),
        };
        let json = serde_json::to_string(&chunk).unwrap();
        assert!(json.contains(r#""type":"text-delta""#));
        assert!(json.contains(r#""delta":"Hello""#));
    }

    #[test]
    fn test_sse_event_format() {
        let chunk = UIMessageChunk::Start {
            message_id: "msg-1".to_string(),
        };
        let sse = chunk.to_sse_event().unwrap();
        assert!(sse.starts_with("data: "));
        assert!(sse.ends_with("\n\n"));
    }

    #[test]
    fn test_roundtrip_deserialization() {
        let chunk = UIMessageChunk::ToolInputAvailable {
            id: "part-2".to_string(),
            input: serde_json::json!({"query": "test"}),
        };
        let json = serde_json::to_string(&chunk).unwrap();
        let parsed: UIMessageChunk = serde_json::from_str(&json).unwrap();
        match parsed {
            UIMessageChunk::ToolInputAvailable { id, input } => {
                assert_eq!(id, "part-2");
                assert_eq!(input["query"], "test");
            }
            _ => panic!("Wrong variant"),
        }
    }
}
