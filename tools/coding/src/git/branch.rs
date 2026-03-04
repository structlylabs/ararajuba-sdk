//! `git_branch` tool — list, create, or delete branches.

use ararajuba_core::tools::tool::{tool, ToolDef};
use git2::Repository;
use serde_json::json;

/// Create the `git_branch` tool.
///
/// Supports `action`: `"list"`, `"create"`, `"delete"`.
pub fn git_branch_tool() -> ToolDef {
    tool("git_branch")
        .description("List, create, or delete git branches.")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["list", "create", "delete"],
                    "description": "Branch operation"
                },
                "name": { "type": "string", "description": "Branch name (for create/delete)" }
            },
            "required": ["action"]
        }))
        .execute(|input| async move {
            let action = input["action"]
                .as_str()
                .ok_or_else(|| "missing required field: action".to_string())?;

            let repo = Repository::discover(".")
                .map_err(|e| format!("failed to open repository: {e}"))?;

            match action {
                "list" => {
                    let branches = repo
                        .branches(None)
                        .map_err(|e| format!("failed to list branches: {e}"))?;

                    let current = repo
                        .head()
                        .ok()
                        .and_then(|h| h.shorthand().map(String::from));

                    let mut result = Vec::new();
                    for branch_result in branches {
                        let (branch, _btype) =
                            branch_result.map_err(|e| format!("branch iter error: {e}"))?;
                        let name = branch
                            .name()
                            .map_err(|e| format!("invalid branch name: {e}"))?
                            .unwrap_or("unknown")
                            .to_string();
                        let is_current = current.as_deref() == Some(&name);
                        result.push(json!({"name": name, "current": is_current}));
                    }

                    Ok(json!({ "branches": result }))
                }

                "create" => {
                    let name = input["name"]
                        .as_str()
                        .ok_or_else(|| "missing required field: name for create".to_string())?;

                    let head_commit = repo
                        .head()
                        .and_then(|h| h.peel_to_commit())
                        .map_err(|e| format!("failed to get HEAD commit: {e}"))?;

                    repo.branch(name, &head_commit, false)
                        .map_err(|e| format!("failed to create branch: {e}"))?;

                    Ok(json!({ "ok": true }))
                }

                "delete" => {
                    let name = input["name"]
                        .as_str()
                        .ok_or_else(|| "missing required field: name for delete".to_string())?;

                    let mut branch = repo
                        .find_branch(name, git2::BranchType::Local)
                        .map_err(|e| format!("branch not found: {e}"))?;

                    branch
                        .delete()
                        .map_err(|e| format!("failed to delete branch: {e}"))?;

                    Ok(json!({ "ok": true }))
                }

                _ => Err(format!("unknown action: {action}")),
            }
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_metadata() {
        let t = git_branch_tool();
        assert_eq!(t.name, "git_branch");
        assert!(t.execute.is_some());
    }
}
