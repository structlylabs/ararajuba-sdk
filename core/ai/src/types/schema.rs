//! Schema wrapper utilities — `json_schema()` and `as_schema()`.
//!
//! Mirrors the TS SDK's `jsonSchema()` and `asSchema()`. In Rust, these wrap
//! a `serde_json::Value` JSON Schema with optional validation.

use serde_json::Value;
use std::sync::Arc;

/// Result of validating an input against a schema.
#[derive(Debug, Clone)]
pub enum ValidationResult {
    /// The value passed validation.
    Valid { value: Value },
    /// The value failed validation.
    Invalid { error: String },
}

/// A validate function that checks whether a value conforms to a schema.
pub type ValidateFn = Arc<dyn Fn(&Value) -> ValidationResult + Send + Sync>;

/// A schema wrapper around a JSON Schema definition.
///
/// Equivalent to the TS SDK's `Schema<OBJECT>` type.
#[derive(Clone)]
pub struct Schema {
    /// The raw JSON Schema object.
    pub json_schema: Value,
    /// Optional validation function.
    pub validate: Option<ValidateFn>,
}

impl std::fmt::Debug for Schema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Schema")
            .field("json_schema", &self.json_schema)
            .field("has_validate", &self.validate.is_some())
            .finish()
    }
}

/// Create a `Schema` wrapper from a raw JSON Schema value.
///
/// Equivalent to the TS SDK's `jsonSchema()`.
///
/// # Examples
/// ```
/// use serde_json::json;
/// use ararajuba_core::json_schema;
///
/// let schema = json_schema(json!({
///     "type": "object",
///     "properties": {
///         "name": { "type": "string" }
///     }
/// }), None);
/// assert!(schema.validate.is_none());
/// ```
pub fn json_schema(schema: Value, validate: Option<ValidateFn>) -> Schema {
    Schema {
        json_schema: schema,
        validate,
    }
}

/// Normalize a `Value` that represents a JSON Schema into a `Schema`.
///
/// If the value is already a `Schema`-like object, treat it as one;
/// otherwise wrap it. This mirrors the TS SDK's `asSchema()`.
pub fn as_schema(schema_value: Value) -> Schema {
    Schema {
        json_schema: schema_value,
        validate: None,
    }
}

impl Schema {
    /// Validate a value against this schema's validate function (if any).
    ///
    /// Returns `Valid` if there is no validate function (permissive by default).
    pub fn validate(&self, value: &Value) -> ValidationResult {
        match &self.validate {
            Some(f) => f(value),
            None => ValidationResult::Valid {
                value: value.clone(),
            },
        }
    }

    /// Returns `true` if this schema has a validation function.
    pub fn has_validate(&self) -> bool {
        self.validate.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_json_schema_no_validate() {
        let s = json_schema(
            json!({
                "type": "object",
                "properties": { "name": { "type": "string" } }
            }),
            None,
        );
        assert!(!s.has_validate());
        assert!(matches!(s.validate(&json!({"name": "test"})), ValidationResult::Valid { .. }));
    }

    #[test]
    fn test_json_schema_with_validate() {
        let s = json_schema(
            json!({"type": "string"}),
            Some(Arc::new(|v: &Value| {
                if v.is_string() {
                    ValidationResult::Valid { value: v.clone() }
                } else {
                    ValidationResult::Invalid {
                        error: "not a string".into(),
                    }
                }
            })),
        );
        assert!(s.has_validate());
        assert!(matches!(
            s.validate(&json!("hello")),
            ValidationResult::Valid { .. }
        ));
        assert!(matches!(
            s.validate(&json!(42)),
            ValidationResult::Invalid { .. }
        ));
    }

    #[test]
    fn test_as_schema() {
        let s = as_schema(json!({"type": "number"}));
        assert_eq!(s.json_schema, json!({"type": "number"}));
        assert!(!s.has_validate());
    }

    #[test]
    fn test_schema_debug_format() {
        let s = json_schema(json!({"type": "boolean"}), None);
        let debug = format!("{:?}", s);
        assert!(debug.contains("json_schema"));
        assert!(debug.contains("has_validate: false"));
    }
}
