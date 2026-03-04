//! OpenAI-compatible error parsing with typed provider errors.

use ararajuba_provider::errors::{Error, ProviderError};
use serde_json::Value;
use std::time::Duration;

/// Typed error for OpenAI-compatible providers.
///
/// Can be extracted from `Error::Provider` via `downcast_ref`:
/// ```ignore
/// if let Error::Provider(pe) = &err {
///     if let Some(e) = pe.downcast_ref::<OpenAICompatibleError>() {
///         match e {
///             OpenAICompatibleError::RateLimited { retry_after } => { /* back off */ }
///             _ => {}
///         }
///     }
/// }
/// ```
#[derive(Debug, thiserror::Error)]
pub enum OpenAICompatibleError {
    /// Rate limited by the API. `retry_after` is the suggested wait time (if provided).
    #[error("Rate limited{}", .retry_after.map(|d| format!(" (retry after {}s)", d.as_secs())).unwrap_or_default())]
    RateLimited { retry_after: Option<Duration> },

    /// Quota / billing limit exceeded.
    #[error("Quota exceeded: {message}")]
    QuotaExceeded { message: String },

    /// Invalid or missing API key.
    #[error("Invalid API key: {message}")]
    InvalidApiKey { message: String },

    /// Content was filtered by the safety system.
    #[error("Content filtered: {reason}")]
    ContentFiltered { reason: String },

    /// Bad request (validation error).
    #[error("[{error_type}] {message}{}",
        .code.as_ref().map(|c| format!(" (code: {c})")).unwrap_or_default())]
    BadRequest {
        message: String,
        error_type: String,
        code: Option<String>,
    },

    /// Server error (5xx).
    #[error("Server error ({status}): {body}")]
    ServerError { status: u16, body: String },

    /// Unknown / unclassified error.
    #[error("{message}")]
    Other { message: String },
}

/// Parse an error response from an OpenAI-compatible API.
///
/// OpenAI error format:
/// ```json
/// {
///   "error": {
///     "message": "...",
///     "type": "...",
///     "code": "..."
///   }
/// }
/// ```
///
/// Returns `Error::Provider(ProviderError)` wrapping an [`OpenAICompatibleError`].
pub fn parse_openai_compatible_error(raw: Value) -> Error {
    parse_openai_compatible_error_with_status(raw, None)
}

/// Parse an error response with an optional HTTP status code for richer classification.
pub fn parse_openai_compatible_error_with_status(raw: Value, status_code: Option<u16>) -> Error {
    let error_obj = raw.get("error");

    let message = error_obj
        .and_then(|e| e.get("message"))
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown error")
        .to_string();

    let error_type = error_obj
        .and_then(|e| e.get("type"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown_error")
        .to_string();

    let error_code = error_obj
        .and_then(|e| e.get("code"))
        .and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            _ => None,
        });

    // Classify the error
    let typed_error = classify_openai_error(&message, &error_type, error_code.as_deref(), status_code);

    Error::Provider(ProviderError::new(typed_error))
}

fn classify_openai_error(
    message: &str,
    error_type: &str,
    error_code: Option<&str>,
    status_code: Option<u16>,
) -> OpenAICompatibleError {
    let msg_lower = message.to_lowercase();

    // Rate limiting (429)
    if status_code == Some(429)
        || error_type == "rate_limit_error"
        || error_code == Some("rate_limit_exceeded")
    {
        return OpenAICompatibleError::RateLimited { retry_after: None };
    }

    // Quota exceeded
    if error_code == Some("insufficient_quota")
        || error_code == Some("billing_hard_limit_reached")
        || msg_lower.contains("quota")
    {
        return OpenAICompatibleError::QuotaExceeded {
            message: message.to_string(),
        };
    }

    // Invalid API key (401)
    if status_code == Some(401)
        || error_code == Some("invalid_api_key")
        || error_type == "authentication_error"
    {
        return OpenAICompatibleError::InvalidApiKey {
            message: message.to_string(),
        };
    }

    // Content filtering
    if error_code == Some("content_filter")
        || error_code == Some("content_policy_violation")
        || msg_lower.contains("content filter")
        || msg_lower.contains("content policy")
    {
        return OpenAICompatibleError::ContentFiltered {
            reason: message.to_string(),
        };
    }

    // Server errors (5xx)
    if let Some(status) = status_code {
        if status >= 500 {
            return OpenAICompatibleError::ServerError {
                status,
                body: message.to_string(),
            };
        }
    }

    // Default: bad request or other
    if error_type != "unknown_error" {
        OpenAICompatibleError::BadRequest {
            message: message.to_string(),
            error_type: error_type.to_string(),
            code: error_code.map(|s| s.to_string()),
        }
    } else {
        OpenAICompatibleError::Other {
            message: message.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_standard_error() {
        let raw = json!({
            "error": {
                "message": "Invalid API key",
                "type": "invalid_request_error",
                "code": "invalid_api_key"
            }
        });
        let err = parse_openai_compatible_error(raw);
        match &err {
            Error::Provider(pe) => {
                let typed = pe.downcast_ref::<OpenAICompatibleError>().unwrap();
                assert!(matches!(typed, OpenAICompatibleError::InvalidApiKey { .. }));
            }
            _ => panic!("Expected Provider error"),
        }
    }

    #[test]
    fn test_parse_rate_limit_error() {
        let raw = json!({
            "error": {
                "message": "Rate limit exceeded",
                "type": "rate_limit_error",
                "code": "rate_limit_exceeded"
            }
        });
        let err = parse_openai_compatible_error(raw);
        match &err {
            Error::Provider(pe) => {
                let typed = pe.downcast_ref::<OpenAICompatibleError>().unwrap();
                assert!(matches!(typed, OpenAICompatibleError::RateLimited { .. }));
            }
            _ => panic!("Expected Provider error"),
        }
    }

    #[test]
    fn test_parse_minimal_error() {
        let raw = json!({});
        let err = parse_openai_compatible_error(raw);
        match &err {
            Error::Provider(pe) => {
                let typed = pe.downcast_ref::<OpenAICompatibleError>().unwrap();
                assert!(matches!(typed, OpenAICompatibleError::Other { .. }));
            }
            _ => panic!("Expected Provider error"),
        }
    }

    #[test]
    fn test_parse_content_filter_error() {
        let raw = json!({
            "error": {
                "message": "Content policy violation detected",
                "type": "invalid_request_error",
                "code": "content_policy_violation"
            }
        });
        let err = parse_openai_compatible_error(raw);
        match &err {
            Error::Provider(pe) => {
                let typed = pe.downcast_ref::<OpenAICompatibleError>().unwrap();
                assert!(matches!(typed, OpenAICompatibleError::ContentFiltered { .. }));
            }
            _ => panic!("Expected Provider error"),
        }
    }

    #[test]
    fn test_parse_server_error_with_status() {
        let raw = json!({
            "error": {
                "message": "Internal server error",
                "type": "server_error"
            }
        });
        let err = parse_openai_compatible_error_with_status(raw, Some(500));
        match &err {
            Error::Provider(pe) => {
                let typed = pe.downcast_ref::<OpenAICompatibleError>().unwrap();
                assert!(matches!(typed, OpenAICompatibleError::ServerError { status: 500, .. }));
            }
            _ => panic!("Expected Provider error"),
        }
    }
}
