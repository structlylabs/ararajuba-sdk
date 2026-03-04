//! Incomplete JSON repair utilities.
//!
//! Attempts to fix common issues with incomplete JSON strings
//! (e.g., from partial streaming responses).

/// Attempt to repair an incomplete JSON string by closing unclosed
/// brackets, braces, and strings.
pub fn repair_incomplete_json(text: &str) -> String {
    let mut result = text.to_string();
    let mut open_braces = 0i32;
    let mut open_brackets = 0i32;
    let mut in_string = false;
    let mut escape_next = false;

    for ch in text.chars() {
        if escape_next {
            escape_next = false;
            continue;
        }

        match ch {
            '\\' if in_string => {
                escape_next = true;
            }
            '"' => {
                in_string = !in_string;
            }
            '{' if !in_string => {
                open_braces += 1;
            }
            '}' if !in_string => {
                open_braces -= 1;
            }
            '[' if !in_string => {
                open_brackets += 1;
            }
            ']' if !in_string => {
                open_brackets -= 1;
            }
            _ => {}
        }
    }

    // Close any open string
    if in_string {
        result.push('"');
    }

    // Close any open brackets/braces
    for _ in 0..open_brackets {
        result.push(']');
    }
    for _ in 0..open_braces {
        result.push('}');
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repair_incomplete_object() {
        let repaired = repair_incomplete_json(r#"{"key": "value"#);
        assert!(repaired.ends_with("\"}"));
    }

    #[test]
    fn test_repair_incomplete_array() {
        let repaired = repair_incomplete_json(r#"[1, 2, 3"#);
        assert!(repaired.ends_with(']'));
    }

    #[test]
    fn test_repair_complete_json() {
        let json = r#"{"key": "value"}"#;
        let repaired = repair_incomplete_json(json);
        assert_eq!(repaired, json);
    }
}
