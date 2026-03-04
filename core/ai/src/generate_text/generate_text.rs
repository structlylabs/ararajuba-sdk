//! The `generate_text` function with tool loop support.

use super::callbacks::{
    FinishEvent, StartEvent, StepStartEvent, ToolCallFinishEvent, ToolCallStartEvent,
};
use super::options::GenerateTextOptions;
use super::prepare_step::PrepareStepContext;
use super::result::{GenerateTextResult, StepResult};
use crate::error::Error;
use crate::tools::tool_approval::{ToolApprovalRequest, ToolApprovalResponse};
use crate::tools::tool_call::ToolCall;
use crate::tools::tool_result::ToolResult;
use crate::types::call_warning::CallWarning;
use crate::types::finish_reason::FinishReason;
use ararajuba_provider::language_model::v4::call_options::{CallOptions, ResponseFormat};
use ararajuba_provider::language_model::v4::content::Content;
use ararajuba_provider::language_model::v4::content_part::{ToolCallPart, ToolResultOutput, ToolResultPart};
use ararajuba_provider::language_model::v4::prompt::Message;
use ararajuba_provider::language_model::v4::usage::Usage;
use futures::StreamExt;

/// Generate text using a language model, with optional multi-step tool execution.
///
/// When `max_steps > 1` and the model returns tool calls, the SDK will:
/// 1. Execute the tools
/// 2. Append the results to the conversation
/// 3. Call the model again
/// 4. Repeat until the model stops calling tools or `max_steps` is reached
///
/// ## Callbacks
///
/// The following callbacks are invoked during generation:
/// - `on_start` — once at the beginning, before any model call
/// - `on_step_start` — before each model invocation
/// - `on_step_finish` — after each step completes (including tool execution)
/// - `on_tool_call_start` — before each individual tool call
/// - `on_tool_call_finish` — after each individual tool call
/// - `on_finish` — once when the entire generation completes
///
/// ## `prepare_step`
///
/// If provided, `prepare_step` is called before each step. It can dynamically
/// change the tool set, system prompt, or call settings for that step.
pub async fn generate_text(options: GenerateTextOptions) -> Result<GenerateTextResult, Error> {
    let _span = tracing::info_span!(
        "generate_text",
        model = %options.model.model_id(),
        provider = %options.model.provider(),
        max_steps = options.max_steps,
    )
    .entered();

    // ── on_start callback ────────────────────────────────────────────
    if let Some(ref cb) = options.on_start {
        cb(&StartEvent {
            model_id: options.model.model_id().to_string(),
            provider: options.model.provider().to_string(),
        });
    }

    let mut messages = build_initial_messages(&options);
    let mut steps: Vec<StepResult> = Vec::new();
    let mut all_warnings: Vec<CallWarning> = Vec::new();

    // Track previous step state for prepare_step context.
    let mut prev_tool_call_count: usize = 0;
    let mut prev_had_tool_calls = false;

    for step in 0..options.max_steps {
        let _step_span = tracing::debug_span!("generate_text.step", step).entered();

        // ── prepare_step hook ────────────────────────────────────────
        let mut step_system = options.system.clone();
        let mut step_tools = options.tools.as_ref();
        let mut step_settings = &options.call_settings;
        let mut override_settings: Option<crate::types::call_settings::CallSettings> = None;
        let mut override_tools: Option<crate::tools::tool_set::ToolSet> = None;

        if let Some(ref prepare) = options.prepare_step {
            let ctx = PrepareStepContext {
                step_index: step,
                previous_tool_call_count: prev_tool_call_count,
                previous_step_had_tool_calls: prev_had_tool_calls,
            };
            if let Some(result) = prepare(ctx).await {
                if let Some(sys) = result.system {
                    step_system = Some(sys);
                }
                if let Some(tools) = result.tools {
                    override_tools = Some(tools);
                }
                if let Some(settings) = result.call_settings {
                    override_settings = Some(settings);
                }
                // Granular overrides
                if result.max_output_tokens.is_some() || result.temperature.is_some() {
                    let mut s = override_settings.unwrap_or_else(|| options.call_settings.clone());
                    if let Some(max) = result.max_output_tokens {
                        s.max_output_tokens = Some(max);
                    }
                    if let Some(temp) = result.temperature {
                        s.temperature = Some(temp);
                    }
                    override_settings = Some(s);
                }
            }
        }

        if let Some(ref tools) = override_tools {
            step_tools = Some(tools);
        }
        if let Some(ref settings) = override_settings {
            step_settings = settings;
        }

        // ── on_step_start callback ───────────────────────────────────
        if let Some(ref cb) = options.on_step_start {
            cb(&StepStartEvent {
                step_index: step,
                model_id: options.model.model_id().to_string(),
            });
        }

        let call_options = build_call_options_for_step(
            &options,
            &messages,
            step_system.as_deref(),
            step_tools,
            step_settings,
        );

        tracing::debug!("Calling model do_generate");
        let result = options.model.do_generate(&call_options).await?;
        tracing::debug!(
            finish_reason = ?result.finish_reason.unified,
            "Model returned"
        );

        // Collect warnings
        let step_warnings: Vec<CallWarning> =
            result.warnings.iter().cloned().map(CallWarning::from).collect();
        all_warnings.extend(step_warnings);

        // Extract text, reasoning, tool calls from content
        let mut text = String::new();
        let mut reasoning = Option::<String>::None;
        let mut tool_calls: Vec<ToolCall> = Vec::new();

        for content in &result.content {
            match content {
                Content::Text { text: t, .. } => text.push_str(t),
                Content::Reasoning { text: r, .. } => {
                    reasoning.get_or_insert_with(String::new).push_str(r);
                }
                Content::ToolCall {
                    tool_call_id,
                    tool_name,
                    input,
                    ..
                } => {
                    let parsed_input: serde_json::Value =
                        serde_json::from_str(input).unwrap_or(serde_json::Value::Null);
                    tool_calls.push(ToolCall {
                        tool_call_id: tool_call_id.clone(),
                        tool_name: tool_name.clone(),
                        input: parsed_input,
                    });
                }
                _ => {}
            }
        }

        let finish_reason = FinishReason::from_provider(&result.finish_reason.unified);

        // Execute tools if the model returned tool calls
        let (tool_results, approval_requests) = if !tool_calls.is_empty() {
            tracing::debug!(count = tool_calls.len(), "Executing tool calls");
            if let Some(tools) = step_tools {
                execute_tool_calls_with_approval(
                    &tool_calls,
                    tools,
                    options.on_tool_approval.as_ref(),
                    options.on_preliminary_tool_result.as_deref(),
                    options.on_tool_call_start.as_deref(),
                    options.on_tool_call_finish.as_deref(),
                )
                .await?
            } else {
                (Vec::new(), Vec::new())
            }
        } else {
            (Vec::new(), Vec::new())
        };

        // Update state for next prepare_step call.
        prev_tool_call_count = tool_calls.len();
        prev_had_tool_calls = finish_reason == FinishReason::ToolCalls;

        let step_result = StepResult {
            text: text.clone(),
            tool_calls: tool_calls.clone(),
            tool_results: tool_results.clone(),
            tool_approval_requests: approval_requests.clone(),
            finish_reason: finish_reason.clone(),
            usage: result.usage.clone(),
            is_continued: step > 0,
        };

        if let Some(ref cb) = options.on_step_finish {
            cb(&step_result);
        }

        steps.push(step_result);

        // If the model didn't make tool calls, we're at the last step,
        // or some tools are pending approval (not executed), stop.
        if finish_reason != FinishReason::ToolCalls
            || step >= options.max_steps - 1
            || step_tools.is_none()
            || !approval_requests.is_empty()
        {
            let gen_result = build_generate_text_result(steps, all_warnings, result.response);

            // ── on_finish callback ───────────────────────────────────
            if let Some(ref cb) = options.on_finish {
                cb(&FinishEvent {
                    text: gen_result.text.clone(),
                    usage: gen_result.usage.clone(),
                    finish_reason: gen_result.finish_reason.clone(),
                    step_count: gen_result.steps.len(),
                });
            }

            return Ok(gen_result);
        }

        // Append tool calls and results to messages for the next iteration.
        append_tool_results_to_messages(&mut messages, &tool_calls, &tool_results);
    }

    // Should be unreachable, but just in case:
    let gen_result = build_generate_text_result(steps, all_warnings, None);

    if let Some(ref cb) = options.on_finish {
        cb(&FinishEvent {
            text: gen_result.text.clone(),
            usage: gen_result.usage.clone(),
            finish_reason: gen_result.finish_reason.clone(),
            step_count: gen_result.steps.len(),
        });
    }

    Ok(gen_result)
}

