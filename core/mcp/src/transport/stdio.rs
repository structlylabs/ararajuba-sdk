//! Stdio transport — spawns a child process and communicates via stdin/stdout.

use crate::error::MCPError;
use crate::transport::MCPTransport;
use crate::types::JsonRpcMessage;
use async_trait::async_trait;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

/// Stdio-based MCP transport.
///
/// Spawns a child process and communicates using newline-delimited JSON
/// over stdin (client→server) and stdout (server→client).
#[derive(Debug)]
pub struct StdioMCPTransport {
    command: String,
    args: Vec<String>,
    env: Option<HashMap<String, String>>,
    cwd: Option<String>,
    inner: Mutex<Option<StdioInner>>,
}

#[derive(Debug)]
struct StdioInner {
    #[allow(dead_code)]
    child: Child,
    stdin: tokio::process::ChildStdin,
    reader: BufReader<tokio::process::ChildStdout>,
}

impl StdioMCPTransport {
    pub fn new(
        command: String,
        args: Vec<String>,
        env: Option<HashMap<String, String>>,
        cwd: Option<String>,
    ) -> Self {
        Self {
            command,
            args,
            env,
            cwd,
            inner: Mutex::new(None),
        }
    }
}

#[async_trait]
impl MCPTransport for StdioMCPTransport {
    async fn start(&mut self) -> Result<(), MCPError> {
        let mut cmd = Command::new(&self.command);
        cmd.args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        if let Some(ref cwd) = self.cwd {
            cmd.current_dir(cwd);
        }

        if let Some(ref env) = self.env {
            for (k, v) in env {
                cmd.env(k, v);
            }
        }

        let mut child = cmd.spawn().map_err(|e| MCPError::Transport {
            message: format!("Failed to spawn '{}': {e}", self.command),
            cause: Some(Box::new(e)),
        })?;

        let stdin = child.stdin.take().ok_or_else(|| MCPError::Transport {
            message: "Failed to capture child stdin".into(),
            cause: None,
        })?;

        let stdout = child.stdout.take().ok_or_else(|| MCPError::Transport {
            message: "Failed to capture child stdout".into(),
            cause: None,
        })?;

        let reader = BufReader::new(stdout);

        *self.inner.lock().await = Some(StdioInner {
            child,
            stdin,
            reader,
        });

        Ok(())
    }

    async fn send(&self, message: &JsonRpcMessage) -> Result<(), MCPError> {
        let mut guard = self.inner.lock().await;
        let inner = guard.as_mut().ok_or(MCPError::ConnectionClosed)?;

        let mut data = serde_json::to_string(message)?;
        data.push('\n');

        inner
            .stdin
            .write_all(data.as_bytes())
            .await
            .map_err(|e| MCPError::Transport {
                message: format!("Failed to write to stdin: {e}"),
                cause: Some(Box::new(e)),
            })?;

        inner.stdin.flush().await.map_err(|e| MCPError::Transport {
            message: format!("Failed to flush stdin: {e}"),
            cause: Some(Box::new(e)),
        })?;

        Ok(())
    }

    async fn recv(&self) -> Result<Option<JsonRpcMessage>, MCPError> {
        let mut guard = self.inner.lock().await;
        let inner = guard.as_mut().ok_or(MCPError::ConnectionClosed)?;

        let mut line = String::new();
        let bytes_read = inner
            .reader
            .read_line(&mut line)
            .await
            .map_err(|e| MCPError::Transport {
                message: format!("Failed to read from stdout: {e}"),
                cause: Some(Box::new(e)),
            })?;

        if bytes_read == 0 {
            return Ok(None); // EOF — process closed
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }

        let msg: JsonRpcMessage = serde_json::from_str(trimmed)?;
        Ok(Some(msg))
    }

    async fn close(&mut self) -> Result<(), MCPError> {
        if let Some(mut inner) = self.inner.lock().await.take() {
            // Drop stdin to signal the child
            drop(inner.stdin);
            // Try to kill the child
            let _ = inner.child.kill().await;
        }
        Ok(())
    }

    fn is_connected(&self) -> bool {
        // We can't lock synchronously, so we assume connected if inner was set
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdio_transport_creation() {
        let transport = StdioMCPTransport::new(
            "echo".into(),
            vec!["hello".into()],
            None,
            None,
        );
        assert_eq!(transport.command, "echo");
        assert_eq!(transport.args, vec!["hello"]);
    }

    #[tokio::test]
    async fn test_send_before_start_fails() {
        let transport = StdioMCPTransport::new("echo".into(), vec![], None, None);
        let msg = JsonRpcMessage::Notification(crate::types::JsonRpcNotification {
            jsonrpc: "2.0".into(),
            method: "test".into(),
            params: None,
        });
        let result = transport.send(&msg).await;
        assert!(result.is_err());
    }
}
