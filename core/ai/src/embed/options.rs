//! Options for `embed` and `embed_many`.

use crate::types::call_settings::CallSettings;

/// Options for `embed()`.
pub struct EmbedOptions {
    /// The embedding model to use.
    pub model: Box<dyn ararajuba_provider::EmbeddingModelV4>,
    /// The text value to embed.
    pub value: String,
    /// Call settings.
    pub call_settings: CallSettings,
}

/// Options for `embed_many()`.
pub struct EmbedManyOptions {
    /// The embedding model to use.
    pub model: Box<dyn ararajuba_provider::EmbeddingModelV4>,
    /// The text values to embed.
    pub values: Vec<String>,
    /// Call settings.
    pub call_settings: CallSettings,
}
