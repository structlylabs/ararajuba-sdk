//! MCP client — manages connection lifecycle, tool discovery, and tool execution.

use crate::error::MCPError;
use crate::transport::MCPTransport;
use crate::types::{
    CallToolResult, ClientCapabilities, ClientInfo, InitializeParams, InitializeResult,
    JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, ListToolsResult,
    JSONRPC_VERSION, LATEST_PROTOCOL_VERSION, SUPPORTED_PROTOCOL_VERSIONS,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Configuration for creating an MCP client.
pub struct MCPClientConfig<T: MCPTransport> {
    /// The transport to use.
    pub transport: T,
    /// Client info to send during initialization.
    pub client_info: Option<ClientInfo>,
    /// Client capabilities to advertise.
    pub capabilities: Option<ClientCapabilities>,
}

/// An MCP client connected to a single MCP server.
pub struct MCPClient<T: MCPTransport> {
    transport: Arc<Mutex<T>>,
    request_id: AtomicU64,
    server_capabilities: Arc<Mutex<Option<crate::types::ServerCapabilities>>>,
    #[allow(dead_code)]
    server_info: Arc<Mutex<Option<crate::types::ServerInfo>>>,
    closed: AtomicBool,
}

impl<T: MCPTransport + 'static> MCPClient<T> {
    /// Create and initialize an MCP client.
    ///
    /// This performs the full initialization handshake:
    /// 1. `transport.start()`
    /// 2. Send `initialize` request
    /// 3. Validate protocol version
    /// 4. Send `notifications/initialized`
    pub async fn connect(config: MCPClientConfig<T>) -> Result<Self, MCPError> {
        let client_info = config.client_info.unwrap_or_default();
        let capabilities = config.capabilities.unwrap_or_default();

        let mut transport = config.transport;
        transport.start().await?;

        let client = Self {
            transport: Arc::new(Mutex::new(transport)),
            request_id: AtomicU64::new(1),
            server_capabilities: Arc::new(Mutex::new(None)),
            server_info: Arc::new(Mutex::new(None)),
            closed: AtomicBool::new(false),
        };

        // Send initialize request
        let init_params = InitializeParams {
            protocol_version: LATEST_PROTOCOL_VERSION.into(),
            capabilities,
            client_info,
        };

        let result = client
            .request("initialize", Some(serde_json::to_value(&init_params)?))
            .await?;

        let init_result: InitializeResult = serde_json::from_value(result).map_err(|e| {
            MCPError::Serialization {
                message: format!("Failed to parse initialize result: {e}"),
                cause: Some(Box::new(e)),
            }
        })?;

        // Validate protocol version
        if !SUPPORTED_PROTOCOL_VERSIONS.contains(&init_result.protocol_version.as_str()) {
            let _ = client.close().await;
            return Err(MCPError::ProtocolVersion {
                expected: SUPPORTED_PROTOCOL_VERSIONS.iter().map(|s| s.to_string()).collect(),
                received: init_result.protocol_version,
            });
        }

        *client.server_capabilities.lock().await = Some(init_result.capabilities);
        *client.server_info.lock().await = Some(init_result.server_info);

        // Send initialized notification
        client.notify("notifications/initialized", None).await?;

        Ok(client)
    }

    /// Send a JSON-RPC request and wait for the response.
    pub async fn request(
        &self,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, MCPError> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(MCPError::ConnectionClosed);
        }

        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        let request = JsonRpcRequest {
            jsonrpc: JSONRPC_VERSION.into(),
            id,
            method: method.into(),
            params,
        };

        let transport = self.transport.lock().await;
        transport.send(&JsonRpcMessage::Request(request)).await?;

        // Wait for response with matching id
        // In a real implementation, this would use channels for concurrent requests.
        // For simplicity, we do a synchronous recv loop.
        loop {
            match transport.recv().await? {
                Some(JsonRpcMessage::Response(resp)) if resp.id == id => {
                    return Ok(resp.result);
                }
                Some(JsonRpcMessage::Error(err)) if err.id == id => {
                    return Err(MCPError::JsonRpc {
                        code: err.error.code,
                        message: err.error.message,
                        data: err.error.data,
                    });
                }
                Some(_) => {
                    // Ignore other messages (notifications, responses for other ids)
                    continue;
                }
                None => {
                    return Err(MCPError::ConnectionClosed);
                }
            }
        }
    }

    /// Send a JSON-RPC notification (no response expected).
    pub async fn notify(&self, method: &str, params: Option<Value>) -> Result<(), MCPError> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(MCPError::ConnectionClosed);
        }

        let notification = JsonRpcNotification {
            jsonrpc: JSONRPC_VERSION.into(),
            method: method.into(),
            params,
        };

        self.transport
            .lock()
            .await
            .send(&JsonRpcMessage::Notification(notification))
            .await
    }

    /// Check if the server supports a given capability.
    fn assert_capability_sync(
        caps: &Option<crate::types::ServerCapabilities>,
        method: &str,
    ) -> Result<(), MCPError> {
        let caps = caps.as_ref().ok_or(MCPError::Other {
            message: "Not initialized".into(),
        })?;

        let supported = match method {
            m if m.starts_with("tools/") => caps.tools.is_some(),
            m if m.starts_with("resources/") => caps.resources.is_some(),
            m if m.starts_with("prompts/") => caps.prompts.is_some(),
            _ => true, // Unknown methods are allowed
        };

        if supported {
            Ok(())
        } else {
            Err(MCPError::CapabilityNotSupported {
                method: method.into(),
            })
        }
    }

    /// List all tools from the MCP server.
    pub async fn list_tools(&self) -> Result<ListToolsResult, MCPError> {
        let caps = self.server_capabilities.lock().await.clone();
        Self::assert_capability_sync(&caps, "tools/list")?;

        let result = self.request("tools/list", None).await?;
        let list: ListToolsResult = serde_json::from_value(result)?;
        Ok(list)
    }

    /// Call a tool on the MCP server.
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: HashMap<String, Value>,
    ) -> Result<CallToolResult, MCPError> {
        let caps = self.server_capabilities.lock().await.clone();
        Self::assert_capability_sync(&caps, "tools/call")?;

        let params = serde_json::json!({
            "name": name,
            "arguments": arguments,
        });

        let result = self.request("tools/call", Some(params)).await?;
        let call_result: CallToolResult = serde_json::from_value(result)?;
        Ok(call_result)
    }

    /// Close the MCP client and its transport.
    pub async fn close(&self) -> Result<(), MCPError> {
        self.closed.store(true, Ordering::SeqCst);
        self.transport.lock().await.close().await
    }

    /// Whether the client has been closed.
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }
}

