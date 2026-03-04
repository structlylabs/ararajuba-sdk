//! Google Generative AI (Gemini) language model implementation.

use crate::chat::convert_messages::convert_to_google_generative_ai_messages;
use crate::chat::finish_reason::{convert_google_usage, map_google_finish_reason};
use crate::chat::prepare_tools::prepare_google_tools;
use crate::error::parse_google_error;
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

/// Configuration for the Google Generative AI chat model.
#[derive(Clone)]
pub struct GoogleChatConfig {
    pub provider: String,
    pub base_url: String,
    pub headers: HashMap<String, String>,
    pub model_id: String,
    pub generate_id: Option<Arc<dyn Fn() -> String + Send + Sync>>,
    pub fetch: Option<
        Arc<
            dyn Fn(reqwest::Request) -> BoxFuture<'static, reqwest::Result<reqwest::Response>>
                + Send
                + Sync,
        >,
    >,
}

/// A Google Generative AI (Gemini) language model.
pub struct GoogleGenerativeAILanguageModel {
    model_id: String,
    config: GoogleChatConfig,
}

impl GoogleGenerativeAILanguageModel {
    pub fn new(model_id: String, config: GoogleChatConfig) -> Self {
        Self { model_id, config }
    }

    /// Get model path — if model_id contains `/`, use as-is; otherwise `models/{model_id}`.
    fn model_path(&self) -> String {
        if self.model_id.contains('/') {
            self.model_id.clone()
        } else {
            format!("models/{}", self.model_id)
        }
    }

    fn generate_id(&self) -> String {
        if let Some(ref gen_fn) = self.config.generate_id {
            gen_fn()
        } else {
            uuid::Uuid::new_v4().to_string()
        }
    }

    /// Build the request body.
    fn build_body(&self, options: &CallOptions) -> (Value, Vec<Warning>) {
        let prompt = convert_to_google_generative_ai_messages(&options.prompt);
        let prepared = prepare_google_tools(
            options.tools.as_deref(),
            options.tool_choice.as_ref(),
        );

        let mut body = json!({
            "contents": prompt.contents,
        });

        // System instruction at top level
        if let Some(si) = prompt.system_instruction {
            body["systemInstruction"] = si;
        }

        // Generation config
        let mut gen_config = json!({});

        if let Some(max) = options.max_output_tokens {
            gen_config["maxOutputTokens"] = json!(max);
        }
        if let Some(temp) = options.temperature {
            gen_config["temperature"] = json!(temp);
        }
        if let Some(tp) = options.top_p {
            gen_config["topP"] = json!(tp);
        }
        if let Some(tk) = options.top_k {
            gen_config["topK"] = json!(tk);
        }
        if let Some(fp) = options.frequency_penalty {
            gen_config["frequencyPenalty"] = json!(fp);
        }
        if let Some(pp) = options.presence_penalty {
            gen_config["presencePenalty"] = json!(pp);
        }
        if let Some(ref seqs) = options.stop_sequences {
            if !seqs.is_empty() {
                gen_config["stopSequences"] = json!(seqs);
            }
        }
        if let Some(seed) = options.seed {
            gen_config["seed"] = json!(seed);
        }

        // Response format
        if let Some(ref rf) = options.response_format {
            use ararajuba_provider::language_model::v4::call_options::ResponseFormat;
            match rf {
                ResponseFormat::Json { schema, .. } => {
                    gen_config["responseMimeType"] = json!("application/json");
                    if let Some(schema) = schema {
                        gen_config["responseSchema"] = schema.clone();
                    }
                }
                ResponseFormat::Text => {}
            }
        }

        // Provider options
        let warnings = prepared.warnings;
        if let Some(ref po) = options.provider_options {
            if let Some(google_opts) = po.get("google") {
                // Thinking config
                if let Some(tc) = google_opts.get("thinkingConfig") {
                    gen_config["thinkingConfig"] = tc.clone();
                }
                // Response modalities
                if let Some(rm) = google_opts.get("responseModalities") {
                    gen_config["responseModalities"] = rm.clone();
                }
                // Audio timestamp
                if let Some(at) = google_opts.get("audioTimestamp") {
                    gen_config["audioTimestamp"] = at.clone();
                }
                // Media resolution
                if let Some(mr) = google_opts.get("mediaResolution") {
                    gen_config["mediaResolution"] = mr.clone();
                }
                // Image config
                if let Some(ic) = google_opts.get("imageConfig") {
                    gen_config["imageConfig"] = ic.clone();
                }
                // Safety settings
                if let Some(ss) = google_opts.get("safetySettings") {
                    body["safetySettings"] = ss.clone();
                }
                // Cached content
                if let Some(cc) = google_opts.get("cachedContent") {
                    body["cachedContent"] = cc.clone();
                }
                // Labels
                if let Some(labels) = google_opts.get("labels") {
                    body["labels"] = labels.clone();
                }
            }
        }

        if gen_config.as_object().map_or(false, |o| !o.is_empty()) {
            body["generationConfig"] = gen_config;
        }

        // Tools
        if let Some(tools) = prepared.tools {
            body["tools"] = json!(tools);
        }
        if let Some(tc) = prepared.tool_config {
            body["toolConfig"] = tc;
        }

        (body, warnings)
    }
}

