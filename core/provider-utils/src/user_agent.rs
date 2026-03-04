//! User-agent utilities.

/// The default user agent string for the AI SDK.
pub const DEFAULT_USER_AGENT: &str = "ai-sdk-rust/0.1.0";

/// Build a user agent string with an optional suffix.
pub fn with_user_agent_suffix(suffix: Option<&str>) -> String {
    match suffix {
        Some(s) => format!("{DEFAULT_USER_AGENT} {s}"),
        None => DEFAULT_USER_AGENT.to_string(),
    }
}
