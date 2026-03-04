//! Data URL parsing and text extraction.

/// Extract the text content from a `data:` URL.
///
/// Supports both base64-encoded and plain text data URLs.
///
/// # Example
/// ```
/// use ararajuba_core::util::data_url::get_text_from_data_url;
///
/// let url = "data:text/plain;base64,SGVsbG8gV29ybGQ=";
/// let text = get_text_from_data_url(url).unwrap();
/// assert_eq!(text, "Hello World");
/// ```
pub fn get_text_from_data_url(url: &str) -> Result<String, DataUrlError> {
    let rest = url
        .strip_prefix("data:")
        .ok_or(DataUrlError::InvalidPrefix)?;

    let (header, data) = rest
        .split_once(',')
        .ok_or(DataUrlError::MissingComma)?;

    if header.ends_with(";base64") {
        // Base64-encoded data.
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(data)
            .map_err(|e| DataUrlError::Base64Decode(e.to_string()))?;
        String::from_utf8(bytes).map_err(|e| DataUrlError::Utf8Decode(e.to_string()))
    } else {
        // URL-encoded plain text.
        Ok(urlencoding::decode(data)
            .map_err(|e| DataUrlError::UrlDecode(e.to_string()))?
            .into_owned())
    }
}

/// Extract the MIME type from a data URL.
///
/// # Example
/// ```
/// use ararajuba_core::util::data_url::get_mime_type_from_data_url;
///
/// let mime = get_mime_type_from_data_url("data:image/png;base64,abc");
/// assert_eq!(mime, Some("image/png".to_string()));
/// ```
pub fn get_mime_type_from_data_url(url: &str) -> Option<String> {
    let rest = url.strip_prefix("data:")?;
    let (header, _) = rest.split_once(',')?;
    let mime = header.strip_suffix(";base64").unwrap_or(header);
    if mime.is_empty() {
        None
    } else {
        Some(mime.to_string())
    }
}

/// Errors from data URL operations.
#[derive(Debug, thiserror::Error)]
pub enum DataUrlError {
    #[error("Not a data URL (missing 'data:' prefix)")]
    InvalidPrefix,
    #[error("Missing comma separator in data URL")]
    MissingComma,
    #[error("Base64 decode error: {0}")]
    Base64Decode(String),
    #[error("UTF-8 decode error: {0}")]
    Utf8Decode(String),
    #[error("URL decode error: {0}")]
    UrlDecode(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_text() {
        let url = "data:text/plain;base64,SGVsbG8gV29ybGQ=";
        assert_eq!(get_text_from_data_url(url).unwrap(), "Hello World");
    }

    #[test]
    fn test_plain_text() {
        let url = "data:text/plain,Hello%20World";
        assert_eq!(get_text_from_data_url(url).unwrap(), "Hello World");
    }

    #[test]
    fn test_invalid_prefix() {
        assert!(get_text_from_data_url("https://example.com").is_err());
    }

    #[test]
    fn test_missing_comma() {
        assert!(get_text_from_data_url("data:text/plain").is_err());
    }

    #[test]
    fn test_get_mime_type() {
        assert_eq!(
            get_mime_type_from_data_url("data:image/png;base64,abc"),
            Some("image/png".to_string())
        );
    }

    #[test]
    fn test_get_mime_type_plain() {
        assert_eq!(
            get_mime_type_from_data_url("data:text/plain,hello"),
            Some("text/plain".to_string())
        );
    }

    #[test]
    fn test_empty_mime() {
        assert_eq!(get_mime_type_from_data_url("data:,hello"), None);
    }
}
