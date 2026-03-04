//! Tool definitions for language model calls.

use crate::shared::ProviderOptions;
use serde::{Deserialize, Serialize};

/// A tool that the language model can call.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Tool {
    /// A function tool with a JSON Schema for input validation.
    #[serde(rename = "function")]
    Function(FunctionTool),
    /// A provider-specific tool.
    #[serde(rename = "provider")]
    Provider(ProviderTool),
}

/// A function tool definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionTool {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// JSON Schema (JSON Schema Draft 7) defining the tool's input.
    pub input_schema: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_examples: Option<Vec<InputExample>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_options: Option<ProviderOptions>,
}

/// An example input for a function tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputExample {
    pub input: serde_json::Value,
}

/// A provider-specific tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderTool {
    /// Namespaced identifier: `provider.tool_name`.
    pub id: String,
    pub name: String,
    pub args: serde_json::Value,
}
