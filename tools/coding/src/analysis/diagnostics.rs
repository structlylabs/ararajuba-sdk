//! `get_diagnostics` tool — run `cargo check` and parse compiler messages.

use ararajuba_core::tools::tool::{tool, ToolDef};
use serde_json::json;
use tokio::process::Command;

/// Create the `get_diagnostics` tool.
///
/// Runs `cargo check --message-format=json` and returns structured compiler
/// errors and warnings. Useful for giving an agent visibility into compilation
/// problems without a full build.
pub fn get_diagnostics_tool() -> ToolDef {
    tool("get_diagnostics")
        .description("Run cargo check and return compiler errors/warnings as structured data.")
        .input_schema(json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Project path to check (default: current dir)" }
            }
        }))
        .execute(|input| async move {
            let cwd = input["path"].as_str().unwrap_or(".");

            let output = Command::new("cargo")
                .args(["check", "--message-format=json"])
                .current_dir(cwd)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .output()
                .await
                .map_err(|e| format!("failed to run cargo check: {e}"))?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut errors = Vec::new();

            for line in stdout.lines() {
                let Ok(msg) = serde_json::from_str::<serde_json::Value>(line) else {
                    continue;
                };

                if msg["reason"].as_str() != Some("compiler-message") {
                    continue;
                }

                let message = &msg["message"];
                let level = message["level"].as_str().unwrap_or("unknown");

                // Only include errors and warnings
                if level != "error" && level != "warning" {
                    continue;
                }

                let text = message["message"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();

                // Extract primary span
                let spans = message["spans"].as_array();
                let primary_span = spans.and_then(|s| {
                    s.iter().find(|sp| sp["is_primary"].as_bool() == Some(true))
                });

                let file = primary_span
                    .and_then(|s| s["file_name"].as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let line_num = primary_span
                    .and_then(|s| s["line_start"].as_u64())
                    .unwrap_or(0);

                errors.push(json!({
                    "file": file,
                    "line": line_num,
                    "message": text,
                    "severity": level
                }));
            }

            Ok(json!({ "errors": errors }))
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_metadata() {
        let t = get_diagnostics_tool();
        assert_eq!(t.name, "get_diagnostics");
        assert!(t.execute.is_some());
        assert!(t.needs_approval.is_none()); // read-only, safe
    }
}
