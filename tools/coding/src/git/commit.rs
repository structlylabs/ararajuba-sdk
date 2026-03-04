//! `git_commit` tool — commit staged changes.

use ararajuba_core::tools::tool::{tool, ToolDef};
use git2::Repository;
use serde_json::json;

/// Create the `git_commit` tool.
///
/// Commits currently staged changes using the supplied message.
/// Reads author/email from git config.
pub fn git_commit_tool() -> ToolDef {
    tool("git_commit")
        .description("Commit staged changes with a message.")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "message": { "type": "string", "description": "Commit message" }
            },
            "required": ["message"]
        }))
        .execute(|input| async move {
            let message = input["message"]
                .as_str()
                .ok_or_else(|| "missing required field: message".to_string())?;

            let repo = Repository::discover(".")
                .map_err(|e| format!("failed to open repository: {e}"))?;

            let sig = repo
                .signature()
                .map_err(|e| format!("failed to get signature (set user.name/email): {e}"))?;

            let mut index = repo
                .index()
                .map_err(|e| format!("failed to get index: {e}"))?;

            let tree_id = index
                .write_tree()
                .map_err(|e| format!("failed to write tree: {e}"))?;

            let tree = repo
                .find_tree(tree_id)
                .map_err(|e| format!("failed to find tree: {e}"))?;

            // Get parent commit (if any — handles initial commit)
            let parents: Vec<git2::Commit> = if let Ok(head) = repo.head() {
                vec![head
                    .peel_to_commit()
                    .map_err(|e| format!("failed to peel HEAD to commit: {e}"))?]
            } else {
                vec![]
            };
            let parent_refs: Vec<&git2::Commit> = parents.iter().collect();

            let oid = repo
                .commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)
                .map_err(|e| format!("failed to commit: {e}"))?;

            let hash = oid.to_string();
            let short = &hash[..7.min(hash.len())];

            Ok(json!({
                "hash": hash,
                "short_hash": short
            }))
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_metadata() {
        let t = git_commit_tool();
        assert_eq!(t.name, "git_commit");
        assert!(t.execute.is_some());
    }
}
