//! The `stream_object` function.

use super::options::GenerateObjectOptions;
use crate::error::Error;
use ararajuba_provider::language_model::v4::call_options::CallOptions;
use ararajuba_provider::language_model::v4::stream_result::ContentDelta;
use futures::stream::{BoxStream, StreamExt};
use std::pin::Pin;

/// Result of `stream_object()`.
pub struct StreamObjectResult {
    /// Stream of partial objects (progressively more complete).
    pub partial_object_stream:
        Pin<Box<dyn futures::Stream<Item = Result<serde_json::Value, Error>> + Send>>,
}

/// Stream a structured object from a language model, emitting partial results.
pub async fn stream_object(options: GenerateObjectOptions) -> Result<StreamObjectResult, Error> {
    let _span = tracing::info_span!(
        "stream_object",
        model = %options.model.model_id(),
        provider = %options.model.provider(),
    )
    .entered();

    let messages = build_messages(&options);
    let response_format = options.output.response_format();

    let call_options = CallOptions {
        prompt: messages,
        max_output_tokens: options.call_settings.max_output_tokens,
        temperature: options.call_settings.temperature,
        stop_sequences: options.call_settings.stop_sequences.clone(),
        top_p: options.call_settings.top_p,
        top_k: options.call_settings.top_k,
        presence_penalty: options.call_settings.presence_penalty,
        frequency_penalty: options.call_settings.frequency_penalty,
        response_format: Some(response_format),
        seed: options.call_settings.seed,
        tools: None,
        tool_choice: None,
        include_raw_chunks: None,
        headers: options.call_settings.headers.clone(),
        provider_options: None,
        cancellation_token: options.call_settings.cancellation_token.clone(),
    };

    let stream_result = options.model.do_stream(&call_options).await?;
    let _output = options.output;

    // Take ownership of the content stream from v4's StreamResult
    let (content_stream, _tool_calls, _metadata, _abort, _req, _resp) = stream_result.into_streams();

    // Accumulate text deltas and emit partial parses of the accumulated text.
    let partial_stream: BoxStream<'static, Result<serde_json::Value, Error>> = {
        struct State {
            accumulated: String,
        }

        let state = std::sync::Arc::new(tokio::sync::Mutex::new(State {
            accumulated: String::new(),
        }));

        content_stream
            .filter_map(move |delta| {
                let state = state.clone();
                async move {
                    match delta {
                        ContentDelta::Text(text) => {
                            let mut st = state.lock().await;
                            st.accumulated.push_str(&text);
                            // Try to parse partial
                            let repaired =
                                ararajuba_provider_utils::parsing::json_repair::repair_incomplete_json(
                                    &st.accumulated,
                                );
                            match serde_json::from_str::<serde_json::Value>(&repaired) {
                                Ok(value) => Some(Ok(value)),
                                Err(_) => None, // Skip until we have parseable JSON
                            }
                        }
                        _ => None, // Skip non-text content (reasoning, files)
                    }
                }
            })
            .boxed()
    };

    Ok(StreamObjectResult {
        partial_object_stream: Box::pin(partial_stream),
    })
}

fn build_messages(
    options: &GenerateObjectOptions,
) -> Vec<ararajuba_provider::language_model::v4::prompt::Message> {
    use ararajuba_provider::language_model::v4::content_part::TextPart;
    use ararajuba_provider::language_model::v4::prompt::{Message, UserContentPart};

    let mut messages = Vec::new();

    if let Some(ref system) = options.system {
        messages.push(Message::System {
            content: system.clone(),
            provider_options: None,
        });
    }

    if let Some(ref prompt) = options.prompt {
        messages.push(Message::User {
            content: vec![UserContentPart::Text(TextPart {
                text: prompt.clone(),
                provider_options: None,
            })],
            provider_options: None,
        });
    }

    if let Some(ref msgs) = options.messages {
        messages.extend(msgs.clone());
    }

    messages
}
