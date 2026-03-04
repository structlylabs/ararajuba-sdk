//! OpenAI Responses API language model.
//!
//! Implements the `LanguageModel` trait against the OpenAI Responses
//! endpoint (`POST /v1/responses`).
//!
//! The Responses API differs from the Chat Completions API in:
//! - Input uses a flat list of content items (or a single string)
//! - Output is a list of "output" items (messages, tool calls, etc.)
//! - Supports conversation continuation via `previous_response_id`
//! - Provides built-in tool execution (`web_search`, `file_search`, `code_interpreter`)
//! - Streaming emits response events (not SSE chat deltas)

use crate::capabilities::{get_openai_model_capabilities, is_search_model};
use crate::responses::openai_responses_options::OpenAIResponsesOptions;
use async_trait::async_trait;
use ararajuba_provider::errors::Error;
use ararajuba_provider::language_model::v4::call_options::CallOptions;
use ararajuba_provider::language_model::v4::content::Content;
use ararajuba_provider::language_model::v4::content_part::{DataContent, ToolResultOutput};
use ararajuba_provider::language_model::v4::finish_reason::{FinishReason, UnifiedFinishReason};
use ararajuba_provider::language_model::v4::generate_result::{
    GenerateResult, RequestMetadata, ResponseMetadata,
};
use ararajuba_provider::language_model::v4::language_model_v4::LanguageModelV4;
use ararajuba_provider::language_model::v4::prompt::{
    AssistantContentPart, Message, Prompt, ToolContentPart, UserContentPart,
};
use ararajuba_provider::language_model::v4::stream_part::StreamPart;
use ararajuba_provider::language_model::v4::stream_result::{
    split_merged_stream, AbortHandle, StreamRequestMetadata, StreamResponseMetadata, StreamResult,
};
use ararajuba_provider::language_model::v4::tool::Tool;
use ararajuba_provider::language_model::v4::tool_choice::ToolChoice;
use ararajuba_provider::language_model::v4::usage::{InputTokens, OutputTokens, Usage};
use ararajuba_provider::shared::Warning;
use ararajuba_provider_utils::http::post_to_api::{post_json_to_api, PostJsonOptions};
use ararajuba_provider_utils::http::response_handler::{
    create_json_error_response_handler, create_json_response_handler,
    create_event_source_response_handler,
};
use futures::future::BoxFuture;
use futures::stream::BoxStream;
use futures::StreamExt;
use ararajuba_openai_compatible::error::parse_openai_compatible_error;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

/// Configuration for the OpenAI Responses language model.
#[derive(Clone)]
pub struct OpenAIResponsesConfig {
    /// Provider identifier (e.g., "openai.responses").
    pub provider: String,
    /// Full URL for the responses endpoint (e.g., "https://api.openai.com/v1/responses").
    pub url: String,
    /// Headers for requests.
    pub headers: HashMap<String, String>,
    /// Custom fetch function (for testing / middleware).
    pub fetch: Option<
        Arc<
            dyn Fn(reqwest::Request) -> BoxFuture<'static, reqwest::Result<reqwest::Response>>
                + Send
                + Sync,
        >,
    >,
}

/// An OpenAI Responses API language model.
pub struct OpenAIResponsesLanguageModel {
    model_id: String,
    config: OpenAIResponsesConfig,
}

impl OpenAIResponsesLanguageModel {
    pub fn new(model_id: String, config: OpenAIResponsesConfig) -> Self {
        Self { model_id, config }
    }

    /// Extract OpenAI Responses options from provider_options.
    fn extract_options(options: &CallOptions) -> OpenAIResponsesOptions {
        options
            .provider_options
            .as_ref()
            .and_then(|po| po.get("openai"))
            .and_then(|obj| {
                let val = serde_json::to_value(obj).ok()?;
                serde_json::from_value::<OpenAIResponsesOptions>(val).ok()
            })
            .unwrap_or_default()
    }

