//! `Agent` — a stateful wrapper around `generate_text` / `stream_text` that
//! manages conversation history, tools, and multi-step execution across
//! multiple `.call()` invocations.
//!
//! Mirrors the TS SDK's `ToolLoopAgent` pattern.

use crate::error::Error;
use crate::generate_text::callbacks::{
    FinishEvent, StartEvent, StepStartEvent, ToolCallFinishEvent, ToolCallStartEvent,
};
use crate::generate_text::generate_text::generate_text;
use crate::generate_text::options::GenerateTextOptions;
use crate::generate_text::prepare_step::PrepareStepFn;
use crate::generate_text::result::{GenerateTextResult, StepResult};
use crate::generate_text::stream_text::{stream_text, StreamTextResult};
use crate::tools::tool_set::ToolSet;
use crate::types::call_settings::CallSettings;
use ararajuba_provider::language_model::v4::language_model_v4::LanguageModelV4;
use ararajuba_provider::language_model::v4::prompt::Message;
use std::sync::Arc;

/// The agent specification version.
pub const AGENT_VERSION: &str = "agent-v1";

/// Settings for creating an `Agent`.
pub struct AgentSettings {
    /// Optional unique identifier for this agent.
    pub id: Option<String>,
    /// The language model to use (shared ownership for multi-call reuse).
    pub model: Arc<dyn LanguageModelV4>,
    /// System prompt / instructions (can be a single string or multiple).
    pub system: Option<String>,
    /// Tool set available to the agent.
    pub tools: Option<ToolSet>,
    /// Maximum tool-loop steps per call (default: 20).
    pub max_steps: usize,
    /// Default call settings applied to every invocation.
    pub call_settings: CallSettings,
    /// Optional callback at the very start of a call.
    pub on_start: Option<Arc<dyn Fn(&StartEvent) + Send + Sync>>,
    /// Optional callback at the start of each step.
    pub on_step_start: Option<Arc<dyn Fn(&StepStartEvent) + Send + Sync>>,
    /// Optional callback after each step completes.
    pub on_step_finish: Option<Arc<dyn Fn(&StepResult) + Send + Sync>>,
    /// Optional callback when a tool call starts.
    pub on_tool_call_start: Option<Arc<dyn Fn(&ToolCallStartEvent) + Send + Sync>>,
    /// Optional callback when a tool call finishes.
    pub on_tool_call_finish: Option<Arc<dyn Fn(&ToolCallFinishEvent) + Send + Sync>>,
    /// Optional callback when the entire call finishes.
    pub on_finish: Option<Arc<dyn Fn(&FinishEvent) + Send + Sync>>,
    /// Optional step preparation function.
    pub prepare_step: Option<PrepareStepFn>,
}

impl AgentSettings {
    /// Create minimal settings with a model and defaults.
    pub fn new(model: Arc<dyn LanguageModelV4>) -> Self {
        Self {
            id: None,
            model,
            system: None,
            tools: None,
            max_steps: 20,
            call_settings: CallSettings::default(),
            on_start: None,
            on_step_start: None,
            on_step_finish: None,
            on_tool_call_start: None,
            on_tool_call_finish: None,
            on_finish: None,
            prepare_step: None,
        }
    }
}

/// A stateful agent that accumulates conversation history across calls.
///
/// Each call to `call()` or `stream()` extends the internal message history
/// and executes the model with the full context.
pub struct Agent {
    /// Agent identifier (optional).
    id: Option<String>,
    model: Arc<dyn LanguageModelV4>,
    system: Option<String>,
    tools: Option<ToolSet>,
    max_steps: usize,
    call_settings: CallSettings,
    history: Vec<Message>,
    on_start: Option<Arc<dyn Fn(&StartEvent) + Send + Sync>>,
    on_step_start: Option<Arc<dyn Fn(&StepStartEvent) + Send + Sync>>,
    on_step_finish: Option<Arc<dyn Fn(&StepResult) + Send + Sync>>,
    on_tool_call_start: Option<Arc<dyn Fn(&ToolCallStartEvent) + Send + Sync>>,
    on_tool_call_finish: Option<Arc<dyn Fn(&ToolCallFinishEvent) + Send + Sync>>,
    on_finish: Option<Arc<dyn Fn(&FinishEvent) + Send + Sync>>,
    prepare_step: Option<PrepareStepFn>,
}

