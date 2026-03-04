//! `git_clone` tool — clone a repository.

use ararajuba_core::tools::tool::{tool, ToolDef};
use git2::Repository;
use serde_json::json;

/// Create the `git_clone` tool.
///
/// Clones a git repository from a URL into a local directory.
pub fn git_clone_tool() -> ToolDef {
    tool("git_clone")
        .description("Clone a git repository from a URL.")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "url":  { "type": "string", "description": "Repository URL to clone" },
                "path": { "type": "string", "description": "Local directory to clone into" }
            },
            "required": ["url", "path"]
        }))
        .execute(|input| async move {
            let url = input["url"]
                .as_str()
                .ok_or_else(|| "missing required field: url".to_string())?;
            let path = input["path"]
                .as_str()
                .ok_or_else(|| "missing required field: path".to_string())?;

            Repository::clone(url, path)
                .map_err(|e| format!("clone failed: {e}"))?;

            Ok(json!({ "ok": true }))
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_metadata() {
        let t = git_clone_tool();
        assert_eq!(t.name, "git_clone");
        assert!(t.execute.is_some());
    }
}
