//! Parse incomplete/streaming JSON — useful for partial object streaming.

use serde_json::Value;

/// Attempt to parse potentially incomplete JSON by closing any open
/// structures (objects, arrays, strings).
///
/// Returns `None` if the input cannot be repaired into valid JSON.
///
/// # Example
/// ```
/// use ararajuba_core::util::parse_partial_json::parse_partial_json;
///
/// let result = parse_partial_json(r#"{"name": "Al"#);
/// assert!(result.is_some());
/// ```
pub fn parse_partial_json(input: &str) -> Option<Value> {
    // First try parsing as-is.
    if let Ok(v) = serde_json::from_str::<Value>(input) {
        return Some(v);
    }

    // Try to close open structures.
    let repaired = close_json(input);
    serde_json::from_str::<Value>(&repaired).ok()
}

/// Close any open JSON structures by appending closing brackets/braces.
fn close_json(input: &str) -> String {
    let mut result = String::from(input.trim());

    // Track structural state
    let mut stack: Vec<char> = Vec::new();
    let mut in_string = false;
    let mut escape_next = false;

    for ch in result.chars() {
        if escape_next {
            escape_next = false;
            continue;
        }

        if ch == '\\' && in_string {
            escape_next = true;
            continue;
        }

        if ch == '"' {
            in_string = !in_string;
            continue;
        }

        if in_string {
            continue;
        }

        match ch {
            '{' => stack.push('}'),
            '[' => stack.push(']'),
            '}' | ']' => {
                stack.pop();
            }
            _ => {}
        }
    }

    // If we're inside an unterminated string, close it.
    if in_string {
        result.push('"');
    }

    // Remove trailing commas before closing.
    let trimmed = result.trim_end();
    if trimmed.ends_with(',') {
        result = trimmed[..trimmed.len() - 1].to_string();
    }

    // Close remaining open structures in reverse order.
    while let Some(closer) = stack.pop() {
        result.push(closer);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complete_json() {
        let v = parse_partial_json(r#"{"name": "Alice"}"#).unwrap();
        assert_eq!(v["name"], "Alice");
    }

    #[test]
    fn test_incomplete_string() {
        let v = parse_partial_json(r#"{"name": "Ali"#).unwrap();
        assert_eq!(v["name"], "Ali");
    }

    #[test]
    fn test_incomplete_object() {
        let v = parse_partial_json(r#"{"name": "Alice""#).unwrap();
        assert_eq!(v["name"], "Alice");
    }

    #[test]
    fn test_incomplete_array() {
        let v = parse_partial_json(r#"[1, 2, 3"#).unwrap();
        let arr = v.as_array().unwrap();
        assert_eq!(arr.len(), 3);
    }

    #[test]
    fn test_nested_incomplete() {
        let v = parse_partial_json(r#"{"items": [{"id": 1}, {"id": 2"#).unwrap();
        let items = v["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_trailing_comma_object() {
        let v = parse_partial_json(r#"{"a": 1,"#).unwrap();
        assert_eq!(v["a"], 1);
    }

    #[test]
    fn test_trailing_comma_array() {
        let v = parse_partial_json(r#"[1, 2,"#).unwrap();
        let arr = v.as_array().unwrap();
        assert_eq!(arr.len(), 2);
    }

    #[test]
    fn test_empty_string() {
        assert!(parse_partial_json("").is_none());
    }

    #[test]
    fn test_just_opening_brace() {
        let v = parse_partial_json("{").unwrap();
        assert!(v.is_object());
    }

    #[test]
    fn test_just_opening_bracket() {
        let v = parse_partial_json("[").unwrap();
        assert!(v.is_array());
    }
}
