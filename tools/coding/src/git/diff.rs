//! `git_diff` tool — show diffs (staged or unstaged).

use ararajuba_core::tools::tool::{tool, ToolDef};
use git2::{DiffOptions, Repository};
use serde_json::json;

/// Create the `git_diff` tool.
///
/// - `staged: false` (default) → working tree vs index
/// - `staged: true` → index vs HEAD
/// - `file` → diff a single file
pub fn git_diff_tool() -> ToolDef {
    tool("git_diff")
        .description("Show git diff. Use staged=true for index vs HEAD.")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "path":   { "type": "string", "description": "Repository path (default: current dir)" },
                "staged": { "type": "boolean", "description": "Diff staged changes vs HEAD (default false)" },
                "file":   { "type": "string", "description": "Diff a single file" }
            }
        }))
        .execute(|input| async move {
            let path = input["path"].as_str().unwrap_or(".");
            let staged = input["staged"].as_bool().unwrap_or(false);
            let file_filter = input["file"].as_str();

            let repo = Repository::discover(path)
                .map_err(|e| format!("failed to open repository: {e}"))?;

            let mut diff_opts = DiffOptions::new();
            if let Some(f) = file_filter {
                diff_opts.pathspec(f);
            }

            let diff = if staged {
                let head_tree = repo
                    .head()
                    .and_then(|h| h.peel_to_tree())
                    .map_err(|e| format!("failed to get HEAD tree: {e}"))?;
                repo.diff_tree_to_index(Some(&head_tree), None, Some(&mut diff_opts))
            } else {
                repo.diff_index_to_workdir(None, Some(&mut diff_opts))
            }
            .map_err(|e| format!("failed to compute diff: {e}"))?;

            let mut diff_text = String::new();
            diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
                let origin = line.origin();
                if origin == '+' || origin == '-' || origin == ' ' {
                    diff_text.push(origin);
                }
                diff_text.push_str(
                    std::str::from_utf8(line.content()).unwrap_or(""),
                );
                true
            })
            .map_err(|e| format!("failed to format diff: {e}"))?;

            Ok(json!({ "diff": diff_text }))
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_metadata() {
        let t = git_diff_tool();
        assert_eq!(t.name, "git_diff");
        assert!(t.execute.is_some());
        assert!(t.needs_approval.is_none());
    }
}
