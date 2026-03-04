//! Google Generative AI tool preparation.
//!
//! Key differences:
//! - Tools wrapped in `functionDeclarations` array inside `tools`.
//! - Tool choice uses `toolConfig.functionCallingConfig.mode` with AUTO/NONE/ANY.
//! - Provider tools: `googleSearch`, `codeExecution`, etc.

use ararajuba_provider::language_model::v4::tool::Tool;
use ararajuba_provider::language_model::v4::tool_choice::ToolChoice;
use ararajuba_provider::shared::Warning;
use serde_json::{json, Value};

/// Prepared tools for Google Generative AI.
pub struct PreparedGoogleTools {
    pub tools: Option<Vec<Value>>,
    pub tool_config: Option<Value>,
    pub warnings: Vec<Warning>,
}

/// Prepare tools for Google Generative AI.
pub fn prepare_google_tools(
    tools: Option<&[Tool]>,
    tool_choice: Option<&ToolChoice>,
) -> PreparedGoogleTools {
    let mut warnings = Vec::new();
    let mut function_declarations: Vec<Value> = Vec::new();
    let mut provider_tools: Vec<Value> = Vec::new();

    if let Some(tools) = tools {
        for tool in tools {
            match tool {
                Tool::Function(ft) => {
                    let mut decl = json!({
                        "name": ft.name,
                        "parameters": ft.input_schema,
                    });
                    if let Some(desc) = &ft.description {
                        decl["description"] = json!(desc);
                    }
                    function_declarations.push(decl);
                }
                Tool::Provider(pt) => {
                    // Map provider-defined tools
                    match pt.id.as_str() {
                        "google.google_search" => {
                            provider_tools.push(json!({ "googleSearch": {} }));
                        }
                        "google.code_execution" => {
                            provider_tools.push(json!({ "codeExecution": {} }));
                        }
                        "google.url_context" => {
                            provider_tools.push(json!({ "urlContext": {} }));
                        }
                        _ => {
                            warnings.push(Warning::Unsupported {
                                feature: format!("provider tool '{}'", pt.id),
                                details: Some("Unknown Google provider tool".into()),
                            });
                        }
                    }
                }
            }
        }
    }

    // Build tools array
    let mut tools_array: Vec<Value> = Vec::new();
    if !function_declarations.is_empty() {
        tools_array.push(json!({ "functionDeclarations": function_declarations }));
    }
    tools_array.extend(provider_tools);

    let tools_value = if tools_array.is_empty() {
        None
    } else {
        Some(tools_array)
    };

    // Tool config
    let tool_config = tool_choice.map(|tc| match tc {
        ToolChoice::Auto => json!({
            "functionCallingConfig": { "mode": "AUTO" }
        }),
        ToolChoice::None => json!({
            "functionCallingConfig": { "mode": "NONE" }
        }),
        ToolChoice::Required => json!({
            "functionCallingConfig": { "mode": "ANY" }
        }),
        ToolChoice::Tool { tool_name } => json!({
            "functionCallingConfig": {
                "mode": "ANY",
                "allowedFunctionNames": [tool_name],
            }
        }),
    });

    PreparedGoogleTools {
        tools: tools_value,
        tool_config,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ararajuba_provider::language_model::v4::tool::FunctionTool;

    #[test]
    fn test_function_declarations() {
        let tools = vec![Tool::Function(FunctionTool {
            name: "get_weather".into(),
            description: Some("Get weather for a city".into()),
            input_schema: json!({"type": "object", "properties": {"city": {"type": "string"}}}),
            input_examples: None,
            strict: None,
            provider_options: None,
        })];
        let result = prepare_google_tools(Some(&tools), Some(&ToolChoice::Auto));
        let tools = result.tools.unwrap();
        assert_eq!(tools.len(), 1);
        let decls = &tools[0]["functionDeclarations"];
        assert_eq!(decls[0]["name"], "get_weather");
        assert_eq!(decls[0]["description"], "Get weather for a city");
    }

    #[test]
    fn test_tool_choice_mapping() {
        let result = prepare_google_tools(None, Some(&ToolChoice::Required));
        let tc = result.tool_config.unwrap();
        assert_eq!(tc["functionCallingConfig"]["mode"], "ANY");

        let result = prepare_google_tools(None, Some(&ToolChoice::None));
        let tc = result.tool_config.unwrap();
        assert_eq!(tc["functionCallingConfig"]["mode"], "NONE");
    }

    #[test]
    fn test_specific_tool_choice() {
        let result = prepare_google_tools(
            None,
            Some(&ToolChoice::Tool {
                tool_name: "search".into(),
            }),
        );
        let tc = result.tool_config.unwrap();
        assert_eq!(tc["functionCallingConfig"]["mode"], "ANY");
        assert_eq!(
            tc["functionCallingConfig"]["allowedFunctionNames"][0],
            "search"
        );
    }
}
