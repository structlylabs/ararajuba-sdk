//! OpenAI-compatible chat language model — implements `LanguageModelV4`.

use crate::chat::convert_messages::convert_to_openai_compatible_chat_messages;
use crate::chat::finish_reason::map_openai_compatible_finish_reason;
use crate::chat::prepare_tools::prepare_tools;
use crate::chat::usage::convert_openai_compatible_usage;
use crate::error::parse_openai_compatible_error;
use async_trait::async_trait;
use ararajuba_provider::errors::Error;
use ararajuba_provider::language_model::v4::call_options::{CallOptions, ResponseFormat};
use ararajuba_provider::language_model::v4::content::Content;
use ararajuba_provider::language_model::v4::generate_result::{
    GenerateResult, RequestMetadata, ResponseMetadata,
};
use ararajuba_provider::language_model::v4::language_model_v4::LanguageModelV4;
use ararajuba_provider::language_model::v4::stream_part::StreamPart;
use ararajuba_provider::language_model::v4::stream_result::{
    split_merged_stream, AbortHandle, StreamRequestMetadata, StreamResponseMetadata, StreamResult,
};
use ararajuba_provider::shared::Warning;
use ararajuba_provider_utils::http::post_to_api::{post_json_to_api, PostJsonOptions};
use ararajuba_provider_utils::http::response_handler::{
    create_event_source_response_handler, create_json_error_response_handler,
    create_json_response_handler,
};
use futures::future::BoxFuture;
use futures::stream::BoxStream;
use futures::StreamExt;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

/// Configuration for the chat language model.
#[derive(Clone)]
pub struct ChatModelConfig {
    /// Provider identifier (e.g., "openai.chat").
    pub provider: String,
    /// Full URL for the chat completions endpoint.
    pub url: String,
    /// Headers to send with each request (authorization, etc.).
    pub headers: HashMap<String, String>,
    /// Whether to include usage in streaming responses.
    pub include_usage: bool,
    /// Whether the provider supports structured outputs (json_schema response format).
    pub supports_structured_outputs: bool,
    /// Custom fetch function (for testing / middleware).
    pub fetch: Option<
        Arc<
            dyn Fn(reqwest::Request) -> BoxFuture<'static, reqwest::Result<reqwest::Response>>
                + Send
                + Sync,
        >,
    >,
}

/// An OpenAI-compatible chat language model.
pub struct OpenAICompatibleChatLanguageModel {
    model_id: String,
    config: ChatModelConfig,
}

impl OpenAICompatibleChatLanguageModel {
    pub fn new(model_id: String, config: ChatModelConfig) -> Self {
        Self { model_id, config }
    }

    /// Build the request body for both generate and stream calls.
    fn build_body(&self, options: &CallOptions, stream: bool) -> (Value, Vec<Warning>) {
        let messages = convert_to_openai_compatible_chat_messages(&options.prompt);
        let prepared = prepare_tools(
            options.tools.as_deref(),
            options.tool_choice.as_ref(),
        );

        let mut body = json!({
            "model": self.model_id,
            "messages": messages,
        });

        // Standard parameters
        if let Some(max) = options.max_output_tokens {
            body["max_tokens"] = json!(max);
        }
        if let Some(temp) = options.temperature {
            body["temperature"] = json!(temp);
        }
        if let Some(tp) = options.top_p {
            body["top_p"] = json!(tp);
        }
        if let Some(fp) = options.frequency_penalty {
            body["frequency_penalty"] = json!(fp);
        }
        if let Some(pp) = options.presence_penalty {
            body["presence_penalty"] = json!(pp);
        }
        if let Some(ref seqs) = options.stop_sequences {
            if !seqs.is_empty() {
                body["stop"] = json!(seqs);
            }
        }
        if let Some(seed) = options.seed {
            body["seed"] = json!(seed);
        }

        // Response format
        if let Some(ref rf) = options.response_format {
            match rf {
                ResponseFormat::Text => {
                    body["response_format"] = json!({"type": "text"});
                }
                ResponseFormat::Json { schema, name, description } if self.config.supports_structured_outputs => {
                    let mut js = json!({
                        "type": "json_schema",
                        "json_schema": {
                            "schema": schema,
                            "name": name.as_deref().unwrap_or("response"),
                            "strict": true,
                        }
                    });
                    if let Some(desc) = description {
                        js["json_schema"]["description"] = json!(desc);
                    }
                    body["response_format"] = js;
                }
                ResponseFormat::Json { .. } => {
                    body["response_format"] = json!({"type": "json_object"});
                }
            }
        }

        // Tools
        if let Some(tools) = prepared.tools {
            body["tools"] = json!(tools);
        }
        if let Some(tc) = prepared.tool_choice {
            body["tool_choice"] = tc;
        }

        // Provider options — spread into root body
        if let Some(ref po) = options.provider_options {
            if let Some(provider_opts) = po.get(&self.config.provider) {
                for (k, v) in provider_opts {
                    // Map known provider option names
                    match k.as_str() {
                        "user" => { body["user"] = v.clone(); }
                        "reasoningEffort" => { body["reasoning_effort"] = v.clone(); }
                        "textVerbosity" => { body["verbosity"] = v.clone(); }
                        _ => { body[k] = v.clone(); }
                    }
                }
            }
        }

        // Streaming options
        if stream {
            body["stream"] = json!(true);
            if self.config.include_usage {
                body["stream_options"] = json!({"include_usage": true});
            }
        }

        let warnings = prepared.warnings;
        (body, warnings)
    }
}