    /// Build the request body for the Responses API.
    fn build_body(&self, options: &CallOptions, stream: bool) -> Value {
        let resp_opts = Self::extract_options(options);
        let caps = get_openai_model_capabilities(&self.model_id);

        // Convert prompt to Responses API input format.
        let input = convert_prompt_to_input(&options.prompt);

        let mut body = json!({
            "model": self.model_id,
            "input": input,
        });

        let obj = body.as_object_mut().unwrap();

        if stream {
            obj.insert("stream".to_string(), json!(true));
        }

        // Standard parameters
        if let Some(max) = options.max_output_tokens {
            obj.insert("max_output_tokens".to_string(), json!(max));
        }

        if let Some(temp) = options.temperature {
            if !caps.is_reasoning_model {
                obj.insert("temperature".to_string(), json!(temp));
            }
        }

        if let Some(ref stop) = options.stop_sequences {
            if !stop.is_empty() && !is_search_model(&self.model_id) {
                // Responses API uses "stop" (not "stop_sequences")
                // but only supports single stop string, not array
            }
        }

        if let Some(top_p) = options.top_p {
            if !caps.is_reasoning_model {
                obj.insert("top_p".to_string(), json!(top_p));
            }
        }

        if let Some(presence) = options.presence_penalty {
            if !caps.is_reasoning_model {
                obj.insert("presence_penalty".to_string(), json!(presence));
            }
        }

        if let Some(frequency) = options.frequency_penalty {
            if !caps.is_reasoning_model {
                obj.insert("frequency_penalty".to_string(), json!(frequency));
            }
        }

        if let Some(seed) = options.seed {
            obj.insert("seed".to_string(), json!(seed));
        }

        // Response format → "text" field in Responses API
        if let Some(ref fmt) = options.response_format {
            match fmt {
                ararajuba_provider::language_model::v4::call_options::ResponseFormat::Text => {
                    obj.insert(
                        "text".to_string(),
                        json!({ "format": { "type": "text" } }),
                    );
                }
                ararajuba_provider::language_model::v4::call_options::ResponseFormat::Json {
                    schema,
                    name,
                    description,
                    ..
                } => {
                    if let Some(schema) = schema {
                        let mut fmt_obj = json!({
                            "type": "json_schema",
                            "strict": true,
                            "schema": schema,
                        });
                        if let Some(name) = name {
                            fmt_obj
                                .as_object_mut()
                                .unwrap()
                                .insert("name".to_string(), json!(name));
                        }
                        if let Some(desc) = description {
                            fmt_obj
                                .as_object_mut()
                                .unwrap()
                                .insert("description".to_string(), json!(desc));
                        }
                        obj.insert("text".to_string(), json!({ "format": fmt_obj }));
                    } else {
                        obj.insert(
                            "text".to_string(),
                            json!({ "format": { "type": "json_object" } }),
                        );
                    }
                }
            }
        }

        // Tools — convert to Responses API format
        if let Some(ref tools) = options.tools {
            let tools_json: Vec<Value> = tools
                .iter()
                .map(|t| convert_tool_to_responses_format(t))
                .collect();
            if !tools_json.is_empty() {
                obj.insert("tools".to_string(), json!(tools_json));
            }
        }

        // Tool choice
        if let Some(ref choice) = options.tool_choice {
            match choice {
                ToolChoice::Auto => {
                    obj.insert("tool_choice".to_string(), json!("auto"));
                }
                ToolChoice::None => {
                    obj.insert("tool_choice".to_string(), json!("none"));
                }
                ToolChoice::Required => {
                    obj.insert("tool_choice".to_string(), json!("required"));
                }
                ToolChoice::Tool { tool_name } => {
                    obj.insert(
                        "tool_choice".to_string(),
                        json!({
                            "type": "function",
                            "name": tool_name,
                        }),
                    );
                }
            }
        }

        // OpenAI Responses-specific options
        if let Some(ref prev_id) = resp_opts.previous_response_id {
            obj.insert("previous_response_id".to_string(), json!(prev_id));
        }

        if let Some(store) = resp_opts.store {
            obj.insert("store".to_string(), json!(store));
        }

        if let Some(ref include) = resp_opts.include {
            obj.insert("include".to_string(), json!(include));
        }

        if let Some(ref truncation) = resp_opts.truncation {
            obj.insert("truncation".to_string(), json!(truncation));
        }

        if let Some(ref conversation) = resp_opts.conversation {
            obj.insert("conversation".to_string(), json!({ "id": conversation }));
        }

        if let Some(ref reasoning_summary) = resp_opts.reasoning_summary {
            obj.insert(
                "reasoning".to_string(),
                json!({ "summary": reasoning_summary }),
            );
        }

        if let Some(max_tool_calls) = resp_opts.max_tool_calls {
            obj.insert("max_tool_calls".to_string(), json!(max_tool_calls));
        }

        body
    }
}

