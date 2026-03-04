//! `find_files` tool — glob-based file search respecting `.gitignore`.

use ararajuba_core::tools::tool::{tool, ToolDef};
use glob::Pattern;
use ignore::WalkBuilder;
use serde_json::json;
use std::path::Path;

/// Create the `find_files` tool.
///
/// Matches files by glob pattern (e.g. `**/*.rs`). Uses the `ignore` crate to
/// respect `.gitignore` automatically.
pub fn find_files_tool() -> ToolDef {
    tool("find_files")
        .description("Find files matching a glob pattern. Respects .gitignore.")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "Glob pattern (e.g. **/*.rs)" },
                "path":    { "type": "string", "description": "Root directory (default: current dir)" }
            },
            "required": ["pattern"]
        }))
        .execute(|input| async move {
            let pattern_str = input["pattern"]
                .as_str()
                .ok_or_else(|| "missing required field: pattern".to_string())?;
            let root = input["path"].as_str().unwrap_or(".");

            let pattern = Pattern::new(pattern_str)
                .map_err(|e| format!("invalid glob pattern: {e}"))?;

            let base = Path::new(root);
            let mut files = Vec::new();

            for entry in WalkBuilder::new(base).build().flatten() {
                if entry.file_type().map(|f| f.is_file()).unwrap_or(false) {
                    let rel = entry
                        .path()
                        .strip_prefix(base)
                        .unwrap_or(entry.path())
                        .to_string_lossy();
                    if pattern.matches(&rel) {
                        files.push(rel.to_string());
                    }
                }
            }

            Ok(json!({ "files": files }))
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_metadata() {
        let t = find_files_tool();
        assert_eq!(t.name, "find_files");
        assert!(t.description.is_some());
        assert!(t.execute.is_some());
    }
}