/// Build the initial message list from the options.
fn build_initial_messages(options: &GenerateTextOptions) -> Vec<Message> {
    let mut messages = Vec::new();

    if let Some(ref system) = options.system {
        messages.push(Message::System {
            content: system.clone(),
            provider_options: None,
        });
    }

    if let Some(ref prompt) = options.prompt {
        messages.push(Message::User {
            content: vec![
                ararajuba_provider::language_model::v4::prompt::UserContentPart::Text(
                    ararajuba_provider::language_model::v4::content_part::TextPart {
                        text: prompt.clone(),
                        provider_options: None,
                    },
                ),
            ],
            provider_options: None,
        });
    }

    if let Some(ref msgs) = options.messages {
        messages.extend(msgs.clone());
    }

    messages
}

/// Build provider-level call options from the SDK options + current messages,
/// applying per-step overrides from `prepare_step`.
fn build_call_options_for_step(
    options: &GenerateTextOptions,
    messages: &[Message],
    _step_system: Option<&str>,
    step_tools: Option<&crate::tools::tool_set::ToolSet>,
    step_settings: &crate::types::call_settings::CallSettings,
) -> CallOptions {
    CallOptions {
        prompt: messages.to_vec(),
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
    }
}

/// Execute tool calls against the tool set, checking approval when needed.
///
/// Returns `(tool_results, approval_requests)`. If a tool needs approval and
/// no callback is provided, the tool call is added to `approval_requests`
/// instead of being executed.
async fn execute_tool_calls_with_approval(
    tool_calls: &[ToolCall],
    tools: &crate::tools::tool_set::ToolSet,
    on_approval: Option<&crate::tools::tool_approval::OnToolApproval>,
    on_preliminary: Option<&(dyn Fn(&ToolResult) + Send + Sync)>,
    on_tool_call_start: Option<&(dyn Fn(&ToolCallStartEvent) + Send + Sync)>,
    on_tool_call_finish: Option<&(dyn Fn(&ToolCallFinishEvent) + Send + Sync)>,
) -> Result<(Vec<ToolResult>, Vec<ToolApprovalRequest>), Error> {
    let mut results = Vec::new();
    let mut approval_requests = Vec::new();

    for tc in tool_calls {
        let tool = tools.get(&tc.tool_name).ok_or_else(|| Error::ToolNotFound {
            tool_name: tc.tool_name.clone(),
        })?;

        // ── Check if approval is needed ──────────────────────────────────
        let approval_needed = tool
            .needs_approval
            .as_ref()
            .is_some_and(|check| check(&tc.input));

        if approval_needed {
            let request = ToolApprovalRequest {
                approval_id: uuid::Uuid::new_v4().to_string(),
                tool_call_id: tc.tool_call_id.clone(),
                tool_name: tc.tool_name.clone(),
                input: tc.input.clone(),
            };

            if let Some(callback) = on_approval {
                // Ask the callback for a decision.
                let response = callback(request.clone()).await;
                match response {
                    ToolApprovalResponse::Approved => {
                        // Approved — execute the tool normally.
                        if let Some(cb) = on_tool_call_start {
                            cb(&ToolCallStartEvent {
                                tool_call: tc.clone(),
                            });
                        }
                        let result = execute_single_tool(tc, tool, on_preliminary).await;
                        if let Some(cb) = on_tool_call_finish {
                            cb(&ToolCallFinishEvent {
                                tool_call: tc.clone(),
                                tool_result: result.clone(),
                            });
                        }
                        results.push(result);
                    }
                    ToolApprovalResponse::Denied { reason } => {
                        // Denied — record a denied result (not an error per se).
                        let msg = reason.unwrap_or_else(|| "Tool execution denied".to_string());
                        results.push(ToolResult {
                            tool_call_id: tc.tool_call_id.clone(),
                            tool_name: tc.tool_name.clone(),
                            result: serde_json::Value::String(msg),
                            is_error: true,
                            preliminary: false,
                        });
                    }
                }
            } else {
                // No callback — park the call as a pending approval request.
                // The loop will stop and the caller can handle it.
                approval_requests.push(request);
            }
        } else {
            // No approval needed — execute immediately.
            if let Some(cb) = on_tool_call_start {
                cb(&ToolCallStartEvent {
                    tool_call: tc.clone(),
                });
            }
            let result = execute_single_tool(tc, tool, on_preliminary).await;
            if let Some(cb) = on_tool_call_finish {
                cb(&ToolCallFinishEvent {
                    tool_call: tc.clone(),
                    tool_result: result.clone(),
                });
            }
            results.push(result);
        }
    }

    Ok((results, approval_requests))
}

