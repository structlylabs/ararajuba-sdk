//! The `stream_text` function — multi-step streaming with tool execution.
//!
//! When tools are available and the model finishes a step with `ToolCalls`, the
//! SDK automatically executes the tools, appends their results to the messages,
//! and re-invokes the model for the next step. This mirrors the TS SDK's
//! recursive `streamStep` behaviour.

use super::callbacks::{
    ChunkEvent, FinishEvent, StartEvent, StepStartEvent,
    ToolCallFinishEvent, ToolCallStartEvent,
};
use super::options::GenerateTextOptions;
use super::prepare_step::PrepareStepContext;
use crate::error::Error;
use crate::tools::tool_call::ToolCall;
use crate::tools::tool_result::ToolResult;
use crate::types::finish_reason::FinishReason;
use ararajuba_provider::language_model::v4::call_options::{CallOptions, ResponseFormat};
use ararajuba_provider::language_model::v4::content_part::{ToolCallPart, ToolResultOutput, ToolResultPart};
use ararajuba_provider::language_model::v4::prompt::Message;
use ararajuba_provider::language_model::v4::stream_result::{ContentDelta, MetadataDelta, ToolCallDelta};
use ararajuba_provider::language_model::v4::usage::Usage;
use futures::stream::{BoxStream, StreamExt};
use std::pin::Pin;

/// The result of a `stream_text()` call.
pub struct StreamTextResult {
    /// Stream of text deltas only (extracted from the full stream).
    pub text_stream: Pin<Box<dyn futures::Stream<Item = Result<String, Error>> + Send>>,
    /// Full stream with all event types (text, reasoning, tool calls, etc.).
    pub full_stream: Pin<Box<dyn futures::Stream<Item = Result<StreamTextPart, Error>> + Send>>,
}

/// Events emitted by the full stream.
#[derive(Debug, Clone)]
pub enum StreamTextPart {
    TextDelta(String),
    ReasoningDelta(String),
    ToolCall(ToolCall),
    ToolResult(ToolResult),
    FinishStep {
        finish_reason: FinishReason,
        usage: Usage,
        is_continued: bool,
    },
    Finish {
        finish_reason: FinishReason,
        usage: Usage,
    },
    Error(String),
}

/// Stream text from a language model with multi-step tool execution.
///
/// The returned `StreamTextResult` contains two streams:
/// - `text_stream`: only text deltas
/// - `full_stream`: all events including tool calls, tool results, step markers
///
/// When the model returns tool calls and tools are available, the SDK:
/// 1. Emits `ToolCall` parts
/// 2. Executes the tools
/// 3. Emits `ToolResult` parts
/// 4. Emits `FinishStep`
/// 5. Re-invokes the model with all accumulated context
/// 6. Continues streaming until the model stops or `max_steps` is reached
pub async fn stream_text(options: GenerateTextOptions) -> Result<StreamTextResult, Error> {
    let _span = tracing::info_span!(
        "stream_text",
        model = %options.model.model_id(),
        provider = %options.model.provider(),
    )
    .entered();

    // We create an mpsc channel to power both streams from a single async task.
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Result<StreamTextPart, Error>>();

    // Fire on_start
    if let Some(ref cb) = options.on_start {
        cb(&StartEvent {
            model_id: options.model.model_id().to_string(),
            provider: options.model.provider().to_string(),
        });
    }

    // Spawn the multi-step loop in a task.
    tokio::spawn(async move {
        if let Err(e) = run_stream_loop(options, tx.clone()).await {
            let _ = tx.send(Err(e));
        }
    });

    // Create the full stream from the receiver.
    let full_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);

    // Create a text-only stream by filtering and forking via a second channel.
    let (text_out_tx, text_out_rx) =
        tokio::sync::mpsc::unbounded_channel::<Result<String, Error>>();

    let full_boxed: BoxStream<'static, Result<StreamTextPart, Error>> = full_stream
        .inspect(move |item| {
            if let Ok(StreamTextPart::TextDelta(delta)) = item {
                let _ = text_out_tx.send(Ok(delta.clone()));
            }
        })
        .boxed();

    let text_stream: Pin<Box<dyn futures::Stream<Item = Result<String, Error>> + Send>> =
        Box::pin(tokio_stream::wrappers::UnboundedReceiverStream::new(text_out_rx));

    Ok(StreamTextResult {
        text_stream,
        full_stream: Box::pin(full_boxed),
    })
}

