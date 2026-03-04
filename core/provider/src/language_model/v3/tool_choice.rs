//! Tool choice options for language model calls.

use serde::{Deserialize, Serialize};

/// How the model should choose which tools to call.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolChoice {
    /// Model decides whether to call tools.
    #[serde(rename = "auto")]
    Auto,
    /// Model should not call any tools.
    #[serde(rename = "none")]
    None,
    /// Model must call at least one tool.
    #[serde(rename = "required")]
    Required,
    /// Model must call the specified tool.
    #[serde(rename = "tool")]
    Tool { tool_name: String },
}
