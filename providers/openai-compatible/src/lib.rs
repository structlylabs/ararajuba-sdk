//! # ararajuba-openai-compatible
//!
//! Reusable building blocks for OpenAI-compatible AI providers.
//!
//! This crate implements the `LanguageModel` and `EmbeddingModel` traits from
//! `ararajuba-provider` against any API that follows the OpenAI chat completions
//! and embeddings format (xAI, Together AI, Fireworks, Groq, etc.).
//!
//! ## Quick start
//!
//! ```ignore
//! use ararajuba_openai_compatible::{create_openai_compatible, openai_compatible_settings};
//!
//! let provider = create_openai_compatible(openai_compatible_settings(
//!     "xai",
//!     "https://api.x.ai/v1",
//!     std::env::var("XAI_API_KEY").unwrap(),
//! ));
//!
//! let model = provider.language_model("grok-3").unwrap();
//! ```

pub mod chat;
pub mod embedding;
pub mod error;
pub mod provider;

// Re-export key types.
pub use chat::{ChatModelConfig, OpenAICompatibleChatLanguageModel};
pub use embedding::{EmbeddingModelConfig, OpenAICompatibleEmbeddingModel};
pub use error::{OpenAICompatibleError, parse_openai_compatible_error};
pub use provider::{
    create_openai_compatible, openai_compatible_settings, OpenAICompatibleProvider,
    OpenAICompatibleSettings,
};
