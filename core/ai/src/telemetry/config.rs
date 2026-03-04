//! `TelemetrySettings` — user-facing configuration for telemetry.

use std::collections::HashMap;

/// Configuration for AI SDK telemetry.
///
/// Controls whether spans are recorded and what data they contain.
///
/// # Example
/// ```ignore
/// use ararajuba_core::telemetry::config::TelemetrySettings;
///
/// let telemetry = TelemetrySettings {
///     is_enabled: true,
///     function_id: Some("chat-handler".to_string()),
///     record_inputs: true,
///     record_outputs: true,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone)]
pub struct TelemetrySettings {
    /// Whether telemetry is enabled (default: `false`).
    ///
    /// When disabled, no spans are recorded regardless of other settings.
    pub is_enabled: bool,

    /// Whether to record model inputs (prompts, messages) as span attributes.
    ///
    /// May be disabled for privacy or to reduce span size.
    pub record_inputs: bool,

    /// Whether to record model outputs (text, tool calls) as span attributes.
    pub record_outputs: bool,

    /// An application-level function identifier (e.g., `"chat-handler"`,
    /// `"summarize"`). Appears as `ai.telemetry.functionId` in spans.
    pub function_id: Option<String>,

    /// Additional key-value metadata to attach to spans.
    ///
    /// All values are recorded as `ai.telemetry.metadata.<key>`.
    pub metadata: HashMap<String, String>,
}

impl Default for TelemetrySettings {
    fn default() -> Self {
        Self {
            is_enabled: false,
            record_inputs: true,
            record_outputs: true,
            function_id: None,
            metadata: HashMap::new(),
        }
    }
}

impl TelemetrySettings {
    /// Create with telemetry enabled and sensible defaults.
    pub fn enabled() -> Self {
        Self {
            is_enabled: true,
            ..Default::default()
        }
    }

    /// Build the `operation.name` attribute value.
    ///
    /// Format: `"ai.<operation> <functionId>"` or just `"ai.<operation>"` if
    /// no function ID is set.
    pub fn operation_name(&self, operation: &str) -> String {
        match &self.function_id {
            Some(fid) => format!("ai.{operation} {fid}"),
            None => format!("ai.{operation}"),
        }
    }
}

/// Well-known span attribute names following the emerging GenAI semantic conventions.
pub mod attributes {
    // ── AI SDK attributes ─────────────────────────────────────────────
    pub const AI_MODEL_PROVIDER: &str = "ai.model.provider";
    pub const AI_MODEL_ID: &str = "ai.model.id";
    pub const AI_OPERATION_ID: &str = "ai.operationId";
    pub const AI_PROMPT: &str = "ai.prompt";
    pub const AI_RESPONSE_TEXT: &str = "ai.response.text";
    pub const AI_RESPONSE_FINISH_REASON: &str = "ai.response.finishReason";
    pub const AI_RESPONSE_TOOL_CALLS: &str = "ai.response.toolCalls";
    pub const AI_USAGE_PROMPT_TOKENS: &str = "ai.usage.promptTokens";
    pub const AI_USAGE_COMPLETION_TOKENS: &str = "ai.usage.completionTokens";
    pub const AI_TELEMETRY_FUNCTION_ID: &str = "ai.telemetry.functionId";

    // ── GenAI semantic convention attributes ───────────────────────────
    pub const GEN_AI_SYSTEM: &str = "gen_ai.system";
    pub const GEN_AI_REQUEST_MODEL: &str = "gen_ai.request.model";
    pub const GEN_AI_REQUEST_TEMPERATURE: &str = "gen_ai.request.temperature";
    pub const GEN_AI_REQUEST_MAX_TOKENS: &str = "gen_ai.request.max_tokens";
    pub const GEN_AI_RESPONSE_FINISH_REASONS: &str = "gen_ai.response.finish_reasons";
    pub const GEN_AI_USAGE_INPUT_TOKENS: &str = "gen_ai.usage.input_tokens";
    pub const GEN_AI_USAGE_OUTPUT_TOKENS: &str = "gen_ai.usage.output_tokens";
}
