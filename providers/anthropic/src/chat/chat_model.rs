//! Anthropic Messages API language model implementation.

use crate::chat::convert_messages::convert_to_anthropic_messages_prompt;
use crate::chat::finish_reason::{convert_anthropic_usage, map_anthropic_stop_reason};
use crate::chat::prepare_tools::prepare_anthropic_tools;
use crate::error::parse_anthropic_error;
use async_trait::async_trait;
use ararajuba_provider::errors::Error;
use ararajuba_provider::language_model::v4::call_options::CallOptions;
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

/// Configuration for the Anthropic chat model.
#[derive(Clone)]
pub struct AnthropicChatConfig {
    pub provider: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub fetch: Option<
        Arc<
            dyn Fn(reqwest::Request) -> BoxFuture<'static, reqwest::Result<reqwest::Response>>
                + Send
                + Sync,
        >,
    >,
}

/// An Anthropic Messages API language model.
pub struct AnthropicMessagesLanguageModel {
    model_id: String,
    config: AnthropicChatConfig,
}

impl AnthropicMessagesLanguageModel {
    pub fn new(model_id: String, config: AnthropicChatConfig) -> Self {
        Self { model_id, config }
    }

    /// Build the request body.
    fn build_body(&self, options: &CallOptions, stream: bool) -> (Value, Vec<Warning>) {
        let prompt = convert_to_anthropic_messages_prompt(&options.prompt);
        let prepared = prepare_anthropic_tools(
            options.tools.as_deref(),
            options.tool_choice.as_ref(),
        );

        let mut body = json!({
            "model": self.model_id,
            "messages": prompt.messages,
        });

        // System is top-level
        if let Some(system) = prompt.system {
            body["system"] = json!(system);
        }

        // max_tokens is required for Anthropic (default 4096)
        body["max_tokens"] = json!(options.max_output_tokens.unwrap_or(4096));

        if let Some(temp) = options.temperature {
            // Clamp to [0, 1]
            body["temperature"] = json!(temp.min(1.0));
        }
        if let Some(tp) = options.top_p {
            body["top_p"] = json!(tp);
        }
        if let Some(tk) = options.top_k {
            body["top_k"] = json!(tk);
        }
        if let Some(ref seqs) = options.stop_sequences {
            if !seqs.is_empty() {
                body["stop_sequences"] = json!(seqs);
            }
        }

        // Warn for unsupported params
        let mut warnings = prepared.warnings;
        if options.frequency_penalty.is_some() {
            warnings.push(Warning::Unsupported {
                feature: "frequency_penalty".into(),
                details: Some("Not supported by Anthropic".into()),
            });
        }
        if options.presence_penalty.is_some() {
            warnings.push(Warning::Unsupported {
                feature: "presence_penalty".into(),
                details: Some("Not supported by Anthropic".into()),
            });
        }
        if options.seed.is_some() {
            warnings.push(Warning::Unsupported {
                feature: "seed".into(),
                details: Some("Not supported by Anthropic".into()),
            });
        }

        // Response format
        if let Some(ref rf) = options.response_format {
            use ararajuba_provider::language_model::v4::call_options::ResponseFormat;
            match rf {
                ResponseFormat::Json { schema, .. } if schema.is_some() => {
                    // Use output_config for structured output
                    body["output_config"] = json!({
                        "format": {
                            "type": "json_schema",
                            "schema": schema,
                        }
                    });
                }
                _ => {} // text or json without schema: no special handling
            }
        }

        // Tools
        if let Some(tools) = prepared.tools {
            if !tools.is_empty() {
                body["tools"] = json!(tools);
            }
        }
        if let Some(tc) = prepared.tool_choice {
            body["tool_choice"] = tc;
        }

        // Provider options
        if let Some(ref po) = options.provider_options {
            if let Some(anthropic_opts) = po.get("anthropic") {
                // Thinking
                if let Some(thinking) = anthropic_opts.get("thinking") {
                    body["thinking"] = thinking.clone();
                }
                // Effort
                if let Some(effort) = anthropic_opts.get("effort") {
                    if body.get("output_config").is_none() {
                        body["output_config"] = json!({});
                    }
                    body["output_config"]["effort"] = effort.clone();
                }
                // Speed
                if let Some(speed) = anthropic_opts.get("speed") {
                    body["speed"] = speed.clone();
                }
                // Cache control
                if let Some(cc) = anthropic_opts.get("cacheControl")
                    .or_else(|| anthropic_opts.get("cache_control"))
                {
                    body["cache_control"] = cc.clone();
                }
            }
        }

        if stream {
            body["stream"] = json!(true);
        }

        (body, warnings)
    }
}