#[async_trait]
impl LanguageModelV4 for OpenAIResponsesLanguageModel {
    fn provider(&self) -> &str {
        &self.config.provider
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }

    async fn do_generate(&self, options: &CallOptions) -> Result<GenerateResult, Error> {
        let body = self.build_body(options, false);

        let response_handler = create_json_response_handler(|v: Value| Ok(v));
        let error_handler =
            create_json_error_response_handler(parse_openai_compatible_error);

        let raw = post_json_to_api(PostJsonOptions {
            url: self.config.url.clone(),
            headers: Some(self.config.headers.clone()),
            body: body.clone(),
            successful_response_handler: response_handler,
            failed_response_handler: error_handler,
            fetch: self.config.fetch.clone(),
            retry: None,
            cancellation_token: options.cancellation_token.clone(),
        })
        .await?;

        // Parse the Responses API result.
        parse_responses_result(&raw, body)
    }

    async fn do_stream(&self, options: &CallOptions) -> Result<StreamResult, Error> {
        let body = self.build_body(options, true);

        let parse_chunk = Arc::new(|v: Value| Ok(v));
        let response_handler = create_event_source_response_handler(parse_chunk);
        let error_handler =
            create_json_error_response_handler(parse_openai_compatible_error);

            let raw_stream: BoxStream<'static, Result<Value, Error>> =
                post_json_to_api(PostJsonOptions {
                    url: self.config.url.clone(),
                    headers: Some(self.config.headers.clone()),
                    body: body.clone(),
                    successful_response_handler: response_handler,
                    failed_response_handler: error_handler,
                    fetch: self.config.fetch.clone(),
                    retry: None,
                    cancellation_token: options.cancellation_token.clone(),
                })
                .await?;

            let merged_stream: BoxStream<'static, Result<StreamPart, Error>> =
                transform_responses_sse_to_stream_parts(raw_stream);

            Ok(split_merged_stream(
                merged_stream,
                AbortHandle::noop(),
                Some(StreamRequestMetadata {
                    body: Some(body),
                }),
                Some(StreamResponseMetadata { headers: None }),
            ))
    }
}

// ---------------------------------------------------------------------------
// Prompt conversion
// ---------------------------------------------------------------------------

/// Convert a `Prompt` (list of messages) to Responses API input format.
///
/// The Responses API accepts:
/// - A string (for simple prompts)
/// - An array of input items with role + content
fn convert_prompt_to_input(prompt: &Prompt) -> Value {
    let items: Vec<Value> = prompt
        .iter()
        .map(|msg| match msg {
            Message::System { content, .. } => {
                // Responses API uses "developer" role for system messages.
                // System content is a plain String.
                json!({
                    "role": "developer",
                    "content": content,
                })
            }
            Message::User { content, .. } => {
                json!({
                    "role": "user",
                    "content": user_parts_to_value(content),
                })
            }
            Message::Assistant { content, .. } => {
                json!({
                    "role": "assistant",
                    "content": assistant_parts_to_value(content),
                })
            }
            Message::Tool { content, .. } => {
                // Tool results → function_call_output items
                let outputs: Vec<Value> = content
                    .iter()
                    .filter_map(|part| {
                        if let ToolContentPart::ToolResult(tr) = part {
                            let output_str = match &tr.output {
                                ToolResultOutput::Text { value } => value.clone(),
                                ToolResultOutput::Json { value } => value.to_string(),
                                ToolResultOutput::ErrorText { value } => value.clone(),
                                ToolResultOutput::ErrorJson { value } => value.to_string(),
                                _ => String::new(),
                            };
                            Some(json!({
                                "type": "function_call_output",
                                "call_id": tr.tool_call_id,
                                "output": output_str,
                            }))
                        } else {
                            None
                        }
                    })
                    .collect();
                json!(outputs)
            }
        })
        .collect();

    // Flatten tool message arrays
    let mut flat_items = Vec::new();
    for item in items {
        if let Value::Array(arr) = item {
            flat_items.extend(arr);
        } else {
            flat_items.push(item);
        }
    }

    json!(flat_items)
}

