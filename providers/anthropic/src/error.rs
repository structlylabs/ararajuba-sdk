//! Anthropic API error parsing with typed provider errors.

use ararajuba_provider::errors::{Error, ProviderError};
use serde_json::Value;
use std::time::Duration;

/// Typed error for the Anthropic provider.
///
/// Can be extracted from `Error::Provider` via `downcast_ref`:
/// ```ignore
/// if let Error::Provider(pe) = &err {
///     if let Some(e) = pe.downcast_ref::<AnthropicError>() {
///         match e {
///             AnthropicError::RateLimited { retry_after } => { /* back off */ }
///             AnthropicError::Overloaded => { /* retry later */ }
///             _ => {}
///         }
///     }
/// }
/// ```
#[derive(Debug, thiserror::Error)]
pub enum AnthropicError {
    /// Rate limited by the API.
    #[error("Rate limited{}", .retry_after.map(|d| format!(" (retry after {}s)", d.as_secs())).unwrap_or_default())]
    RateLimited { retry_after: Option<Duration> },

    /// The API is overloaded (529 status).
    #[error("Anthropic API overloaded")]
    Overloaded,

    /// Invalid or missing API key.
    #[error("Invalid API key: {message}")]
    InvalidApiKey { message: String },

    /// Content was filtered by the safety system.
    #[error("Content filtered: {reason}")]
    ContentFiltered { reason: String },

    /// Bad request (validation error).
    #[error("[{error_type}] {message}")]
    BadRequest { message: String, error_type: String },

    /// Server error (5xx).
    #[error("Server error: {message}")]
    ServerError { status: u16, message: String },

    /// Unknown / unclassified error.
    #[error("[{error_type}] {message}")]
    Other { error_type: String, message: String },
}

/// Parse an Anthropic error response.
///
/// Anthropic errors follow the format:
/// ```json
/// {
///   "type": "error",
///   "error": {
///     "type": "invalid_request_error",
///     "message": "..."
///   }
/// }
/// ```
///
/// Returns `Error::Provider(ProviderError)` wrapping an [`AnthropicError`].
pub fn parse_anthropic_error(data: Value) -> Error {
    parse_anthropic_error_with_status(data, None)
}

/// Parse an Anthropic error response with optional HTTP status code.
pub fn parse_anthropic_error_with_status(data: Value, status_code: Option<u16>) -> Error {
    let error_obj = data.get("error").unwrap_or(&data);
    let error_type = error_obj
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown_error")
        .to_string();
    let message = error_obj
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown error")
        .to_string();

    let typed_error = classify_anthropic_error(&message, &error_type, status_code);
    Error::Provider(ProviderError::new(typed_error))
}

fn classify_anthropic_error(
    message: &str,
    error_type: &str,
    status_code: Option<u16>,
) -> AnthropicError {
    // Overloaded (529)
    if status_code == Some(529) || error_type == "overloaded_error" {
        return AnthropicError::Overloaded;
    }

    // Rate limiting (429)
    if status_code == Some(429) || error_type == "rate_limit_error" {
        return AnthropicError::RateLimited { retry_after: None };
    }

    // Authentication error (401)
    if status_code == Some(401) || error_type == "authentication_error" {
        return AnthropicError::InvalidApiKey {
            message: message.to_string(),
        };
    }

    // Content filtering
    let msg_lower = message.to_lowercase();
    if msg_lower.contains("content filter")
        || msg_lower.contains("safety")
        || msg_lower.contains("harmful")
    {
        return AnthropicError::ContentFiltered {
            reason: message.to_string(),
        };
    }

    // Server errors (5xx)
    if let Some(status) = status_code {
        if status >= 500 {
            return AnthropicError::ServerError {
                status,
                message: message.to_string(),
            };
        }
    }

    // Bad request
    if error_type == "invalid_request_error" || error_type == "not_found_error" {
        return AnthropicError::BadRequest {
            message: message.to_string(),
            error_type: error_type.to_string(),
        };
    }

    // Default
    AnthropicError::Other {
        error_type: error_type.to_string(),
        message: message.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_standard_error() {
        let data = json!({
            "type": "error",
            "error": {
                "type": "invalid_request_error",
                "message": "max_tokens: 100000 > 8192, which is the maximum for model"
            }
        });

        let err = parse_anthropic_error(data);
        match &err {
            Error::Provider(pe) => {
                let typed = pe.downcast_ref::<AnthropicError>().unwrap();
                assert!(matches!(typed, AnthropicError::BadRequest { .. }));
            }
            _ => panic!("Expected Provider error"),
        }
    }

    #[test]
    fn test_parse_overloaded_error() {
        let data = json!({
            "type": "error",
            "error": {
                "type": "overloaded_error",
                "message": "Overloaded"
            }
        });

        let err = parse_anthropic_error(data);
        match &err {
            Error::Provider(pe) => {
                let typed = pe.downcast_ref::<AnthropicError>().unwrap();
                assert!(matches!(typed, AnthropicError::Overloaded));
            }
            _ => panic!("Expected Provider error"),
        }
    }

    #[test]
    fn test_parse_rate_limit_error() {
        let data = json!({
            "type": "error",
            "error": {
                "type": "rate_limit_error",
                "message": "Too many requests"
            }
        });

        let err = parse_anthropic_error(data);
        match &err {
            Error::Provider(pe) => {
                let typed = pe.downcast_ref::<AnthropicError>().unwrap();
                assert!(matches!(typed, AnthropicError::RateLimited { .. }));
            }
            _ => panic!("Expected Provider error"),
        }
    }

    #[test]
    fn test_parse_empty_error() {
        let data = json!({});
        let err = parse_anthropic_error(data);
        match &err {
            Error::Provider(pe) => {
                let typed = pe.downcast_ref::<AnthropicError>().unwrap();
                assert!(matches!(typed, AnthropicError::Other { .. }));
            }
            _ => panic!("Expected Provider error"),
        }
    }
}
