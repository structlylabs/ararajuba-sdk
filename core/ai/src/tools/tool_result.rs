//! A tool result after execution.

use serde::{Deserialize, Serialize};

/// The result of executing a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// The tool call ID this result corresponds to.
    pub tool_call_id: String,
    /// Name of the tool.
    pub tool_name: String,
    /// The output value (JSON).
    pub result: serde_json::Value,
    /// Whether this result represents an error.
    pub is_error: bool,
    /// Whether this is a preliminary (intermediate) result.
    ///
    /// Preliminary results are emitted by streaming tool executions before
    /// the final result is available. They are useful for updating UIs with
    /// progress indicators but are **not** fed back to the model.
    #[serde(default)]
    pub preliminary: bool,
}
