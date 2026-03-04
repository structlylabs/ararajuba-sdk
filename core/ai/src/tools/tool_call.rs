//! A tool call made by the model.

use serde::{Deserialize, Serialize};

/// Represents a tool call made by the language model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique ID for this tool call.
    pub tool_call_id: String,
    /// Name of the tool that was called.
    pub tool_name: String,
    /// The input passed to the tool (parsed JSON).
    pub input: serde_json::Value,
}
