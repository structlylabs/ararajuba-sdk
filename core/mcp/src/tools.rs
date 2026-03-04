//! Convert MCP tool definitions to AI SDK `Tool` objects.

use crate::types::{CallToolResult, MCPContent, MCPToolDefinition};
use ararajuba_provider::language_model::v4::tool::{FunctionTool, Tool};
use serde_json::Value;

/// Convert a list of MCP tool definitions to AI SDK `Tool` objects.
///
/// Each MCP tool becomes a `Tool::Function` with:
/// - `name` from the MCP tool name
/// - `description` from the MCP tool description or title
/// - `input_schema` from the MCP tool input schema (with `additionalProperties: false` added)
pub fn mcp_tools_to_sdk_tools(definitions: &[MCPToolDefinition]) -> Vec<Tool> {
    definitions.iter().map(mcp_tool_to_sdk_tool).collect()
}

/// Convert a single MCP tool definition to an AI SDK `Tool`.
fn mcp_tool_to_sdk_tool(def: &MCPToolDefinition) -> Tool {
    let mut schema = def.input_schema.clone();

    // Ensure additionalProperties: false for stricter validation
    if let Value::Object(ref mut map) = schema {
        if !map.contains_key("properties") {
            map.insert("properties".into(), Value::Object(Default::default()));
        }
        map.insert("additionalProperties".into(), Value::Bool(false));
    }

    let description = def
        .description
        .clone()
        .or_else(|| {
            def.annotations
                .as_ref()
                .and_then(|a| a.title.clone())
        })
        .or_else(|| def.title.clone());

    Tool::Function(FunctionTool {
        name: def.name.clone(),
        description,
        input_schema: schema,
        input_examples: None,
        strict: None,
        provider_options: None,
    })
}

/// Convert an MCP `CallToolResult` to a text representation suitable for
/// returning as a tool result.
pub fn call_tool_result_to_text(result: &CallToolResult) -> String {
    // If there's structured content, serialize it
    if let Some(ref structured) = result.structured_content {
        return serde_json::to_string(structured).unwrap_or_default();
    }

    // Convert content parts to text
    let parts: Vec<String> = result
        .content
        .iter()
        .filter_map(|c| match c {
            MCPContent::Text { text } => Some(text.clone()),
            MCPContent::Image { data, mime_type } => {
                Some(format!("[image: {mime_type}, {len} bytes]", len = data.len()))
            }
            MCPContent::Resource { resource } => {
                Some(serde_json::to_string(resource).unwrap_or_default())
            }
        })
        .collect();

    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MCPToolAnnotations;

    #[test]
    fn test_convert_single_tool() {
        let def = MCPToolDefinition {
            name: "get_weather".into(),
            title: None,
            description: Some("Get current weather for a city".into()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "city": { "type": "string" }
                }
            }),
            output_schema: None,
            annotations: None,
            meta: None,
        };

        let tools = mcp_tools_to_sdk_tools(&[def]);
        assert_eq!(tools.len(), 1);

        match &tools[0] {
            Tool::Function(ft) => {
                assert_eq!(ft.name, "get_weather");
                assert_eq!(
                    ft.description.as_deref(),
                    Some("Get current weather for a city")
                );
                // Should have additionalProperties: false
                assert_eq!(
                    ft.input_schema["additionalProperties"],
                    Value::Bool(false)
                );
            }
            _ => panic!("Expected Function tool"),
        }
    }

    #[test]
    fn test_convert_tool_with_annotation_title_fallback() {
        let def = MCPToolDefinition {
            name: "search".into(),
            title: None,
            description: None,
            input_schema: serde_json::json!({"type": "object"}),
            output_schema: None,
            annotations: Some(MCPToolAnnotations {
                title: Some("Search the web".into()),
                description: None,
                read_only_hint: None,
                destructive_hint: None,
                idempotent_hint: None,
                open_world_hint: None,
            }),
            meta: None,
        };

        let tools = mcp_tools_to_sdk_tools(&[def]);
        match &tools[0] {
            Tool::Function(ft) => {
                assert_eq!(ft.description.as_deref(), Some("Search the web"));
            }
            _ => panic!("Expected Function tool"),
        }
    }

    #[test]
    fn test_convert_empty_schema_gets_properties() {
        let def = MCPToolDefinition {
            name: "no_args".into(),
            title: None,
            description: None,
            input_schema: serde_json::json!({"type": "object"}),
            output_schema: None,
            annotations: None,
            meta: None,
        };

        let tools = mcp_tools_to_sdk_tools(&[def]);
        match &tools[0] {
            Tool::Function(ft) => {
                assert!(ft.input_schema["properties"].is_object());
            }
            _ => panic!("Expected Function tool"),
        }
    }

    #[test]
    fn test_call_tool_result_text() {
        let result = CallToolResult {
            content: vec![MCPContent::Text {
                text: "72°F in NYC".into(),
            }],
            structured_content: None,
            is_error: false,
        };
        assert_eq!(call_tool_result_to_text(&result), "72°F in NYC");
    }

    #[test]
    fn test_call_tool_result_structured() {
        let result = CallToolResult {
            content: vec![],
            structured_content: Some(serde_json::json!({"temp": 72})),
            is_error: false,
        };
        assert_eq!(call_tool_result_to_text(&result), r#"{"temp":72}"#);
    }

    #[test]
    fn test_call_tool_result_multiple_parts() {
        let result = CallToolResult {
            content: vec![
                MCPContent::Text {
                    text: "Line 1".into(),
                },
                MCPContent::Text {
                    text: "Line 2".into(),
                },
            ],
            structured_content: None,
            is_error: false,
        };
        assert_eq!(call_tool_result_to_text(&result), "Line 1\nLine 2");
    }
}
