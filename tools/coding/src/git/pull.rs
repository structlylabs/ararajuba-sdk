//! `git_pull` tool — pull from a remote. **Requires approval.**

use ararajuba_core::tools::tool::{tool, ToolDef};
use git2::Repository;
use serde_json::json;

/// Create the `git_pull` tool.
///
/// Fetches and merges from the specified remote/branch. Requires approval as
/// it modifies working directory from a remote source.
pub fn git_pull_tool() -> ToolDef {
    tool("git_pull")
        .description("Pull (fetch + merge) from a remote. Requires approval.")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "remote": { "type": "string", "description": "Remote name (default: origin)" },
                "branch": { "type": "string", "description": "Branch to pull (default: current)" }
            }
        }))
        .execute(|input| async move {
            let remote_name = input["remote"].as_str().unwrap_or("origin");

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

            // Fetch
            let mut remote = repo
                .find_remote(remote_name)
                .map_err(|e| format!("remote not found: {e}"))?;

            remote
                .fetch(&[&branch_name], None, None)
                .map_err(|e| format!("fetch failed: {e}"))?;

            // Get the fetch head
            let fetch_head = repo
                .find_reference(&format!("refs/remotes/{remote_name}/{branch_name}"))
                .map_err(|e| format!("failed to find fetched ref: {e}"))?;

            let fetch_commit = repo
                .reference_to_annotated_commit(&fetch_head)
                .map_err(|e| format!("failed to get annotated commit: {e}"))?;

            // Merge analysis
            let (merge_analysis, _) = repo
                .merge_analysis(&[&fetch_commit])
                .map_err(|e| format!("merge analysis failed: {e}"))?;

            if merge_analysis.is_up_to_date() {
                return Ok(json!({ "ok": true, "commits_pulled": 0 }));
            }

            if merge_analysis.is_fast_forward() {
                // Fast-forward
                let refname = format!("refs/heads/{branch_name}");
                let mut reference = repo
                    .find_reference(&refname)
                    .map_err(|e| format!("failed to find reference: {e}"))?;
                reference
                    .set_target(fetch_commit.id(), "fast-forward pull")
                    .map_err(|e| format!("failed to fast-forward: {e}"))?;
                repo.set_head(&refname)
                    .map_err(|e| format!("failed to set HEAD: {e}"))?;
                repo.checkout_head(Some(
                    git2::build::CheckoutBuilder::new().force(),
                ))
                .map_err(|e| format!("failed to checkout: {e}"))?;

                Ok(json!({ "ok": true, "commits_pulled": 1 }))
            } else {
                // Normal merge — perform a merge commit
                repo.merge(&[&fetch_commit], None, None)
                    .map_err(|e| format!("merge failed: {e}"))?;

                Ok(json!({ "ok": true, "commits_pulled": 1 }))
            }
        })
        .needs_approval(|_input| true)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_metadata() {
        let t = git_pull_tool();
        assert_eq!(t.name, "git_pull");
        assert!(t.execute.is_some());
        assert!(t.needs_approval.is_some());
    }
}