/// Convert user content parts to a JSON value.
fn user_parts_to_value(parts: &[UserContentPart]) -> Value {
    if parts.len() == 1 {
        if let UserContentPart::Text(tp) = &parts[0] {
            return json!(tp.text);
        }
    }

    let items: Vec<Value> = parts
        .iter()
        .filter_map(|part| match part {
            UserContentPart::Text(tp) => Some(json!({
                "type": "input_text",
                "text": tp.text,
            })),
            UserContentPart::File(fp) => {
                match &fp.data {
                    DataContent::Text(s) => {
                        if s.starts_with("http://") || s.starts_with("https://") {
                            // URL file / image
                            if fp.media_type.starts_with("image/") {
                                Some(json!({
                                    "type": "input_image",
                                    "image_url": s,
                                }))
                            } else {
                                Some(json!({
                                    "type": "input_file",
                                    "file_url": s,
                                }))
                            }
                        } else {
                            // Base64 data
                            let data_uri = format!("data:{};base64,{}", fp.media_type, s);
                            if fp.media_type.starts_with("image/") {
                                Some(json!({
                                    "type": "input_image",
                                    "image_url": data_uri,
                                }))
                            } else {
                                Some(json!({
                                    "type": "input_file",
                                    "file_data": data_uri,
                                }))
                            }
                        }
                    }
                    DataContent::Bytes(bytes) => {
                        let b64 = base64::Engine::encode(
                            &base64::engine::general_purpose::STANDARD,
                            bytes,
                        );
                        let data_uri = format!("data:{};base64,{}", fp.media_type, b64);
                        if fp.media_type.starts_with("image/") {
                            Some(json!({
                                "type": "input_image",
                                "image_url": data_uri,
                            }))
                        } else {
                            Some(json!({
                                "type": "input_file",
                                "file_data": data_uri,
                            }))
                        }
                    }
                }
            }
        })
        .collect();

    json!(items)
}

/// Convert assistant content parts to a JSON value.
fn assistant_parts_to_value(parts: &[AssistantContentPart]) -> Value {
    let items: Vec<Value> = parts
        .iter()
        .filter_map(|part| match part {
            AssistantContentPart::Text(tp) => Some(json!({
                "type": "output_text",
                "text": tp.text,
            })),
            _ => None,
        })
        .collect();

    if items.len() == 1 {
        if let Some(text) = items[0].get("text").and_then(|v| v.as_str()) {
            return json!(text);
        }
    }

    json!(items)
}

// ---------------------------------------------------------------------------
// Tool conversion
// ---------------------------------------------------------------------------

