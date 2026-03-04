# Changelog — ararajuba-tools-coding

All notable changes to the `ararajuba-tools-coding` crate (formerly `tools-coding`) will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.0] — 2026-03-04

### Changed
- **Renamed** from `tools-coding` to `ararajuba-tools-coding`

## [1.0.0] — 2026-03-04

### Added

#### Tool Collections
- `all_tools()` — returns a `ToolSet` with all 17 coding tools
- `safe_tools()` — returns a `ToolSet` with read-only tools (no approval required)

#### File System Tools
- `read_file` — read file contents with optional line range
- `write_file` — create or overwrite a file
- `patch_file` — apply targeted edits (find & replace)
- `list_directory` — list directory contents
- `find_files` — find files by glob pattern
- `search_files` — search file contents by regex pattern

#### Git Tools
- `git_status` — show working tree status
- `git_diff` — show diffs (staged or unstaged)
- `git_log` — show commit history
- `git_add` — stage files
- `git_commit` — create a commit (⚠️ requires approval)
- `git_branch` — list or create branches
- `git_checkout` — switch branches
- `git_push` — push to remote (⚠️ requires approval)
- `git_pull` — pull from remote (⚠️ requires approval)
- `git_clone` — clone a repository (⚠️ requires approval)

#### Shell Tools
- `execute_command` — run arbitrary shell commands (⚠️ requires approval)

#### Analysis Tools
- `get_diagnostics` — get compiler/linter diagnostics for a file
