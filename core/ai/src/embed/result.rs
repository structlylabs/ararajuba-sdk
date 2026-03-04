//! Result types for `embed` and `embed_many`.

use ararajuba_provider::embedding_model::v4::result::Embedding;

/// Result of `embed()`.
#[derive(Debug, Clone)]
pub struct EmbedResult {
    /// The embedding vector.
    pub embedding: Embedding,
    /// Token usage, if available.
    pub usage: Option<u64>,
}

/// Result of `embed_many()`.
#[derive(Debug, Clone)]
pub struct EmbedManyResult {
    /// One embedding per input value.
    pub embeddings: Vec<Embedding>,
    /// Token usage, if available.
    pub usage: Option<u64>,
}
