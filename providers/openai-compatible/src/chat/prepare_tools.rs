//! Tool preparation: SDK `Tool` + `ToolChoice` → OpenAI-compatible format.

use ararajuba_provider::language_model::v4::tool::{Tool, FunctionTool};
use ararajuba_provider::language_model::v4::tool_choice::ToolChoice;
use ararajuba_provider::shared::Warning;
use serde_json::{json, Value};

/// Prepared tools ready for the OpenAI API.
pub struct PreparedTools {
    /// OpenAI-format tool objects (may be empty if no tools).
    pub tools: Option<Vec<Value>>,
    /// OpenAI-format tool choice.
    pub tool_choice: Option<Value>,
    /// Warnings generated during preparation.
    pub warnings: Vec<Warning>,
}

/// Convert SDK tools and tool_choice to OpenAI-compatible format.
pub fn prepare_tools(
    tools: Option<&[Tool]>,
    tool_choice: Option<&ToolChoice>,
) -> PreparedTools {
    let mut warnings = Vec::new();

    let openai_tools: Option<Vec<Value>> = tools.map(|tools| {
        tools
            .iter()
            .filter_map(|tool| match tool {
                Tool::Function(ft) => Some(convert_function_tool(ft)),
                Tool::Provider(_pt) => {
                    warnings.push(Warning::Other {
                        message: "Provider tools are not supported in OpenAI-compatible mode."
                            .into(),
                    });
                    None
                }
            })
            .collect()
    });

    // Don't send empty tools array.
    let openai_tools = openai_tools.filter(|t| !t.is_empty());

    let openai_tool_choice = tool_choice.map(|tc| match tc {
        ToolChoice::Auto => Value::String("auto".into()),
        ToolChoice::None => Value::String("none".into()),
        ToolChoice::Required => Value::String("required".into()),
        ToolChoice::Tool { tool_name } => json!({
            "type": "function",
            "function": { "name": tool_name },
        }),
    });

    PreparedTools {
        tools: openai_tools,
        tool_choice: openai_tool_choice,
        warnings,
    }
}

fn convert_function_tool(ft: &FunctionTool) -> Value {
    let mut func = json!({
        "name": ft.name,
        "parameters": ft.input_schema,
    });

    if let Some(desc) = &ft.description {
        func["description"] = Value::String(desc.clone());
    }

    if let Some(strict) = ft.strict {
        func["strict"] = Value::Bool(strict);
    }

    json!({
        "type": "function",
        "function": func,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prepare_no_tools() {
        let result = prepare_tools(None, None);
        assert!(result.tools.is_none());
        assert!(result.tool_choice.is_none());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_prepare_function_tool() {
        let tools = vec![Tool::Function(FunctionTool {
            name: "search".into(),
            description: Some("Search the web".into()),
            input_schema: serde_json::json!({"type": "object", "properties": {"q": {"type": "string"}}}),
            input_examples: None,
            strict: Some(true),
            provider_options: None,
        })];
        let result = prepare_tools(Some(&tools), Some(&ToolChoice::Required));
        let tools = result.tools.unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["type"], "function");
        assert_eq!(tools[0]["function"]["name"], "search");
        assert_eq!(tools[0]["function"]["strict"], true);
        assert_eq!(result.tool_choice.unwrap(), "required");
    }

    #[test]
    fn test_tool_choice_specific() {
        let result = prepare_tools(None, Some(&ToolChoice::Tool { tool_name: "calc".into() }));
        let tc = result.tool_choice.unwrap();
        assert_eq!(tc["type"], "function");
        assert_eq!(tc["function"]["name"], "calc");
    }
}
