//! Agent UI stream helpers — bridge between Agent streaming and UIMessageChunk.
//!
//! Mirrors the TS SDK's `createAgentUIStream`, `createAgentUIStreamResponse`,
//! and `pipeAgentUIStreamToResponse`.

use crate::agent::agent::Agent;
use crate::error::Error;
use crate::generate_text::stream_text::StreamTextPart;
use crate::ui::chunk::UIMessageChunk;
use crate::ui::convert::convert_to_model_messages;
use crate::ui::response::SseResponse;
use crate::ui::types::UIMessage;
use futures::stream::BoxStream;
use futures::StreamExt;

/// Options for `create_agent_ui_stream`.
pub struct AgentUIStreamOptions {
    /// The UI messages to send (including the latest user message).
    pub ui_messages: Vec<UIMessage>,
    /// Message ID for the response.
    pub message_id: String,
}

/// Create a stream of `UIMessageChunk` from an agent by converting UI messages
/// to model messages, streaming from the agent, and converting back to UI chunks.
///
/// This is the Rust equivalent of the TS SDK's `createAgentUIStream()`.
pub async fn create_agent_ui_stream(
    agent: &mut Agent,
    opts: AgentUIStreamOptions,
) -> Result<BoxStream<'static, UIMessageChunk>, Error> {
    // Convert UI messages to model messages
    let _model_messages = convert_to_model_messages(&opts.ui_messages)?;

    // Extract the last user text to use as the prompt
    let prompt_text = opts
        .ui_messages
        .iter()
        .rev()
        .filter(|m| m.role == crate::ui::types::UIMessageRole::User)
        .flat_map(|m| m.parts.iter())
        .find_map(|p| match p {
            crate::ui::types::UIPart::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .unwrap_or_default();

    // Stream from the agent
    let stream_result = agent.stream(&prompt_text).await?;

    // Convert StreamTextPart → UIMessageChunk
    let part_id = opts.message_id;
    let chunk_stream = stream_result.full_stream.filter_map(move |result| {
        let part_id = part_id.clone();
        async move {
            match result {
                Ok(part) => stream_part_to_chunk(part, &part_id),
                Err(_) => None,
            }
        }
    });

    Ok(Box::pin(chunk_stream))
}

/// Convert a `StreamTextPart` to a `UIMessageChunk`.
fn stream_part_to_chunk(part: StreamTextPart, part_id: &str) -> Option<UIMessageChunk> {
    match part {
        StreamTextPart::TextDelta(delta) => Some(UIMessageChunk::TextDelta {
            id: part_id.to_string(),
            delta,
        }),
        StreamTextPart::ReasoningDelta(delta) => Some(UIMessageChunk::ReasoningDelta {
            id: part_id.to_string(),
            delta,
        }),
        StreamTextPart::Error(error) => Some(UIMessageChunk::Error { error }),
        StreamTextPart::Finish {
            finish_reason,
            ..
        } => Some(UIMessageChunk::Finish {
            message_id: part_id.to_string(),
            finish_reason: format!("{:?}", finish_reason),
        }),
        // Tool calls, tool results, and step finishes are internal to the agent loop
        _ => None,
    }
}

/// Create an SSE response from an agent stream.
///
/// This is the Rust equivalent of the TS SDK's `createAgentUIStreamResponse()`.
/// Uses the Agent's `stream()` and wraps the result in an SSE response.
pub async fn create_agent_ui_stream_response(
    agent: &mut Agent,
    opts: AgentUIStreamOptions,
) -> Result<SseResponse, Error> {
    // Extract the last user text
    let prompt_text = opts
        .ui_messages
        .iter()
        .rev()
        .filter(|m| m.role == crate::ui::types::UIMessageRole::User)
        .flat_map(|m| m.parts.iter())
        .find_map(|p| match p {
            crate::ui::types::UIPart::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .unwrap_or_default();

    let stream_result = agent.stream(&prompt_text).await?;
    let response =
        crate::ui::response::create_ui_message_stream_response(stream_result, opts.message_id);
    Ok(response)
}

/// Pipe an agent UI stream to an SSE response.
///
/// This is the Rust equivalent of the TS SDK's `pipeAgentUIStreamToResponse()`.
pub async fn pipe_agent_ui_stream_to_response(
    agent: &mut Agent,
    opts: AgentUIStreamOptions,
) -> Result<SseResponse, Error> {
    create_agent_ui_stream_response(agent, opts).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_part_to_chunk_text_delta() {
        let chunk = stream_part_to_chunk(StreamTextPart::TextDelta("hello".into()), "p1");
        assert!(matches!(chunk, Some(UIMessageChunk::TextDelta { .. })));
    }

    #[test]
    fn test_stream_part_to_chunk_error() {
        let chunk = stream_part_to_chunk(StreamTextPart::Error("oops".into()), "p1");
        assert!(matches!(chunk, Some(UIMessageChunk::Error { .. })));
    }

    #[test]
    fn test_stream_part_to_chunk_tool_call_is_none() {
        use crate::tools::tool_call::ToolCall;
        let chunk = stream_part_to_chunk(
            StreamTextPart::ToolCall(ToolCall {
                tool_call_id: "tc1".into(),
                tool_name: "search".into(),
                input: serde_json::json!({}),
            }),
            "p1",
        );
        assert!(chunk.is_none());
    }
}
