//! `git_log` tool — show commit history.

use ararajuba_core::tools::tool::{tool, ToolDef};
use git2::Repository;
use serde_json::json;

/// Create the `git_log` tool.
///
/// Returns recent commits. Use `limit` to control how many (default 20).
/// Use `file` to show history of a specific file.
pub fn git_log_tool() -> ToolDef {
    tool("git_log")
        .description("Show git commit log. Use limit to control count, file for history of one file.")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "path":  { "type": "string",  "description": "Repository path (default: current dir)" },
                "limit": { "type": "integer", "description": "Max commits to return (default 20)" },
                "file":  { "type": "string",  "description": "Show history for a specific file" }
            }
        }))
        .execute(|input| async move {
            let path = input["path"].as_str().unwrap_or(".");
            let limit = input["limit"].as_u64().unwrap_or(20) as usize;
            let file_filter = input["file"].as_str();

            let repo = Repository::discover(path)
                .map_err(|e| format!("failed to open repository: {e}"))?;

            let mut revwalk = repo
                .revwalk()
                .map_err(|e| format!("failed to create revwalk: {e}"))?;
            revwalk.push_head().map_err(|e| format!("failed to push HEAD: {e}"))?;
            revwalk
                .set_sorting(git2::Sort::TIME)
                .map_err(|e| format!("failed to set sorting: {e}"))?;

            let mut commits = Vec::new();

            for oid_result in revwalk {
                if commits.len() >= limit {
                    break;
                }
                let oid = oid_result.map_err(|e| format!("revwalk error: {e}"))?;
                let commit = repo
                    .find_commit(oid)
                    .map_err(|e| format!("failed to find commit: {e}"))?;

                // File filter: check if any diff entry touches the file
                if let Some(file_path) = file_filter {
                    let dominated = commit_touches_file(&repo, &commit, file_path);
                    if !dominated {
                        continue;
                    }
                }

                let author = commit.author();
                let hash = oid.to_string();
                let short = &hash[..7.min(hash.len())];

                commits.push(json!({
                    "hash": hash,
                    "short_hash": short,
                    "author": author.name().unwrap_or("unknown"),
                    "date": commit.time().seconds().to_string(),
                    "message": commit.message().unwrap_or("").trim()
                }));
            }

            Ok(json!({ "commits": commits }))
        })
        .build()
}

/// Check if a commit modifies a given file path.
fn commit_touches_file(
    repo: &Repository,
    commit: &git2::Commit,
    file_path: &str,
) -> bool {
    let tree = match commit.tree() {
        Ok(t) => t,
        Err(_) => return false,
    };

    let parent_tree = commit
        .parent(0)
        .ok()
        .and_then(|p| p.tree().ok());

    let diff = repo
        .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)
        .ok();

    if let Some(d) = diff {
        for delta in d.deltas() {
            let old = delta.old_file().path().and_then(|p| p.to_str());
            let new = delta.new_file().path().and_then(|p| p.to_str());
            if old == Some(file_path) || new == Some(file_path) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_metadata() {
        let t = git_log_tool();
        assert_eq!(t.name, "git_log");
        assert!(t.execute.is_some());
        assert!(t.needs_approval.is_none());
    }
}
