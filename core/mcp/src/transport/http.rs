//! HTTP Streamable transport — modern MCP transport (spec 2025-06-18).
//!
//! Uses POST for sending JSON-RPC requests and receives responses
//! either as inline JSON or SSE streams. Supports session management
//! via `mcp-session-id` header.

use crate::error::MCPError;
use crate::transport::MCPTransport;
use crate::types::{JsonRpcMessage, LATEST_PROTOCOL_VERSION};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// HTTP streamable MCP transport.
#[derive(Debug)]
pub struct HttpMCPTransport {
    url: String,
    headers: HashMap<String, String>,
    session_id: Arc<Mutex<Option<String>>>,
    client: reqwest::Client,
    connected: Arc<std::sync::atomic::AtomicBool>,
    /// Buffer for received messages (from SSE responses).
    recv_buffer: Arc<Mutex<Vec<JsonRpcMessage>>>,
}

impl HttpMCPTransport {
    pub fn new(url: String, headers: Option<HashMap<String, String>>) -> Self {
        Self {
            url,
            headers: headers.unwrap_or_default(),
            session_id: Arc::new(Mutex::new(None)),
            client: reqwest::Client::new(),
            connected: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            recv_buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn build_headers(&self, session_id: &Option<String>) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("content-type", "application/json".parse().unwrap());
        headers.insert(
            "accept",
            "application/json, text/event-stream".parse().unwrap(),
        );
        headers.insert(
            "mcp-protocol-version",
            LATEST_PROTOCOL_VERSION.parse().unwrap(),
        );

        if let Some(sid) = session_id {
            headers.insert("mcp-session-id", sid.parse().unwrap());
        }

        for (k, v) in &self.headers {
            if let (Ok(name), Ok(val)) = (
                k.parse::<reqwest::header::HeaderName>(),
                v.parse::<reqwest::header::HeaderValue>(),
            ) {
                headers.insert(name, val);
            }
        }

        headers
    }
}

#[async_trait]
impl MCPTransport for HttpMCPTransport {
    async fn start(&mut self) -> Result<(), MCPError> {
        self.connected
            .store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    async fn send(&self, message: &JsonRpcMessage) -> Result<(), MCPError> {
        if !self.connected.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(MCPError::ConnectionClosed);
        }

        let session_id = self.session_id.lock().await.clone();
        let headers = self.build_headers(&session_id);

        let body = serde_json::to_string(message)?;

        let response = self
            .client
            .post(&self.url)
            .headers(headers)
            .body(body)
            .send()
            .await?;

        let status = response.status();

        // Capture session ID from response headers
        if let Some(sid) = response.headers().get("mcp-session-id") {
            if let Ok(sid_str) = sid.to_str() {
                *self.session_id.lock().await = Some(sid_str.to_string());
            }
        }

        if status == reqwest::StatusCode::ACCEPTED {
            // 202: accepted, no response body
            return Ok(());
        }

        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(MCPError::Transport {
                message: format!("HTTP {status}: {text}"),
                cause: None,
            });
        }

        // Check content type
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        if content_type.contains("text/event-stream") {
            // Parse SSE response
            let text = response.text().await?;
            let mut buffer = self.recv_buffer.lock().await;
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data.trim().is_empty() {
                        continue;
                    }
                    if let Ok(msg) = serde_json::from_str::<JsonRpcMessage>(data) {
                        buffer.push(msg);
                    }
                }
            }
        } else {
            // Parse JSON response
            let text = response.text().await?;
            if !text.trim().is_empty() {
                if let Ok(msg) = serde_json::from_str::<JsonRpcMessage>(&text) {
                    self.recv_buffer.lock().await.push(msg);
                }
            }
        }

        Ok(())
    }

    async fn recv(&self) -> Result<Option<JsonRpcMessage>, MCPError> {
        let mut buffer = self.recv_buffer.lock().await;
        if buffer.is_empty() {
            Ok(None)
        } else {
            Ok(Some(buffer.remove(0)))
        }
    }

    async fn close(&mut self) -> Result<(), MCPError> {
        self.connected
            .store(false, std::sync::atomic::Ordering::SeqCst);

        let session_id = self.session_id.lock().await.clone();
        if session_id.is_some() {
            let headers = self.build_headers(&session_id);
            // Try to send DELETE to terminate session, ignore errors
            let _ = self.client.delete(&self.url).headers(headers).send().await;
        }

        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_transport_creation() {
        let transport = HttpMCPTransport::new(
            "https://mcp.example.com/mcp".into(),
            None,
        );
        assert_eq!(transport.url, "https://mcp.example.com/mcp");
        assert!(!transport.is_connected());
    }

    #[tokio::test]
    async fn test_send_before_start_fails() {
        let transport = HttpMCPTransport::new("https://example.com".into(), None);
        let msg = JsonRpcMessage::Notification(crate::types::JsonRpcNotification {
            jsonrpc: "2.0".into(),
            method: "test".into(),
            params: None,
        });
        let result = transport.send(&msg).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_recv_empty_buffer() {
        let transport = HttpMCPTransport::new("https://example.com".into(), None);
        let result = transport.recv().await.unwrap();
        assert!(result.is_none());
    }
}