#[async_trait]
impl LanguageModelV4 for OpenAICompatibleChatLanguageModel {
    fn provider(&self) -> &str {
        &self.config.provider
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }

    async fn do_generate(&self, options: &CallOptions) -> Result<GenerateResult, Error> {
        let (body, warnings) = self.build_body(options, false);
        let body_clone = body.clone();

            let response_handler = create_json_response_handler(|v: Value| Ok(v));
            let error_handler = create_json_error_response_handler(parse_openai_compatible_error);

            let raw = post_json_to_api(PostJsonOptions {
                url: self.config.url.clone(),
                headers: Some(self.config.headers.clone()),
                body,
                successful_response_handler: response_handler,
                failed_response_handler: error_handler,
                fetch: self.config.fetch.clone(),
                retry: None,
                cancellation_token: options.cancellation_token.clone(),
            })
            .await?;

            // Parse the response
            let choice = raw
                .get("choices")
                .and_then(|c| c.get(0))
                .ok_or_else(|| Error::Other {
                    message: "No choices in response".into(),
                })?;

            let message = choice.get("message").ok_or_else(|| Error::Other {
                message: "No message in choice".into(),
            })?;

            // Extract content
            let mut content = Vec::new();

            // Reasoning content
            let reasoning_text = message
                .get("reasoning_content")
                .or_else(|| message.get("reasoning"))
                .and_then(|v| v.as_str());
            if let Some(r) = reasoning_text {
                if !r.is_empty() {
                    content.push(Content::Reasoning {
                        text: r.to_string(),
                        provider_metadata: None,
                    });
                }
            }

            // Text content
            if let Some(text) = message.get("content").and_then(|v| v.as_str()) {
                if !text.is_empty() {
                    content.push(Content::Text {
                        text: text.to_string(),
                        provider_metadata: None,
                    });
                }
            }

            // Tool calls
            if let Some(tool_calls) = message.get("tool_calls").and_then(|v| v.as_array()) {
                for tc in tool_calls {
                    let id = tc.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let func = tc.get("function").unwrap_or(&Value::Null);
                    let name = func.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let args = func.get("arguments").and_then(|v| v.as_str()).unwrap_or("{}").to_string();
                    content.push(Content::ToolCall {
                        tool_call_id: id,
                        tool_name: name,
                        input: args,
                        provider_executed: None,
                        dynamic: None,
                        provider_metadata: None,
                    });
                }
            }

            let finish_reason_raw = choice
                .get("finish_reason")
                .and_then(|v| v.as_str());
            let finish_reason = map_openai_compatible_finish_reason(finish_reason_raw);

            let usage = convert_openai_compatible_usage(raw.get("usage"));

            // Response metadata
            let response_id = raw.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
            let response_model = raw.get("model").and_then(|v| v.as_str()).map(|s| s.to_string());
            let response_timestamp = raw
                .get("created")
                .and_then(|v| v.as_i64())
                .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0));

            Ok(GenerateResult {
                content,
                finish_reason,
                usage,
                provider_metadata: None,
                request: Some(RequestMetadata {
                    body: Some(body_clone),
                }),
                response: Some(ResponseMetadata {
                    id: response_id,
                    timestamp: response_timestamp,
                    model_id: response_model,
                    headers: None,
                    body: Some(raw),
                }),
                warnings,
            })
    }

    async fn do_stream(&self, options: &CallOptions) -> Result<StreamResult, Error> {
        let (body, warnings) = self.build_body(options, true);
        let body_clone = body.clone();

        let chunk_parser: Arc<dyn Fn(Value) -> Result<Value, Error> + Send + Sync> =
            Arc::new(|v| Ok(v));
        let response_handler = create_event_source_response_handler(chunk_parser);
        let error_handler = create_json_error_response_handler(parse_openai_compatible_error);

        let raw_stream: BoxStream<'static, Result<Value, Error>> =
            post_json_to_api(PostJsonOptions {
                url: self.config.url.clone(),
                headers: Some(self.config.headers.clone()),
                body,
                successful_response_handler: response_handler,
                failed_response_handler: error_handler,
                fetch: self.config.fetch.clone(),
                retry: None,
                cancellation_token: options.cancellation_token.clone(),
            })
            .await?;

        // Transform raw SSE chunks into StreamPart events, then split into v4 typed streams
        let merged_stream: BoxStream<'static, Result<StreamPart, Error>> =
            Box::pin(transform_sse_to_stream_parts(raw_stream, warnings));

        Ok(split_merged_stream(
            merged_stream,
            AbortHandle::noop(),
            Some(StreamRequestMetadata {
                body: Some(body_clone),
            }),
            Some(StreamResponseMetadata { headers: None }),
        ))
    }
}

