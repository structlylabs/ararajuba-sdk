//! Tool approval types for human-in-the-loop tool execution.

use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A request for approval before executing a tool call.
///
/// When a tool's `needs_approval` function returns `true`, the SDK emits
/// this request. If an `on_tool_approval` callback is provided, it is called
/// inline. Otherwise the tool call is skipped (not executed) and the approval
/// request is returned in the step result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolApprovalRequest {
    /// Unique ID for this approval request.
    pub approval_id: String,
    /// The tool call ID this approval relates to.
    pub tool_call_id: String,
    /// Name of the tool.
    pub tool_name: String,
    /// The input that will be passed to the tool.
    pub input: serde_json::Value,
}

/// The response to a tool approval request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolApprovalResponse {
    /// Approve — the tool will be executed normally.
    Approved,
    /// Deny — the tool will NOT be executed. An optional reason is recorded.
    Denied { reason: Option<String> },
}

/// Callback type for handling tool approval requests.
///
/// Receives a [`ToolApprovalRequest`] and must return a [`ToolApprovalResponse`].
/// This is an async callback so it can perform I/O (e.g. prompt a user over HTTP).
pub type OnToolApproval = Arc<
    dyn Fn(ToolApprovalRequest) -> BoxFuture<'static, ToolApprovalResponse> + Send + Sync,
>;
