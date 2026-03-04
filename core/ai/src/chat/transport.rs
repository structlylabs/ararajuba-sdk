//! `ChatTransport` — abstract interface for sending/receiving chat messages.
//!
//! Mirrors the TS SDK's `ChatTransport` interface.

use crate::chat::types::{ReconnectOptions, SendMessagesOptions};
use crate::ui::chunk::UIMessageChunk;
use async_trait::async_trait;
use futures::stream::BoxStream;

/// A transport handles the network (or in-process) communication for a chat
/// session.  Implementations include HTTP-based, SSE-based, or direct
/// (in-process) transports.
#[async_trait]
pub trait ChatTransport: Send + Sync {
    /// Send user messages and return a stream of UI message chunks.
    async fn send_messages(
        &self,
        options: SendMessagesOptions,
    ) -> Result<BoxStream<'static, UIMessageChunk>, crate::error::Error>;

    /// Attempt to reconnect to an in-progress stream (e.g. after page reload).
    /// Returns `None` if reconnection is not supported.
    async fn reconnect_to_stream(
        &self,
        options: ReconnectOptions,
    ) -> Result<Option<BoxStream<'static, UIMessageChunk>>, crate::error::Error>;
}
