//! The `rerank` high-level function.

use crate::error::Error;
use crate::types::call_warning::CallWarning;
use ararajuba_provider::reranking_model::v4::reranking_model_v4::RerankingModelV4;
use ararajuba_provider::reranking_model::v4::{
    RankedDocument, RerankCallOptions, RerankResponseMetadata, RerankResult, RerankUsage,
};
use std::collections::HashMap;

/// Options for `rerank()`.
pub struct RerankOptions {
    /// The reranking model to use.
    pub model: Box<dyn RerankingModelV4>,
    /// The query to rank documents against.
    pub query: String,
    /// The documents to rerank. Each can be a plain string or a JSON object.
    pub documents: Vec<serde_json::Value>,
    /// Maximum number of results to return.
    pub top_n: Option<usize>,
    /// Provider-specific options.
    pub provider_options: Option<HashMap<String, HashMap<String, serde_json::Value>>>,
    /// Additional headers.
    pub headers: Option<HashMap<String, String>>,
}

/// Result of `rerank()`.
pub struct RerankHighLevelResult {
    /// Ranked results, ordered by relevance (most relevant first).
    pub results: Vec<RankedDocument>,
    /// Token usage.
    pub usage: Option<RerankUsage>,
    /// Warnings from the model.
    pub warnings: Vec<CallWarning>,
    /// Response metadata.
    pub response: RerankResponseMetadata,
}

/// Rerank documents by relevance to a query.
pub async fn rerank(options: RerankOptions) -> Result<RerankHighLevelResult, Error> {
    let _span = tracing::info_span!(
        "rerank",
        documents = options.documents.len(),
        top_n = ?options.top_n,
    )
    .entered();

    let call_options = RerankCallOptions {
        query: options.query,
        documents: options.documents,
        top_n: options.top_n,
        provider_options: options.provider_options.unwrap_or_default(),
        headers: options.headers,
    };

    let result: RerankResult = options.model.do_rerank(&call_options).await?;

    Ok(RerankHighLevelResult {
        results: result.results,
        usage: result.usage,
        warnings: result.warnings.into_iter().map(CallWarning::from).collect(),
        response: result.response,
    })
}
