//! The v4 EmbeddingModel trait — async-native.

use super::call_options::EmbeddingCallOptions;
use super::result::EmbeddingResult;
use crate::errors::Error;
use async_trait::async_trait;

/// v4 Embedding model trait.
///
/// Changes from v3:
/// - `async fn` instead of `BoxFuture`
/// - Specification version `"v4"`
#[async_trait]
pub trait EmbeddingModelV4: Send + Sync {
    fn specification_version(&self) -> &'static str {
        "v4"
    }

    fn provider(&self) -> &str;
    fn model_id(&self) -> &str;

    /// Maximum number of values that can be embedded in a single call.
    fn max_embeddings_per_call(&self) -> Option<usize>;

    /// Whether this model supports parallel calls.
    fn supports_parallel_calls(&self) -> bool;

    /// Embed the given values.
    async fn do_embed(&self, options: &EmbeddingCallOptions) -> Result<EmbeddingResult, Error>;
}