/// Create an MCP client with the given transport, performing the initialization handshake.
///
/// # Example
///
/// ```rust,no_run
/// use ararajuba_mcp::transport::stdio::StdioMCPTransport;
/// use ararajuba_mcp::client::create_ararajuba_mcp;
/// use ararajuba_mcp::client::MCPClientConfig;
///
/// # async fn example() -> Result<(), ararajuba_mcp::error::MCPError> {
/// let transport = StdioMCPTransport::new(
///     "npx".into(),
///     vec!["-y".into(), "@anthropic/mcp-server-github".into()],
///     None,
///     None,
/// );
///
/// let client = create_ararajuba_mcp(MCPClientConfig {
///     transport,
///     client_info: None,
///     capabilities: None,
/// }).await?;
///
/// let tools = client.list_tools().await?;
/// println!("Available tools: {:?}", tools.tools.len());
///
/// client.close().await?;
/// # Ok(())
/// # }
/// ```
pub async fn create_ararajuba_mcp<T: MCPTransport + 'static>(
    config: MCPClientConfig<T>,
) -> Result<MCPClient<T>, MCPError> {
    MCPClient::connect(config).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::MCPTransport;
    use crate::types::{
        JsonRpcErrorData, JsonRpcErrorResponse, JsonRpcMessage, JsonRpcResponse, ServerCapabilities,
        ServerInfo,
    };
    use async_trait::async_trait;
    use std::sync::Mutex as StdMutex;

    /// A mock transport for testing the client.
    #[derive(Debug)]
    struct MockTransport {
        sent: Arc<StdMutex<Vec<String>>>,
        responses: Arc<StdMutex<Vec<JsonRpcMessage>>>,
        started: bool,
    }

    impl MockTransport {
        fn new(responses: Vec<JsonRpcMessage>) -> Self {
            Self {
                sent: Arc::new(StdMutex::new(Vec::new())),
                responses: Arc::new(StdMutex::new(responses)),
                started: false,
            }
        }
    }

    #[async_trait]
    impl MCPTransport for MockTransport {
        async fn start(&mut self) -> Result<(), MCPError> {
            self.started = true;
            Ok(())
        }

        async fn send(&self, message: &JsonRpcMessage) -> Result<(), MCPError> {
            let json = serde_json::to_string(message).unwrap();
            self.sent.lock().unwrap().push(json);
            Ok(())
        }

        async fn recv(&self) -> Result<Option<JsonRpcMessage>, MCPError> {
            let mut responses = self.responses.lock().unwrap();
            if responses.is_empty() {
                Ok(None)
            } else {
                Ok(Some(responses.remove(0)))
            }
        }

        async fn close(&mut self) -> Result<(), MCPError> {
            Ok(())
        }

        fn is_connected(&self) -> bool {
            self.started
        }
    }

    fn make_init_response(id: u64) -> JsonRpcMessage {
        JsonRpcMessage::Response(JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id,
            result: serde_json::to_value(InitializeResult {
                protocol_version: LATEST_PROTOCOL_VERSION.into(),
                capabilities: ServerCapabilities {
                    tools: Some(Value::Object(Default::default())),
                    resources: None,
                    prompts: None,
                    logging: None,
                },
                server_info: ServerInfo {
                    name: "test-server".into(),
                    version: Some("1.0.0".into()),
                },
                instructions: None,
            })
            .unwrap(),
        })
    }

    #[tokio::test]
    async fn test_client_initialization() {
        let transport = MockTransport::new(vec![make_init_response(1)]);
        let sent = transport.sent.clone();

        let client = MCPClient::connect(MCPClientConfig {
            transport,
            client_info: None,
            capabilities: None,
        })
        .await
        .unwrap();

        assert!(!client.is_closed());

        // Should have sent: initialize request + initialized notification
        let sent = sent.lock().unwrap();
        assert_eq!(sent.len(), 2);
        assert!(sent[0].contains("initialize"));
        assert!(sent[1].contains("notifications/initialized"));
    }

    #[tokio::test]
    async fn test_client_protocol_version_mismatch() {
        let bad_response = JsonRpcMessage::Response(JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: 1,
            result: serde_json::json!({
                "protocolVersion": "1999-01-01",
                "capabilities": {},
                "serverInfo": { "name": "old-server" }
            }),
        });

        let transport = MockTransport::new(vec![bad_response]);
        let result = MCPClient::connect(MCPClientConfig {
            transport,
            client_info: None,
            capabilities: None,
        })
        .await;

        assert!(matches!(result, Err(MCPError::ProtocolVersion { .. })));
    }

    #[tokio::test]
    async fn test_client_list_tools() {
        let tools_response = JsonRpcMessage::Response(JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: 2,
            result: serde_json::json!({
                "tools": [
                    {
                        "name": "get_weather",
                        "description": "Get current weather",
                        "inputSchema": { "type": "object" }
                    }
                ]
            }),
        });

        let transport = MockTransport::new(vec![make_init_response(1), tools_response]);

        let client = MCPClient::connect(MCPClientConfig {
            transport,
            client_info: None,
            capabilities: None,
        })
        .await
        .unwrap();

        let tools = client.list_tools().await.unwrap();
        assert_eq!(tools.tools.len(), 1);
        assert_eq!(tools.tools[0].name, "get_weather");
    }

    #[tokio::test]
    async fn test_client_call_tool() {
        let call_response = JsonRpcMessage::Response(JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: 2,
            result: serde_json::json!({
                "content": [
                    { "type": "text", "text": "72°F in NYC" }
                ],
                "isError": false
            }),
        });

        let transport = MockTransport::new(vec![make_init_response(1), call_response]);

        let client = MCPClient::connect(MCPClientConfig {
            transport,
            client_info: None,
            capabilities: None,
        })
        .await
        .unwrap();

        let mut args = HashMap::new();
        args.insert("city".into(), Value::String("NYC".into()));

        let result = client.call_tool("get_weather", args).await.unwrap();
        assert_eq!(result.content.len(), 1);
        assert!(!result.is_error);
    }

    #[tokio::test]
    async fn test_client_jsonrpc_error() {
        let error_response = JsonRpcMessage::Error(JsonRpcErrorResponse {
            jsonrpc: "2.0".into(),
            id: 2,
            error: JsonRpcErrorData {
                code: -32601,
                message: "Method not found".into(),
                data: None,
            },
        });

        let transport = MockTransport::new(vec![make_init_response(1), error_response]);

        let client = MCPClient::connect(MCPClientConfig {
            transport,
            client_info: None,
            capabilities: None,
        })
        .await
        .unwrap();

        let result = client.list_tools().await;
        assert!(matches!(result, Err(MCPError::JsonRpc { code: -32601, .. })));
    }

    #[tokio::test]
    async fn test_capability_not_supported() {
        // Server with no tools capability
        let init_response = JsonRpcMessage::Response(JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: 1,
            result: serde_json::json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "serverInfo": { "name": "no-tools-server" }
            }),
        });

        let transport = MockTransport::new(vec![init_response]);

        let client = MCPClient::connect(MCPClientConfig {
            transport,
            client_info: None,
            capabilities: None,
        })
        .await
        .unwrap();

        let result = client.list_tools().await;
        assert!(matches!(
            result,
            Err(MCPError::CapabilityNotSupported { .. })
        ));
    }

    #[tokio::test]
    async fn test_close() {
        let transport = MockTransport::new(vec![make_init_response(1)]);

        let client = MCPClient::connect(MCPClientConfig {
            transport,
            client_info: None,
            capabilities: None,
        })
        .await
        .unwrap();

        assert!(!client.is_closed());
        client.close().await.unwrap();
        assert!(client.is_closed());
    }
}
