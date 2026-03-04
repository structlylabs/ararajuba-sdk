//! Google Generative AI (Gemini) provider for the AI SDK.
//!
//! This crate implements the Google Generative AI API, providing
//! language and embedding model support for Gemini models.
//!
//! # Usage
//!
//! ```rust,no_run
//! use ararajuba_google::{create_google, GoogleSettings};
//! use ararajuba_provider::provider::Provider;
//!
//! let provider = create_google(GoogleSettings {
//!     api_key: Some("AIza-xxx".into()),
//!     ..Default::default()
//! });
//!
//! let model = provider.language_model("gemini-2.0-flash").unwrap();
//! ```

pub mod chat;
pub mod embedding;
pub mod error;
pub mod image;
pub mod provider;
pub mod video;

pub use chat::chat_model::{GoogleChatConfig, GoogleGenerativeAILanguageModel};
pub use chat::options::GoogleChatOptions;
pub use embedding::embedding_model::{GoogleEmbeddingConfig, GoogleEmbeddingModel};
pub use error::{parse_google_error, GoogleError};
pub use image::google_image_model::{GoogleImageConfig, GoogleImageModel};
pub use provider::{create_google, google, GoogleProvider, GoogleSettings};
pub use video::google_video_model::{GoogleVideoConfig, GoogleVideoModel};
