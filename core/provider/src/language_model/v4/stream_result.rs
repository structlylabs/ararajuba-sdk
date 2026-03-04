//! v4 Stream result with typed, split streams and Drop-based cancellation.
//!
//! Unlike v3 which used a single `BoxStream<StreamPart>` with 15+ variants,
//! v4 splits the stream into three typed channels:
//!
//! - **content**: text deltas, reasoning deltas, file data
//! - **tool_calls**: tool call lifecycle events
//! - **metadata**: usage, finish reason, provider metadata
//!
//! Dropping a `StreamResult` automatically cancels the underlying HTTP request
//! via the abort handle.

use super::finish_reason::FinishReason;
use super::usage::Usage;
use crate::shared::{Headers, ProviderMetadata};
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Typed delta enums
// ---------------------------------------------------------------------------

/// Content events emitted during streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentDelta {
    /// A chunk of generated text.
    #[serde(rename = "text")]
    Text(String),
    /// A chunk of reasoning/chain-of-thought text.
    #[serde(rename = "reasoning")]
    Reasoning(String),
    /// A file chunk (e.g. inline image from the model).
    #[serde(rename = "file")]
    File {
        mime_type: String,
        data: Vec<u8>,
    },
}

/// Tool call lifecycle events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolCallDelta {
    /// A new tool call has started.
    #[serde(rename = "start")]
    Start {
        id: String,
        name: String,
    },
    /// An incremental delta of the tool call's JSON input.
    #[serde(rename = "input-delta")]
    InputDelta(String),
    /// The tool call is complete with fully parsed input.
    #[serde(rename = "complete")]
    Complete {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

/// Metadata events (usage, finish, provider metadata).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MetadataDelta {
    /// Token usage update.
    #[serde(rename = "usage")]
    Usage(Usage),
    /// Generation finished.
    #[serde(rename = "finish-reason")]
    FinishReason(FinishReason),
    /// Provider-specific metadata.
    #[serde(rename = "provider-metadata")]
    ProviderMetadata(ProviderMetadata),
}

// ---------------------------------------------------------------------------
// Abort handle
// ---------------------------------------------------------------------------

/// A handle that cancels an in-flight HTTP request when dropped or aborted.
pub struct AbortHandle {
    inner: Option<tokio::task::AbortHandle>,
}

impl AbortHandle {
    /// Create a new abort handle wrapping a Tokio `AbortHandle`.
    pub fn new(handle: tokio::task::AbortHandle) -> Self {
        Self {
            inner: Some(handle),
        }
    }

    /// Create an inert (no-op) abort handle.
    pub fn noop() -> Self {
        Self { inner: None }
    }

    /// Explicitly abort the underlying request.
    pub fn abort(&mut self) {
        if let Some(handle) = self.inner.take() {
            handle.abort();
        }
    }
}

impl Drop for AbortHandle {
    fn drop(&mut self) {
        self.abort();
    }
}

// ---------------------------------------------------------------------------
// StreamResult
// ---------------------------------------------------------------------------

/// Result of initiating a v4 streaming language model generation.
///
/// Contains three typed streams plus an abort handle for cancellation.
/// **Dropping** this struct automatically cancels the underlying HTTP request.
///
/// # Streams
///
/// - [`content`](Self::content) — text, reasoning, and file deltas
/// - [`tool_calls`](Self::tool_calls) — tool call lifecycle events
/// - [`metadata`](Self::metadata) — usage, finish reason, provider metadata
///
/// # Cancellation
///
/// The stream is cancelled when:
/// 1. The `StreamResult` is dropped (automatic via abort handle)
/// 2. An explicit `CancellationToken` is triggered (opt-in via `CallOptions`)
pub struct StreamResult {
    /// Stream of content deltas (text, reasoning, files).
    pub content: BoxStream<'static, ContentDelta>,
    /// Stream of tool call events.
    pub tool_calls: BoxStream<'static, ToolCallDelta>,
    /// Stream of metadata events (usage, finish reason).
    pub metadata: BoxStream<'static, MetadataDelta>,
    /// Abort handle — cancels the HTTP request on drop.
    pub abort_handle: AbortHandle,
    /// Request metadata.
    pub request: Option<StreamRequestMetadata>,
    /// Response metadata (from the initial HTTP response).
    pub response: Option<StreamResponseMetadata>,
}

