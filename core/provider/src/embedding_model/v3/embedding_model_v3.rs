//! The EmbeddingModel trait.

use super::call_options::EmbeddingCallOptions;
use super::result::EmbeddingResult;
use crate::errors::Error;
use futures::future::BoxFuture;

/// Trait for embedding models.
pub trait EmbeddingModel: Send + Sync {
    fn specification_version(&self) -> &'static str {
        "v3"
    }

    fn provider(&self) -> &str;
    fn model_id(&self) -> &str;

    /// Maximum number of values that can be embedded in a single call.
    fn max_embeddings_per_call(&self) -> Option<usize>;

    /// Whether this model supports parallel calls.
    fn supports_parallel_calls(&self) -> bool;

    /// Embed the given values.
    fn do_embed<'a>(
        &'a self,
        options: &'a EmbeddingCallOptions,
    ) -> BoxFuture<'a, Result<EmbeddingResult, Error>>;
}
