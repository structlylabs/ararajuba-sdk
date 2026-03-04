//! SSE (Server-Sent Events) transport — legacy MCP transport.
//!
//! Opens a long-lived GET SSE connection for server→client messages,
//! and POSTs JSON-RPC messages to the endpoint received via the SSE stream.

use crate::error::MCPError;
use crate::transport::MCPTransport;
use crate::types::JsonRpcMessage;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// SSE-based MCP transport (legacy).
#[derive(Debug)]
pub struct SseMCPTransport {
    url: String,
    headers: HashMap<String, String>,
    /// The endpoint URL for POSTing messages (received from SSE `endpoint` event).
    post_url: Arc<Mutex<Option<String>>>,
    client: reqwest::Client,
    connected: Arc<std::sync::atomic::AtomicBool>,
    recv_buffer: Arc<Mutex<Vec<JsonRpcMessage>>>,
}

impl SseMCPTransport {
    pub fn new(url: String, headers: Option<HashMap<String, String>>) -> Self {
        Self {
            url,
            headers: headers.unwrap_or_default(),
            post_url: Arc::new(Mutex::new(None)),
            client: reqwest::Client::new(),
            connected: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            recv_buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Build request headers for the SSE connection and POST requests.
    fn request_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
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

    /// Resolve the POST endpoint URL, validating origin matches.
    fn resolve_endpoint(&self, endpoint: &str) -> Result<String, MCPError> {
        if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
            // Validate origin matches
            let base = url_origin(&self.url);
            let ep = url_origin(endpoint);
            if base != ep {
                return Err(MCPError::Transport {
                    message: format!("Endpoint origin mismatch: expected {base}, got {ep}"),
                    cause: None,
                });
            }
            Ok(endpoint.to_string())
        } else {
            // Relative URL
            let base = url_base(&self.url);
            Ok(format!("{base}{endpoint}"))
        }
    }
}

/// Extract origin (scheme + host + port) from URL.
fn url_origin(url: &str) -> String {
    // Simple extraction: up to the third slash
    let without_scheme = if let Some(rest) = url.strip_prefix("https://") {
        ("https://", rest)
    } else if let Some(rest) = url.strip_prefix("http://") {
        ("http://", rest)
    } else {
        return url.to_string();
    };
    let (scheme, rest) = without_scheme;
    let host_end = rest.find('/').unwrap_or(rest.len());
    format!("{}{}", scheme, &rest[..host_end])
}

/// Extract base URL (up to and excluding the path component).
fn url_base(url: &str) -> String {
    url_origin(url)
}

#[async_trait]
impl MCPTransport for SseMCPTransport {
    async fn start(&mut self) -> Result<(), MCPError> {
        // Open SSE connection and wait for the `endpoint` event
        let mut req_headers = self.request_headers();
        req_headers.insert("accept", "text/event-stream".parse().unwrap());

        let response = self
            .client
            .get(&self.url)
            .headers(req_headers)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(MCPError::Transport {
                message: format!("SSE connection failed: HTTP {}", response.status()),
                cause: None,
            });
        }

        // Read the SSE stream to find the endpoint event
        let text = response.text().await?;
        let mut endpoint_url = None;

        let mut current_event = String::new();
        for line in text.lines() {
            if let Some(event_name) = line.strip_prefix("event: ") {
                current_event = event_name.trim().to_string();
            } else if let Some(data) = line.strip_prefix("data: ") {
                if current_event == "endpoint" {
                    endpoint_url = Some(self.resolve_endpoint(data.trim())?);
                } else if current_event == "message" || current_event.is_empty() {
                    if let Ok(msg) = serde_json::from_str::<JsonRpcMessage>(data) {
                        self.recv_buffer.lock().await.push(msg);
                    }
                }
            }
        }

        if let Some(url) = endpoint_url {
            *self.post_url.lock().await = Some(url);
            self.connected
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        } else {
            Err(MCPError::Transport {
                message: "No endpoint event received from SSE connection".into(),
                cause: None,
            })
        }
    }

    async fn send(&self, message: &JsonRpcMessage) -> Result<(), MCPError> {
        let post_url = self
            .post_url
            .lock()
            .await
            .clone()
            .ok_or(MCPError::ConnectionClosed)?;

        let mut headers = self.request_headers();
        headers.insert("content-type", "application/json".parse().unwrap());

        let body = serde_json::to_string(message)?;

        let response = self
            .client
            .post(&post_url)
            .headers(headers)
            .body(body)
            .send()
            .await?;

        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(MCPError::Transport {
                message: format!("SSE POST failed: {text}"),
                cause: None,
            });
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
        *self.post_url.lock().await = None;
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
    fn test_sse_transport_creation() {
        let transport = SseMCPTransport::new("https://mcp.example.com/sse".into(), None);
        assert_eq!(transport.url, "https://mcp.example.com/sse");
        assert!(!transport.is_connected());
    }

    #[test]
    fn test_url_origin() {
        assert_eq!(url_origin("https://example.com/path"), "https://example.com");
        assert_eq!(
            url_origin("http://localhost:3000/sse"),
            "http://localhost:3000"
        );
    }

    #[test]
    fn test_resolve_endpoint_relative() {
        let transport =
            SseMCPTransport::new("https://mcp.example.com/sse".into(), None);
        let resolved = transport.resolve_endpoint("/messages").unwrap();
        assert_eq!(resolved, "https://mcp.example.com/messages");
    }

    #[test]
    fn test_resolve_endpoint_absolute_match() {
        let transport =
            SseMCPTransport::new("https://mcp.example.com/sse".into(), None);
        let resolved = transport
            .resolve_endpoint("https://mcp.example.com/messages")
            .unwrap();
        assert_eq!(resolved, "https://mcp.example.com/messages");
    }

    #[test]
    fn test_resolve_endpoint_origin_mismatch() {
        let transport =
            SseMCPTransport::new("https://mcp.example.com/sse".into(), None);
        let result = transport.resolve_endpoint("https://evil.com/messages");
        assert!(result.is_err());
    }
}
