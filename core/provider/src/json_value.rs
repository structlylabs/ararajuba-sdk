//! JSON value types mirroring the TypeScript JSONValue, JSONObject, JSONArray.

use std::collections::HashMap;

/// A JSON value that can be null, string, number, boolean, object, or array.
pub type JSONValue = serde_json::Value;

/// A JSON object (string keys, JSONValue values).
pub type JSONObject = HashMap<String, serde_json::Value>;

/// A JSON array.
pub type JSONArray = Vec<serde_json::Value>;
