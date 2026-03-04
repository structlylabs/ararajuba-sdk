//! Result types for `generate_text`.

use crate::tools::tool_approval::ToolApprovalRequest;
use crate::tools::tool_call::ToolCall;
use crate::tools::tool_result::ToolResult;
use crate::types::call_warning::CallWarning;
use crate::types::finish_reason::FinishReason;
use ararajuba_provider::language_model::v4::generate_result::ResponseMetadata;
use ararajuba_provider::language_model::v4::usage::Usage;
use serde::{Deserialize, Serialize};

/// The result of a `generate_text()` call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateTextResult {
    /// The generated text (concatenated from all text content parts).
    pub text: String,
    /// Reasoning text, if the model produced reasoning.
    pub reasoning: Option<String>,
    /// Tool calls made by the model.
    pub tool_calls: Vec<ToolCall>,
    /// Results from executed tools.
    pub tool_results: Vec<ToolResult>,
    /// Tool calls that are pending approval (not yet executed).
    ///
    /// These are present when a tool's `needs_approval` returned `true` and no
    /// `on_tool_approval` callback was provided. The caller should collect
    /// approval responses and pass them in a follow-up `generate_text` call.
    pub tool_approval_requests: Vec<ToolApprovalRequest>,
    /// Token usage.
    pub usage: Usage,
    /// Why the model stopped.
    pub finish_reason: FinishReason,
    /// Response metadata from the provider.
    pub response: Option<ResponseMetadata>,
    /// All steps in the generation (each tool-loop iteration is a step).
    pub steps: Vec<StepResult>,
    /// Warnings from the call.
    pub warnings: Vec<CallWarning>,
}

/// A single step in the generate_text tool loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// Text generated in this step.
    pub text: String,
    /// Tool calls made in this step.
    pub tool_calls: Vec<ToolCall>,
    /// Tool results from this step.
    pub tool_results: Vec<ToolResult>,
    /// Tool approval requests from this step (tools awaiting approval).
    pub tool_approval_requests: Vec<ToolApprovalRequest>,
    /// Finish reason for this step.
    pub finish_reason: FinishReason,
    /// Token usage for this step.
    pub usage: Usage,
    /// Whether this step is a continuation.
    pub is_continued: bool,
}
