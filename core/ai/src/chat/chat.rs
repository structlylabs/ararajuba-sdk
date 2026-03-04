//! `Chat` — manages a conversation session with message history,
//! streaming, and tool handling.
//!
//! Mirrors the TS SDK's `AbstractChat`.

use crate::chat::transport::ChatTransport;
use crate::chat::types::{
    ChatRequestOptions, ChatStatus, ChatTrigger, OnChatError, OnChatFinish, SendMessagesOptions,
};
use crate::ui::chunk::UIMessageChunk;
use crate::ui::types::{UIMessage, UIMessageRole, UIPart};
use crate::util::serial_executor::SerialJobExecutor;
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

/// Unique ID generator function.
pub type IdGenerator = Arc<dyn Fn() -> String + Send + Sync>;

/// Default ID generator using UUID v4.
pub fn default_id_generator() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Initialization options for `Chat`.
pub struct ChatInit {
    /// Unique chat session ID. Generated if omitted.
    pub id: Option<String>,
    /// Initial messages.
    pub messages: Option<Vec<UIMessage>>,
    /// ID generator for messages.
    pub generate_id: Option<IdGenerator>,
    /// The transport to use (HTTP, direct, etc.).
    pub transport: Box<dyn ChatTransport>,
    /// Error callback.
    pub on_error: Option<OnChatError>,
    /// Finish callback.
    pub on_finish: Option<OnChatFinish>,
}

/// Active streaming response state.
struct ActiveResponse {
    cancel: CancellationToken,
}

/// A chat session managing messages, streaming, and transport.
pub struct Chat {
    /// Session ID.
    id: String,
    /// ID generator.
    generate_id: IdGenerator,
    /// Messages.
    messages: Arc<RwLock<Vec<UIMessage>>>,
    /// Current status.
    status: Arc<RwLock<ChatStatus>>,
    /// Last error message (stored as string since Error isn't Clone).
    error_message: Arc<RwLock<Option<String>>>,
    /// Transport.
    transport: Arc<dyn ChatTransport>,
    /// Active streaming response handle.
    active_response: Arc<RwLock<Option<ActiveResponse>>>,
    /// Serial executor for sequencing operations.
    _executor: SerialJobExecutor,
    /// Callbacks.
    on_error: Option<OnChatError>,
    on_finish: Option<OnChatFinish>,
}

impl Chat {
    /// Create a new chat session.
    pub fn new(init: ChatInit) -> Self {
        let id = init
            .id
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let generate_id: IdGenerator = init
            .generate_id
            .unwrap_or_else(|| Arc::new(default_id_generator));

        Self {
            id,
            generate_id,
            messages: Arc::new(RwLock::new(init.messages.unwrap_or_default())),
            status: Arc::new(RwLock::new(ChatStatus::Ready)),
            error_message: Arc::new(RwLock::new(None)),
            transport: Arc::from(init.transport),
            active_response: Arc::new(RwLock::new(None)),
            _executor: SerialJobExecutor::new(),
            on_error: init.on_error,
            on_finish: init.on_finish,
        }
    }

    /// Get the chat ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the current status.
    pub async fn status(&self) -> ChatStatus {
        *self.status.read().await
    }

    /// Get the last error message (if any).
    pub async fn error_message(&self) -> Option<String> {
        self.error_message.read().await.clone()
    }

    /// Get a snapshot of all messages.
    pub async fn messages(&self) -> Vec<UIMessage> {
        self.messages.read().await.clone()
    }

    /// Get the last message (if any).
    pub async fn last_message(&self) -> Option<UIMessage> {
        self.messages.read().await.last().cloned()
    }

    /// Replace the full message list.
    pub async fn set_messages(&self, messages: Vec<UIMessage>) {
        *self.messages.write().await = messages;
    }

    /// Clear any active error and set status to Ready.
    pub async fn clear_error(&self) {
        *self.error_message.write().await = None;
        *self.status.write().await = ChatStatus::Ready;
    }

    /// Send a text message.
    pub async fn send_message(
        &self,
        text: &str,
        options: Option<ChatRequestOptions>,
    ) -> Result<(), crate::error::Error> {
        let msg_id = (self.generate_id)();
        let message = UIMessage {
            id: msg_id.clone(),
            role: UIMessageRole::User,
            parts: vec![UIPart::Text(crate::ui::types::TextUIPart {
                id: (self.generate_id)(),
                text: text.to_string(),
            })],
            metadata: None,
            created_at: Some(chrono::Utc::now().to_string()),
        };

        // Add user message
        self.messages.write().await.push(message);

        // Update status
        *self.status.write().await = ChatStatus::Submitted;

        // Send via transport
        self.process_stream(
            ChatTrigger::SubmitMessage,
            Some(msg_id),
            options.unwrap_or_default(),
        )
        .await
    }

