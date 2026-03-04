//! Tool execution loop — shared logic used by `generate_text`.
//!
//! The tool loop is implemented directly in `generate_text::generate_text`.
//! This module provides helpers for tool execution orchestration.

use crate::error::Error;
use crate::tools::tool_call::ToolCall;
use crate::tools::tool_result::ToolResult;
use crate::tools::tool_set::ToolSet;

/// Execute a batch of tool calls against a tool set.
pub async fn execute_tools(
    tool_calls: &[ToolCall],
    tools: &ToolSet,
) -> Result<Vec<ToolResult>, Error> {
    let mut results = Vec::new();

    for tc in tool_calls {
        let tool = tools.get(&tc.tool_name).ok_or_else(|| Error::ToolNotFound {
            tool_name: tc.tool_name.clone(),
        })?;

        if let Some(ref execute) = tool.execute {
            match execute(tc.input.clone()).await {
                Ok(output) => {
                    results.push(ToolResult {
                        tool_call_id: tc.tool_call_id.clone(),
                        tool_name: tc.tool_name.clone(),
                        result: output,
                        is_error: false,
                        preliminary: false,
                    });
                }
                Err(err) => {
                    results.push(ToolResult {
                        tool_call_id: tc.tool_call_id.clone(),
                        tool_name: tc.tool_name.clone(),
                        result: serde_json::Value::String(err),
                        is_error: true,
                        preliminary: false,
                    });
                }
            }
        }
    }

    Ok(results)
}