/// Transform raw SSE JSON chunks from an OpenAI-compatible API into `StreamPart` events.
fn transform_sse_to_stream_parts(
    raw_stream: BoxStream<'static, Result<Value, Error>>,
    initial_warnings: Vec<Warning>,
) -> impl futures::Stream<Item = Result<StreamPart, Error>> {
    use async_stream::stream;

    stream! {
        let mut is_first_chunk = true;
        let mut active_text_id: Option<String> = None;
        let mut active_reasoning_id: Option<String> = None;
        let mut tool_calls: HashMap<usize, ToolCallAccumulator> = HashMap::new();
        let mut final_finish_reason: Option<String> = None;
        let mut final_usage: Option<Value> = None;

        futures::pin_mut!(raw_stream);

        while let Some(chunk_result) = raw_stream.next().await {
            let chunk = match chunk_result {
                Ok(v) => v,
                Err(e) => {
                    yield Err(e);
                    continue;
                }
            };

            // First chunk: emit stream-start + response-metadata
            if is_first_chunk {
                is_first_chunk = false;

                yield Ok(StreamPart::StreamStart {
                    warnings: initial_warnings.clone(),
                });

                let resp_id = chunk.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
                let resp_model = chunk.get("model").and_then(|v| v.as_str()).map(|s| s.to_string());
                let resp_ts = chunk.get("created").and_then(|v| v.as_i64())
                    .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0));

                yield Ok(StreamPart::ResponseMetadata(ResponseMetadata {
                    id: resp_id,
                    model_id: resp_model,
                    timestamp: resp_ts,
                    headers: None,
                    body: None,
                }));
            }

            let choice = match chunk.get("choices").and_then(|c| c.get(0)) {
                Some(c) => c,
                None => {
                    // Check for usage-only chunk (last chunk with usage field)
                    if let Some(usage) = chunk.get("usage") {
                        final_usage = Some(usage.clone());
                    }
                    continue;
                }
            };

            let delta = match choice.get("delta") {
                Some(d) => d,
                None => continue,
            };

            // Finish reason
            if let Some(fr) = choice.get("finish_reason").and_then(|v| v.as_str()) {
                final_finish_reason = Some(fr.to_string());
            }

            // Usage
            if let Some(usage) = chunk.get("usage") {
                final_usage = Some(usage.clone());
            }

            // Reasoning content
            let reasoning_delta = delta
                .get("reasoning_content")
                .or_else(|| delta.get("reasoning"))
                .and_then(|v| v.as_str());

            if let Some(r) = reasoning_delta {
                if !r.is_empty() {
                    if active_reasoning_id.is_none() {
                        let id = uuid::Uuid::new_v4().to_string();
                        active_reasoning_id = Some(id.clone());
                        yield Ok(StreamPart::ReasoningStart {
                            id: id.clone(),
                            provider_metadata: None,
                        });
                    }
                    yield Ok(StreamPart::ReasoningDelta {
                        id: active_reasoning_id.clone().unwrap_or_default(),
                        delta: r.to_string(),
                        provider_metadata: None,
                    });
                }
            }

            // Text content
            if let Some(text) = delta.get("content").and_then(|v| v.as_str()) {
                if !text.is_empty() {
                    // End reasoning if active
                    if let Some(rid) = active_reasoning_id.take() {
                        yield Ok(StreamPart::ReasoningEnd {
                            id: rid,
                            provider_metadata: None,
                        });
                    }

                    if active_text_id.is_none() {
                        let id = uuid::Uuid::new_v4().to_string();
                        active_text_id = Some(id.clone());
                        yield Ok(StreamPart::TextStart {
                            id: id.clone(),
                            provider_metadata: None,
                        });
                    }
                    yield Ok(StreamPart::TextDelta {
                        id: active_text_id.clone().unwrap_or_default(),
                        delta: text.to_string(),
                        provider_metadata: None,
                    });
                }
            }

            // Tool calls
            if let Some(tc_array) = delta.get("tool_calls").and_then(|v| v.as_array()) {
                // End reasoning/text if active
                if let Some(rid) = active_reasoning_id.take() {
                    yield Ok(StreamPart::ReasoningEnd {
                        id: rid,
                        provider_metadata: None,
                    });
                }

                for tc in tc_array {
                    let index = tc.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

                    let acc = tool_calls.entry(index).or_insert_with(|| ToolCallAccumulator {
                        id: String::new(),
                        name: String::new(),
                        arguments: String::new(),
                        started: false,
                    });

                    // New tool call (has id + function.name)
                    if let Some(id) = tc.get("id").and_then(|v| v.as_str()) {
                        acc.id = id.to_string();
                    }
                    if let Some(name) = tc.get("function").and_then(|f| f.get("name")).and_then(|v| v.as_str()) {
                        acc.name = name.to_string();
                    }

                    // Accumulate arguments
                    if let Some(args_delta) = tc.get("function").and_then(|f| f.get("arguments")).and_then(|v| v.as_str()) {
                        acc.arguments.push_str(args_delta);

                        if !acc.started && !acc.name.is_empty() {
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

                        if acc.started {
                            yield Ok(StreamPart::ToolInputDelta {
                                id: acc.id.clone(),
                                delta: args_delta.to_string(),
                                provider_metadata: None,
                            });
                        }
                    }
                }
            }
        }

        // === Flush: close open blocks ===

        // End reasoning
        if let Some(rid) = active_reasoning_id.take() {
            yield Ok(StreamPart::ReasoningEnd {
                id: rid,
                provider_metadata: None,
            });
        }

        // End text
        if let Some(tid) = active_text_id.take() {
            yield Ok(StreamPart::TextEnd {
                id: tid,
                provider_metadata: None,
            });
        }

        // Flush tool calls that accumulated complete JSON
        for (_index, acc) in &tool_calls {
            if acc.started {
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

        // Emit finish
        let finish_reason = map_openai_compatible_finish_reason(final_finish_reason.as_deref());
        let usage = convert_openai_compatible_usage(final_usage.as_ref());

        yield Ok(StreamPart::Finish {
            finish_reason,
            usage,
            provider_metadata: None,
        });
    }
}

/// Accumulator for streaming tool call arguments.
struct ToolCallAccumulator {
    id: String,
    name: String,
    arguments: String,
    started: bool,
}

// ---------------------------------------------------------------------------
// Capability trait implementations
// ---------------------------------------------------------------------------

use ararajuba_provider::capabilities::{
    ImageFormat, SupportsImages, SupportsStructuredOutput, SupportsToolCalling,
};

impl SupportsToolCalling for OpenAICompatibleChatLanguageModel {
    fn max_tools(&self) -> Option<usize> {
        None // varies by provider
    }

    fn supports_parallel_calls(&self) -> bool {
        true
    }

    fn supports_strict_schemas(&self) -> bool {
        self.config.supports_structured_outputs
    }
}

impl SupportsImages for OpenAICompatibleChatLanguageModel {
    fn supported_image_formats(&self) -> Vec<ImageFormat> {
        vec![
            ImageFormat::Jpeg,
            ImageFormat::Png,
            ImageFormat::Gif,
            ImageFormat::Webp,
        ]
    }
}

impl SupportsStructuredOutput for OpenAICompatibleChatLanguageModel {
    fn supports_json_mode(&self) -> bool {
        true
    }

    fn supports_json_schema(&self) -> bool {
        self.config.supports_structured_outputs
    }
}
