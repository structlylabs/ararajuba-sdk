//! The v4 RerankingModel trait — async-native.

use super::RerankCallOptions;
use super::RerankResult;
use crate::errors::Error;
use async_trait::async_trait;

/// v4 Reranking model trait.
#[async_trait]
pub trait RerankingModelV4: Send + Sync {
    fn specification_version(&self) -> &'static str {
        "v4"
    }

    fn provider(&self) -> &str;
    fn model_id(&self) -> &str;

    /// Rerank documents by relevance to a query.
    async fn do_rerank(&self, options: &RerankCallOptions) -> Result<RerankResult, Error>;
}
