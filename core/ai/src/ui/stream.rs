//! `create_ui_message_stream` — transforms a `StreamTextResult` into a stream
//! of `UIMessageChunk` events suitable for SSE delivery.

use super::chunk::{PartId, UIMessageChunk};
use crate::error::Error;
use crate::generate_text::stream_text::{StreamTextPart, StreamTextResult};
use futures::stream::{BoxStream, StreamExt};

/// Create a UI message stream from a `StreamTextResult`.
///
/// Transforms the model's full stream into typed `UIMessageChunk` events
/// that can be serialised as JSON lines / SSE.
pub fn create_ui_message_stream(
    result: StreamTextResult,
    message_id: String,
) -> BoxStream<'static, Result<UIMessageChunk, Error>> {
    let full_stream = result.full_stream;

    let mut text_part_counter: u32 = 0;
    let mut reasoning_part_counter: u32 = 0;
    let mut tool_part_counter: u32 = 0;
    let mut current_text_id: Option<PartId> = None;
    let mut current_reasoning_id: Option<PartId> = None;

    let msg_id = message_id.clone();

    // Prefix: Start event
    let start_chunk = UIMessageChunk::Start {
        message_id: message_id.clone(),
    };
    let start_step_chunk = UIMessageChunk::StartStep {
        message_id: message_id.clone(),
    };

    let mapped = full_stream.flat_map(move |item| {
        let mut chunks: Vec<Result<UIMessageChunk, Error>> = Vec::new();

        match item {
            Err(e) => {
                chunks.push(Ok(UIMessageChunk::Error {
                    error: e.to_string(),
                }));
            }
            Ok(part) => match part {
                StreamTextPart::TextDelta(delta) => {
                    // Start a new text part if needed.
                    if current_text_id.is_none() {
                        text_part_counter += 1;
                        let id = format!("text-{text_part_counter}");
                        chunks.push(Ok(UIMessageChunk::TextStart { id: id.clone() }));
                        current_text_id = Some(id);
                    }
                    let id = current_text_id.as_ref().unwrap().clone();
                    chunks.push(Ok(UIMessageChunk::TextDelta { id, delta }));
                }
                StreamTextPart::ReasoningDelta(delta) => {
                    if current_reasoning_id.is_none() {
                        reasoning_part_counter += 1;
                        let id = format!("reasoning-{reasoning_part_counter}");
                        chunks.push(Ok(UIMessageChunk::ReasoningStart { id: id.clone() }));
                        current_reasoning_id = Some(id);
                    }
                    let id = current_reasoning_id.as_ref().unwrap().clone();
                    chunks.push(Ok(UIMessageChunk::ReasoningDelta { id, delta }));
                }
                StreamTextPart::ToolCall(tc) => {
                    // Close open text/reasoning parts.
                    if let Some(id) = current_text_id.take() {
                        chunks.push(Ok(UIMessageChunk::TextEnd { id }));
                    }
                    if let Some(id) = current_reasoning_id.take() {
                        chunks.push(Ok(UIMessageChunk::ReasoningEnd { id }));
                    }

                    tool_part_counter += 1;
                    let id = format!("tool-{tool_part_counter}");
                    chunks.push(Ok(UIMessageChunk::ToolInputStart {
                        id: id.clone(),
                        tool_call_id: tc.tool_call_id.clone(),
                        tool_name: tc.tool_name.clone(),
                    }));
                    chunks.push(Ok(UIMessageChunk::ToolInputAvailable {
                        id,
                        input: tc.input,
                    }));
                }
                StreamTextPart::ToolResult(tr) => {
                    // Find matching tool part ID.
                    let id = format!("tool-result-{}", tr.tool_call_id);
                    if tr.is_error {
                        chunks.push(Ok(UIMessageChunk::ToolOutputError {
                            id,
                            error: tr.result.to_string(),
                        }));
                    } else {
                        chunks.push(Ok(UIMessageChunk::ToolOutputAvailable {
                            id,
                            output: tr.result,
                        }));
                    }
                }
                StreamTextPart::FinishStep {
                    finish_reason,
                    ..
                } => {
                    // Close open text/reasoning parts.
                    if let Some(id) = current_text_id.take() {
                        chunks.push(Ok(UIMessageChunk::TextEnd { id }));
                    }
                    if let Some(id) = current_reasoning_id.take() {
                        chunks.push(Ok(UIMessageChunk::ReasoningEnd { id }));
                    }
                    chunks.push(Ok(UIMessageChunk::FinishStep {
                        message_id: msg_id.clone(),
                        finish_reason: format!("{:?}", finish_reason),
                    }));
                    // Next step starts.
                    chunks.push(Ok(UIMessageChunk::StartStep {
                        message_id: msg_id.clone(),
                    }));
                }
                StreamTextPart::Finish {
                    finish_reason,
                    ..
                } => {
                    // Close open text/reasoning parts.
                    if let Some(id) = current_text_id.take() {
                        chunks.push(Ok(UIMessageChunk::TextEnd { id }));
                    }
                    if let Some(id) = current_reasoning_id.take() {
                        chunks.push(Ok(UIMessageChunk::ReasoningEnd { id }));
                    }
                    chunks.push(Ok(UIMessageChunk::Finish {
                        message_id: msg_id.clone(),
                        finish_reason: format!("{:?}", finish_reason),
                    }));
                }
                StreamTextPart::Error(msg) => {
                    chunks.push(Ok(UIMessageChunk::Error { error: msg }));
                }
            },
        }

        futures::stream::iter(chunks)
    });

    // Prepend start events.
    let prefix = futures::stream::iter(vec![
        Ok(start_chunk),
        Ok(start_step_chunk),
    ]);

    // The finish chunk is emitted by the mapped stream on StreamTextPart::Finish.
    let combined = prefix.chain(mapped);

    Box::pin(combined)
}

/// Helper: serialize a stream of `UIMessageChunk` into SSE-formatted strings.
pub fn chunks_to_sse(
    stream: BoxStream<'static, Result<UIMessageChunk, Error>>,
) -> BoxStream<'static, Result<String, Error>> {
    let mapped = stream.map(|chunk_result| {
        chunk_result.and_then(|chunk| {
            chunk
                .to_sse_event()
                .map_err(|e| Error::UIMessageStream {
                    message: format!("SSE serialization error: {e}"),
                })
        })
    });
    Box::pin(mapped)
}
