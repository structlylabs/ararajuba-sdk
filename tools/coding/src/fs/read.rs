//! `read_file` tool — read file contents with optional line range.

use ararajuba_core::tools::tool::{tool, ToolDef};
use serde_json::json;

/// Create the `read_file` tool.
///
/// Reads a file relative to the workspace root. Supports `offset` / `limit`
/// (1-based line numbers) for reading large files in slices.
pub fn read_file_tool() -> ToolDef {
    tool("read_file")
        .description("Read the contents of a file. Use offset/limit for large files.")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "path":   { "type": "string", "description": "File path to read" },
                "offset": { "type": "integer", "description": "Start line (1-based)" },
                "limit":  { "type": "integer", "description": "Max lines to return" }
            },
            "required": ["path"]
        }))
        .execute(|input| async move {
            let path = input["path"]
                .as_str()
                .ok_or_else(|| "missing required field: path".to_string())?;

            let content = tokio::fs::read_to_string(path)
                .await
                .map_err(|e| format!("failed to read file: {e}"))?;

            let lines: Vec<&str> = content.lines().collect();
            let total = lines.len();

            let offset = input["offset"]
                .as_u64()
                .unwrap_or(1)
                .max(1) as usize
                - 1;
            let limit = input["limit"].as_u64().unwrap_or(u64::MAX) as usize;

            let end = (offset + limit).min(total);
            let slice = &lines[offset.min(total)..end];

            Ok(json!({
                "content": slice.join("\n"),
                "lines": total,
                "truncated": end < total
            }))
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_metadata() {
        let t = read_file_tool();
        assert_eq!(t.name, "read_file");
        assert!(t.description.is_some());
        assert!(t.execute.is_some());
        assert!(t.needs_approval.is_none());
    }
}
