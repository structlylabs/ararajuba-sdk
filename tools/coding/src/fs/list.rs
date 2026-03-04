//! `list_directory` tool — list directory entries, optionally recursive.

use ararajuba_core::tools::tool::{tool, ToolDef};
use ignore::WalkBuilder;
use serde_json::json;
use std::path::Path;

/// Create the `list_directory` tool.
///
/// Lists entries in a directory. Respects `.gitignore` via the `ignore` crate.
/// Set `recursive: true` to walk subdirectories.
pub fn list_directory_tool() -> ToolDef {
    tool("list_directory")
        .description("List files and directories. Respects .gitignore.")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "path":      { "type": "string", "description": "Directory path" },
                "recursive": { "type": "boolean", "description": "Walk subdirectories (default false)" }
            },
            "required": ["path"]
        }))
        .execute(|input| async move {
            let path = input["path"]
                .as_str()
                .ok_or_else(|| "missing required field: path".to_string())?;
            let recursive = input["recursive"].as_bool().unwrap_or(false);

            let base = Path::new(path);
            if !base.is_dir() {
                return Err(format!("{path} is not a directory"));
            }

            let max_depth = if recursive { None } else { Some(1) };

            let mut entries = Vec::new();
            let mut builder = WalkBuilder::new(base);
            if let Some(d) = max_depth {
                builder.max_depth(Some(d));
            }

            for entry in builder.build().flatten() {
                // Skip the root directory itself
                if entry.path() == base {
                    continue;
                }
                let ft = entry.file_type();
                let is_dir = ft.map(|f| f.is_dir()).unwrap_or(false);
                let name = entry
                    .path()
                    .strip_prefix(base)
                    .unwrap_or(entry.path())
                    .to_string_lossy()
                    .to_string();

                let mut e = json!({
                    "name": name,
                    "type": if is_dir { "dir" } else { "file" }
                });
                if !is_dir {
                    if let Ok(meta) = entry.metadata() {
                        e["size"] = json!(meta.len());
                    }
                }
                entries.push(e);
            }

            Ok(json!({ "entries": entries }))
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_metadata() {
        let t = list_directory_tool();
        assert_eq!(t.name, "list_directory");
        assert!(t.description.is_some());
        assert!(t.execute.is_some());
    }
}
