//! The LanguageModel trait — the core interface that providers implement.
//!
//! Uses `BoxFuture` from the `futures` crate directly (no Tokio, no async-trait)
//! to keep this spec/trait crate dependency-free.

use super::call_options::CallOptions;
use super::generate_result::GenerateResult;
use super::stream_result::StreamResult;
use crate::errors::Error;
use futures::future::BoxFuture;

/// The core language model trait that all providers must implement.
///
/// Providers return `BoxFuture` rather than using `async fn` in traits,
/// keeping this crate free of `async-trait` and Tokio dependencies.
///
/// # Example (provider implementation)
/// ```ignore
/// impl LanguageModel for MyModel {
///     fn provider(&self) -> &str { "my-provider" }
///     fn model_id(&self) -> &str { &self.model_id }
///
///     fn do_generate<'a>(
///         &'a self,
///         options: &'a CallOptions,
///     ) -> BoxFuture<'a, Result<GenerateResult, Error>> {
///         Box::pin(async move {
///             // ... make HTTP call, parse response ...
///             Ok(result)
///         })
///     }
///
///     fn do_stream<'a>(
///         &'a self,
///         options: &'a CallOptions,
///     ) -> BoxFuture<'a, Result<StreamResult, Error>> {
///         Box::pin(async move {
///             // ... make HTTP call, return stream ...
///             Ok(stream_result)
///         })
///     }
/// }
/// ```
pub trait LanguageModel: Send + Sync {
    /// The specification version this model implements.
    fn specification_version(&self) -> &'static str {
        "v3"
    }

    /// The provider identifier (e.g., "openai", "anthropic").
    fn provider(&self) -> &str;

    /// The model identifier (e.g., "gpt-4o", "claude-3-opus").
    fn model_id(&self) -> &str;

    /// Generate a complete response (non-streaming).
    fn do_generate<'a>(
        &'a self,
        options: &'a CallOptions,
    ) -> BoxFuture<'a, Result<GenerateResult, Error>>;

    /// Generate a streaming response.
    fn do_stream<'a>(
        &'a self,
        options: &'a CallOptions,
    ) -> BoxFuture<'a, Result<StreamResult, Error>>;
}
