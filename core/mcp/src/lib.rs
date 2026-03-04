//! MCP (Model Context Protocol) client for the Rust AI SDK.
//!
//! This crate provides a client for connecting to MCP servers and
//! using their tools with the AI SDK's `generateText` / `streamText`.
//!
//! # Supported Transports
//!
//! - **Stdio**: Spawn a child process and communicate via stdin/stdout
//! - **HTTP**: Streamable HTTP transport (MCP spec 2025-06-18)
//! - **SSE**: Legacy Server-Sent Events transport
//!
//! # Usage
//!
//! ```rust,no_run
//! use ararajuba_mcp::client::{create_ararajuba_mcp, MCPClientConfig};
//! use ararajuba_mcp::transport::stdio::StdioMCPTransport;
//! use ararajuba_mcp::tools::mcp_tools_to_sdk_tools;
//!
//! # async fn example() -> Result<(), ararajuba_mcp::error::MCPError> {
//! let transport = StdioMCPTransport::new(
//!     "npx".into(),
//!     vec!["-y".into(), "@anthropic/mcp-server-github".into()],
//!     None,
//!     None,
//! );
//!
//! let client = create_ararajuba_mcp(MCPClientConfig {
//!     transport,
//!     client_info: None,
//!     capabilities: None,
//! }).await?;
//!
//! let list = client.list_tools().await?;
//! let tools = mcp_tools_to_sdk_tools(&list.tools);
//! println!("Available tools: {}", tools.len());
//!
//! client.close().await?;
//! # Ok(())
//! # }
//! ```

pub mod client;
pub mod error;
pub mod tools;
pub mod transport;
pub mod types;

pub use client::{create_ararajuba_mcp, MCPClient, MCPClientConfig};
pub use error::MCPError;
pub use tools::{call_tool_result_to_text, mcp_tools_to_sdk_tools};
pub use transport::MCPTransport;
