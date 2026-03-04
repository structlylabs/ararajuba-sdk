//! Options for `generate_object` and `stream_object`.

use super::output::Output;
use crate::types::call_settings::CallSettings;
use ararajuba_provider::language_model::v4::prompt::Message;
use futures::future::BoxFuture;
use std::sync::Arc;

/// A function that attempts to repair malformed model output text before
/// parsing. Receives `(text, schema, error_message)` and returns the repaired
/// text or `None` to skip repair.
pub type RepairTextFn = Arc<
    dyn Fn(String, serde_json::Value, String) -> BoxFuture<'static, Option<String>> + Send + Sync,
>;

/// Event emitted when `generate_object` finishes successfully.
#[derive(Debug, Clone)]
pub struct GenerateObjectFinishEvent {
    /// The parsed object.
    pub object: serde_json::Value,
    /// Token usage.
    pub usage: ararajuba_provider::language_model::v4::usage::Usage,
}

/// Options for `generate_object()`.
pub struct GenerateObjectOptions {
    /// The language model to use.
    pub model: Box<dyn ararajuba_provider::LanguageModelV4>,
    /// The output format/parser.
    pub output: Box<dyn Output>,
    /// System prompt.
    pub system: Option<String>,
    /// Simple text prompt.
    pub prompt: Option<String>,
    /// Multi-turn messages.
    pub messages: Option<Vec<Message>>,
    /// Call settings.
    pub call_settings: CallSettings,
    /// Optional repair function for malformed model output.
    pub repair_text: Option<RepairTextFn>,
    /// Callback invoked on successful completion.
    pub on_finish:
        Option<Box<dyn Fn(&GenerateObjectFinishEvent) + Send + Sync>>,
    /// Callback invoked on error.
    pub on_error: Option<Box<dyn Fn(&crate::error::Error) + Send + Sync>>,
}
