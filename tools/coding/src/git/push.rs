//! `git_push` tool — push to a remote. **Requires approval.**

use ararajuba_core::tools::tool::{tool, ToolDef};
use git2::Repository;
use serde_json::json;

/// Create the `git_push` tool.
///
/// Pushes the current branch (or specified branch) to a remote.
/// Has `needs_approval` set — force pushes always require approval,
/// normal pushes also require approval by default (high-risk remote op).
pub fn git_push_tool() -> ToolDef {
    tool("git_push")
        .description("Push to a remote. Requires approval (especially for force push).")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "remote": { "type": "string", "description": "Remote name (default: origin)" },
                "branch": { "type": "string", "description": "Branch to push (default: current)" },
                "force":  { "type": "boolean", "description": "Force push (default false)" }
            }
        }))
        .execute(|input| async move {
            let remote_name = input["remote"].as_str().unwrap_or("origin");
            let force = input["force"].as_bool().unwrap_or(false);

            let repo = Repository::discover(".")
                .map_err(|e| format!("failed to open repository: {e}"))?;

            let branch_name = if let Some(b) = input["branch"].as_str() {
                b.to_string()
            } else {
                repo.head()
                    .ok()
                    .and_then(|h| h.shorthand().map(String::from))
                    .ok_or_else(|| "cannot determine current branch".to_string())?
            };

            let mut remote = repo
                .find_remote(remote_name)
                .map_err(|e| format!("remote not found: {e}"))?;

            let refspec = if force {
                format!("+refs/heads/{branch_name}:refs/heads/{branch_name}")
            } else {
                format!("refs/heads/{branch_name}:refs/heads/{branch_name}")
            };

            remote
                .push(&[&refspec], None)
                .map_err(|e| format!("push failed: {e}"))?;

            Ok(json!({ "ok": true }))
        })
        .needs_approval(|_input| true)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_metadata() {
        let t = git_push_tool();
        assert_eq!(t.name, "git_push");
        assert!(t.execute.is_some());
        assert!(t.needs_approval.is_some());
    }
}
