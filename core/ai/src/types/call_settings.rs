//! Call settings shared across high-level SDK functions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

/// Timeout configuration for SDK calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Timeout {
    /// Single total timeout for the entire call.
    Total(#[serde(with = "humantime_serde_compat")] Duration),
    /// Fine-grained timeouts.
    Granular {
        /// Total timeout for the entire call (including all steps).
        #[serde(skip_serializing_if = "Option::is_none")]
        total: Option<Duration>,
        /// Timeout per agentic step.
        #[serde(skip_serializing_if = "Option::is_none")]
        step: Option<Duration>,
        /// Timeout per stream chunk (inactivity timeout).
        #[serde(skip_serializing_if = "Option::is_none")]
        chunk: Option<Duration>,
    },
}

/// Helper module for Duration serialization.
mod humantime_serde_compat {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        d.as_millis().serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        let ms = u64::deserialize(d)?;
        Ok(Duration::from_millis(ms))
    }
}

/// Settings common to all `generate_text`, `generate_object`, etc. calls.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CallSettings {
    /// Maximum number of output tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,

    /// Temperature for sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    /// Nucleus sampling probability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,

    /// Top-K sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,

    /// Presence penalty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,

    /// Frequency penalty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,

    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,

    /// Random seed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,

    /// Additional headers for the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,

    /// Provider-specific options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_options: Option<HashMap<String, serde_json::Value>>,

    /// Timeout configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<Timeout>,

    /// Cancellation token for cooperative cancellation.
    #[serde(skip)]
    pub cancellation_token: Option<CancellationToken>,
}