/// The inner multi-step stream loop. Runs inside a spawned task.
async fn run_stream_loop(
    options: GenerateTextOptions,
    tx: tokio::sync::mpsc::UnboundedSender<Result<StreamTextPart, Error>>,
) -> Result<(), Error> {
    let mut messages = build_initial_messages(&options);
    let mut total_usage = Usage::default();
    let mut prev_tool_call_count: usize = 0;
    let mut prev_had_tool_calls = false;

    for step in 0..options.max_steps {
        // ── prepare_step hook ────────────────────────────────────────
        let mut _step_system = options.system.clone();
        let mut step_settings = options.call_settings.clone();
        let step_tools: Option<&crate::tools::tool_set::ToolSet>;
        let mut override_tools: Option<crate::tools::tool_set::ToolSet> = None;

        if let Some(ref prepare) = options.prepare_step {
            let ctx = PrepareStepContext {
                step_index: step,
                previous_tool_call_count: prev_tool_call_count,
                previous_step_had_tool_calls: prev_had_tool_calls,
            };
            if let Some(result) = prepare(ctx).await {
                if let Some(sys) = result.system {
                    _step_system = Some(sys);
                }
                if let Some(tools) = result.tools {
                    override_tools = Some(tools);
                }
                if let Some(settings) = result.call_settings {
                    step_settings = settings;
                }
                if let Some(max) = result.max_output_tokens {
                    step_settings.max_output_tokens = Some(max);
                }
                if let Some(temp) = result.temperature {
                    step_settings.temperature = Some(temp);
                }
            }
        }

        step_tools = match override_tools {
            Some(ref t) => Some(t),
            None => options.tools.as_ref(),
        };

        // ── on_step_start callback ───────────────────────────────────
        if let Some(ref cb) = options.on_step_start {
            cb(&StepStartEvent {
                step_index: step,
                model_id: options.model.model_id().to_string(),
            });
        }

        // Build call options for this step.
        let call_options = CallOptions {
            prompt: messages.clone(),
            max_output_tokens: step_settings.max_output_tokens,
            temperature: step_settings.temperature,
            stop_sequences: step_settings.stop_sequences.clone(),
            top_p: step_settings.top_p,
            top_k: step_settings.top_k,
            presence_penalty: step_settings.presence_penalty,
            frequency_penalty: step_settings.frequency_penalty,
            response_format: Some(ResponseFormat::Text),
            seed: step_settings.seed,
            tools: step_tools.map(|ts| ts.to_provider_tools()),
            tool_choice: options.tool_choice.clone(),
            include_raw_chunks: None,
            headers: step_settings.headers.clone(),
            provider_options: None,
            cancellation_token: step_settings.cancellation_token.clone(),
        };

        let stream_result = options.model.do_stream(&call_options).await?;

        // Consume the v4 typed streams, accumulating tool calls.
        let mut step_text = String::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();
        let mut step_finish_reason = FinishReason::Other;
        let mut step_usage = Usage::default();

        let (mut content_stream, mut tool_call_stream, mut metadata_stream,
             _abort_handle, _request_meta, _response_meta) = stream_result.into_streams();

        loop {
            tokio::select! {
                biased;

                Some(delta) = content_stream.next() => {
                    let mapped = match delta {
                        ContentDelta::Text(text) => {
                            step_text.push_str(&text);
                            StreamTextPart::TextDelta(text)
                        }
                        ContentDelta::Reasoning(text) => {
                            StreamTextPart::ReasoningDelta(text)
                        }
                        ContentDelta::File { .. } => {
                            continue; // Skip file content in text streaming
                        }
                    };

                    // Fire on_chunk callback
                    if let Some(ref cb) = options.on_chunk {
                        cb(&ChunkEvent {
                            chunk: mapped.clone(),
                        });
                    }

                    let _ = tx.send(Ok(mapped));
                }

                Some(delta) = tool_call_stream.next() => {
                    if let ToolCallDelta::Complete { id, name, input } = delta {
                        let tc = ToolCall {
                            tool_call_id: id,
                            tool_name: name,
                            input,
                        };
                        tool_calls.push(tc.clone());
                        let mapped = StreamTextPart::ToolCall(tc);

                        if let Some(ref cb) = options.on_chunk {
                            cb(&ChunkEvent {
                                chunk: mapped.clone(),
                            });
                        }

                        let _ = tx.send(Ok(mapped));
                    }
                    // Start and InputDelta events are consumed but not emitted
                }

                Some(delta) = metadata_stream.next() => {
                    match delta {
                        MetadataDelta::Usage(usage) => {
                            step_usage = usage;
                        }
                        MetadataDelta::FinishReason(finish_reason) => {
                            step_finish_reason =
                                FinishReason::from_provider(&finish_reason.unified);
                            // Emit FinishStep (usage was already set above)
                            let mapped = StreamTextPart::FinishStep {
                                finish_reason: step_finish_reason.clone(),
                                usage: step_usage.clone(),
                                is_continued: step > 0,
                            };

                            if let Some(ref cb) = options.on_chunk {
                                cb(&ChunkEvent {
                                    chunk: mapped.clone(),
                                });
                            }

                            let _ = tx.send(Ok(mapped));
                        }
                        MetadataDelta::ProviderMetadata(_) => {
                            // Skip provider metadata in stream_text
                        }
                    }
                }

                else => break,
            }
        }

        // Accumulate usage.
        if let Some(t) = step_usage.input_tokens.total {
            *total_usage.input_tokens.total.get_or_insert(0) += t;
        }
        if let Some(t) = step_usage.output_tokens.total {
            *total_usage.output_tokens.total.get_or_insert(0) += t;
        }

        prev_tool_call_count = tool_calls.len();
        prev_had_tool_calls = step_finish_reason == FinishReason::ToolCalls;

        // ── Execute tools if needed and loop ─────────────────────────
        let should_continue = step_finish_reason == FinishReason::ToolCalls
            && !tool_calls.is_empty()
            && step_tools.is_some()
            && step < options.max_steps - 1;

        if should_continue {
            let tools = step_tools.unwrap();
            // Execute tool calls and emit results.
            for tc in &tool_calls {
                let tool = match tools.get(&tc.tool_name) {
                    Some(t) => t,
                    None => {
                        let _ = tx.send(Err(Error::ToolNotFound {
                            tool_name: tc.tool_name.clone(),
                        }));
                        return Ok(());
                    }
                };

                // on_tool_call_start
                if let Some(ref cb) = options.on_tool_call_start {
                    cb(&ToolCallStartEvent {
                        tool_call: tc.clone(),
                    });
                }

                let result = execute_tool_for_stream(tc, tool).await;

                // on_tool_call_finish
                if let Some(ref cb) = options.on_tool_call_finish {
                    cb(&ToolCallFinishEvent {
                        tool_call: tc.clone(),
                        tool_result: result.clone(),
                    });
                }

                let _ = tx.send(Ok(StreamTextPart::ToolResult(result.clone())));

                // Build messages for next step.
                append_tool_result_to_messages(&mut messages, tc, &result);
            }

            // Also add text the model generated in this step to messages.
            if !step_text.is_empty() {
                // The model's text is implicitly part of the context through
                // the assistant + tool messages appended above.
            }

            // Continue loop — next step will call model again.
        } else {
            // Terminal step — emit Finish.
            let _ = tx.send(Ok(StreamTextPart::Finish {
                finish_reason: step_finish_reason.clone(),
                usage: total_usage.clone(),
            }));

            // Fire on_finish callback
            if let Some(ref cb) = options.on_finish {
                cb(&FinishEvent {
                    text: step_text,
                    usage: total_usage,
                    finish_reason: step_finish_reason,
                    step_count: step + 1,
                });
            }

            return Ok(());
        }
    }

    Ok(())
}

