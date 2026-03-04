//! Provider metadata types.

use std::collections::HashMap;

/// Provider-specific metadata returned from SDK calls.
pub type ProviderMetadata = HashMap<String, HashMap<String, serde_json::Value>>;
