//! Google Generative AI error parsing with typed provider errors.

use ararajuba_provider::errors::{Error, ProviderError};
use serde_json::Value;

/// Typed error for the Google Generative AI provider.
///
/// Can be extracted from `Error::Provider` via `downcast_ref`:
/// ```ignore
/// if let Error::Provider(pe) = &err {
///     if let Some(e) = pe.downcast_ref::<GoogleError>() {
///         match e {
///             GoogleError::RateLimited => { /* back off */ }
///             GoogleError::QuotaExceeded { message } => { /* check billing */ }
///             _ => {}
///         }
///     }
/// }
/// ```
#[derive(Debug, thiserror::Error)]
pub enum GoogleError {
    /// Rate limited (429 / RESOURCE_EXHAUSTED).
    #[error("Rate limited")]
    RateLimited,

    /// Quota exceeded / billing limit.
    #[error("Quota exceeded: {message}")]
    QuotaExceeded { message: String },

    /// Invalid or missing API key.
    #[error("Invalid API key: {message}")]
    InvalidApiKey { message: String },

    /// Model not found.
    #[error("Model not found: {model}")]
    NotFound { model: String },

    /// Bad request (validation error).
    #[error("[{status}] {message}")]
    BadRequest {
        status: String,
        code: u64,
        message: String,
    },

    /// Server error (5xx / INTERNAL).
    #[error("Server error ({code}): {message}")]
    ServerError { code: u64, message: String },

    /// Unknown / unclassified error.
    #[error("[{status} ({code})] {message}")]
    Other {
        status: String,
        code: u64,
        message: String,
    },
}

/// Parse a Google API error response.
///
/// Google errors follow the format:
/// ```json
/// { "error": { "code": 400, "message": "...", "status": "INVALID_ARGUMENT" } }
/// ```
///
/// Returns `Error::Provider(ProviderError)` wrapping a [`GoogleError`].
pub fn parse_google_error(data: Value) -> Error {
    let error_obj = data.get("error").unwrap_or(&data);
    let code = error_obj
        .get("code")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let message = error_obj
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown error")
        .to_string();
    let status = error_obj
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("UNKNOWN")
        .to_string();

    let typed_error = classify_google_error(&message, &status, code);
    Error::Provider(ProviderError::new(typed_error))
}

fn classify_google_error(message: &str, status: &str, code: u64) -> GoogleError {
    // Rate limiting (429 / RESOURCE_EXHAUSTED)
    if code == 429 || status == "RESOURCE_EXHAUSTED" {
        let msg_lower = message.to_lowercase();
        if msg_lower.contains("quota") || msg_lower.contains("billing") {
            return GoogleError::QuotaExceeded {
                message: message.to_string(),
            };
        }
        return GoogleError::RateLimited;
    }

    // Authentication (401, 403 / UNAUTHENTICATED / PERMISSION_DENIED)
    if code == 401
        || code == 403
        || status == "UNAUTHENTICATED"
        || status == "PERMISSION_DENIED"
    {
        return GoogleError::InvalidApiKey {
            message: message.to_string(),
        };
    }

    // Not found (404 / NOT_FOUND)
    if code == 404 || status == "NOT_FOUND" {
        return GoogleError::NotFound {
            model: message.to_string(),
        };
    }

    // Server errors (5xx / INTERNAL / UNAVAILABLE)
    if code >= 500 || status == "INTERNAL" || status == "UNAVAILABLE" {
        return GoogleError::ServerError {
            code,
            message: message.to_string(),
        };
    }

    // Bad request (400 / INVALID_ARGUMENT / FAILED_PRECONDITION)
    if code == 400 || status == "INVALID_ARGUMENT" || status == "FAILED_PRECONDITION" {
        return GoogleError::BadRequest {
            status: status.to_string(),
            code,
            message: message.to_string(),
        };
    }

    // Default
    GoogleError::Other {
        status: status.to_string(),
        code,
        message: message.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_google_error() {
        let data = json!({
            "error": {
                "code": 400,
                "message": "API key not valid.",
                "status": "INVALID_ARGUMENT"
            }
        });
        let err = parse_google_error(data);
        match &err {
            Error::Provider(pe) => {
                let typed = pe.downcast_ref::<GoogleError>().unwrap();
                assert!(matches!(typed, GoogleError::BadRequest { .. }));
            }
            _ => panic!("Expected Provider error"),
        }
    }

    #[test]
    fn test_parse_auth_error() {
        let data = json!({
            "error": {
                "code": 401,
                "message": "Request had invalid authentication credentials.",
                "status": "UNAUTHENTICATED"
            }
        });
        let err = parse_google_error(data);
        match &err {
            Error::Provider(pe) => {
                let typed = pe.downcast_ref::<GoogleError>().unwrap();
                assert!(matches!(typed, GoogleError::InvalidApiKey { .. }));
            }
            _ => panic!("Expected Provider error"),
        }
    }

    #[test]
    fn test_parse_rate_limit_error() {
        let data = json!({
            "error": {
                "code": 429,
                "message": "Too many requests",
                "status": "RESOURCE_EXHAUSTED"
            }
        });
        let err = parse_google_error(data);
        match &err {
            Error::Provider(pe) => {
                let typed = pe.downcast_ref::<GoogleError>().unwrap();
                assert!(matches!(typed, GoogleError::RateLimited));
            }
            _ => panic!("Expected Provider error"),
        }
    }

    #[test]
    fn test_parse_empty_error() {
        let data = json!({});
        let err = parse_google_error(data);
        match &err {
            Error::Provider(pe) => {
                let typed = pe.downcast_ref::<GoogleError>().unwrap();
                assert!(matches!(typed, GoogleError::Other { .. }));
            }
            _ => panic!("Expected Provider error"),
        }
    }
}
