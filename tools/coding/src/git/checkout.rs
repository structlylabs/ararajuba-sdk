//! `git_checkout` tool — switch branches.

use ararajuba_core::tools::tool::{tool, ToolDef};
use git2::Repository;
use serde_json::json;

/// Create the `git_checkout` tool.
///
/// Switches to the given branch. Set `create: true` to create and switch
/// (equivalent to `git checkout -b`).
pub fn git_checkout_tool() -> ToolDef {
    tool("git_checkout")
        .description("Switch to a branch. Use create=true for checkout -b.")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "branch": { "type": "string", "description": "Branch to switch to" },
                "create": { "type": "boolean", "description": "Create branch first (default false)" }
            },
            "required": ["branch"]
        }))
        .execute(|input| async move {
            let branch_name = input["branch"]
                .as_str()
                .ok_or_else(|| "missing required field: branch".to_string())?;
            let create = input["create"].as_bool().unwrap_or(false);

            let repo = Repository::discover(".")
                .map_err(|e| format!("failed to open repository: {e}"))?;

            if create {
                let head_commit = repo
                    .head()
                    .and_then(|h| h.peel_to_commit())
                    .map_err(|e| format!("failed to get HEAD: {e}"))?;

                repo.branch(branch_name, &head_commit, false)
                    .map_err(|e| format!("failed to create branch: {e}"))?;
            }

            // Set HEAD to the branch
            let refname = format!("refs/heads/{branch_name}");
            repo.set_head(&refname)
                .map_err(|e| format!("failed to set HEAD: {e}"))?;

            // Checkout working directory to match
            repo.checkout_head(Some(
                git2::build::CheckoutBuilder::new().force(),
            ))
            .map_err(|e| format!("failed to checkout: {e}"))?;

            Ok(json!({
                "ok": true,
                "branch": branch_name
            }))
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_metadata() {
        let t = git_checkout_tool();
        assert_eq!(t.name, "git_checkout");
        assert!(t.execute.is_some());
    }
}
