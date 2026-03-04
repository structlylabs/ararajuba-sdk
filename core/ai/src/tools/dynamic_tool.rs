//! Dynamic tool calls — tool calls for tools not registered in the `ToolSet`.
//!
//! When a model invokes a tool name that is not present in the static tool set
//! (e.g., a provider-native tool like OpenAI's web search or code interpreter),
//! the SDK creates a `DynamicToolCall` instead of failing.
//!
//! The `dynamic_tool()` builder creates a `ToolDef` that accepts **any** input
//! (empty JSON schema) and delegates execution to a user-provided closure.

use crate::tools::tool::ToolBuilder;
use serde::{Deserialize, Serialize};

/// A tool call where the tool was not found in the static `ToolSet`.
///
/// Dynamic tools may be provider-managed tools whose schemas are not known
/// to the SDK at compile time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicToolCall {
    /// Unique ID for this tool call.
    pub tool_call_id: String,
    /// Name of the tool that was called.
    pub tool_name: String,
    /// The raw input (may not be valid JSON if parsing failed).
    pub input: serde_json::Value,
    /// Whether the input was invalid (e.g., unparseable JSON).
    #[serde(default)]
    pub invalid: bool,
    /// Error message if the tool call was malformed.
    #[serde(default)]
    pub error: Option<String>,
}

/// The result of a dynamic tool execution.
///
/// Dynamic tool results are not produced by SDK-managed execution — they must
/// be supplied externally (e.g., by the provider or by an `on_dynamic_tool_call`
/// callback).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicToolResult {
    /// The tool call ID this result corresponds to.
    pub tool_call_id: String,
    /// Name of the tool.
    pub tool_name: String,
    /// The output value.
    pub result: serde_json::Value,
    /// Whether this result represents an error.
    #[serde(default)]
    pub is_error: bool,
}

/// A unified tool call that may be either static (known tool) or dynamic
/// (provider-managed / unknown tool).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum TypedToolCall {
    /// A tool call for a tool registered in the `ToolSet`.
    Static(crate::tools::tool_call::ToolCall),
    /// A tool call for a tool not found in the `ToolSet`.
    Dynamic(DynamicToolCall),
}

impl TypedToolCall {
    /// The tool call ID regardless of variant.
    pub fn tool_call_id(&self) -> &str {
        match self {
            Self::Static(tc) => &tc.tool_call_id,
            Self::Dynamic(tc) => &tc.tool_call_id,
        }
    }

    /// The tool name regardless of variant.
    pub fn tool_name(&self) -> &str {
        match self {
            Self::Static(tc) => &tc.tool_name,
            Self::Dynamic(tc) => &tc.tool_name,
        }
    }

    /// Whether this is a dynamic tool call.
    pub fn is_dynamic(&self) -> bool {
        matches!(self, Self::Dynamic(_))
    }
}

/// Create a dynamic tool builder.
///
/// Dynamic tools accept any input (no schema validation) and are useful for
/// tools whose parameters are determined at runtime, such as provider-managed
/// tools or user-configured tools.
///
/// # Example
/// ```
/// use ararajuba_core::tools::dynamic_tool::dynamic_tool;
///
/// let tool = dynamic_tool("search")
///     .description("Search the web")
///     .execute(|input| async move {
///         Ok(serde_json::json!({"results": []}))
///     })
///     .build();
/// ```
pub fn dynamic_tool(name: impl Into<String>) -> ToolBuilder {
    ToolBuilder::new_dynamic(name)
}

impl ToolBuilder {
    /// Create a new tool builder with an empty (accept-anything) JSON schema.
    ///
    /// Internal constructor used by `dynamic_tool()`.
    pub(crate) fn new_dynamic(name: impl Into<String>) -> Self {
        crate::tools::tool::tool(name)
            .input_schema(serde_json::json!({}))
    }
}
