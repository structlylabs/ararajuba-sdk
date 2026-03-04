//! DeepSeek provider for the AI SDK.
//!
//! This crate implements the DeepSeek API, providing language model
//! support for DeepSeek Chat and Reasoner models.
//!
//! # Usage
//!
//! ```rust,no_run
//! use ararajuba_deepseek::{create_deepseek, DeepSeekSettings};
//! use ararajuba_provider::provider::Provider;
//!
//! let provider = create_deepseek(DeepSeekSettings {
//!     api_key: Some("sk-xxx".into()),
//!     ..Default::default()
//! });
//!
//! let model = provider.language_model("deepseek-chat").unwrap();
//! ```

pub mod chat;
pub mod provider;

pub use chat::chat_model::DeepSeekChatLanguageModel;
pub use chat::options::DeepSeekChatOptions;
pub use provider::{create_deepseek, deepseek, DeepSeekProvider, DeepSeekSettings};

// DeepSeek uses the OpenAI-compatible API format, so errors are typed as OpenAICompatibleError.
pub use ararajuba_openai_compatible::OpenAICompatibleError as DeepSeekError;
