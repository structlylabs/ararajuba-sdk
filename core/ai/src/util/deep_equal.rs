//! Deep equality comparison for JSON values.
//!
//! Provides a more robust equality check than `==` for `serde_json::Value`,
//! handling floating-point comparisons and nested structures.

use serde_json::Value;

/// Check if two JSON values are deeply equal.
///
/// This handles object key ordering (serde_json's Map is already ordered by key),
/// array ordering, and nested structures.
///
/// # Example
/// ```
/// use ararajuba_core::util::deep_equal::is_deep_equal_data;
/// use serde_json::json;
///
/// assert!(is_deep_equal_data(&json!({"a": 1}), &json!({"a": 1})));
/// assert!(!is_deep_equal_data(&json!({"a": 1}), &json!({"a": 2})));
/// ```
pub fn is_deep_equal_data(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Number(a), Value::Number(b)) => {
            // Compare as f64 to handle integer/float differences.
            match (a.as_f64(), b.as_f64()) {
                (Some(fa), Some(fb)) => (fa - fb).abs() < f64::EPSILON,
                _ => a == b,
            }
        }
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Array(a), Value::Array(b)) => {
            if a.len() != b.len() {
                return false;
            }
            a.iter().zip(b.iter()).all(|(ai, bi)| is_deep_equal_data(ai, bi))
        }
        (Value::Object(a), Value::Object(b)) => {
            if a.len() != b.len() {
                return false;
            }
            a.iter().all(|(key, val)| {
                b.get(key)
                    .map(|bval| is_deep_equal_data(val, bval))
                    .unwrap_or(false)
            })
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_null_equality() {
        assert!(is_deep_equal_data(&json!(null), &json!(null)));
    }

    #[test]
    fn test_bool_equality() {
        assert!(is_deep_equal_data(&json!(true), &json!(true)));
        assert!(!is_deep_equal_data(&json!(true), &json!(false)));
    }

    #[test]
    fn test_number_equality() {
        assert!(is_deep_equal_data(&json!(42), &json!(42)));
        assert!(is_deep_equal_data(&json!(3.14), &json!(3.14)));
        assert!(!is_deep_equal_data(&json!(1), &json!(2)));
    }

    #[test]
    fn test_string_equality() {
        assert!(is_deep_equal_data(&json!("hello"), &json!("hello")));
        assert!(!is_deep_equal_data(&json!("hello"), &json!("world")));
    }

    #[test]
    fn test_array_equality() {
        assert!(is_deep_equal_data(&json!([1, 2, 3]), &json!([1, 2, 3])));
        assert!(!is_deep_equal_data(&json!([1, 2, 3]), &json!([1, 2])));
        assert!(!is_deep_equal_data(&json!([1, 2, 3]), &json!([1, 3, 2])));
    }

    #[test]
    fn test_object_equality() {
        assert!(is_deep_equal_data(
            &json!({"a": 1, "b": "x"}),
            &json!({"a": 1, "b": "x"})
        ));
        assert!(!is_deep_equal_data(
            &json!({"a": 1}),
            &json!({"a": 2})
        ));
    }

    #[test]
    fn test_nested_equality() {
        let a = json!({"users": [{"name": "Alice", "age": 30}]});
        let b = json!({"users": [{"name": "Alice", "age": 30}]});
        assert!(is_deep_equal_data(&a, &b));
    }

    #[test]
    fn test_nested_inequality() {
        let a = json!({"users": [{"name": "Alice", "age": 30}]});
        let b = json!({"users": [{"name": "Alice", "age": 31}]});
        assert!(!is_deep_equal_data(&a, &b));
    }

    #[test]
    fn test_type_mismatch() {
        assert!(!is_deep_equal_data(&json!(1), &json!("1")));
        assert!(!is_deep_equal_data(&json!(null), &json!(false)));
    }

    #[test]
    fn test_extra_key() {
        assert!(!is_deep_equal_data(
            &json!({"a": 1}),
            &json!({"a": 1, "b": 2})
        ));
    }
}
