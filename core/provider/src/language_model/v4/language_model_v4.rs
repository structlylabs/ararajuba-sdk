//! The v4 LanguageModel trait — async-native, with typed errors and
//! split streams.
//!
//! Uses `#[async_trait]` for dyn-compatibility (`Box<dyn LanguageModel>`).
//!
//! # Example (provider implementation)
//! ```ignore
//! use async_trait::async_trait;
//! use ararajuba_provider::language_model::v4::language_model_v4::LanguageModelV4;
//! use ararajuba_provider::language_model::v4::call_options::CallOptions;
//! use ararajuba_provider::language_model::v4::generate_result::GenerateResult;
//! use ararajuba_provider::language_model::v4::stream_result::StreamResult;
//! use ararajuba_provider::errors::Error;
//!
//! struct MyModel { model_id: String }
//!
//! #[async_trait]
//! impl LanguageModelV4 for MyModel {
//!     fn provider(&self) -> &str { "my-provider" }
//!     fn model_id(&self) -> &str { &self.model_id }
//!
//!     async fn do_generate(&self, options: &CallOptions) -> Result<GenerateResult, Error> {
//!         // ... make HTTP call, parse response ...
//!         todo!()
//!     }
//!
//!     async fn do_stream(&self, options: &CallOptions) -> Result<StreamResult, Error> {
//!         // ... make HTTP call, return typed streams ...
//!         todo!()
//!     }
//! }
//! ```

use super::call_options::CallOptions;
use super::generate_result::GenerateResult;
use super::stream_result::StreamResult;
use crate::errors::Error;
use async_trait::async_trait;

/// The core v4 language model trait that all providers must implement.
///
/// Changes from v3:
/// - `async fn` instead of `BoxFuture` (via `#[async_trait]`)
/// - Returns v4 `StreamResult` with typed, split streams
/// - Specification version is `"v4"`
#[async_trait]
pub trait LanguageModelV4: Send + Sync {
    /// The specification version this model implements.
    fn specification_version(&self) -> &'static str {
        "v4"
    }

    /// The provider identifier (e.g., "openai.chat", "anthropic.chat").
    fn provider(&self) -> &str;

    /// The model identifier (e.g., "gpt-4o", "claude-sonnet-4-20250514").
    fn model_id(&self) -> &str;

    /// Generate a complete response (non-streaming).
    async fn do_generate(&self, options: &CallOptions) -> Result<GenerateResult, Error>;

    /// Generate a streaming response with typed, split streams.
    async fn do_stream(&self, options: &CallOptions) -> Result<StreamResult, Error>;
}