impl Agent {
    /// Create a new agent from settings.
    pub fn new(settings: AgentSettings) -> Self {
        Self {
            id: settings.id,
            model: settings.model,
            system: settings.system,
            tools: settings.tools,
            max_steps: settings.max_steps,
            call_settings: settings.call_settings,
            history: Vec::new(),
            on_start: settings.on_start,
            on_step_start: settings.on_step_start,
            on_step_finish: settings.on_step_finish,
            on_tool_call_start: settings.on_tool_call_start,
            on_tool_call_finish: settings.on_tool_call_finish,
            on_finish: settings.on_finish,
            prepare_step: settings.prepare_step,
        }
    }

    /// The agent specification version.
    pub fn version(&self) -> &'static str {
        AGENT_VERSION
    }

    /// The agent identifier (if set).
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// Get the tool set (if any).
    pub fn tools(&self) -> Option<&ToolSet> {
        self.tools.as_ref()
    }

    /// Get the current message history.
    pub fn history(&self) -> &[Message] {
        &self.history
    }

    /// Clear the conversation history.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Send a user message and get a text response.
    ///
    /// The model will loop through tool calls (up to `max_steps`) and return
    /// the final text. The conversation history is updated with both the user
    /// message and the assistant's response.
    pub async fn call(&mut self, prompt: &str) -> Result<GenerateTextResult, Error> {
        // Append user message to history
        self.history.push(Message::User {
            content: vec![
                ararajuba_provider::language_model::v4::prompt::UserContentPart::Text(
                    ararajuba_provider::language_model::v4::content_part::TextPart {
                        text: prompt.to_string(),
                        provider_options: None,
                    },
                ),
            ],
            provider_options: None,
        });

        let options = self.build_options();
        let result = generate_text(options).await?;

        // Append assistant response to history
        self.append_assistant_response(&result);

        Ok(result)
    }

    /// Send a user message and get a streaming response.
    ///
    /// **Note**: Streaming does not automatically append the assistant response
    /// to history. After consuming the stream, call `append_text()` to update
    /// history manually.
    pub async fn stream(&mut self, prompt: &str) -> Result<StreamTextResult, Error> {
        self.history.push(Message::User {
            content: vec![
                ararajuba_provider::language_model::v4::prompt::UserContentPart::Text(
                    ararajuba_provider::language_model::v4::content_part::TextPart {
                        text: prompt.to_string(),
                        provider_options: None,
                    },
                ),
            ],
            provider_options: None,
        });

        let options = self.build_options();
        stream_text(options).await
    }

    /// Manually append an assistant text response to history.
    ///
    /// Use this after consuming a `stream()` result.
    pub fn append_text(&mut self, text: &str) {
        use ararajuba_provider::language_model::v4::content_part::TextPart;
        use ararajuba_provider::language_model::v4::prompt::AssistantContentPart;

        self.history.push(Message::Assistant {
            content: vec![AssistantContentPart::Text(TextPart {
                text: text.to_string(),
                provider_options: None,
            })],
            provider_options: None,
        });
    }

    fn build_options(&self) -> GenerateTextOptions {
        // Re-build tool set from references — ToolDef closures are Arc-wrapped,
        // so this is a cheap clone of Arcs + strings.
        let tools = self.tools.as_ref().map(|ts| {
            let mut new_set = ToolSet::new();
            for (_name, tool_def) in ts.iter() {
                new_set = new_set.add_ref(tool_def);
            }
            new_set
        });

        GenerateTextOptions {
            model: Box::new(ArcModel(Arc::clone(&self.model))),
            system: self.system.clone(),
            prompt: None,
            messages: Some(self.history.clone()),
            tools,
            max_steps: self.max_steps,
            call_settings: self.call_settings.clone(),
            telemetry: Default::default(),
            tool_choice: None,
            on_start: self.on_start.as_ref().map(|cb| {
                let cb = Arc::clone(cb);
                let boxed: Box<dyn Fn(&StartEvent) + Send + Sync> =
                    Box::new(move |ev| cb(ev));
                boxed
            }),
            on_step_start: self.on_step_start.as_ref().map(|cb| {
                let cb = Arc::clone(cb);
                let boxed: Box<dyn Fn(&StepStartEvent) + Send + Sync> =
                    Box::new(move |ev| cb(ev));
                boxed
            }),
            on_step_finish: self.on_step_finish.as_ref().map(|cb| {
                let cb = Arc::clone(cb);
                let boxed: Box<dyn Fn(&StepResult) + Send + Sync> =
                    Box::new(move |step| cb(step));
                boxed
            }),
            on_tool_call_start: self.on_tool_call_start.as_ref().map(|cb| {
                let cb = Arc::clone(cb);
                let boxed: Box<dyn Fn(&ToolCallStartEvent) + Send + Sync> =
                    Box::new(move |ev| cb(ev));
                boxed
            }),
            on_tool_call_finish: self.on_tool_call_finish.as_ref().map(|cb| {
                let cb = Arc::clone(cb);
                let boxed: Box<dyn Fn(&ToolCallFinishEvent) + Send + Sync> =
                    Box::new(move |ev| cb(ev));
                boxed
            }),
            on_finish: self.on_finish.as_ref().map(|cb| {
                let cb = Arc::clone(cb);
                let boxed: Box<dyn Fn(&FinishEvent) + Send + Sync> =
                    Box::new(move |ev| cb(ev));
                boxed
            }),
            on_chunk: None,
            on_error: None,
            on_tool_approval: None,
            on_preliminary_tool_result: None,
            prepare_step: self.prepare_step.clone(),
        }
    }

    fn append_assistant_response(&mut self, result: &GenerateTextResult) {
        use ararajuba_provider::language_model::v4::content_part::TextPart;
        use ararajuba_provider::language_model::v4::prompt::AssistantContentPart;

        if !result.text.is_empty() {
            self.history.push(Message::Assistant {
                content: vec![AssistantContentPart::Text(TextPart {
                    text: result.text.clone(),
                    provider_options: None,
                })],
                provider_options: None,
            });
        }
    }
}

/// A thin wrapper around `Arc<dyn LanguageModelV4>` that implements `LanguageModelV4`,
/// allowing shared ownership of a model across multiple `GenerateTextOptions`.
struct ArcModel(Arc<dyn LanguageModelV4>);

#[async_trait::async_trait]
impl LanguageModelV4 for ArcModel {
    fn specification_version(&self) -> &'static str {
        self.0.specification_version()
    }

    fn provider(&self) -> &str {
        self.0.provider()
    }

    fn model_id(&self) -> &str {
        self.0.model_id()
    }

    async fn do_generate(
        &self,
        options: &ararajuba_provider::language_model::v4::call_options::CallOptions,
    ) -> Result<
        ararajuba_provider::language_model::v4::generate_result::GenerateResult,
        ararajuba_provider::errors::Error,
    > {
        self.0.do_generate(options).await
    }

    async fn do_stream(
        &self,
        options: &ararajuba_provider::language_model::v4::call_options::CallOptions,
    ) -> Result<
        ararajuba_provider::language_model::v4::stream_result::StreamResult,
        ararajuba_provider::errors::Error,
    > {
        self.0.do_stream(options).await
    }
}
