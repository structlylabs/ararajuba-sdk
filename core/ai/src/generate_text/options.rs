//! Options for `generate_text` and `stream_text`.

use crate::generate_text::callbacks::{
    ChunkEvent, ErrorEvent, FinishEvent, StartEvent, StepStartEvent, ToolCallFinishEvent,
    ToolCallStartEvent,
};
use crate::generate_text::prepare_step::PrepareStepFn;
use crate::generate_text::result::StepResult;
use crate::telemetry::config::TelemetrySettings;
use crate::tools::tool_approval::OnToolApproval;
use crate::tools::tool_set::ToolSet;
use crate::types::call_settings::CallSettings;
use ararajuba_provider::language_model::v4::prompt::Message;
use ararajuba_provider::language_model::v4::tool_choice::ToolChoice;

/// Options for `generate_text()` and `stream_text()`.
pub struct GenerateTextOptions {
    /// The language model to use.
    pub model: Box<dyn ararajuba_provider::LanguageModelV4>,
    /// System prompt.
    pub system: Option<String>,
    /// Simple text prompt (alternative to `messages`).
    pub prompt: Option<String>,
    /// Multi-turn messages (alternative to `prompt`).
    pub messages: Option<Vec<Message>>,
    /// Tools the model may call.
    pub tools: Option<ToolSet>,
    /// How the model should choose tools.
    pub tool_choice: Option<ToolChoice>,
    /// Maximum number of steps (tool loop iterations). Default: 1.
    pub max_steps: usize,
    /// Call settings (temperature, max_tokens, etc.).
    pub call_settings: CallSettings,
    /// Telemetry settings.
    pub telemetry: TelemetrySettings,

    // ── Callbacks ────────────────────────────────────────────────────────

    /// Callback invoked once at the very start, before any model call.
    pub on_start: Option<Box<dyn Fn(&StartEvent) + Send + Sync>>,
    /// Callback invoked at the start of each step.
    pub on_step_start: Option<Box<dyn Fn(&StepStartEvent) + Send + Sync>>,
    /// Callback invoked after each step.
    pub on_step_finish: Option<Box<dyn Fn(&StepResult) + Send + Sync>>,
    /// Callback invoked when a tool call is about to start.
    pub on_tool_call_start: Option<Box<dyn Fn(&ToolCallStartEvent) + Send + Sync>>,
    /// Callback invoked after a tool call finishes.
    pub on_tool_call_finish: Option<Box<dyn Fn(&ToolCallFinishEvent) + Send + Sync>>,
    /// Callback invoked when the entire generation completes.
    pub on_finish: Option<Box<dyn Fn(&FinishEvent) + Send + Sync>>,
    /// Callback invoked for each stream chunk (stream_text only).
    pub on_chunk: Option<Box<dyn Fn(&ChunkEvent) + Send + Sync>>,
    /// Callback invoked on stream error (stream_text only).
    pub on_error: Option<Box<dyn Fn(&ErrorEvent) + Send + Sync>>,

    /// Callback for tool approval (human-in-the-loop).
    ///
    /// When a tool has `needs_approval` set and it returns `true` for a given
    /// input, this callback is invoked before execution. If [`Approved`] is
    /// returned the tool runs normally. If [`Denied`] the tool is skipped and
    /// a denied result is recorded.
    ///
    /// If no callback is provided and approval is needed, the tool call is
    /// **not executed** and an approval request is returned in the step result
    /// (the loop stops, similar to the TS SDK behaviour).
    ///
    /// [`Approved`]: crate::tools::tool_approval::ToolApprovalResponse::Approved
    /// [`Denied`]: crate::tools::tool_approval::ToolApprovalResponse::Denied
    pub on_tool_approval: Option<OnToolApproval>,
    /// Callback for preliminary (intermediate) tool results.
    ///
    /// When a tool uses `execute_streaming`, each yielded value except the
    /// final one is emitted as a preliminary result. This callback allows the
    /// caller to observe intermediate progress (e.g. for live UI updates).
    pub on_preliminary_tool_result:
        Option<Box<dyn Fn(&crate::tools::tool_result::ToolResult) + Send + Sync>>,

    // ── Hooks ────────────────────────────────────────────────────────────

    /// Per-step hook to dynamically change tools, system prompt, or settings.
    pub prepare_step: Option<PrepareStepFn>,
}

impl Default for GenerateTextOptions {
    fn default() -> Self {
        Self {
            model: panic_model(),
            system: None,
            prompt: None,
            messages: None,
            tools: None,
            tool_choice: None,
            max_steps: 1,
            call_settings: CallSettings::default(),
            telemetry: TelemetrySettings::default(),
            on_start: None,
            on_step_start: None,
            on_step_finish: None,
            on_tool_call_start: None,
            on_tool_call_finish: None,
            on_finish: None,
            on_chunk: None,
            on_error: None,
            on_tool_approval: None,
            on_preliminary_tool_result: None,
            prepare_step: None,
        }
    }
}

/// Placeholder model that panics — forces the user to provide a real model.
fn panic_model() -> Box<dyn ararajuba_provider::LanguageModelV4> {
    struct PanicModel;

    #[async_trait::async_trait]
    impl ararajuba_provider::LanguageModelV4 for PanicModel {
        fn provider(&self) -> &str {
            panic!("No model provided")
        }
        fn model_id(&self) -> &str {
            panic!("No model provided")
        }
        async fn do_generate(
            &self,
            _options: &ararajuba_provider::language_model::v4::call_options::CallOptions,
        ) -> Result<
            ararajuba_provider::language_model::v4::generate_result::GenerateResult,
            ararajuba_provider::errors::Error,
        > {
            panic!("No model provided")
        }
        async fn do_stream(
            &self,
            _options: &ararajuba_provider::language_model::v4::call_options::CallOptions,
        ) -> Result<
            ararajuba_provider::language_model::v4::stream_result::StreamResult,
            ararajuba_provider::errors::Error,
        > {
            panic!("No model provided")
        }
    }

    Box::new(PanicModel)
}