    /// Regenerate the last assistant message.
    pub async fn regenerate(
        &self,
        options: Option<ChatRequestOptions>,
    ) -> Result<(), crate::error::Error> {
        // Remove last assistant message if present
        {
            let mut msgs = self.messages.write().await;
            if let Some(last) = msgs.last() {
                if last.role == UIMessageRole::Assistant {
                    msgs.pop();
                }
            }
        }

        *self.status.write().await = ChatStatus::Submitted;

        self.process_stream(
            ChatTrigger::RegenerateMessage,
            None,
            options.unwrap_or_default(),
        )
        .await
    }

    /// Stop the currently active streaming response.
    pub async fn stop(&self) -> Result<(), crate::error::Error> {
        if let Some(active) = self.active_response.write().await.take() {
            active.cancel.cancel();
        }
        *self.status.write().await = ChatStatus::Ready;
        Ok(())
    }

    /// Internal: run a stream from the transport and process chunks.
    async fn process_stream(
        &self,
        trigger: ChatTrigger,
        message_id: Option<String>,
        request: ChatRequestOptions,
    ) -> Result<(), crate::error::Error> {
        let cancel = CancellationToken::new();
        *self.active_response.write().await = Some(ActiveResponse {
            cancel: cancel.clone(),
        });

        let messages = self.messages.read().await.clone();
        let send_opts = SendMessagesOptions {
            trigger,
            chat_id: self.id.clone(),
            message_id,
            messages,
            abort: Some(cancel.clone()),
            request,
        };

        let stream_result = self.transport.send_messages(send_opts).await;
        let mut stream = match stream_result {
            Ok(s) => s,
            Err(e) => {
                *self.status.write().await = ChatStatus::Error;
                let msg = e.to_string();
                *self.error_message.write().await = Some(msg);
                if let Some(ref on_err) = self.on_error {
                    on_err(&e);
                }
                return Err(e);
            }
        };

        *self.status.write().await = ChatStatus::Streaming;

        // Create an empty assistant message placeholder
        let assistant_msg = UIMessage {
            id: (self.generate_id)(),
            role: UIMessageRole::Assistant,
            parts: Vec::new(),
            metadata: None,
            created_at: Some(chrono::Utc::now().to_string()),
        };
        self.messages.write().await.push(assistant_msg);

        // Process stream chunks
        while let Some(chunk) = tokio::select! {
            c = stream.next() => c,
            _ = cancel.cancelled() => None,
        } {
            self.apply_chunk(chunk).await;
        }

        // Clean up
        *self.active_response.write().await = None;
        let final_status = *self.status.read().await;
        if final_status == ChatStatus::Streaming {
            *self.status.write().await = ChatStatus::Ready;
        }

        // Fire on_finish
        if let Some(ref on_finish) = self.on_finish {
            let msgs = self.messages.read().await;
            if let Some(last) = msgs.last() {
                let info = crate::chat::types::ChatFinishInfo {
                    message: last.clone(),
                    messages: msgs.clone(),
                    is_abort: cancel.is_cancelled(),
                    is_disconnect: false,
                    is_error: final_status == ChatStatus::Error,
                    finish_reason: None,
                };
                on_finish(info);
            }
        }

        Ok(())
    }

    /// Apply a single UIMessageChunk to the current assistant message.
    async fn apply_chunk(&self, chunk: UIMessageChunk) {
        let mut msgs = self.messages.write().await;
        let last = match msgs.last_mut() {
            Some(m) if m.role == UIMessageRole::Assistant => m,
            _ => return,
        };

        match chunk {
            UIMessageChunk::TextDelta { delta, .. } => {
                // Append to the last text part, or create one
                if let Some(UIPart::Text(t)) = last.parts.last_mut() {
                    t.text.push_str(&delta);
                } else {
                    last.parts.push(UIPart::Text(crate::ui::types::TextUIPart {
                        id: (self.generate_id)(),
                        text: delta,
                    }));
                }
            }
            UIMessageChunk::ReasoningDelta { delta, .. } => {
                if let Some(UIPart::Reasoning(r)) = last.parts.last_mut() {
                    r.reasoning.push_str(&delta);
                } else {
                    last.parts
                        .push(UIPart::Reasoning(crate::ui::types::ReasoningUIPart {
                            id: (self.generate_id)(),
                            reasoning: delta,
                            details: None,
                        }));
                }
            }
            UIMessageChunk::Error { error } => {
                let err = crate::error::Error::UIMessageStream {
                    message: error.clone(),
                };
                *self.error_message.write().await = Some(error);
                *self.status.write().await = ChatStatus::Error;
                if let Some(ref on_err) = self.on_error {
                    on_err(&err);
                }
            }
            // Other chunks are currently no-ops in this basic implementation
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_id_generator() {
        let id = default_id_generator();
        // UUID v4 format: 8-4-4-4-12
        assert_eq!(id.len(), 36);
        assert_eq!(id.chars().filter(|c| *c == '-').count(), 4);
    }
}
