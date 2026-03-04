//! `DirectChatTransport` — in-process transport that calls an `Agent` directly.
//!
//! Mirrors the TS SDK's `DirectChatTransport`. Instead of making HTTP requests,
//! it invokes the agent's `stream()` method directly and converts the result
//! into a stream of `UIMessageChunk`.

use crate::agent::agent::Agent;
use crate::chat::transport::ChatTransport;
use crate::chat::types::{ReconnectOptions, SendMessagesOptions};
use crate::ui::chunk::UIMessageChunk;
use crate::ui::convert::convert_to_model_messages;
use async_trait::async_trait;
use futures::stream::BoxStream;
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::Mutex;

/// A transport that invokes an `Agent` directly, without HTTP.
///
/// This is useful for server-side rendering, testing, or any scenario
/// where the agent runs in the same process as the chat UI.
pub struct DirectChatTransport {
    agent: Arc<Mutex<Agent>>,
}

impl DirectChatTransport {
    /// Create a new direct transport around an agent.
    pub fn new(agent: Agent) -> Self {
        Self {
            agent: Arc::new(Mutex::new(agent)),
        }
    }

    /// Create from a shared agent reference.
    pub fn from_shared(agent: Arc<Mutex<Agent>>) -> Self {
        Self { agent }
    }
}

#[async_trait]
impl ChatTransport for DirectChatTransport {
    async fn send_messages(
        &self,
        options: SendMessagesOptions,
    ) -> Result<BoxStream<'static, UIMessageChunk>, crate::error::Error> {
        // Convert UI messages to model messages
        let model_messages = convert_to_model_messages(&options.messages)?;

        // Build a combined prompt from the model messages
        let prompt_text = model_messages
            .iter()
            .filter_map(|m| match m {
                ararajuba_provider::language_model::v4::prompt::Message::User { content, .. } => {
                    Some(
                        content
                            .iter()
                            .filter_map(|part| match part {
                                ararajuba_provider::language_model::v4::prompt::UserContentPart::Text(
                                    t,
                                ) => Some(t.text.as_str()),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join(""),
                    )
                }
                _ => None,
            })
            .last()
            .unwrap_or_default();

        // Stream from the agent
        let mut agent = self.agent.lock().await;
        let stream_result = agent.stream(&prompt_text).await?;

        // Convert the text stream into UIMessageChunks
        let text_stream = stream_result.text_stream;
        let id: String = uuid::Uuid::new_v4().to_string();
        let chunk_stream = text_stream.filter_map(move |result| {
            let id = id.clone();
            async move {
                match result {
                    Ok(delta) => Some(UIMessageChunk::TextDelta { id, delta }),
                    Err(_) => None, // Skip error chunks
                }
            }
        });

        Ok(Box::pin(chunk_stream))
    }

    async fn reconnect_to_stream(
        &self,
        _options: ReconnectOptions,
    ) -> Result<Option<BoxStream<'static, UIMessageChunk>>, crate::error::Error> {
        // Direct transport doesn't support reconnection
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direct_transport_created() {
        // This is mostly a compile test — creating the transport requires
        // an Agent, which requires a model. Just verify the type exists.
        let _: fn(Agent) -> DirectChatTransport = DirectChatTransport::new;
    }
}
