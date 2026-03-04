//! Callback event types for `generate_text` and `stream_text`.
//!
//! Each event struct captures the data available at the point the callback
//! fires. Callbacks are typed as `Box<dyn Fn(&Event) + Send + Sync>` for
//! synchronous observation, or as async closures for approval flows.

use crate::tools::tool_call::ToolCall;
use crate::tools::tool_result::ToolResult;
use crate::types::finish_reason::FinishReason;
use ararajuba_provider::language_model::v4::usage::Usage;

/// Event emitted at the very start of a `generate_text` / `stream_text` call,
/// before any model invocations.
#[derive(Debug, Clone)]
pub struct StartEvent {
    /// The model ID.
    pub model_id: String,
    /// The provider name.
    pub provider: String,
}

/// Event emitted at the start of each step, before calling the model.
#[derive(Debug, Clone)]
pub struct StepStartEvent {
    /// Zero-based step index.
    pub step_index: usize,
    /// The model ID used for this step.
    pub model_id: String,
}

/// Event emitted when a tool call starts (the model requested it, but it
/// hasn't been executed yet).
#[derive(Debug, Clone)]
pub struct ToolCallStartEvent {
    /// The tool call about to be executed.
    pub tool_call: ToolCall,
}

/// Event emitted after a tool call finishes execution.
#[derive(Debug, Clone)]
pub struct ToolCallFinishEvent {
    /// The original tool call.
    pub tool_call: ToolCall,
    /// The result of executing the tool.
    pub tool_result: ToolResult,
}

/// Event emitted when the entire `generate_text` / `stream_text` completes.
#[derive(Debug, Clone)]
pub struct FinishEvent {
    /// Concatenated text from all steps.
    pub text: String,
    /// Total token usage across all steps.
    pub usage: Usage,
    /// Finish reason of the last step.
    pub finish_reason: FinishReason,
    /// Number of steps executed.
    pub step_count: usize,
}

/// Event emitted for each stream chunk (only for `stream_text`).
#[derive(Debug, Clone)]
pub struct ChunkEvent {
    /// The raw stream chunk.
    pub chunk: crate::generate_text::stream_text::StreamTextPart,
}

/// Event emitted when a stream error occurs (only for `stream_text`).
#[derive(Debug, Clone)]
pub struct ErrorEvent {
    /// The error message.
    pub error: String,
}
