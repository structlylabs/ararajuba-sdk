//! `write_file` tool — write content to a file, creating parent dirs.

use ararajuba_core::tools::tool::{tool, ToolDef};
use serde_json::json;
use std::path::Path;

/// Create the `write_file` tool.
///
/// Overwrites the target file. Creates parent directories if needed.
/// For partial edits prefer `patch_file`.
pub fn write_file_tool() -> ToolDef {
    tool("write_file")
        .description("Write content to a file. Creates parent directories if needed.")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "path":    { "type": "string", "description": "File path to write" },
                "content": { "type": "string", "description": "Content to write" }
            },
            "required": ["path", "content"]
        }))
        .execute(|input| async move {
            let path = input["path"]
                .as_str()
                .ok_or_else(|| "missing required field: path".to_string())?;
            let content = input["content"]
                .as_str()
                .ok_or_else(|| "missing required field: content".to_string())?;

            // Create parent directories
            if let Some(parent) = Path::new(path).parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| format!("failed to create directories: {e}"))?;
            }

            let bytes = content.as_bytes().len();
            tokio::fs::write(path, content)
                .await
                .map_err(|e| format!("failed to write file: {e}"))?;

            Ok(json!({
                "ok": true,
                "bytes_written": bytes
            }))
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_metadata() {
        let t = write_file_tool();
        assert_eq!(t.name, "write_file");
        assert!(t.description.is_some());
        assert!(t.execute.is_some());
    }
}
