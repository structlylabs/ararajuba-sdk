//! JSON parsing utilities with safe error handling.

use ararajuba_provider::errors::Error;
use serde::de::DeserializeOwned;

/// Parse a JSON string, returning an `Error::JsonParse` on failure.
pub fn parse_json<T: DeserializeOwned>(text: &str) -> Result<T, Error> {
    serde_json::from_str(text).map_err(|e| Error::JsonParse {
        message: e.to_string(),
        text: text.to_string(),
    })
}

/// Safely parse a JSON string, returning `None` on failure instead of an error.
pub fn safe_parse_json<T: DeserializeOwned>(text: &str) -> Option<T> {
    serde_json::from_str(text).ok()
}

/// Parse a JSON Value into a typed structure.
pub fn parse_json_value<T: DeserializeOwned>(value: serde_json::Value) -> Result<T, Error> {
    serde_json::from_value(value.clone()).map_err(|e| Error::JsonParse {
        message: e.to_string(),
        text: value.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_valid() {
        let result: serde_json::Value = parse_json(r#"{"key": "value"}"#).unwrap();
        assert_eq!(result["key"], "value");
    }

    #[test]
    fn test_parse_json_invalid() {
        let result: Result<serde_json::Value, _> = parse_json("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_safe_parse_json_returns_none() {
        let result: Option<serde_json::Value> = safe_parse_json("not json");
        assert!(result.is_none());
    }
}
