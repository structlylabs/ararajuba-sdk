//! Shared types used across the provider spec layer.

use crate::json_value::JSONObject;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Provider-specific metadata attached to responses.
/// Maps provider name → metadata object.
pub type ProviderMetadata = HashMap<String, JSONObject>;

/// Provider-specific options passed in requests.
/// Maps provider name → options object.
pub type ProviderOptions = HashMap<String, JSONObject>;

/// Headers map.
pub type Headers = HashMap<String, String>;

/// Warnings emitted by the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Warning {
    /// The model does not support the requested feature.
    #[serde(rename = "unsupported")]
    Unsupported {
        feature: String,
        details: Option<String>,
    },
    /// The feature is supported but may not work as expected.
    #[serde(rename = "compatibility")]
    Compatibility {
        feature: String,
        details: Option<String>,
    },
    /// Any other warning.
    #[serde(rename = "other")]
    Other { message: String },
}
