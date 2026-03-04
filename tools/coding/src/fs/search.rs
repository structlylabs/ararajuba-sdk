//! `search_files` tool — regex search across files (ripgrep-style).

use ararajuba_core::tools::tool::{tool, ToolDef};
use glob::Pattern;
use ignore::WalkBuilder;
use regex::Regex;
use serde_json::json;
use std::path::Path;

/// Create the `search_files` tool.
///
/// Performs a regex search across files in a directory, returning matching
/// lines with optional surrounding context. Respects `.gitignore`.
pub fn search_files_tool() -> ToolDef {
    tool("search_files")
        .description(
            "Search files with a regex pattern (ripgrep-style). \
             Returns matching lines with optional context.",
        )
        .input_schema(json!({
            "type": "object",
            "properties": {
                "pattern":          { "type": "string", "description": "Regex pattern to search for" },
                "path":             { "type": "string", "description": "Root directory (default: current dir)" },
                "glob":             { "type": "string", "description": "File glob filter (e.g. *.rs)" },
                "case_insensitive": { "type": "boolean", "description": "Case-insensitive search (default false)" },
                "context_lines":    { "type": "integer", "description": "Lines of context before/after each match" }
            },
            "required": ["pattern"]
        }))
        .execute(|input| async move {
            let pattern_str = input["pattern"]
                .as_str()
                .ok_or_else(|| "missing required field: pattern".to_string())?;
            let root = input["path"].as_str().unwrap_or(".");
            let glob_filter = input["glob"].as_str();
            let case_insensitive = input["case_insensitive"].as_bool().unwrap_or(false);
            let context_lines = input["context_lines"].as_u64().unwrap_or(0) as usize;

            let regex_pattern = if case_insensitive {
                format!("(?i){pattern_str}")
            } else {
                pattern_str.to_string()
            };
            let re = Regex::new(&regex_pattern)
                .map_err(|e| format!("invalid regex: {e}"))?;

            let glob_pat = glob_filter
                .map(|g| Pattern::new(g))
                .transpose()
                .map_err(|e| format!("invalid glob filter: {e}"))?;

            let base = Path::new(root);
            let mut matches = Vec::new();

            for entry in WalkBuilder::new(base).build().flatten() {
                if !entry.file_type().map(|f| f.is_file()).unwrap_or(false) {
                    continue;
                }

                let rel = entry
                    .path()
                    .strip_prefix(base)
                    .unwrap_or(entry.path())
                    .to_string_lossy()
                    .to_string();

                if let Some(ref gp) = glob_pat {
                    if !gp.matches(&rel) {
                        continue;
                    }
                }

                // Read file — skip binary / unreadable
                let content = match tokio::fs::read_to_string(entry.path()).await {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                let lines: Vec<&str> = content.lines().collect();
                for (idx, line) in lines.iter().enumerate() {
                    if re.is_match(line) {
                        let start = idx.saturating_sub(context_lines);
                        let end = (idx + context_lines + 1).min(lines.len());
                        let ctx: Vec<String> =
                            lines[start..end].iter().map(|l| l.to_string()).collect();

                        matches.push(json!({
                            "file": rel,
                            "line": idx + 1,
                            "content": line,
                            "context": ctx
                        }));
                    }
                }
            }

            Ok(json!({ "matches": matches }))
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_metadata() {
        let t = search_files_tool();
        assert_eq!(t.name, "search_files");
        assert!(t.description.is_some());
        assert!(t.execute.is_some());
    }
}
