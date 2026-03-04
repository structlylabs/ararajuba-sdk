//! `patch_file` tool — find-and-replace a unique string in a file.

use ararajuba_core::tools::tool::{tool, ToolDef};
use serde_json::json;

/// Create the `patch_file` tool.
///
/// Finds the exact occurrence of `old_string` in the file and replaces it with
/// `new_string`. Errors if the string is not found or is ambiguous (multiple
/// occurrences).
pub fn patch_file_tool() -> ToolDef {
    tool("patch_file")
        .description(
            "Find and replace an exact string in a file. \
             Errors if the old_string is not found or matches more than once.",
        )
        .input_schema(json!({
            "type": "object",
            "properties": {
                "path":       { "type": "string", "description": "File path to patch" },
                "old_string": { "type": "string", "description": "Exact text to find" },
                "new_string": { "type": "string", "description": "Replacement text" }
            },
            "required": ["path", "old_string", "new_string"]
        }))
        .execute(|input| async move {
            let path = input["path"]
                .as_str()
                .ok_or_else(|| "missing required field: path".to_string())?;
            let old_string = input["old_string"]
                .as_str()
                .ok_or_else(|| "missing required field: old_string".to_string())?;
            let new_string = input["new_string"]
                .as_str()
                .ok_or_else(|| "missing required field: new_string".to_string())?;

            let content = tokio::fs::read_to_string(path)
                .await
                .map_err(|e| format!("failed to read file: {e}"))?;

            let count = content.matches(old_string).count();
            if count == 0 {
                return Err("old_string not found in file".to_string());
            }
            if count > 1 {
                return Err(format!(
                    "old_string is ambiguous — found {count} occurrences, provide more context"
                ));
            }

            let new_content = content.replacen(old_string, new_string, 1);
            tokio::fs::write(path, &new_content)
                .await
                .map_err(|e| format!("failed to write file: {e}"))?;

            Ok(json!({
                "ok": true,
                "replaced": 1
            }))
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_metadata() {
        let t = patch_file_tool();
        assert_eq!(t.name, "patch_file");
        assert!(t.description.is_some());
        assert!(t.execute.is_some());
    }
}