/// Convert a v3 `Tool` to the Responses API tool format.
fn convert_tool_to_responses_format(tool: &Tool) -> Value {
    match tool {
        Tool::Function(f) => json!({
            "type": "function",
            "name": f.name,
            "description": f.description,
            "parameters": f.input_schema,
            "strict": f.strict.unwrap_or(true),
        }),
        Tool::Provider(p) => json!({
            "type": p.id,
            "name": p.name,
        }),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a `FinishReason` from a unified reason and a raw string.
fn finish(unified: UnifiedFinishReason, raw: &str) -> FinishReason {
    FinishReason {
        unified,
        raw: Some(raw.to_string()),
    }
}

/// Create a `Usage` from input/output token counts.
fn make_usage(input: u64, output: u64) -> Usage {
    Usage {
        input_tokens: InputTokens {
            total: Some(input),
            ..Default::default()
        },
        output_tokens: OutputTokens {
            total: Some(output),
            ..Default::default()
        },
        raw: None,
    }
}

// ---------------------------------------------------------------------------
// Response parsing (non-streaming)
// ---------------------------------------------------------------------------

/// Parse a Responses API JSON result into a `GenerateResult`.
fn parse_responses_result(raw: &Value, request_body: Value) -> Result<GenerateResult, Error> {
    let response_id = raw.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
    let model_id = raw
        .get("model")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Parse output items
    let output = raw
        .get("output")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut content = Vec::new();
    let mut warnings = Vec::new();

    for item in &output {
        let item_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
        match item_type {
            "message" => {
                // Extract text content from the message
                if let Some(msg_content) = item.get("content").and_then(|v| v.as_array()) {
                    for part in msg_content {
                        let part_type = part.get("type").and_then(|v| v.as_str()).unwrap_or("");
                        match part_type {
                            "output_text" => {
                                if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
                                    content.push(Content::Text {
                                        text: text.to_string(),
                                        provider_metadata: None,
                                    });
                                }
                            }
                            "refusal" => {
                                if let Some(refusal) =
                                    part.get("refusal").and_then(|v| v.as_str())
                                {
                                    warnings.push(Warning::Other {
                                        message: format!("Model refusal: {refusal}"),
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            "function_call" => {
                // Tool call output
                let id = item
                    .get("call_id")
                    .or_else(|| item.get("id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let name = item
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let arguments = item
                    .get("arguments")
                    .and_then(|v| v.as_str())
                    .unwrap_or("{}")
                    .to_string();

                content.push(Content::ToolCall {
                    tool_call_id: id,
                    tool_name: name,
                    input: arguments,
                    provider_executed: None,
                    dynamic: None,
                    provider_metadata: None,
                });
            }
            "reasoning" => {
                // Reasoning content (if included via `include` option)
                if let Some(summary) = item.get("summary").and_then(|v| v.as_array()) {
                    for part in summary {
                        if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
                            content.push(Content::Reasoning {
                                text: text.to_string(),
                                provider_metadata: None,
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Parse usage
    let usage = raw
        .get("usage")
        .map(|u| {
            let input = u.get("input_tokens").and_then(|v| v.as_u64());
            let output = u.get("output_tokens").and_then(|v| v.as_u64());
            Usage {
                input_tokens: InputTokens {
                    total: input,
                    ..Default::default()
                },
                output_tokens: OutputTokens {
                    total: output,
                    ..Default::default()
                },
                raw: Some(u.clone()),
            }
        })
        .unwrap_or_default();

    // Map status to finish reason
    let status = raw
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("completed");
    let finish_reason = match status {
        "completed" => {
            if content.iter().any(|c| matches!(c, Content::ToolCall { .. })) {
                finish(UnifiedFinishReason::ToolCalls, status)
            } else {
                finish(UnifiedFinishReason::Stop, status)
            }
        }
        "incomplete" => {
            let reason = raw
                .get("incomplete_details")
                .and_then(|v| v.get("reason"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            match reason {
                "max_output_tokens" => finish(UnifiedFinishReason::Length, status),
                "content_filter" => finish(UnifiedFinishReason::ContentFilter, status),
                _ => finish(UnifiedFinishReason::Other, status),
            }
        }
        "failed" => finish(UnifiedFinishReason::Error, status),
        "cancelled" => finish(UnifiedFinishReason::Other, status),
        _ => finish(UnifiedFinishReason::Other, status),
    };

    Ok(GenerateResult {
        content,
        finish_reason,
        usage,
        provider_metadata: None,
        request: Some(RequestMetadata {
            body: Some(request_body),
        }),
        response: Some(ResponseMetadata {
            id: response_id,
            timestamp: None,
            model_id,
            headers: None,
            body: Some(raw.clone()),
        }),
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Streaming
// ---------------------------------------------------------------------------

/// Transform Responses API SSE events into `StreamPart`s.
///
/// The Responses API streams events like:
/// - `response.created` — initial response
/// - `response.output_item.added` — new output item
/// - `response.content_part.added` — new content part
/// - `response.output_text.delta` — text delta
/// - `response.output_text.done` — text complete
/// - `response.function_call_arguments.delta` — tool call arg delta
/// - `response.function_call_arguments.done` — tool call complete
/// - `response.reasoning_summary_text.delta` — reasoning delta
/// - `response.completed` — final event with usage
fn transform_responses_sse_to_stream_parts(
    raw_stream: BoxStream<'static, Result<Value, Error>>,
) -> BoxStream<'static, Result<StreamPart, Error>> {
    let stream = async_stream::stream! {
        let mut is_first = true;
        let mut active_text_id: Option<String> = None;
        let mut active_reasoning_id: Option<String> = None;
        let mut tool_calls: HashMap<String, ToolCallAcc> = HashMap::new();
        let mut final_usage: Option<Usage> = None;
        let mut final_finish_reason: Option<FinishReason> = None;

        futures::pin_mut!(raw_stream);

        while let Some(chunk_result) = raw_stream.next().await {
            let chunk = match chunk_result {
                Ok(c) => c,
                Err(e) => {
                    yield Err(e);
                    continue;
                }
            };

            let event_type = chunk.get("type").and_then(|v| v.as_str()).unwrap_or("");

            match event_type {
                "response.created" => {
                    if is_first {
                        is_first = false;
                        yield Ok(StreamPart::StreamStart {
                            warnings: vec![],
                        });

                        // Extract response metadata
                        if let Some(resp) = chunk.get("response") {
                            let id = resp.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
                            let model = resp.get("model").and_then(|v| v.as_str()).map(|s| s.to_string());

                            yield Ok(StreamPart::ResponseMetadata(ResponseMetadata {
                                id,
                                model_id: model,
                                timestamp: None,
                                headers: None,
                                body: None,
                            }));
                        }
                    }
                }

                "response.output_text.delta" => {
                    let delta = chunk.get("delta").and_then(|v| v.as_str()).unwrap_or("");
                    if delta.is_empty() {
                        continue;
                    }

                    // Ensure text block is started
                    let text_id = if let Some(ref id) = active_text_id {
                        id.clone()
                    } else {
                        let id = uuid::Uuid::new_v4().to_string();
                        // Close any open reasoning block first
                        if let Some(ref rid) = active_reasoning_id.take() {
                            yield Ok(StreamPart::ReasoningEnd {
                                id: rid.clone(),
                                provider_metadata: None,
                            });
                        }
                        yield Ok(StreamPart::TextStart {
                            id: id.clone(),
                            provider_metadata: None,
                        });
                        active_text_id = Some(id.clone());
                        id
                    };

                    yield Ok(StreamPart::TextDelta {
                        id: text_id,
                        delta: delta.to_string(),
                        provider_metadata: None,
                    });
                }

                "response.output_text.done" => {
                    if let Some(ref id) = active_text_id.take() {
                        yield Ok(StreamPart::TextEnd {
                            id: id.clone(),
                            provider_metadata: None,
                        });
                    }
                }

                "response.reasoning_summary_text.delta" => {
                    let delta = chunk.get("delta").and_then(|v| v.as_str()).unwrap_or("");
                    if delta.is_empty() {
                        continue;
                    }

                    let reason_id = if let Some(ref id) = active_reasoning_id {
                        id.clone()
                    } else {
                        let id = uuid::Uuid::new_v4().to_string();
                        yield Ok(StreamPart::ReasoningStart {
                            id: id.clone(),
                            provider_metadata: None,
                        });
                        active_reasoning_id = Some(id.clone());
                        id
                    };

                    yield Ok(StreamPart::ReasoningDelta {
                        id: reason_id,
                        delta: delta.to_string(),
                        provider_metadata: None,
                    });
                }

                "response.reasoning_summary_text.done" => {
                    if let Some(ref id) = active_reasoning_id.take() {
                        yield Ok(StreamPart::ReasoningEnd {
                            id: id.clone(),
                            provider_metadata: None,
                        });
                    }
                }

                "response.function_call_arguments.delta" => {
                    let item_id = chunk.get("item_id").and_then(|v| v.as_str()).unwrap_or("");
                    let delta = chunk.get("delta").and_then(|v| v.as_str()).unwrap_or("");

                    let acc = tool_calls.entry(item_id.to_string()).or_insert_with(|| {
                        ToolCallAcc {
                            id: item_id.to_string(),
                            name: String::new(),
                            arguments: String::new(),
                            started: false,
                        }
                    });
                    acc.arguments.push_str(delta);

                    if acc.started {
                        yield Ok(StreamPart::ToolInputDelta {
                            id: acc.id.clone(),
                            delta: delta.to_string(),
                            provider_metadata: None,
                        });
                    }
                }

                "response.output_item.added" => {
                    // A new output item (could be function_call, message, etc.)
                    if let Some(item) = chunk.get("item") {
                        let item_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
                        if item_type == "function_call" {
                            let item_id = item.get("id").and_then(|v| v.as_str()).unwrap_or("");
                            let call_id = item.get("call_id").and_then(|v| v.as_str()).unwrap_or(item_id);
                            let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("");

                            let acc = tool_calls.entry(call_id.to_string()).or_insert_with(|| {
                                ToolCallAcc {
                                    id: call_id.to_string(),
                                    name: name.to_string(),
                                    arguments: String::new(),
                                    started: false,
                                }
                            });
                            acc.name = name.to_string();

                            if !acc.started {
                                acc.started = true;
                                yield Ok(StreamPart::ToolInputStart {
                                    id: acc.id.clone(),
                                    tool_name: acc.name.clone(),
                                    provider_metadata: None,
                                    provider_executed: None,
                                    dynamic: None,
                                    title: None,
                                });
                            }
                        }
                    }
                }

                "response.function_call_arguments.done" => {
                    let item_id = chunk.get("item_id").and_then(|v| v.as_str()).unwrap_or("");
                    if let Some(acc) = tool_calls.get(item_id) {
                        yield Ok(StreamPart::ToolInputEnd {
                            id: acc.id.clone(),
                            provider_metadata: None,
                        });

                        yield Ok(StreamPart::ToolCall {
                            tool_call_id: acc.id.clone(),
                            tool_name: acc.name.clone(),
                            input: acc.arguments.clone(),
                            provider_executed: None,
                            dynamic: None,
                            provider_metadata: None,
                        });
                    }
                }

                "response.completed" => {
                    if let Some(resp) = chunk.get("response") {
                        // Usage
                        if let Some(u) = resp.get("usage") {
                            let input = u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                            let output = u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                            final_usage = Some(make_usage(input, output));
                        }

                        // Status → finish reason
                        let status = resp.get("status").and_then(|v| v.as_str()).unwrap_or("completed");
                        final_finish_reason = Some(match status {
                            "completed" => {
                                if tool_calls.values().any(|tc| !tc.name.is_empty()) {
                                    finish(UnifiedFinishReason::ToolCalls, status)
                                } else {
                                    finish(UnifiedFinishReason::Stop, status)
                                }
                            }
                            "incomplete" => finish(UnifiedFinishReason::Length, status),
                            _ => finish(UnifiedFinishReason::Other, status),
                        });
                    }
                }

                _ => {
                    // Ignore unrecognized events
                }
            }
        }

        // --- Flush phase ---

        // Close open blocks
        if let Some(ref rid) = active_reasoning_id {
            yield Ok(StreamPart::ReasoningEnd {
                id: rid.clone(),
                provider_metadata: None,
            });
        }
        if let Some(ref tid) = active_text_id {
            yield Ok(StreamPart::TextEnd {
                id: tid.clone(),
                provider_metadata: None,
            });
        }

        // Emit finish
        yield Ok(StreamPart::Finish {
            finish_reason: final_finish_reason.unwrap_or_else(|| finish(UnifiedFinishReason::Stop, "stop")),
            usage: final_usage.unwrap_or_default(),
            provider_metadata: None,
        });
    };

    Box::pin(stream)
}

/// Accumulator for tool call arguments during streaming.
struct ToolCallAcc {
    id: String,
    name: String,
    arguments: String,
    started: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_responses_model_metadata() {
        let config = OpenAIResponsesConfig {
            provider: "openai.responses".into(),
            url: "https://api.openai.com/v1/responses".into(),
            headers: HashMap::new(),
            fetch: None,
        };
        let model = OpenAIResponsesLanguageModel::new("gpt-4o".into(), config);
        assert_eq!(model.provider(), "openai.responses");
        assert_eq!(model.model_id(), "gpt-4o");
    }

    #[test]
    fn test_convert_prompt_to_input() {
        use ararajuba_provider::language_model::v4::content_part::TextPart;

        let prompt = vec![
            Message::System {
                content: "You are helpful".into(),
                provider_options: None,
            },
            Message::User {
                content: vec![UserContentPart::Text(TextPart {
                    text: "Hello".into(),
                    provider_options: None,
                })],
                provider_options: None,
            },
        ];

        let input = convert_prompt_to_input(&prompt);
        let arr = input.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["role"], "developer");
        assert_eq!(arr[1]["role"], "user");
    }

    #[test]
    fn test_parse_responses_result() {
        let raw = json!({
            "id": "resp_123",
            "model": "gpt-4o",
            "status": "completed",
            "output": [
                {
                    "type": "message",
                    "content": [
                        { "type": "output_text", "text": "Hello!" }
                    ]
                }
            ],
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5
            }
        });

        let result = parse_responses_result(&raw, json!({})).unwrap();
        assert_eq!(result.content.len(), 1);
        assert_eq!(result.finish_reason.unified, UnifiedFinishReason::Stop);
        assert_eq!(result.usage.input_tokens.total, Some(10));
        assert_eq!(result.usage.output_tokens.total, Some(5));
    }

    #[test]
    fn test_parse_responses_result_with_tool_calls() {
        let raw = json!({
            "id": "resp_456",
            "model": "gpt-4o",
            "status": "completed",
            "output": [
                {
                    "type": "function_call",
                    "call_id": "call_1",
                    "name": "get_weather",
                    "arguments": "{\"city\":\"London\"}"
                }
            ],
            "usage": {
                "input_tokens": 15,
                "output_tokens": 20
            }
        });

        let result = parse_responses_result(&raw, json!({})).unwrap();
        assert_eq!(result.content.len(), 1);
        assert_eq!(result.finish_reason.unified, UnifiedFinishReason::ToolCalls);
        if let Content::ToolCall {
            tool_call_id,
            tool_name,
            input,
            ..
        } = &result.content[0]
        {
            assert_eq!(tool_call_id, "call_1");
            assert_eq!(tool_name, "get_weather");
            assert!(input.contains("London"));
        } else {
            panic!("Expected ToolCall content");
        }
    }
}

// ---------------------------------------------------------------------------
// Capability trait implementations
// ---------------------------------------------------------------------------

use ararajuba_provider::capabilities::{
    ImageFormat, ReasoningConfig, SupportsImages, SupportsReasoning, SupportsStructuredOutput,
    SupportsToolCalling,
};

impl SupportsReasoning for OpenAIResponsesLanguageModel {
    fn reasoning_config(&self) -> ReasoningConfig {
        let caps = crate::capabilities::get_openai_model_capabilities(&self.model_id);
        ReasoningConfig {
            enabled: caps.is_reasoning_model,
            default_effort: Some("medium".to_string()),
            max_reasoning_tokens: None,
        }
    }
}

impl SupportsToolCalling for OpenAIResponsesLanguageModel {
    fn max_tools(&self) -> Option<usize> {
        Some(128)
    }

    fn supports_parallel_calls(&self) -> bool {
        true
    }

    fn supports_strict_schemas(&self) -> bool {
        true
    }
}

impl SupportsImages for OpenAIResponsesLanguageModel {
    fn supported_image_formats(&self) -> Vec<ImageFormat> {
        vec![
            ImageFormat::Jpeg,
            ImageFormat::Png,
            ImageFormat::Gif,
            ImageFormat::Webp,
        ]
    }
}

impl SupportsStructuredOutput for OpenAIResponsesLanguageModel {
    fn supports_json_mode(&self) -> bool {
        true
    }

    fn supports_json_schema(&self) -> bool {
        true
    }
}
