//! # ararajuba-openai
//!
//! OpenAI provider for the AI SDK.
//!
//! Implements `LanguageModel`, `EmbeddingModel`, `ImageModel`, `SpeechModel`,
//! and `TranscriptionModel` against the OpenAI APIs, with full support for
//! reasoning models (o-series, gpt-5), parameter stripping, and
//! OpenAI-specific provider options.
//!
//! ## Quick start
//!
//! ```ignore
//! use ararajuba_openai::{create_openai, OpenAISettings};
//! use ararajuba_provider::Provider;
//!
//! let provider = create_openai(OpenAISettings {
//!     api_key: Some("sk-...".into()),
//!     ..Default::default()
//! });
//!
//! let model = provider.language_model("gpt-4o").unwrap();
//! ```

pub mod capabilities;
pub mod chat;
pub mod embedding;
pub mod image;
pub mod provider;
pub mod responses;
pub mod speech;
pub mod transcription;

// Re-export key types.
pub use capabilities::{get_openai_model_capabilities, OpenAIModelCapabilities, SystemMessageMode};
pub use chat::{OpenAIChatLanguageModel, OpenAIChatOptions};
pub use embedding::OpenAIEmbeddingModel;
pub use image::{OpenAIImageModel, OpenAIImageOptions};
pub use responses::{OpenAIResponsesLanguageModel, OpenAIResponsesOptions};
pub use speech::{OpenAISpeechModel, OpenAISpeechOptions};
pub use transcription::{OpenAITranscriptionModel, OpenAITranscriptionOptions};
pub use provider::{create_openai, openai, OpenAIProvider, OpenAISettings};
