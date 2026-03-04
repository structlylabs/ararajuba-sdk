//! MCP client error types.

use std::fmt;

/// Errors that can occur in the MCP client.
#[derive(Debug)]
pub enum MCPError {
    /// Transport failed to connect or communicate.
    Transport { message: String, cause: Option<Box<dyn std::error::Error + Send + Sync>> },
    /// JSON-RPC error response from the server.
    JsonRpc { code: i64, message: String, data: Option<serde_json::Value> },
    /// Protocol version mismatch.
    ProtocolVersion { expected: Vec<String>, received: String },
    /// Server does not support the requested capability.
    CapabilityNotSupported { method: String },
    /// Connection is closed.
    ConnectionClosed,
    /// Serialization/deserialization error.
    Serialization { message: String, cause: Option<Box<dyn std::error::Error + Send + Sync>> },
    /// Timeout waiting for response.
    Timeout { request_id: u64 },
    /// Generic error.
    Other { message: String },
}

impl fmt::Display for MCPError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MCPError::Transport { message, .. } => write!(f, "MCP transport error: {message}"),
            MCPError::JsonRpc { code, message, .. } => {
                write!(f, "MCP JSON-RPC error ({code}): {message}")
            }
            MCPError::ProtocolVersion { expected, received } => {
                write!(
                    f,
                    "MCP protocol version mismatch: expected one of {expected:?}, got {received}"
                )
            }
            MCPError::CapabilityNotSupported { method } => {
                write!(f, "MCP server does not support: {method}")
            }
            MCPError::ConnectionClosed => write!(f, "MCP connection closed"),
            MCPError::Serialization { message, .. } => {
                write!(f, "MCP serialization error: {message}")
            }
            MCPError::Timeout { request_id } => {
                write!(f, "MCP request timeout (id={request_id})")
            }
            MCPError::Other { message } => write!(f, "MCP error: {message}"),
        }
    }
}

impl std::error::Error for MCPError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MCPError::Transport { cause: Some(e), .. } => Some(e.as_ref()),
            MCPError::Serialization { cause: Some(e), .. } => Some(e.as_ref()),
            _ => None,
        }
    }
}

impl From<serde_json::Error> for MCPError {
    fn from(e: serde_json::Error) -> Self {
        MCPError::Serialization {
            message: e.to_string(),
            cause: Some(Box::new(e)),
        }
    }
}

impl From<reqwest::Error> for MCPError {
    fn from(e: reqwest::Error) -> Self {
        MCPError::Transport {
            message: e.to_string(),
            cause: Some(Box::new(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let e = MCPError::JsonRpc {
            code: -32601,
            message: "Method not found".into(),
            data: None,
        };
        assert!(e.to_string().contains("-32601"));
        assert!(e.to_string().contains("Method not found"));
    }

    #[test]
    fn test_connection_closed() {
        let e = MCPError::ConnectionClosed;
        assert!(e.to_string().contains("closed"));
    }

    #[test]
    fn test_capability_not_supported() {
        let e = MCPError::CapabilityNotSupported {
            method: "tools/call".into(),
        };
        assert!(e.to_string().contains("tools/call"));
    }
}
