//! All spec-level errors for the AI SDK provider layer.
//!
//! v4 adds the `Provider` variant for wrapping provider-specific errors
//! with support for downcasting to the concrete error type.

use thiserror::Error;

/// Errors that can occur at the AI SDK provider spec level.
#[derive(Debug, Error)]
pub enum Error {
    /// Error from an API call.
    #[error("API call error (status {status_code:?}): {message}")]
    ApiCallError {
        message: String,
        url: String,
        status_code: Option<u16>,
        response_body: Option<String>,
        is_retryable: bool,
        data: Option<serde_json::Value>,
    },

    /// Failed to parse JSON.
    #[error("JSON parse error: {message}")]
    JsonParse { message: String, text: String },

    /// Type validation failed.
    #[error("Type validation error: {message}")]
    TypeValidation {
        message: String,
        value: serde_json::Value,
    },

    /// Invalid prompt provided.
    #[error("Invalid prompt: {message}")]
    InvalidPrompt { message: String },

    /// Invalid argument.
    #[error("Invalid argument '{parameter}': {message}")]
    InvalidArgument { parameter: String, message: String },

    /// Model not found.
    #[error("No such model: {model_id} (type: {model_type})")]
    NoSuchModel {
        model_id: String,
        model_type: String,
    },

    /// Failed to load an API key.
    #[error("Load API key error: {message}")]
    LoadApiKey { message: String },

    /// Failed to load a setting.
    #[error("Load setting error: {message}")]
    LoadSetting { message: String },

    /// The provider does not support the requested functionality.
    #[error("Unsupported functionality: {functionality}")]
    UnsupportedFunctionality { functionality: String },

    /// The response body was empty.
    #[error("Empty response body")]
    EmptyResponseBody,

    /// No content was generated.
    #[error("No content generated")]
    NoContentGenerated,

    /// Invalid response data from the API.
    #[error("Invalid response data: {message}")]
    InvalidResponseData { message: String },

    /// Too many embedding values for a single call.
    #[error("Too many embedding values: max {max}, got {actual}")]
    TooManyEmbeddingValues { max: usize, actual: usize },

    /// HTTP transport error.
    #[error("HTTP error: {message}")]
    Http { message: String },

    /// Serialization / deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Provider-specific error (v4).
    ///
    /// Wraps a provider-specific error type that can be downcast to the
    /// concrete error type (e.g., `AnthropicError`, `OpenAIError`).
    ///
    /// # Example
    /// ```ignore
    /// match &error {
    ///     Error::Provider(pe) => {
    ///         if let Some(anthropic_err) = pe.downcast_ref::<AnthropicError>() {
    ///             match anthropic_err {
    ///                 AnthropicError::RateLimited { retry_after } => { /* ... */ }
    ///                 _ => {}
    ///             }
    ///         }
    ///     }
    ///     _ => {}
    /// }
    /// ```
    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),

    /// Generic / other error.
    #[error("{message}")]
    Other { message: String },
}

/// A type-erased provider-specific error with downcast support.
///
/// Providers create this by wrapping their concrete error type:
/// ```ignore
/// Error::Provider(ProviderError::new(AnthropicError::RateLimited { retry_after: Duration::from_secs(5) }))
/// ```
#[derive(Debug)]
pub struct ProviderError {
    inner: Box<dyn std::error::Error + Send + Sync>,
}

impl ProviderError {
    /// Create a new provider error wrapping any concrete error type.
    pub fn new<E: std::error::Error + Send + Sync + 'static>(error: E) -> Self {
        Self {
            inner: Box::new(error),
        }
    }

    /// Attempt to downcast to a concrete error type.
    pub fn downcast_ref<E: std::error::Error + 'static>(&self) -> Option<&E> {
        self.inner.downcast_ref()
    }

    /// Attempt to downcast to a concrete error type (consuming).
    pub fn downcast<E: std::error::Error + 'static>(
        self,
    ) -> Result<E, Self> {
        match self.inner.downcast::<E>() {
            Ok(e) => Ok(*e),
            Err(inner) => Err(Self { inner }),
        }
    }

    /// Get a reference to the inner error.
    pub fn inner(&self) -> &(dyn std::error::Error + Send + Sync) {
        &*self.inner
    }
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl std::error::Error for ProviderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.inner)
    }
}
