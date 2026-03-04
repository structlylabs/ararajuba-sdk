//! MCPTransport trait and transport creation.

pub mod http;
pub mod sse;
pub mod stdio;

use crate::error::MCPError;
use crate::types::JsonRpcMessage;
use async_trait::async_trait;
use std::fmt::Debug;

/// Trait for MCP transports that handle physical communication with an MCP server.
///
/// Transports are responsible for:
/// - Establishing the connection (`start`)
/// - Sending JSON-RPC messages (`send`)
/// - Receiving JSON-RPC messages (`recv`)
/// - Closing the connection (`close`)
#[async_trait]
pub trait MCPTransport: Send + Sync + Debug {
    /// Start the transport / open the connection.
    async fn start(&mut self) -> Result<(), MCPError>;

    /// Send a JSON-RPC message to the server.
    async fn send(&self, message: &JsonRpcMessage) -> Result<(), MCPError>;

    /// Receive the next JSON-RPC message from the server.
    /// Returns `None` if the connection is closed.
    async fn recv(&self) -> Result<Option<JsonRpcMessage>, MCPError>;

    /// Close the transport.
    async fn close(&mut self) -> Result<(), MCPError>;

    /// Whether the transport is currently connected.
    fn is_connected(&self) -> bool;
}

/// Configuration for creating a transport.
#[derive(Debug, Clone)]
pub enum TransportConfig {
    /// Stdio transport — spawn a child process.
    Stdio(StdioTransportConfig),
    /// HTTP Streamable transport.
    Http(HttpTransportConfig),
    /// SSE transport (legacy).
    Sse(SseTransportConfig),
}

/// Configuration for the stdio transport.
#[derive(Debug, Clone)]
pub struct StdioTransportConfig {
    /// Command to spawn.
    pub command: String,
    /// Arguments for the command.
    pub args: Vec<String>,
    /// Additional environment variables.
    pub env: Option<std::collections::HashMap<String, String>>,
    /// Working directory.
    pub cwd: Option<String>,
}

/// Configuration for the HTTP streamable transport.
#[derive(Debug, Clone)]
pub struct HttpTransportConfig {
    /// URL of the MCP server endpoint.
    pub url: String,
    /// Additional headers.
    pub headers: Option<std::collections::HashMap<String, String>>,
}

/// Configuration for the SSE transport (legacy).
#[derive(Debug, Clone)]
pub struct SseTransportConfig {
    /// URL to open the SSE connection.
    pub url: String,
    /// Additional headers.
    pub headers: Option<std::collections::HashMap<String, String>>,
}
