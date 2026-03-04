//! ID generation utilities.

use uuid::Uuid;

/// Generate a random alphanumeric ID (default 16 chars).
pub fn generate_id() -> String {
    let uuid = Uuid::new_v4();
    // Take the first 16 hex chars (no hyphens).
    uuid.simple().to_string()[..16].to_string()
}

/// Create an ID generator function with optional prefix and size.
pub fn create_id_generator(
    prefix: Option<&str>,
    size: Option<usize>,
) -> Box<dyn Fn() -> String + Send + Sync> {
    let prefix = prefix.unwrap_or("").to_string();
    let size = size.unwrap_or(16);

    Box::new(move || {
        let uuid = Uuid::new_v4();
        let hex = uuid.simple().to_string();
        let id_part = if size <= hex.len() {
            &hex[..size]
        } else {
            &hex
        };
        format!("{prefix}{id_part}")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_id_length() {
        let id = generate_id();
        assert_eq!(id.len(), 16);
    }

    #[test]
    fn test_generate_id_unique() {
        let id1 = generate_id();
        let id2 = generate_id();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_create_id_generator_with_prefix() {
        let id_gen = create_id_generator(Some("msg-"), Some(8));
        let id = id_gen();
        assert!(id.starts_with("msg-"));
        assert_eq!(id.len(), 12); // 4 prefix + 8 id
    }
}