/// Execute a single tool call, supporting both regular and streaming execute.
async fn execute_single_tool(
    tc: &ToolCall,
    tool: &crate::tools::tool::ToolDef,
    on_preliminary: Option<&(dyn Fn(&ToolResult) + Send + Sync)>,
) -> ToolResult {
    let _span = tracing::debug_span!(
        "execute_tool",
        tool_name = %tc.tool_name,
        tool_call_id = %tc.tool_call_id,
    )
    .entered();
    // Prefer streaming execute if available.
    if let Some(ref execute_streaming) = tool.execute_streaming {
        let mut stream = execute_streaming(tc.input.clone());
        let mut last_output: Option<Result<serde_json::Value, String>> = None;

        while let Some(item) = stream.next().await {
            // Emit previous item as preliminary (if any).
            if let Some(prev) = last_output.take() {
                let preliminary_result = match prev {
                    Ok(val) => ToolResult {
                        tool_call_id: tc.tool_call_id.clone(),
                        tool_name: tc.tool_name.clone(),
                        result: val,
                        is_error: false,
                        preliminary: true,
                    },
                    Err(err) => ToolResult {
                        tool_call_id: tc.tool_call_id.clone(),
                        tool_name: tc.tool_name.clone(),
                        result: serde_json::Value::String(err),
                        is_error: true,
                        preliminary: true,
                    },
                };
                if let Some(cb) = on_preliminary {
                    cb(&preliminary_result);
                }
            }
            last_output = Some(item);
        }

        // Last item is the final result.
        match last_output {
            Some(Ok(val)) => ToolResult {
                tool_call_id: tc.tool_call_id.clone(),
                tool_name: tc.tool_name.clone(),
                result: val,
                is_error: false,
                preliminary: false,
            },
            Some(Err(err)) => ToolResult {
                tool_call_id: tc.tool_call_id.clone(),
                tool_name: tc.tool_name.clone(),
                result: serde_json::Value::String(err),
                is_error: true,
                preliminary: false,
            },
            None => ToolResult {
                tool_call_id: tc.tool_call_id.clone(),
                tool_name: tc.tool_name.clone(),
                result: serde_json::Value::Null,
                is_error: false,
                preliminary: false,
            },
        }
    } else if let Some(ref execute) = tool.execute {
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
        // Tool has no execute function — return the call as-is for the caller.
        ToolResult {
            tool_call_id: tc.tool_call_id.clone(),
            tool_name: tc.tool_name.clone(),
            result: serde_json::Value::Null,
            is_error: false,
            preliminary: false,
        }
    }
}

