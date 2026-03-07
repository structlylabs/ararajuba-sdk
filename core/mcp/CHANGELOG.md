# Changelog – `ararajuba-mcp`

## 0.1.2 (unreleased)

### Changed
- organization name structlylabs -> atlantichq

## 0.1.1 (unreleased)

### Changed
- **Renamed** from `mcp-client` to `ararajuba-mcp`

## 0.1.0 (unreleased)

### Added
- `MCPClient` — manages connection lifecycle, tool discovery, and tool execution
  via the Model Context Protocol (JSON-RPC 2.0).
- `create_mcp_client()` convenience function with initialization handshake.
- `MCPTransport` trait for pluggable transport backends.
- **Stdio transport** (`StdioMCPTransport`) — spawns a child process, communicates
  via newline-delimited JSON over stdin/stdout.
- **HTTP transport** (`HttpMCPTransport`) — streamable HTTP transport (MCP spec
  2025-06-18) with session management via `mcp-session-id`.
- **SSE transport** (`SseMCPTransport`) — legacy Server-Sent Events transport with
  endpoint discovery and origin validation.
- `mcp_tools_to_sdk_tools()` — converts MCP tool definitions to AI SDK `Tool`
  objects (function tools with JSON Schema).
- `call_tool_result_to_text()` — converts MCP tool call results to text.
- Protocol version negotiation (supports 2025-06-18, 2025-03-26, 2024-11-05).
- Server capability checking before requests.
- Full JSON-RPC 2.0 message types (request, response, error, notification).
- MCP protocol types (initialize, tools/list, tools/call).
- `MCPError` enum for transport, protocol, serialization, and capability errors.
- 32 unit tests + 2 doc-tests.
