//! Call options for language model invocations.

use super::prompt::Prompt;
use super::tool::Tool;
use super::tool_choice::ToolChoice;
use crate::shared::{Headers, ProviderOptions};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

/// Options passed to `do_generate` and `do_stream`.
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

    /// Provider-specific options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_options: Option<ProviderOptions>,

    /// Cancellation token for cooperative cancellation.
    /// When cancelled, ongoing API calls and streaming should abort.
    #[serde(skip)]
    pub cancellation_token: Option<CancellationToken>,
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
