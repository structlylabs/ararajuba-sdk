//! v4 Call options for language model invocations.
//!
//! Identical structure to v3 but provides typed provider-option helpers
//! and is the canonical import path for v4 consumers.

use super::prompt::Prompt;
use super::tool::Tool;
use super::tool_choice::ToolChoice;
use crate::shared::{Headers, ProviderOptions};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

/// Options passed to `do_generate` and `do_stream`.
///
/// Provider-specific options are stored in [`provider_options`](Self::provider_options)
/// as a `HashMap<String, JSONObject>`. Use the typed helper [`Self::set_provider_options`]
/// to serialize a strongly-typed options struct.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CallOptions {
    /// The prompt messages.
    pub prompt: Prompt,

    /// Maximum number of output tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,

    /// Temperature for sampling (0.0 – 2.0 typically).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,

    /// Nucleus sampling probability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,

    /// Top-K sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,

    /// Presence penalty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,

    /// Frequency penalty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,

    /// Expected response format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,

    /// Random seed for reproducible results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,

    /// Tools the model may call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,

    /// How the model should choose tools.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,

    /// Whether to include raw chunks in the stream.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_raw_chunks: Option<bool>,

    /// Additional headers for the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Headers>,

    /// Provider-specific options (type-erased).
    ///
    /// Use [`Self::set_provider_options`] or [`Self::get_provider_options`]
    /// for type-safe access.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_options: Option<ProviderOptions>,

    /// Cancellation token for cooperative cancellation (opt-in).
    ///
    /// When cancelled, ongoing API calls and streaming should abort.
    /// Note: v4 streams also support automatic cancellation via `Drop`.
    #[serde(skip)]
    pub cancellation_token: Option<CancellationToken>,
}

impl CallOptions {
    /// Set typed provider options for a specific provider.
    ///
    /// Serializes `opts` into the `provider_options` map under the given key.
    ///
    /// # Example
    /// ```ignore
    /// options.set_provider_options("openai.chat", &OpenAIChatOptions {
    ///     reasoning_effort: Some("high".into()),
    ///     ..Default::default()
    /// });
    /// ```
    pub fn set_provider_options<T: Serialize>(
        &mut self,
        provider_key: &str,
        opts: &T,
    ) -> Result<(), serde_json::Error> {
        let value = serde_json::to_value(opts)?;
        let map: crate::json_value::JSONObject = serde_json::from_value(value)?;
        self.provider_options
            .get_or_insert_with(Default::default)
            .insert(provider_key.to_string(), map);
        Ok(())
    }

    /// Get typed provider options for a specific provider.
    ///
    /// Deserializes from the `provider_options` map.
    pub fn get_provider_options<T: for<'de> Deserialize<'de>>(
        &self,
        provider_key: &str,
    ) -> Option<T> {
        self.provider_options
            .as_ref()?
            .get(provider_key)
            .and_then(|obj| {
                let value = serde_json::to_value(obj).ok()?;
                serde_json::from_value(value).ok()
            })
    }
}

/// Response format requested from the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseFormat {
    /// Plain text response.
    #[serde(rename = "text")]
    Text,
    /// JSON response, optionally constrained by a schema.
    #[serde(rename = "json")]
    Json {
        #[serde(skip_serializing_if = "Option::is_none")]
        schema: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
}

// Conversion between v3 and v4 CallOptions
impl From<super::super::v3::call_options::CallOptions> for CallOptions {
    fn from(v3: super::super::v3::call_options::CallOptions) -> Self {
        Self {
            prompt: v3.prompt,
            max_output_tokens: v3.max_output_tokens,
            temperature: v3.temperature,
            stop_sequences: v3.stop_sequences,
            top_p: v3.top_p,
            top_k: v3.top_k,
            presence_penalty: v3.presence_penalty,
            frequency_penalty: v3.frequency_penalty,
            response_format: v3.response_format.map(|rf| match rf {
                super::super::v3::call_options::ResponseFormat::Text => ResponseFormat::Text,
                super::super::v3::call_options::ResponseFormat::Json {
                    schema,
                    name,
                    description,
                } => ResponseFormat::Json {
                    schema,
                    name,
                    description,
                },
            }),
            seed: v3.seed,
            tools: v3.tools,
            tool_choice: v3.tool_choice,
            include_raw_chunks: v3.include_raw_chunks,
            headers: v3.headers,
            provider_options: v3.provider_options,
            cancellation_token: v3.cancellation_token,
        }
    }
}
