//! v4 Speech Model interface — async-native.

pub mod speech_model_v4;

// Re-export all types from v3 (unchanged)
pub use super::v3::speech_model_v3::{
    AudioData, SpeechCallOptions, SpeechGenerateResult, SpeechResponseMetadata,
};