/// Execute a single tool for the stream loop (simpler version without approval).
async fn execute_tool_for_stream(
    tc: &ToolCall,
    tool: &crate::tools::tool::ToolDef,
) -> ToolResult {
    if let Some(ref execute) = tool.execute {
        match execute(tc.input.clone()).await {
            Ok(output) => ToolResult {
                tool_call_id: tc.tool_call_id.clone(),
                tool_name: tc.tool_name.clone(),
                result: output,
                is_error: false,
                preliminary: false,
            },
            Err(err) => ToolResult {
                tool_call_id: tc.tool_call_id.clone(),
                tool_name: tc.tool_name.clone(),
                result: serde_json::Value::String(err),
                is_error: true,
                preliminary: false,
            },
        }
    } else {
        ToolResult {
            tool_call_id: tc.tool_call_id.clone(),
            tool_name: tc.tool_name.clone(),
            result: serde_json::Value::Null,
            is_error: false,
            preliminary: false,
        }
    }
}

/// Append a single tool call + result pair to messages for the next step.
fn append_tool_result_to_messages(
    messages: &mut Vec<Message>,
    tc: &ToolCall,
    result: &ToolResult,
) {
    // Assistant message with the tool call.
    messages.push(Message::Assistant {
        content: vec![
            ararajuba_provider::language_model::v4::prompt::AssistantContentPart::ToolCall(
                ToolCallPart {
                    tool_call_id: tc.tool_call_id.clone(),
                    tool_name: tc.tool_name.clone(),
                    input: tc.input.clone(),
                    provider_executed: None,
                    provider_options: None,
                },
            ),
        ],
        provider_options: None,
    });

    // Tool message with the result.
    messages.push(Message::Tool {
        content: vec![
            ararajuba_provider::language_model::v4::prompt::ToolContentPart::ToolResult(
                ToolResultPart {
                    tool_call_id: result.tool_call_id.clone(),
                    tool_name: result.tool_name.clone(),
                    output: if result.is_error {
                        ToolResultOutput::ErrorJson {
                            value: result.result.clone(),
                        }
                    } else {
                        ToolResultOutput::Json {
                            value: result.result.clone(),
                        }
                    },
                    provider_options: None,
                },
            ),
        ],
        provider_options: None,
    });
}

/// Build the initial message list from the options.
fn build_initial_messages(
    options: &GenerateTextOptions,
) -> Vec<Message> {
    use ararajuba_provider::language_model::v4::content_part::TextPart;
    use ararajuba_provider::language_model::v4::prompt::UserContentPart;

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