impl StreamResult {
    /// Take ownership of the three typed streams, consuming this `StreamResult`.
    ///
    /// Returns `(content, tool_calls, metadata)` streams along with the abort handle.
    /// The caller becomes responsible for the abort handle (dropping it cancels the request).
    pub fn into_streams(
        mut self,
    ) -> (
        BoxStream<'static, ContentDelta>,
        BoxStream<'static, ToolCallDelta>,
        BoxStream<'static, MetadataDelta>,
        AbortHandle,
        Option<StreamRequestMetadata>,
        Option<StreamResponseMetadata>,
    ) {
        let content = std::mem::replace(&mut self.content, Box::pin(futures::stream::empty()));
        let tool_calls =
            std::mem::replace(&mut self.tool_calls, Box::pin(futures::stream::empty()));
        let metadata = std::mem::replace(&mut self.metadata, Box::pin(futures::stream::empty()));
        let abort = std::mem::replace(&mut self.abort_handle, AbortHandle::noop());
        let request = self.request.take();
        let response = self.response.take();
        // Prevent the Drop from aborting since we moved the handle out
        std::mem::forget(self);
        (content, tool_calls, metadata, abort, request, response)
    }
}

impl Drop for StreamResult {
    fn drop(&mut self) {
        self.abort_handle.abort();
    }
}

/// Request metadata for streams.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamRequestMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
}

/// Response metadata for streams.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamResponseMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Headers>,
}

// ---------------------------------------------------------------------------
// Conversion helpers: v3 merged stream → v4 split streams
// ---------------------------------------------------------------------------

use super::stream_part::StreamPart;
use futures::StreamExt;

/// Convert a v3-style merged `BoxStream<StreamPart>` into v4 split streams.
///
/// This is a compatibility helper for providers that still emit a single
/// merged stream internally. It spawns a demux task that routes events
/// to the appropriate typed channel.
pub fn split_merged_stream(
    merged: BoxStream<'static, Result<StreamPart, crate::errors::Error>>,
    abort_handle: AbortHandle,
    request: Option<StreamRequestMetadata>,
    response: Option<StreamResponseMetadata>,
) -> StreamResult {
    let (content_tx, content_rx) = tokio::sync::mpsc::unbounded_channel::<ContentDelta>();
    let (tool_tx, tool_rx) = tokio::sync::mpsc::unbounded_channel::<ToolCallDelta>();
    let (meta_tx, meta_rx) = tokio::sync::mpsc::unbounded_channel::<MetadataDelta>();

    // Spawn background demux task
    tokio::spawn(async move {
        let mut merged = std::pin::pin!(merged);
        while let Some(result) = merged.next().await {
            let part = match result {
                Ok(p) => p,
                Err(_) => break,
            };

            match part {
                // Content events
                StreamPart::TextDelta { delta, .. } => {
                    let _ = content_tx.send(ContentDelta::Text(delta));
                }
                StreamPart::ReasoningDelta { delta, .. } => {
                    let _ = content_tx.send(ContentDelta::Reasoning(delta));
                }
                StreamPart::File {
                    media_type, data, ..
                } => {
                    let bytes = data.into_bytes();
                    let _ = content_tx.send(ContentDelta::File {
                        mime_type: media_type,
                        data: bytes,
                    });
                }
                // Tool call events
                StreamPart::ToolInputStart { id, tool_name, .. } => {
                    let _ = tool_tx.send(ToolCallDelta::Start {
                        id,
                        name: tool_name,
                    });
                }
                StreamPart::ToolInputDelta { delta, .. } => {
                    let _ = tool_tx.send(ToolCallDelta::InputDelta(delta));
                }
                StreamPart::ToolCall {
                    tool_call_id,
                    tool_name,
                    input,
                    ..
                } => {
                    let parsed_input =
                        serde_json::from_str(&input).unwrap_or(serde_json::Value::String(input));
                    let _ = tool_tx.send(ToolCallDelta::Complete {
                        id: tool_call_id,
                        name: tool_name,
                        input: parsed_input,
                    });
                }
                // Metadata events
                StreamPart::Finish {
                    usage,
                    finish_reason,
                    provider_metadata,
                } => {
                    let _ = meta_tx.send(MetadataDelta::Usage(usage));
                    let _ = meta_tx.send(MetadataDelta::FinishReason(finish_reason));
                    if let Some(pm) = provider_metadata {
                        let _ = meta_tx.send(MetadataDelta::ProviderMetadata(pm));
                    }
                }
                // Ignored in v4 typed streams (lifecycle events)
                StreamPart::TextStart { .. }
                | StreamPart::TextEnd { .. }
                | StreamPart::ReasoningStart { .. }
                | StreamPart::ReasoningEnd { .. }
                | StreamPart::ToolInputEnd { .. }
                | StreamPart::StreamStart { .. }
                | StreamPart::ResponseMetadata(_)
                | StreamPart::Raw { .. }
                | StreamPart::Error { .. }
                | StreamPart::ToolResult { .. }
                | StreamPart::ToolApprovalRequest { .. }
                | StreamPart::Source(_) => {}
            }
        }
    });

    use tokio_stream::wrappers::UnboundedReceiverStream;

    StreamResult {
        content: Box::pin(UnboundedReceiverStream::new(content_rx)),
        tool_calls: Box::pin(UnboundedReceiverStream::new(tool_rx)),
        metadata: Box::pin(UnboundedReceiverStream::new(meta_rx)),
        abort_handle,
        request,
        response,
    }
}
