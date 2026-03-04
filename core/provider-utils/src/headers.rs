//! Header combination and normalization utilities.

use std::collections::HashMap;

/// Combine multiple optional header maps into a single map.
/// Later maps override earlier maps for the same key.
pub fn combine_headers(
    header_maps: Vec<Option<HashMap<String, String>>>,
) -> HashMap<String, String> {
    let mut result = HashMap::new();
    for map in header_maps.into_iter().flatten() {
        for (k, v) in map {
            if v.is_empty() {
                result.remove(&k);
            } else {
                result.insert(k, v);
            }
        }
    }
    result
}

/// Normalize header keys to lowercase.
pub fn normalize_headers(headers: HashMap<String, String>) -> HashMap<String, String> {
    headers
        .into_iter()
        .map(|(k, v)| (k.to_lowercase(), v))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_combine_headers_overrides() {
        let h1 = Some(HashMap::from([
            ("Authorization".to_string(), "Bearer old".to_string()),
            ("X-Custom".to_string(), "value".to_string()),
        ]));
        let h2 = Some(HashMap::from([(
            "Authorization".to_string(),
            "Bearer new".to_string(),
        )]));

        let result = combine_headers(vec![h1, h2]);
        assert_eq!(result.get("Authorization").unwrap(), "Bearer new");
        assert_eq!(result.get("X-Custom").unwrap(), "value");
    }

    #[test]
    fn test_combine_headers_empty_removes() {
        let h1 = Some(HashMap::from([(
            "X-Remove".to_string(),
            "value".to_string(),
        )]));
        let h2 = Some(HashMap::from([(
            "X-Remove".to_string(),
            String::new(),
        )]));

        let result = combine_headers(vec![h1, h2]);
        assert!(!result.contains_key("X-Remove"));
    }

    #[test]
    fn test_normalize_headers() {
        let headers = HashMap::from([
            ("Content-Type".to_string(), "application/json".to_string()),
            ("X-API-KEY".to_string(), "secret".to_string()),
        ]);
        let normalized = normalize_headers(headers);
        assert!(normalized.contains_key("content-type"));
        assert!(normalized.contains_key("x-api-key"));
    }
}
