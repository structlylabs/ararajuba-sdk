//! Stream result returned by `do_stream`.

use super::stream_part::StreamPart;
use crate::shared::Headers;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};

/// Result of initiating a streaming language model generation.
pub struct StreamResult {
    /// The stream of parts.
    pub stream: BoxStream<'static, Result<StreamPart, crate::errors::Error>>,
    /// Request metadata.
    pub request: Option<StreamRequestMetadata>,
    /// Response metadata (headers from the initial HTTP response).
    pub response: Option<StreamResponseMetadata>,
}

/// Request metadata for streams.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamRequestMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
}

/// Response metadata for streams.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamResponseMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Headers>,
}