#[async_trait]
impl LanguageModelV4 for GoogleGenerativeAILanguageModel {
    fn provider(&self) -> &str {
        &self.config.provider
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }

    async fn do_generate(&self, options: &CallOptions) -> Result<GenerateResult, Error> {
            let (body, warnings) = self.build_body(options);
            let body_clone = body.clone();

            let url = format!(
                "{}/{}:generateContent",
                self.config.base_url,
                self.model_path()
            );

            let response_handler = create_json_response_handler(|v: Value| Ok(v));
            let error_handler = create_json_error_response_handler(parse_google_error);

            let raw = post_json_to_api(PostJsonOptions {
                url,
                headers: Some(self.config.headers.clone()),
                body,
                successful_response_handler: response_handler,
                failed_response_handler: error_handler,
                fetch: self.config.fetch.clone(),
                retry: None,
                cancellation_token: options.cancellation_token.clone(),
            })
            .await?;

            // Parse Google response
            let candidate = raw
                .get("candidates")
                .and_then(|c| c.as_array())
                .and_then(|a| a.first())
                .unwrap_or(&Value::Null);

            let parts = candidate
                .get("content")
                .and_then(|c| c.get("parts"))
                .and_then(|p| p.as_array());

            let mut content = Vec::new();
            let mut has_tool_calls = false;

            if let Some(parts) = parts {
                for part in parts {
                    if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
                        let is_thought = part.get("thought").and_then(|v| v.as_bool()).unwrap_or(false);
                        if is_thought {
                            content.push(Content::Reasoning {
                                text: text.to_string(),
                                provider_metadata: None,
                            });
                        } else {
                            content.push(Content::Text {
                                text: text.to_string(),
                                provider_metadata: None,
                            });
                        }
                    }
                    if let Some(fc) = part.get("functionCall") {
                        has_tool_calls = true;
                        let name = fc.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let args = fc.get("args").map(|v| v.to_string()).unwrap_or("{}".into());
                        content.push(Content::ToolCall {
                            tool_call_id: self.generate_id(),
                            tool_name: name,
                            input: args,
                            provider_executed: None,
                            dynamic: None,
                            provider_metadata: None,
                        });
                    }
                }
            }

            let finish_reason_raw = candidate.get("finishReason").and_then(|v| v.as_str());
            let finish_reason = map_google_finish_reason(finish_reason_raw, has_tool_calls);