/// Append tool calls and results as messages for the next model invocation.
fn append_tool_results_to_messages(
    messages: &mut Vec<Message>,
    tool_calls: &[ToolCall],
    tool_results: &[ToolResult],
) {
    // Add assistant message with tool calls
    let assistant_parts: Vec<ararajuba_provider::language_model::v4::prompt::AssistantContentPart> =
        tool_calls
            .iter()
            .map(|tc| {
                ararajuba_provider::language_model::v4::prompt::AssistantContentPart::ToolCall(
                    ToolCallPart {
                        tool_call_id: tc.tool_call_id.clone(),
                        tool_name: tc.tool_name.clone(),
                        input: tc.input.clone(),
                        provider_executed: None,
                        provider_options: None,
                    },
                )
            })
            .collect();

    messages.push(Message::Assistant {
        content: assistant_parts,
        provider_options: None,
    });

    // Add tool message with results
    let tool_parts: Vec<ararajuba_provider::language_model::v4::prompt::ToolContentPart> = tool_results
        .iter()
        .map(|tr| {
            ararajuba_provider::language_model::v4::prompt::ToolContentPart::ToolResult(
                ToolResultPart {
                    tool_call_id: tr.tool_call_id.clone(),
                    tool_name: tr.tool_name.clone(),
                    output: if tr.is_error {
                        ToolResultOutput::ErrorJson {
                            value: tr.result.clone(),
                        }
                    } else {
                        ToolResultOutput::Json {
                            value: tr.result.clone(),
                        }
                    },
                    provider_options: None,
                },
            )
        })
        .collect();

    messages.push(Message::Tool {
        content: tool_parts,
        provider_options: None,
    });
}

