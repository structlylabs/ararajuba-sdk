//! API key loading utilities.

use ararajuba_provider::errors::Error;

/// Load an API key from an explicit value or an environment variable.
///
/// # Arguments
/// * `api_key` - An explicitly provided API key.
/// * `env_var` - The environment variable to check if `api_key` is `None`.
/// * `description` - A description of the API key for error messages (e.g., "OpenAI").
///
/// # Errors
/// Returns `Error::LoadApiKey` if neither the explicit key nor the env var is set.
pub fn load_api_key(
    api_key: Option<String>,
    env_var: &str,
    description: &str,
) -> Result<String, Error> {
    if let Some(key) = api_key {
        if key.is_empty() {
            return Err(Error::LoadApiKey {
                message: format!(
                    "{description} API key is empty. Pass it using the 'api_key' option or the {env_var} environment variable."
                ),
            });
        }
        return Ok(key);
    }

    match std::env::var(env_var) {
        Ok(key) if !key.is_empty() => Ok(key),
        _ => Err(Error::LoadApiKey {
            message: format!(
                "{description} API key is missing. Pass it using the 'api_key' option or the {env_var} environment variable."
            ),
        }),
    }
}

/// Load an optional setting from an explicit value or an environment variable.
pub fn load_optional_setting(value: Option<String>, env_var: &str) -> Option<String> {
    if value.is_some() {
        return value;
    }
    std::env::var(env_var).ok().filter(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_api_key_explicit() {
        let key = load_api_key(Some("sk-test-key".to_string()), "FAKE_VAR", "Test").unwrap();
        assert_eq!(key, "sk-test-key");
    }

    #[test]
    fn test_load_api_key_empty_explicit_errors() {
        let result = load_api_key(Some(String::new()), "FAKE_VAR", "Test");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_api_key_missing_errors() {
        let result = load_api_key(None, "VERY_UNLIKELY_ENV_VAR_12345", "Test");
        assert!(result.is_err());
    }
}
