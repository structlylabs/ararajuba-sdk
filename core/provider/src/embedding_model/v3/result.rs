//! Embedding model result types.

use crate::shared::{Headers, ProviderMetadata, Warning};
use serde::{Deserialize, Serialize};

/// A single embedding vector.
pub type Embedding = Vec<f64>;

/// Result of an embedding model call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResult {
    /// One embedding per input value.
    pub embeddings: Vec<Embedding>,
    /// Token usage, if reported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<EmbeddingUsage>,
    /// Provider-specific metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_metadata: Option<ProviderMetadata>,
    /// Response metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<EmbeddingResponseMetadata>,
    /// Warnings.
    pub warnings: Vec<Warning>,
}

/// Token usage for embedding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingUsage {
    pub tokens: u64,
}

/// Response metadata for embedding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponseMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Headers>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
}