/// Build the final `GenerateTextResult` from all collected steps.
fn build_generate_text_result(
    steps: Vec<StepResult>,
    warnings: Vec<CallWarning>,
    response: Option<ararajuba_provider::language_model::v4::generate_result::ResponseMetadata>,
) -> GenerateTextResult {
    let last_step = steps.last().expect("At least one step must exist");

    // Aggregate usage across all steps
    let total_usage = steps.iter().fold(Usage::default(), |mut acc, s| {
        if let Some(t) = s.usage.input_tokens.total {
            *acc.input_tokens.total.get_or_insert(0) += t;
        }
        if let Some(t) = s.usage.output_tokens.total {
            *acc.output_tokens.total.get_or_insert(0) += t;
        }
        acc
    });

    // Concatenate text from all steps
    let text = steps.iter().map(|s| s.text.as_str()).collect::<String>();

    GenerateTextResult {
        text,
        reasoning: None,
        tool_calls: last_step.tool_calls.clone(),
        tool_results: last_step.tool_results.clone(),
        tool_approval_requests: last_step.tool_approval_requests.clone(),
        usage: total_usage,
        finish_reason: last_step.finish_reason.clone(),
        response,
        steps,
        warnings,
    }
}

// ── Stop condition helpers ─────────────────────────────────────────────────

/// Returns `true` if the given step's tool calls include a call to the named tool.
///
/// Useful as a stop-condition predicate:
/// ```ignore
/// // Stop the tool loop if "submit_answer" was called
/// if has_tool_call(&step, "submit_answer") { break; }
/// ```
pub fn has_tool_call(step: &StepResult, tool_name: &str) -> bool {
    step.tool_calls.iter().any(|tc| tc.tool_name == tool_name)
}

/// Returns `true` if the current step index equals the given count.
///
/// ```ignore
/// if step_count_is(current_step, 5) { /* reached step 5 */ }
/// ```
pub fn step_count_is(step_index: usize, count: usize) -> bool {
    step_index == count
}
