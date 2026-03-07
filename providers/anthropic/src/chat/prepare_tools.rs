//! Anthropic tool preparation.
//!
//! Key differences from OpenAI:
//! - Flat format: `{ name, description, input_schema }` (no wrapping `function` key).
//! - `required` → `{ type: "any" }` (not `"required"`).
//! - `none` → tools removed entirely.

use ararajuba_provider::language_model::v4::tool::Tool;
use ararajuba_provider::language_model::v4::tool_choice::ToolChoice;
use ararajuba_provider::shared::Warning;
use serde_json::{json, Value};

/// Prepared tools for Anthropic.
pub struct PreparedAnthropicTools {
    pub tools: Option<Vec<Value>>,
    pub tool_choice: Option<Value>,
    pub warnings: Vec<Warning>,
}

/// Prepare tools for Anthropic Messages API.
pub fn prepare_anthropic_tools(
    tools: Option<&[Tool]>,
    tool_choice: Option<&ToolChoice>,
) -> PreparedAnthropicTools {
    let mut warnings = Vec::new();

    // If tool_choice is None, remove all tools
    if matches!(tool_choice, Some(ToolChoice::None)) {
        return PreparedAnthropicTools {
            tools: None,
            tool_choice: None,
            warnings,
        };
    }

    let tools_value = tools.map(|t| {
        let mut list = t
            .iter()
            .filter_map(|tool| match tool {
                Tool::Function(ft) => {
                    let mut obj = json!({
                        "name": ft.name,
                        "input_schema": ft.input_schema,
                    });
                    if let Some(desc) = &ft.description {
                        obj["description"] = json!(desc);
                    }
                    if ft.strict == Some(true) {
                        obj["strict"] = json!(true);
                    }
                    Some(obj)
                }
                Tool::Provider(pt) => {
                    warnings.push(Warning::Unsupported {
                        feature: format!("provider tool '{}'", pt.id),
                        details: Some("Provider tools not implemented for Anthropic".into()),
                    });
                    None
                }
            })
            .collect::<Vec<_>>();

        // Mark the last tool with cache_control so Anthropic caches the entire
        // tools block across turns (prompt caching).
        if let Some(last) = list.last_mut() {
            last["cache_control"] = json!({ "type": "ephemeral" });
        }

        list
    });

    let tc_value = tool_choice.map(|tc| match tc {
        ToolChoice::Auto => json!({ "type": "auto" }),
        ToolChoice::Required => json!({ "type": "any" }),
        ToolChoice::None => json!({ "type": "auto" }), // won't reach (handled above)
        ToolChoice::Tool { tool_name } => {
            json!({ "type": "tool", "name": tool_name })
        }
    });

    PreparedAnthropicTools {
        tools: tools_value,
        tool_choice: tc_value,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ararajuba_provider::language_model::v4::tool::FunctionTool;

    #[test]
    fn test_prepare_function_tool() {
        let tools = vec![Tool::Function(FunctionTool {
            name: "get_weather".into(),
            description: Some("Get weather".into()),
            input_schema: json!({"type": "object"}),
            input_examples: None,
            strict: None,
            provider_options: None,
        })];
        let result = prepare_anthropic_tools(Some(&tools), Some(&ToolChoice::Auto));
        let tools = result.tools.unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "get_weather");
        assert_eq!(tools[0]["input_schema"]["type"], "object");
        assert!(tools[0].get("type").is_none()); // No wrapping type field
    }

    #[test]
    fn test_tool_choice_required_maps_to_any() {
        let result = prepare_anthropic_tools(None, Some(&ToolChoice::Required));
        let tc = result.tool_choice.unwrap();
        assert_eq!(tc["type"], "any");
    }

    #[test]
    fn test_tool_choice_none_removes_tools() {
        let tools = vec![Tool::Function(FunctionTool {
            name: "test".into(),
            description: None,
            input_schema: json!({}),
            input_examples: None,
            strict: None,
            provider_options: None,
        })];
        let result = prepare_anthropic_tools(Some(&tools), Some(&ToolChoice::None));
        assert!(result.tools.is_none());
        assert!(result.tool_choice.is_none());
    }
}
