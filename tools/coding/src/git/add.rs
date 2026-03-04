//! `git_add` tool — stage files.

use ararajuba_core::tools::tool::{tool, ToolDef};
use git2::Repository;
use serde_json::json;
use std::path::Path;

/// Create the `git_add` tool.
///
/// Stages the listed files. Use `"."` to stage all changes.
pub fn git_add_tool() -> ToolDef {
    tool("git_add")
        .description("Stage files for commit. Use [\".\"] to stage all changes.")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "files": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Files to stage (relative to repo root)"
                }
            },
            "required": ["files"]
        }))
        .execute(|input| async move {
            let files = input["files"]
                .as_array()
                .ok_or_else(|| "missing required field: files".to_string())?;

            let repo = Repository::discover(".")
                .map_err(|e| format!("failed to open repository: {e}"))?;

            let mut index = repo
                .index()
                .map_err(|e| format!("failed to get index: {e}"))?;

            let mut staged = Vec::new();

            for f in files {
                let file_str = f
                    .as_str()
                    .ok_or_else(|| "file entry must be a string".to_string())?;

                if file_str == "." {
                    index
                        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
                        .map_err(|e| format!("failed to add all: {e}"))?;
                    staged.push(".".to_string());
                } else {
                    index
                        .add_path(Path::new(file_str))
                        .map_err(|e| format!("failed to add {file_str}: {e}"))?;
                    staged.push(file_str.to_string());
                }
            }

            index
                .write()
                .map_err(|e| format!("failed to write index: {e}"))?;

            Ok(json!({ "staged": staged }))
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_metadata() {
        let t = git_add_tool();
        assert_eq!(t.name, "git_add");
        assert!(t.execute.is_some());
    }
}
