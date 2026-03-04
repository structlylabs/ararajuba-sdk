//! Anthropic provider for the AI SDK.
//!
//! This crate implements the Anthropic Messages API, providing
//! language model support for Claude models.
//!
//! # Usage
//!
//! ```rust,no_run
//! use ararajuba_anthropic::{create_anthropic, AnthropicSettings};
//! use ararajuba_provider::provider::Provider;
//!
//! let provider = create_anthropic(AnthropicSettings {
//!     api_key: Some("sk-ant-xxx".into()),
//!     ..Default::default()
//! });
//!
//! let model = provider.language_model("claude-sonnet-4-20250514").unwrap();
//! ```

pub mod chat;
pub mod error;
pub mod provider;

pub use chat::chat_model::{AnthropicChatConfig, AnthropicMessagesLanguageModel};
pub use chat::options::AnthropicChatOptions;
pub use error::{parse_anthropic_error, AnthropicError};
pub use provider::{create_anthropic, anthropic, AnthropicProvider, AnthropicSettings};
