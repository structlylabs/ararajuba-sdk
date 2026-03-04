//! `simulate_streaming_middleware` — wraps non-streaming models to look streaming.
//!
//! When `do_stream` is called, this middleware calls `do_generate` instead,
//! then synthesizes a stream of `StreamPart` events from the generate result.

use crate::middleware::wrap_language_model::LanguageModelMiddleware;
use ararajuba_provider::language_model::v4::content::Content;
use ararajuba_provider::language_model::v4::stream_part::StreamPart;
use ararajuba_provider::language_model::v4::stream_result::{split_merged_stream, AbortHandle, StreamRequestMetadata, StreamResponseMetadata};
use futures::stream;

/// Creates middleware that simulates streaming by calling `do_generate`
/// and emitting the result as a series of stream events.
///
/// Useful for wrapping models that only support non-streaming generation
/// to make them compatible with `stream_text()`.
pub fn simulate_streaming_middleware() -> LanguageModelMiddleware {
    LanguageModelMiddleware {
        wrap_stream: Some(Box::new(|do_generate, _do_stream, params, _model_ref| {
            Box::pin(async move {
                // Call generate instead of stream.
                let result = do_generate(params).await?;

                // Build a synthetic stream from the generate result.
                let mut parts: Vec<Result<StreamPart, ararajuba_provider::errors::Error>> = Vec::new();

                // Emit stream start.
                parts.push(Ok(StreamPart::StreamStart {
                    warnings: result.warnings.clone(),
                }));

                // Emit content as stream parts.
                let mut text_id_counter = 0u32;
                for content in &result.content {
                    match content {
                        Content::Text { text, provider_metadata } => {
                            let id = format!("text-{text_id_counter}");
                            text_id_counter += 1;
                            parts.push(Ok(StreamPart::TextStart {
                                id: id.clone(),
                                provider_metadata: provider_metadata.clone(),
                            }));
                            parts.push(Ok(StreamPart::TextDelta {
                                id: id.clone(),
                                delta: text.clone(),
                                provider_metadata: None,
                            }));
                            parts.push(Ok(StreamPart::TextEnd {
                                id,
                                provider_metadata: None,
                            }));
                        }
                        Content::Reasoning { text, provider_metadata } => {
                            let id = format!("reasoning-{text_id_counter}");
                            text_id_counter += 1;
                            parts.push(Ok(StreamPart::ReasoningStart {
                                id: id.clone(),
                                provider_metadata: provider_metadata.clone(),
                            }));
                            parts.push(Ok(StreamPart::ReasoningDelta {
                                id: id.clone(),
                                delta: text.clone(),
                                provider_metadata: None,
                            }));
                            parts.push(Ok(StreamPart::ReasoningEnd {
                                id,
                                provider_metadata: None,
                            }));
                        }
                        Content::ToolCall {
                            tool_call_id,
                            tool_name,
                            input,
                            provider_executed,
                            dynamic,
                            provider_metadata,
                        } => {
                            parts.push(Ok(StreamPart::ToolCall {
                                tool_call_id: tool_call_id.clone(),
                                tool_name: tool_name.clone(),
                                input: input.clone(),
                                provider_executed: *provider_executed,
                                dynamic: *dynamic,
                                provider_metadata: provider_metadata.clone(),
                            }));
                        }
                        _ => {}
                    }
                }

                // Emit response metadata if available.
                if let Some(response) = &result.response {
                    parts.push(Ok(StreamPart::ResponseMetadata(response.clone())));
                }

                // Emit finish.
                parts.push(Ok(StreamPart::Finish {
                    usage: result.usage,
                    finish_reason: result.finish_reason,
                    provider_metadata: result.provider_metadata,
                }));

                let request_meta = result.request.map(|r| {
                    StreamRequestMetadata { body: r.body }
                });
                let response_meta = result.response.and_then(|r| {
                    Some(StreamResponseMetadata { headers: r.headers })
                });

                Ok(split_merged_stream(
                    Box::pin(stream::iter(parts)),
                    AbortHandle::noop(),
                    request_meta,
                    response_meta,
                ))
            })
        })),
        ..LanguageModelMiddleware::default()
    }
}
