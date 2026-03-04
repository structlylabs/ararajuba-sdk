//! HTTP response helpers for UI message streams and text streams.
//!
//! These helpers create SSE-formatted responses and provide piping utilities
//! for integrating with HTTP frameworks (axum, actix-web, etc.).

use super::chunk::UIMessageChunk;
use super::stream::chunks_to_sse;
use crate::error::Error;
use crate::generate_text::stream_text::StreamTextResult;
use futures::stream::{BoxStream, StreamExt};

/// An HTTP-ready SSE response body.
///
/// Contains the SSE text stream and recommended headers. Framework adapters
/// should set these headers on the HTTP response.
pub struct SseResponse {
    /// The SSE text stream (each item is a `data: ...\n\n` line).
    pub stream: BoxStream<'static, Result<String, Error>>,
    /// Content-Type header value.
    pub content_type: &'static str,
    /// Cache-Control header value.
    pub cache_control: &'static str,
}

/// Create an SSE response from a UI message stream result.
///
/// Transforms a `StreamTextResult` into SSE events suitable for HTTP delivery.
///
/// # Example
/// ```ignore
/// let response = create_ui_message_stream_response(stream_result, "msg-123".into());
/// // Set headers: Content-Type = response.content_type
/// // Stream body: response.stream
/// ```
pub fn create_ui_message_stream_response(
    result: StreamTextResult,
    message_id: String,
) -> SseResponse {
    let chunk_stream = super::stream::create_ui_message_stream(result, message_id);
    let sse_stream = chunks_to_sse(chunk_stream);

    SseResponse {
        stream: sse_stream,
        content_type: "text/event-stream; charset=utf-8",
        cache_control: "no-cache, no-transform",
    }
}

/// Pipe a UI message chunk stream to an SSE response.
///
/// Use this when you already have a `UIMessageChunk` stream and want to
/// convert it to an HTTP SSE response.
pub fn pipe_ui_message_stream_to_response(
    chunk_stream: BoxStream<'static, Result<UIMessageChunk, Error>>,
) -> SseResponse {
    let sse_stream = chunks_to_sse(chunk_stream);

    SseResponse {
        stream: sse_stream,
        content_type: "text/event-stream; charset=utf-8",
        cache_control: "no-cache, no-transform",
    }
}

/// Create a plain text streaming response from a `StreamTextResult`.
///
/// Only emits the text deltas (no structured events).
pub fn create_text_stream_response(result: StreamTextResult) -> SseResponse {
    let text_stream = result.text_stream;
    let mapped = text_stream.map(|chunk| {
        chunk.map(|text| format!("data: {text}\n\n"))
    });

    SseResponse {
        stream: Box::pin(mapped),
        content_type: "text/event-stream; charset=utf-8",
        cache_control: "no-cache, no-transform",
    }
}

/// Pipe a text stream to an SSE response.
///
/// Each text chunk is wrapped in an SSE `data:` event.
pub fn pipe_text_stream_to_response(
    text_stream: BoxStream<'static, Result<String, Error>>,
) -> SseResponse {
    let mapped = text_stream.map(|chunk| {
        chunk.map(|text| format!("data: {text}\n\n"))
    });

    SseResponse {
        stream: Box::pin(mapped),
        content_type: "text/event-stream; charset=utf-8",
        cache_control: "no-cache, no-transform",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_response_headers() {
        let stream: BoxStream<'static, Result<String, Error>> =
            Box::pin(futures::stream::empty());
        let response = pipe_text_stream_to_response(stream);
        assert_eq!(response.content_type, "text/event-stream; charset=utf-8");
        assert_eq!(response.cache_control, "no-cache, no-transform");
    }
}
