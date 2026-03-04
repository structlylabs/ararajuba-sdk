//! Call warning types.

use serde::{Deserialize, Serialize};

/// A warning emitted during an SDK call.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CallWarning {
    #[serde(rename = "unsupported")]
    Unsupported {
        feature: String,
        details: Option<String>,
    },
    #[serde(rename = "compatibility")]
    Compatibility {
        feature: String,
        details: Option<String>,
    },
    #[serde(rename = "other")]
    Other { message: String },
}

impl From<ararajuba_provider::shared::Warning> for CallWarning {
    fn from(w: ararajuba_provider::shared::Warning) -> Self {
        match w {
            ararajuba_provider::shared::Warning::Unsupported { feature, details } => {
                CallWarning::Unsupported { feature, details }
            }
            ararajuba_provider::shared::Warning::Compatibility { feature, details } => {
                CallWarning::Compatibility { feature, details }
            }
            ararajuba_provider::shared::Warning::Other { message } => CallWarning::Other { message },
        }
    }
}