#[async_trait]
impl LanguageModelV4 for AnthropicMessagesLanguageModel {
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
            let error_handler = create_json_error_response_handler(parse_anthropic_error);

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

            // Parse Anthropic response
            let mut content = Vec::new();

            if let Some(blocks) = raw.get("content").and_then(|v| v.as_array()) {
                for block in blocks {
                    let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    match block_type {
                        "text" => {
                            if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                                content.push(Content::Text {
                                    text: text.to_string(),
                                    provider_metadata: None,
                                });
                            }
                        }
                        "thinking" => {
                            if let Some(thinking) = block.get("thinking").and_then(|v| v.as_str()) {
                                let mut pm = None;
                                if let Some(sig) = block.get("signature").and_then(|v| v.as_str()) {
                                    let mut meta = HashMap::new();
                                    let mut anthropic_meta = HashMap::new();
                                    anthropic_meta.insert(
                                        "signature".to_string(),
                                        json!(sig),
                                    );
                                    meta.insert("anthropic".to_string(), anthropic_meta);
                                    pm = Some(meta);
                                }
                                content.push(Content::Reasoning {
                                    text: thinking.to_string(),
                                    provider_metadata: pm,
                                });
                            }
                        }
                        "tool_use" => {
                            let id = block.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let name = block.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let input = block.get("input").map(|v| v.to_string()).unwrap_or("{}".into());
                            content.push(Content::ToolCall {
                                tool_call_id: id,
                                tool_name: name,
                                input,
                                provider_executed: None,
                                dynamic: None,
                                provider_metadata: None,
                            });
                        }
                        _ => {} // server_tool_use, mcp_tool_use, etc.
                    }
                }
            }

            let stop_reason = raw.get("stop_reason").and_then(|v| v.as_str());
            let finish_reason = map_anthropic_stop_reason(stop_reason, false);

            let usage_obj = raw.get("usage");
            let input_tokens = usage_obj
                .and_then(|u| u.get("input_tokens"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let output_tokens = usage_obj
                .and_then(|u| u.get("output_tokens"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let cache_creation = usage_obj
                .and_then(|u| u.get("cache_creation_input_tokens"))
                .and_then(|v| v.as_u64());
            let cache_read = usage_obj
                .and_then(|u| u.get("cache_read_input_tokens"))
                .and_then(|v| v.as_u64());

            let usage = convert_anthropic_usage(input_tokens, output_tokens, cache_creation, cache_read);

            let response_id = raw.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
            let response_model = raw.get("model").and_then(|v| v.as_str()).map(|s| s.to_string());

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
                    timestamp: None,
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
        let error_handler = create_json_error_response_handler(parse_anthropic_error);

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

        let merged_stream: BoxStream<'static, Result<StreamPart, Error>> =
            Box::pin(transform_anthropic_stream(raw_stream, warnings));

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

/// Content block tracking for streaming.
struct ContentBlock {
    block_type: String,
    id: String,       // UUID for text/reasoning, tool_use_id for tool
    tool_name: String, // Only for tool blocks
    text: String,      // Accumulated text/thinking/arguments
    started: bool,
}

/// Transform Anthropic SSE events into StreamPart events.
fn transform_anthropic_stream(
    raw_stream: BoxStream<'static, Result<Value, Error>>,
    initial_warnings: Vec<Warning>,
) -> impl futures::Stream<Item = Result<StreamPart, Error>> {
    use async_stream::stream;

    stream! {
        let mut is_first = true;
        let mut content_blocks: HashMap<usize, ContentBlock> = HashMap::new();
        let mut input_tokens: u64 = 0;
        let mut output_tokens: u64 = 0;
        let mut cache_creation_tokens: Option<u64> = None;
        let mut cache_read_tokens: Option<u64> = None;
        let mut stop_reason: Option<String> = None;

        futures::pin_mut!(raw_stream);

        while let Some(chunk_result) = raw_stream.next().await {
            let chunk = match chunk_result {
                Ok(v) => v,
                Err(e) => {
                    yield Err(e);
                    continue;
                }
            };

            let event_type = chunk.get("type").and_then(|v| v.as_str()).unwrap_or("");

            match event_type {
                "message_start" => {
                    if is_first {
                        is_first = false;
                        yield Ok(StreamPart::StreamStart {
                            warnings: initial_warnings.clone(),
                        });
                    }

                    let message = chunk.get("message").unwrap_or(&Value::Null);
                    let resp_id = message.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let resp_model = message.get("model").and_then(|v| v.as_str()).map(|s| s.to_string());

                    yield Ok(StreamPart::ResponseMetadata(ResponseMetadata {
                        id: resp_id,
                        timestamp: None,
                        model_id: resp_model,
                        headers: None,
                        body: None,
                    }));

                    // Extract input usage
                    if let Some(usage) = message.get("usage") {
                        input_tokens = usage.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                        cache_creation_tokens = usage.get("cache_creation_input_tokens").and_then(|v| v.as_u64());
                        cache_read_tokens = usage.get("cache_read_input_tokens").and_then(|v| v.as_u64());
                    }
                }

                "content_block_start" => {
                    let index = chunk.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                    let block = chunk.get("content_block").unwrap_or(&Value::Null);
                    let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("").to_string();

                    let id = uuid::Uuid::new_v4().to_string();
                    let mut cb = ContentBlock {
                        block_type: block_type.clone(),
                        id: id.clone(),
                        tool_name: String::new(),
                        text: String::new(),
                        started: false,
                    };

                    match block_type.as_str() {
                        "text" => {
                            cb.started = true;
                            yield Ok(StreamPart::TextStart {
                                id: id.clone(),
                                provider_metadata: None,
                            });
                        }
                        "thinking" => {
                            cb.started = true;
                            yield Ok(StreamPart::ReasoningStart {
                                id: id.clone(),
                                provider_metadata: None,
                            });
                        }
                        "tool_use" => {
                            let tool_id = block.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let name = block.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            cb.id = tool_id.clone();
                            cb.tool_name = name.clone();
                            cb.started = true;
                            yield Ok(StreamPart::ToolInputStart {
                                id: tool_id,
                                tool_name: name,
                                provider_metadata: None,
                                provider_executed: None,
                                dynamic: None,
                                title: None,
                            });
                        }
                        _ => {} // server_tool_use, mcp_tool_use, etc.
                    }

                    content_blocks.insert(index, cb);
                }

                "content_block_delta" => {
                    let index = chunk.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                    let delta = chunk.get("delta").unwrap_or(&Value::Null);
                    let delta_type = delta.get("type").and_then(|v| v.as_str()).unwrap_or("");

                    if let Some(cb) = content_blocks.get_mut(&index) {
                        match delta_type {
                            "text_delta" => {
                                if let Some(text) = delta.get("text").and_then(|v| v.as_str()) {
                                    cb.text.push_str(text);
                                    yield Ok(StreamPart::TextDelta {
                                        id: cb.id.clone(),
                                        delta: text.to_string(),
                                        provider_metadata: None,
                                    });
                                }
                            }
                            "thinking_delta" => {
                                if let Some(thinking) = delta.get("thinking").and_then(|v| v.as_str()) {
                                    cb.text.push_str(thinking);
                                    yield Ok(StreamPart::ReasoningDelta {
                                        id: cb.id.clone(),
                                        delta: thinking.to_string(),
                                        provider_metadata: None,
                                    });
                                }
                            }
                            "input_json_delta" => {
                                if let Some(json_str) = delta.get("partial_json").and_then(|v| v.as_str()) {
                                    cb.text.push_str(json_str);
                                    yield Ok(StreamPart::ToolInputDelta {
                                        id: cb.id.clone(),
                                        delta: json_str.to_string(),
                                        provider_metadata: None,
                                    });
                                }
                            }
                            _ => {} // signature_delta, citations_delta, etc.
                        }
                    }
                }

                "content_block_stop" => {
                    let index = chunk.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

                    if let Some(cb) = content_blocks.get(&index) {
                        match cb.block_type.as_str() {
                            "text" => {
                                yield Ok(StreamPart::TextEnd {
                                    id: cb.id.clone(),
                                    provider_metadata: None,
                                });
                            }
                            "thinking" => {
                                yield Ok(StreamPart::ReasoningEnd {
                                    id: cb.id.clone(),
                                    provider_metadata: None,
                                });
                            }
                            "tool_use" => {
                                yield Ok(StreamPart::ToolInputEnd {
                                    id: cb.id.clone(),
                                    provider_metadata: None,
                                });
                                yield Ok(StreamPart::ToolCall {
                                    tool_call_id: cb.id.clone(),
                                    tool_name: cb.tool_name.clone(),
                                    input: cb.text.clone(),
                                    provider_executed: None,
                                    dynamic: None,
                                    provider_metadata: None,
                                });
                            }
                            _ => {}
                        }
                    }
                }

                "message_delta" => {
                    if let Some(delta) = chunk.get("delta") {
                        stop_reason = delta.get("stop_reason").and_then(|v| v.as_str()).map(|s| s.to_string());
                    }
                    if let Some(usage) = chunk.get("usage") {
                        output_tokens = usage.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(output_tokens);
                    }
                }

                "message_stop" => {
                    // Emit finish
                    let finish_reason = map_anthropic_stop_reason(stop_reason.as_deref(), false);
                    let usage = convert_anthropic_usage(
                        input_tokens,
                        output_tokens,
                        cache_creation_tokens,
                        cache_read_tokens,
                    );

                    yield Ok(StreamPart::Finish {
                        finish_reason,
                        usage,
                        provider_metadata: None,
                    });
                }

                "error" => {
                    let msg = chunk.get("error")
                        .and_then(|e| e.get("message"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown streaming error");
                    yield Ok(StreamPart::Error {
                        error: msg.to_string(),
                    });
                }

                "ping" => {} // keep-alive, ignore

                _ => {} // unknown event types
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Capability trait implementations
// ---------------------------------------------------------------------------

use ararajuba_provider::capabilities::{
    CacheConfig, ImageFormat, ReasoningConfig, SupportsCaching, SupportsImages, SupportsReasoning,
    SupportsStructuredOutput, SupportsToolCalling,
};

impl SupportsReasoning for AnthropicMessagesLanguageModel {
    fn reasoning_config(&self) -> ReasoningConfig {
        ReasoningConfig {
            enabled: true,
            default_effort: Some("high".to_string()),
            max_reasoning_tokens: None, // controlled via thinking.budget_tokens
        }
    }
}

impl SupportsCaching for AnthropicMessagesLanguageModel {
    fn cache_config(&self) -> CacheConfig {
        CacheConfig {
            supports_auto_cache: false,
            supports_cache_control: true,
            max_cache_tokens: None,
        }
    }
}

impl SupportsToolCalling for AnthropicMessagesLanguageModel {
    fn max_tools(&self) -> Option<usize> {
        None // no documented limit
    }

    fn supports_parallel_calls(&self) -> bool {
        true
    }
}

impl SupportsImages for AnthropicMessagesLanguageModel {
    fn supported_image_formats(&self) -> Vec<ImageFormat> {
        vec![
            ImageFormat::Jpeg,
            ImageFormat::Png,
            ImageFormat::Gif,
            ImageFormat::Webp,
        ]
    }
}

impl SupportsStructuredOutput for AnthropicMessagesLanguageModel {
    fn supports_json_mode(&self) -> bool {
        false // Anthropic uses json_schema, not plain json mode
    }

    fn supports_json_schema(&self) -> bool {
        true
    }
}
