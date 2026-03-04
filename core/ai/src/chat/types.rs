//! Chat framework types: status, state, request options, callbacks.

use crate::ui::types::UIMessage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Status of the chat session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChatStatus {
    /// Message submitted, waiting for response.
    Submitted,
    /// Response is streaming.
    Streaming,
    /// Idle, ready for new messages.
    Ready,
    /// An error occurred.
    Error,
}

impl Default for ChatStatus {
    fn default() -> Self {
        Self::Ready
    }
}

/// Options that can be sent with each chat request.
#[derive(Debug, Clone, Default)]
pub struct ChatRequestOptions {
    /// Additional HTTP headers.
    pub headers: Option<HashMap<String, String>>,
    /// Additional body payload merged into the request.
    pub body: Option<serde_json::Value>,
    /// Arbitrary metadata attached to the request.
    pub metadata: Option<serde_json::Value>,
}

/// Reason the chat finished.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChatFinishReason {
    /// Model decided to stop.
    Stop,
    /// Tool calls were made.
    ToolCalls,
    /// Maximum tokens reached.
    Length,
    /// Content was filtered.
    ContentFilter,
    /// An error occurred.
    Error,
    /// Other / unknown.
    Other,
}

/// Info provided to the `on_finish` callback.
#[derive(Debug, Clone)]
pub struct ChatFinishInfo {
    /// The last assistant message.
    pub message: UIMessage,
    /// All messages in the conversation.
    pub messages: Vec<UIMessage>,
    /// Whether the stream was user-aborted.
    pub is_abort: bool,
    /// Whether a disconnect happened.
    pub is_disconnect: bool,
    /// Whether an error occurred.
    pub is_error: bool,
    /// The finish reason if available.
    pub finish_reason: Option<ChatFinishReason>,
}

/// Callback: called when an error occurs.
pub type OnChatError = Box<dyn Fn(&crate::error::Error) + Send + Sync>;

/// Callback: called when the response stream finishes.
pub type OnChatFinish = Box<dyn Fn(ChatFinishInfo) + Send + Sync>;

/// Trigger for why a message is being sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatTrigger {
    /// User submitted a new message.
    SubmitMessage,
    /// Regenerating a previous assistant message.
    RegenerateMessage,
}

/// Options passed to `ChatTransport::send_messages`.
#[derive(Debug)]
pub struct SendMessagesOptions {
    /// Why this send was triggered.
    pub trigger: ChatTrigger,
    /// Unique chat session ID.
    pub chat_id: String,
    /// ID of the specific message (for regeneration).
    pub message_id: Option<String>,
    /// Full message list.
    pub messages: Vec<UIMessage>,
    /// Cancellation token.
    pub abort: Option<tokio_util::sync::CancellationToken>,
    /// Request options.
    pub request: ChatRequestOptions,
}

/// Options passed to `ChatTransport::reconnect_to_stream`.
#[derive(Debug)]
pub struct ReconnectOptions {
    /// Chat session ID.
    pub chat_id: String,
    /// Request options.
    pub request: ChatRequestOptions,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_status_default() {
        assert_eq!(ChatStatus::default(), ChatStatus::Ready);
    }

    #[test]
    fn test_chat_status_serde_roundtrip() {
        let s = serde_json::to_string(&ChatStatus::Streaming).unwrap();
        assert_eq!(s, "\"streaming\"");
        let back: ChatStatus = serde_json::from_str(&s).unwrap();
        assert_eq!(back, ChatStatus::Streaming);
    }

    #[test]
    fn test_chat_trigger_eq() {
        assert_ne!(ChatTrigger::SubmitMessage, ChatTrigger::RegenerateMessage);
    }
}
