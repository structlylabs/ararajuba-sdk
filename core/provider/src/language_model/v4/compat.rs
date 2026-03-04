//! Compatibility adapter: wraps a v3 `LanguageModel` as v4 `LanguageModelV4`.
//!
//! This allows existing v3 implementations to be used through the v4 interface
//! without rewriting them. Providers can migrate to native v4 incrementally.
//!
//! # Example
//! ```ignore
//! use ararajuba_provider::language_model::v4::compat::V3LanguageModelAdapter;
//!
//! let v3_model: Box<dyn LanguageModel> = provider.language_model("gpt-4o").unwrap();
//! let v4_model = V3LanguageModelAdapter::new(v3_model);
//! // Now v4_model implements LanguageModelV4
//! ```

use super::call_options::CallOptions as V4CallOptions;
use super::language_model_v4::LanguageModelV4;
use super::stream_result::{split_merged_stream, AbortHandle, StreamResult as V4StreamResult};
use crate::errors::Error;
use crate::language_model::v3::call_options::{
    CallOptions as V3CallOptions, ResponseFormat as V3ResponseFormat,
};
use crate::language_model::v3::generate_result::GenerateResult;
use crate::language_model::v3::language_model_v3::LanguageModel;
use async_trait::async_trait;

/// Adapter that wraps a v3 `LanguageModel` to provide `LanguageModelV4`.
pub struct V3LanguageModelAdapter {
    inner: Box<dyn LanguageModel>,
}

impl V3LanguageModelAdapter {
    /// Wrap a v3 language model.
    pub fn new(inner: Box<dyn LanguageModel>) -> Self {
        Self { inner }
    }

    /// Get a reference to the inner v3 model.
    pub fn inner(&self) -> &dyn LanguageModel {
        &*self.inner
    }
}

#[async_trait]
impl LanguageModelV4 for V3LanguageModelAdapter {
    fn provider(&self) -> &str {
        self.inner.provider()
    }

    fn model_id(&self) -> &str {
        self.inner.model_id()
    }

    async fn do_generate(&self, options: &V4CallOptions) -> Result<GenerateResult, Error> {
        let v3_options = v4_to_v3_options(options);
        self.inner.do_generate(&v3_options).await
    }

    async fn do_stream(&self, options: &V4CallOptions) -> Result<V4StreamResult, Error> {
        let v3_options = v4_to_v3_options(options);
        let v3_result = self.inner.do_stream(&v3_options).await?;

        let request = v3_result.request.map(|r| {
            super::stream_result::StreamRequestMetadata { body: r.body }
        });
        let response = v3_result.response.map(|r| {
            super::stream_result::StreamResponseMetadata {
                headers: r.headers,
            }
        });

        Ok(split_merged_stream(
            v3_result.stream,
            AbortHandle::noop(),
            request,
            response,
        ))
    }
}

/// Convert v4 `CallOptions` to v3 `CallOptions`.
pub fn v4_to_v3_options(v4: &V4CallOptions) -> V3CallOptions {
    V3CallOptions {
        prompt: v4.prompt.clone(),
        max_output_tokens: v4.max_output_tokens,
        temperature: v4.temperature,
        stop_sequences: v4.stop_sequences.clone(),
        top_p: v4.top_p,
        top_k: v4.top_k,
        presence_penalty: v4.presence_penalty,
        frequency_penalty: v4.frequency_penalty,
        response_format: v4.response_format.as_ref().map(|rf| match rf {
            super::call_options::ResponseFormat::Text => V3ResponseFormat::Text,
            super::call_options::ResponseFormat::Json {
                schema,
                name,
                description,
            } => V3ResponseFormat::Json {
                schema: schema.clone(),
                name: name.clone(),
                description: description.clone(),
            },
        }),
        seed: v4.seed,
        tools: v4.tools.clone(),
        tool_choice: v4.tool_choice.clone(),
        include_raw_chunks: v4.include_raw_chunks,
        headers: v4.headers.clone(),
        provider_options: v4.provider_options.clone(),
        cancellation_token: v4.cancellation_token.clone(),
    }
}
