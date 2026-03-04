//! SDK-level errors wrapping the provider error type.
//!
//! Each variant maps to a specific error class in the Vercel AI SDK,
//! enabling callers to match on fine-grained error conditions.

use thiserror::Error as ThisError;

/// Errors from high-level SDK functions.
#[derive(Debug, ThisError)]
pub enum Error {
    // ── Provider / transport ─────────────────────────────────────────────

    /// An error from the provider layer.
    #[error(transparent)]
    Provider(#[from] ararajuba_provider::errors::Error),

    /// No provider registered with the given prefix.
    #[error("No provider registered with prefix '{provider_id}'")]
    NoSuchProvider { provider_id: String },

    /// A retry budget was exhausted.
    #[error("Retry failed after {attempts} attempts: {message}")]
    Retry { attempts: usize, message: String },

    /// A download (e.g. fetching a URL for an image/file) failed.
    #[error("Download error: {url} — {message}")]
    Download { url: String, message: String },

    // ── Generation content ───────────────────────────────────────────────

    /// The model returned no text content.
    #[error("No content generated (finish reason: {finish_reason})")]
    NoContentGenerated { finish_reason: String },

    /// The model did not produce a valid structured object.
    #[error("No object generated: {message}")]
    NoObjectGenerated { message: String },

    /// The model did not produce an image.
    #[error("No image generated")]
    NoImageGenerated,

    /// The model did not produce speech audio.
    #[error("No speech generated")]
    NoSpeechGenerated,

    /// The model did not produce a transcript.
    #[error("No transcript generated")]
    NoTranscriptGenerated,

    /// The model did not produce a video.
    #[error("No video generated")]
    NoVideoGenerated,

    // ── Tool loop errors ─────────────────────────────────────────────────

    /// Maximum number of steps exceeded in tool loop.
    #[error("Maximum number of steps ({max_steps}) exceeded")]
    MaxStepsExceeded { max_steps: usize },

    /// Tool not found in the tool set.
    #[error("Tool not found: {tool_name}")]
    ToolNotFound { tool_name: String },

    /// Tool execution failed.
    #[error("Tool execution error for '{tool_name}': {message}")]
    ToolExecutionError { tool_name: String, message: String },

    /// The tool input failed validation against the tool's schema.
    #[error("Invalid tool input for '{tool_name}': {message}")]
    InvalidToolInput { tool_name: String, message: String },

    /// An invalid approval response was received for a tool call.
    #[error("Invalid tool approval for '{tool_name}': {message}")]
    InvalidToolApproval { tool_name: String, message: String },

    /// No matching tool call was found for the given approval ID.
    #[error("Tool call not found for approval ID '{approval_id}'")]
    ToolCallNotFoundForApproval { approval_id: String },

    /// Required tool results were not provided for pending tool calls.
    #[error("Missing tool results for tool calls: {tool_call_ids:?}")]
    MissingToolResults { tool_call_ids: Vec<String> },

    /// Attempted to repair a tool call input but the repair failed.
    #[error("Tool call repair failed for '{tool_name}': {message}")]
    ToolCallRepair { tool_name: String, message: String },

    // ── Schema / parsing ─────────────────────────────────────────────────

    /// Schema validation error.
    #[error("Schema validation error: {message}")]
    SchemaValidation { message: String },

    /// Output parsing error.
    #[error("Output parse error: {message}")]
    OutputParse { message: String },

    // ── Streaming ────────────────────────────────────────────────────────

    /// An invalid or unexpected stream part was received.
    #[error("Invalid stream part: {message}")]
    InvalidStreamPart { message: String },

    /// UI message stream error (serialization, protocol violation, etc.).
    #[error("UI message stream error: {message}")]
    UIMessageStream { message: String },

    // ── Model compatibility ──────────────────────────────────────────────

    /// The model does not support the requested specification version.
    #[error("Unsupported model version: expected '{expected}', got '{actual}'")]
    UnsupportedModelVersion { expected: String, actual: String },

    // ── Data / conversion ────────────────────────────────────────────────

    /// Invalid data content (e.g. a malformed data URL or base64).
    #[error("Invalid data content: {message}")]
    InvalidDataContent { message: String },

    /// An invalid or unrecognised message role was encountered.
    #[error("Invalid message role: '{role}'")]
    InvalidMessageRole { role: String },

    /// Failed to convert between message formats.
    #[error("Message conversion error: {message}")]
    MessageConversion { message: String },

    // ── General ──────────────────────────────────────────────────────────

    /// An invalid argument was provided to an SDK function.
    #[error("Invalid argument: {message}")]
    InvalidArgument { message: String },

    /// Operation was cancelled via CancellationToken.
    #[error("Operation cancelled")]
    Cancelled,

    /// Generic error (use only when no specific variant applies).
    #[error("{message}")]
    Other { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_such_provider_display() {
        let err = Error::NoSuchProvider {
            provider_id: "acme".into(),
        };
        assert_eq!(
            err.to_string(),
            "No provider registered with prefix 'acme'"
        );
    }

    #[test]
    fn test_no_content_generated_display() {
        let err = Error::NoContentGenerated {
            finish_reason: "length".into(),
        };
        assert!(err.to_string().contains("No content generated"));
        assert!(err.to_string().contains("length"));
    }

    #[test]
    fn test_invalid_tool_input_display() {
        let err = Error::InvalidToolInput {
            tool_name: "search".into(),
            message: "missing 'query' field".into(),
        };
        assert!(err.to_string().contains("search"));
        assert!(err.to_string().contains("missing 'query' field"));
    }

    #[test]
    fn test_retry_display() {
        let err = Error::Retry {
            attempts: 3,
            message: "connection refused".into(),
        };
        assert!(err.to_string().contains("3 attempts"));
    }

    #[test]
    fn test_tool_call_not_found_for_approval_display() {
        let err = Error::ToolCallNotFoundForApproval {
            approval_id: "abc-123".into(),
        };
        assert!(err.to_string().contains("abc-123"));
    }

    #[test]
    fn test_missing_tool_results_display() {
        let err = Error::MissingToolResults {
            tool_call_ids: vec!["tc-1".into(), "tc-2".into()],
        };
        let s = err.to_string();
        assert!(s.contains("tc-1"));
        assert!(s.contains("tc-2"));
    }

    #[test]
    fn test_invalid_stream_part_display() {
        let err = Error::InvalidStreamPart {
            message: "unknown type tag".into(),
        };
        assert!(err.to_string().contains("unknown type tag"));
    }

    #[test]
    fn test_download_error_display() {
        let err = Error::Download {
            url: "https://example.com/img.png".into(),
            message: "404 not found".into(),
        };
        assert!(err.to_string().contains("example.com"));
        assert!(err.to_string().contains("404"));
    }

    #[test]
    fn test_unsupported_model_version_display() {
        let err = Error::UnsupportedModelVersion {
            expected: "v3".into(),
            actual: "v2".into(),
        };
        assert!(err.to_string().contains("v3"));
        assert!(err.to_string().contains("v2"));
    }

    #[test]
    fn test_no_model_variants_display() {
        assert!(Error::NoImageGenerated.to_string().contains("image"));
        assert!(Error::NoSpeechGenerated.to_string().contains("speech"));
        assert!(
            Error::NoTranscriptGenerated
                .to_string()
                .contains("transcript")
        );
        assert!(Error::NoVideoGenerated.to_string().contains("video"));
    }

    #[test]
    fn test_message_conversion_display() {
        let err = Error::MessageConversion {
            message: "cannot convert system to tool".into(),
        };
        assert!(err.to_string().contains("system to tool"));
    }

    #[test]
    fn test_invalid_argument_display() {
        let err = Error::InvalidArgument {
            message: "max_steps must be >= 1".into(),
        };
        assert!(err.to_string().contains("max_steps"));
    }
}
