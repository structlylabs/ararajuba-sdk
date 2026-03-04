//! `execute_command` tool — run a shell command. **Requires approval.**

use ararajuba_core::tools::tool::{tool, ToolDef};
use serde_json::json;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

/// Create the `execute_command` tool.
///
/// Runs an arbitrary command with optional arguments, working directory, and
/// timeout. **High-risk tool** — `needs_approval` is always set.
pub fn execute_command_tool() -> ToolDef {
    tool("execute_command")
        .description(
            "Execute a shell command. High risk — always requires approval in production.",
        )
        .input_schema(json!({
            "type": "object",
            "properties": {
                "command":      { "type": "string", "description": "Command to run" },
                "args":         { "type": "array", "items": { "type": "string" }, "description": "Arguments" },
                "cwd":          { "type": "string", "description": "Working directory" },
                "timeout_secs": { "type": "integer", "description": "Timeout in seconds (default 30)" }
            },
            "required": ["command"]
        }))
        .execute(|input| async move {
            let command = input["command"]
                .as_str()
                .ok_or_else(|| "missing required field: command".to_string())?;

            let args: Vec<String> = input["args"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            let cwd = input["cwd"].as_str().unwrap_or(".");
            let timeout_secs = input["timeout_secs"].as_u64().unwrap_or(30);

            let mut cmd = Command::new(command);
            cmd.args(&args)
                .current_dir(cwd)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());

            let child = cmd
                .spawn()
                .map_err(|e| format!("failed to spawn command: {e}"))?;

            let result = timeout(Duration::from_secs(timeout_secs), child.wait_with_output())
                .await
                .map_err(|_| format!("command timed out after {timeout_secs}s"))?
                .map_err(|e| format!("command failed: {e}"))?;

            let stdout = String::from_utf8_lossy(&result.stdout).to_string();
            let stderr = String::from_utf8_lossy(&result.stderr).to_string();
            let exit_code = result.status.code().unwrap_or(-1);

            Ok(json!({
                "stdout": stdout,
                "stderr": stderr,
                "exit_code": exit_code
            }))
        })
        .needs_approval(|_input| true)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_metadata() {
        let t = execute_command_tool();
        assert_eq!(t.name, "execute_command");
        assert!(t.execute.is_some());
        assert!(t.needs_approval.is_some());
    }
}
