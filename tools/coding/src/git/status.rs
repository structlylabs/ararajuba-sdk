//! `git_status` tool — show repository status.

use ararajuba_core::tools::tool::{tool, ToolDef};
use git2::{Repository, StatusOptions};
use serde_json::json;

/// Create the `git_status` tool.
///
/// Returns the current branch, staged/unstaged changes, and untracked files.
pub fn git_status_tool() -> ToolDef {
    tool("git_status")
        .description("Show git repository status: branch, staged, unstaged, and untracked files.")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Repository path (default: current dir)" }
            }
        }))
        .execute(|input| async move {
            let path = input["path"].as_str().unwrap_or(".");

            let repo = Repository::discover(path)
                .map_err(|e| format!("failed to open repository: {e}"))?;

            // Current branch
            let branch = repo
                .head()
                .ok()
                .and_then(|h| h.shorthand().map(String::from))
                .unwrap_or_else(|| "HEAD (detached)".into());

            let mut opts = StatusOptions::new();
            opts.include_untracked(true)
                .recurse_untracked_dirs(true);

            let statuses = repo
                .statuses(Some(&mut opts))
                .map_err(|e| format!("failed to get status: {e}"))?;

            let mut staged = Vec::new();
            let mut unstaged = Vec::new();
            let mut untracked = Vec::new();

            for entry in statuses.iter() {
                let path_str = entry.path().unwrap_or("").to_string();
                let s = entry.status();

                if s.is_index_new() {
                    staged.push(json!({"path": path_str, "status": "added"}));
                } else if s.is_index_modified() {
                    staged.push(json!({"path": path_str, "status": "modified"}));
                } else if s.is_index_deleted() {
                    staged.push(json!({"path": path_str, "status": "deleted"}));
                }

                if s.is_wt_modified() {
                    unstaged.push(json!({"path": path_str, "status": "modified"}));
                } else if s.is_wt_deleted() {
                    unstaged.push(json!({"path": path_str, "status": "deleted"}));
                }

                if s.is_wt_new() {
                    untracked.push(json!(path_str));
                }
            }

            Ok(json!({
                "branch": branch,
                "staged": staged,
                "unstaged": unstaged,
                "untracked": untracked
            }))
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_metadata() {
        let t = git_status_tool();
        assert_eq!(t.name, "git_status");
        assert!(t.execute.is_some());
        assert!(t.needs_approval.is_none());
    }
}
