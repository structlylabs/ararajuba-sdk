//! # ararajuba-tools-coding
//!
//! Standard coding-agent tool set for building Cursor/Claude Code style agents.
//!
//! Provides pre-built `ToolDef` instances for file system operations, git,
//! shell execution, and code analysis. Combine with `ararajuba-core`'s `generate_text`
//! in a tool loop to create autonomous coding agents.

pub mod analysis;
pub mod fs;
pub mod git;
pub mod shell;

use ararajuba_core::tools::tool_set::ToolSet;

/// Returns a `ToolSet` containing **all** coding tools.
///
/// High-risk tools (`execute_command`, `git_push`, `git_pull`) have
/// `needs_approval` set so they require explicit confirmation.
pub fn all_tools() -> ToolSet {
    ToolSet::new()
        // File system
        .add(fs::read::read_file_tool())
        .add(fs::write::write_file_tool())
        .add(fs::patch::patch_file_tool())
        .add(fs::list::list_directory_tool())
        .add(fs::find::find_files_tool())
        .add(fs::search::search_files_tool())
        // Git
        .add(git::status::git_status_tool())
        .add(git::diff::git_diff_tool())
        .add(git::log::git_log_tool())
        .add(git::add::git_add_tool())
        .add(git::commit::git_commit_tool())
        .add(git::branch::git_branch_tool())
        .add(git::checkout::git_checkout_tool())
        .add(git::push::git_push_tool())
        .add(git::pull::git_pull_tool())
        .add(git::clone::git_clone_tool())
        // Shell
        .add(shell::exec::execute_command_tool())
        // Analysis
        .add(analysis::diagnostics::get_diagnostics_tool())
}

/// Returns a `ToolSet` containing only **safe** (read-only) tools that can
/// run autonomously without user approval.
pub fn safe_tools() -> ToolSet {
    ToolSet::new()
        .add(fs::read::read_file_tool())
        .add(fs::list::list_directory_tool())
        .add(fs::find::find_files_tool())
        .add(fs::search::search_files_tool())
        .add(git::status::git_status_tool())
        .add(git::diff::git_diff_tool())
        .add(git::log::git_log_tool())
        .add(analysis::diagnostics::get_diagnostics_tool())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_tools_has_expected_count() {
        let tools = all_tools();
        assert_eq!(tools.len(), 18);
    }

    #[test]
    fn safe_tools_has_expected_count() {
        let tools = safe_tools();
        assert_eq!(tools.len(), 8);
    }

    #[test]
    fn safe_tools_subset_of_all() {
        let all = all_tools();
        let safe = safe_tools();
        for (name, _) in safe.iter() {
            assert!(all.get(name).is_some(), "safe tool {name} not in all_tools");
        }
    }
}
