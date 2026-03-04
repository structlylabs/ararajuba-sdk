//! `prepareStep` — per-step hook for dynamic tool / setting changes.
//!
//! Before each step in the tool loop, `prepareStep` (if provided) is called
//! with the current step context. It can modify the tool set, system prompt,
//! and call settings for the upcoming model invocation.

use crate::tools::tool_set::ToolSet;
use crate::types::call_settings::CallSettings;
use futures::future::BoxFuture;
use std::sync::Arc;

/// Context passed to the `prepare_step` hook before each model call.
#[derive(Debug, Clone)]
pub struct PrepareStepContext {
    /// Zero-based step index.
    pub step_index: usize,
    /// Number of tool calls made in the previous step (0 for the first step).
    pub previous_tool_call_count: usize,
    /// Whether the previous step had the `ToolCalls` finish reason.
    pub previous_step_had_tool_calls: bool,
}

/// The result returned by a `prepare_step` hook. Each field is optional;
/// `None` means "keep the original value".
pub struct PrepareStepResult {
    /// Override the tool set for this step.
    pub tools: Option<ToolSet>,
    /// Override the system prompt for this step.
    pub system: Option<String>,
    /// Override call settings for this step.
    pub call_settings: Option<CallSettings>,
    /// Override max tokens for this step.
    pub max_output_tokens: Option<u32>,
    /// Override temperature for this step.
    pub temperature: Option<f64>,
}

impl Default for PrepareStepResult {
    fn default() -> Self {
        Self {
            tools: None,
            system: None,
            call_settings: None,
            max_output_tokens: None,
            temperature: None,
        }
    }
}

/// The `prepareStep` callback type.
///
/// Takes a `PrepareStepContext` and returns a future resolving to an optional
/// `PrepareStepResult`. Returning `None` means "no changes for this step".
pub type PrepareStepFn = Arc<
    dyn Fn(PrepareStepContext) -> BoxFuture<'static, Option<PrepareStepResult>> + Send + Sync,
>;