            let usage_meta = raw.get("usageMetadata");
            let prompt_tokens = usage_meta
                .and_then(|u| u.get("promptTokenCount"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let candidates_tokens = usage_meta
                .and_then(|u| u.get("candidatesTokenCount"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let cached_tokens = usage_meta
                .and_then(|u| u.get("cachedContentTokenCount"))
                .and_then(|v| v.as_u64());
            let thoughts_tokens = usage_meta
                .and_then(|u| u.get("thoughtsTokenCount"))
                .and_then(|v| v.as_u64());

            let usage = convert_google_usage(prompt_tokens, candidates_tokens, cached_tokens, thoughts_tokens);

            Ok(GenerateResult {
                content,
                finish_reason,
                usage,
                provider_metadata: None,
                request: Some(RequestMetadata {
                    body: Some(body_clone),
                }),
                response: Some(ResponseMetadata {
                    id: None,
                    timestamp: None,
                    model_id: None,
                    headers: None,
                    body: Some(raw),
                }),
                warnings,
            })
    }

    async fn do_stream(&self, options: &CallOptions) -> Result<StreamResult, Error> {
        let (body, warnings) = self.build_body(options);
        let body_clone = body.clone();

        let url = format!(
            "{}/{}:streamGenerateContent?alt=sse",
            self.config.base_url,
            self.model_path()
        );

        let chunk_parser: Arc<dyn Fn(Value) -> Result<Value, Error> + Send + Sync> =
            Arc::new(|v| Ok(v));
        let response_handler = create_event_source_response_handler(chunk_parser);
        let error_handler = create_json_error_response_handler(parse_google_error);

        let raw_stream: BoxStream<'static, Result<Value, Error>> =
            post_json_to_api(PostJsonOptions {
                url,
                headers: Some(self.config.headers.clone()),
                body,
                successful_response_handler: response_handler,
                failed_response_handler: error_handler,
                fetch: self.config.fetch.clone(),
                retry: None,
                cancellation_token: options.cancellation_token.clone(),
            })
            .await?;

        let generate_id_fn = self.config.generate_id.clone();
        let merged_stream: BoxStream<'static, Result<StreamPart, Error>> =
            Box::pin(transform_google_stream(raw_stream, warnings, generate_id_fn));

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

/// Transform Google SSE stream into StreamPart events.
///
/// Google streaming sends full response-shaped JSON in each SSE chunk.
/// Each chunk contains `candidates[0].content.parts`, and the final chunk
/// has `finishReason` and `usageMetadata`.
fn transform_google_stream(
    raw_stream: BoxStream<'static, Result<Value, Error>>,
    initial_warnings: Vec<Warning>,
    generate_id_fn: Option<Arc<dyn Fn() -> String + Send + Sync>>,
) -> impl futures::Stream<Item = Result<StreamPart, Error>> {
    use async_stream::stream;

    stream! {
        let mut is_first = true;
        let mut prompt_tokens: u64 = 0;
        let mut candidates_tokens: u64 = 0;
        let mut cached_tokens: Option<u64> = None;
        let mut thoughts_tokens: Option<u64> = None;
        let mut last_finish_reason: Option<String> = None;
        let mut accumulated_tool_calls = false;

        // Track active content block IDs
        let mut active_text_id: Option<String> = None;
        let mut active_reasoning_id: Option<String> = None;

        let gen_id = move || -> String {
            if let Some(ref f) = generate_id_fn {
                f()
            } else {
                uuid::Uuid::new_v4().to_string()
            }
        };

        futures::pin_mut!(raw_stream);

        while let Some(chunk_result) = raw_stream.next().await {
            let chunk = match chunk_result {
                Ok(v) => v,
                Err(e) => {
                    yield Err(e);
                    continue;
                }
            };

            if is_first {
                is_first = false;
                yield Ok(StreamPart::StreamStart {
                    warnings: initial_warnings.clone(),
                });
                yield Ok(StreamPart::ResponseMetadata(ResponseMetadata {
                    id: None,
                    timestamp: None,
                    model_id: None,
                    headers: None,
                    body: None,
                }));
            }

            // Extract usage from each chunk (final one wins)
            if let Some(usage) = chunk.get("usageMetadata") {
                prompt_tokens = usage.get("promptTokenCount").and_then(|v| v.as_u64()).unwrap_or(prompt_tokens);
                candidates_tokens = usage.get("candidatesTokenCount").and_then(|v| v.as_u64()).unwrap_or(candidates_tokens);
                cached_tokens = usage.get("cachedContentTokenCount").and_then(|v| v.as_u64()).or(cached_tokens);
                thoughts_tokens = usage.get("thoughtsTokenCount").and_then(|v| v.as_u64()).or(thoughts_tokens);
            }

            // Extract candidate
            let candidate = chunk
                .get("candidates")
                .and_then(|c| c.as_array())
                .and_then(|a| a.first());

            if let Some(candidate) = candidate {
                // Extract finish reason (present in final chunk typically)
                if let Some(fr) = candidate.get("finishReason").and_then(|v| v.as_str()) {
                    last_finish_reason = Some(fr.to_string());
                }

                // Process parts
                if let Some(parts) = candidate.get("content").and_then(|c| c.get("parts")).and_then(|p| p.as_array()) {
                    for part in parts {
                        // Text part
                        if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
                            let is_thought = part.get("thought").and_then(|v| v.as_bool()).unwrap_or(false);

                            if is_thought {
                                // Reasoning
                                if active_reasoning_id.is_none() {
                                    let id = gen_id();
                                    active_reasoning_id = Some(id.clone());
                                    yield Ok(StreamPart::ReasoningStart {
                                        id,
                                        provider_metadata: None,
                                    });
                                }
                                yield Ok(StreamPart::ReasoningDelta {
                                    id: active_reasoning_id.clone().unwrap(),
                                    delta: text.to_string(),
                                    provider_metadata: None,
                                });
                            } else {
                                // Close reasoning if transitioning to text
                                if let Some(rid) = active_reasoning_id.take() {
                                    yield Ok(StreamPart::ReasoningEnd {
                                        id: rid,
                                        provider_metadata: None,
                                    });
                                }

                                // Text
                                if active_text_id.is_none() {
                                    let id = gen_id();
                                    active_text_id = Some(id.clone());
                                    yield Ok(StreamPart::TextStart {
                                        id,
                                        provider_metadata: None,
                                    });
                                }
                                yield Ok(StreamPart::TextDelta {
                                    id: active_text_id.clone().unwrap(),
                                    delta: text.to_string(),
                                    provider_metadata: None,
                                });
                            }
                        }

                        // Function call
                        if let Some(fc) = part.get("functionCall") {
                            accumulated_tool_calls = true;
                            let name = fc.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let args = fc.get("args").map(|v| v.to_string()).unwrap_or("{}".into());
                            let tool_id = gen_id();

                            yield Ok(StreamPart::ToolInputStart {
                                id: tool_id.clone(),
                                tool_name: name.clone(),
                                provider_metadata: None,
                                provider_executed: None,
                                dynamic: None,
                                title: None,
                            });
                            yield Ok(StreamPart::ToolInputDelta {
                                id: tool_id.clone(),
                                delta: args.clone(),
                                provider_metadata: None,
                            });
                            yield Ok(StreamPart::ToolInputEnd {
                                id: tool_id.clone(),
                                provider_metadata: None,
                            });
                            yield Ok(StreamPart::ToolCall {
                                tool_call_id: tool_id,
                                tool_name: name,
                                input: args,
                                provider_executed: None,
                                dynamic: None,
                                provider_metadata: None,
                            });
                        }
                    }
                }
            }
        }

        // Close any open blocks
        if let Some(tid) = active_text_id.take() {
            yield Ok(StreamPart::TextEnd {
                id: tid,
                provider_metadata: None,
            });
        }
        if let Some(rid) = active_reasoning_id.take() {
            yield Ok(StreamPart::ReasoningEnd {
                id: rid,
                provider_metadata: None,
            });
        }

        // Emit finish
        let finish_reason = map_google_finish_reason(
            last_finish_reason.as_deref(),
            accumulated_tool_calls,
        );
        let usage = convert_google_usage(
            prompt_tokens,
            candidates_tokens,
            cached_tokens,
            thoughts_tokens,
        );

        yield Ok(StreamPart::Finish {
            finish_reason,
            usage,
            provider_metadata: None,
        });
    }
}

// ---------------------------------------------------------------------------
// Capability trait implementations
// ---------------------------------------------------------------------------

use ararajuba_provider::capabilities::{
    CacheConfig, ImageFormat, ReasoningConfig, SupportsCaching, SupportsImages, SupportsReasoning,
    SupportsStructuredOutput, SupportsToolCalling,
};

impl SupportsReasoning for GoogleGenerativeAILanguageModel {
    fn reasoning_config(&self) -> ReasoningConfig {
        ReasoningConfig {
            enabled: true,
            default_effort: None,
            max_reasoning_tokens: None, // controlled via thinkingConfig.thinkingBudget
        }
    }
}

impl SupportsCaching for GoogleGenerativeAILanguageModel {
    fn cache_config(&self) -> CacheConfig {
        CacheConfig {
            supports_auto_cache: false,
            supports_cache_control: false,
            max_cache_tokens: None,
        }
    }
}

impl SupportsToolCalling for GoogleGenerativeAILanguageModel {
    fn max_tools(&self) -> Option<usize> {
        None
    }

    fn supports_parallel_calls(&self) -> bool {
        true
    }
}

impl SupportsImages for GoogleGenerativeAILanguageModel {
    fn supported_image_formats(&self) -> Vec<ImageFormat> {
        vec![
            ImageFormat::Jpeg,
            ImageFormat::Png,
            ImageFormat::Gif,
            ImageFormat::Webp,
        ]
    }
}

impl SupportsStructuredOutput for GoogleGenerativeAILanguageModel {
    fn supports_json_mode(&self) -> bool {
        true // responseMimeType: application/json
    }

    fn supports_json_schema(&self) -> bool {
        true // responseSchema
    }
}
