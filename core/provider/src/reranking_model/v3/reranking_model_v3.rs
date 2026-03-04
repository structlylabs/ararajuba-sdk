//! The RerankingModel trait and associated types.

use crate::errors::Error;
use crate::shared::{Headers, ProviderMetadata, ProviderOptions, Warning};
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};

/// Options passed to `do_rerank` for reranking models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankCallOptions {
    /// The query to rank documents against.
    pub query: String,
    /// The documents to rerank. Each can be a plain string or a JSON object.
    pub documents: Vec<serde_json::Value>,
    /// Maximum number of results to return.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_n: Option<usize>,
    /// Provider-specific options.
    pub provider_options: ProviderOptions,
    /// Additional headers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Headers>,
}

/// Result of a reranking call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankResult {
    /// Ranked results, ordered by relevance (most relevant first).
    pub results: Vec<RankedDocument>,
    /// Token usage (if provided by the model).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<RerankUsage>,
    /// Warnings.
    pub warnings: Vec<Warning>,
    /// Provider-specific metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_metadata: Option<ProviderMetadata>,
    /// Response metadata.
    pub response: RerankResponseMetadata,
}

/// A document with its relevance score after reranking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedDocument {
    /// The original index of this document in the input list.
    pub index: usize,
    /// Relevance score (higher = more relevant). Range depends on the model.
    pub relevance_score: f64,
    /// The original document value.
    pub document: serde_json::Value,
}

/// Usage reported by the reranking model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankUsage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u64>,
}

/// Response metadata for reranking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankResponseMetadata {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub model_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Headers>,
}

/// Trait for reranking models.
pub trait RerankingModel: Send + Sync {
    fn specification_version(&self) -> &'static str {
        "v3"
    }

    fn provider(&self) -> &str;
    fn model_id(&self) -> &str;

    /// Rerank documents by relevance to a query.
    fn do_rerank<'a>(
        &'a self,
        options: &'a RerankCallOptions,
    ) -> BoxFuture<'a, Result<RerankResult, Error>>;
}
