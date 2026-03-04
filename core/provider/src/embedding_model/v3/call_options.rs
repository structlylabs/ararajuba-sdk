//! Call options for embedding model invocations.

use crate::shared::{Headers, ProviderOptions};
use serde::{Deserialize, Serialize};

/// Options passed to `do_embed`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingCallOptions {
    /// The text values to embed.
    pub values: Vec<String>,
    /// Provider-specific options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_options: Option<ProviderOptions>,
    /// Additional headers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Headers>,
}
