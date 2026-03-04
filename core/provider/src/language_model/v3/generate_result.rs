//! Generate result returned by `do_generate`.

use super::content::Content;
use super::finish_reason::FinishReason;
use super::usage::Usage;
use crate::shared::{Headers, ProviderMetadata, Warning};
use serde::{Deserialize, Serialize};

/// Result of a non-streaming language model generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateResult {
    /// Generated content parts.
    pub content: Vec<Content>,
    /// Why the model stopped generating.
    pub finish_reason: FinishReason,
    /// Token usage.
    pub usage: Usage,
    /// Provider-specific metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_metadata: Option<ProviderMetadata>,
    /// Request metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<RequestMetadata>,
    /// Response metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<ResponseMetadata>,
    /// Warnings from the model.
    pub warnings: Vec<Warning>,
}

/// Metadata about the request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
}

/// Metadata about the response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Headers>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
}
