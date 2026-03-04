//! v4 Transcription Model interface — async-native.

pub mod transcription_model_v4;

// Re-export all types from v3 (unchanged)
pub use super::v3::transcription_model_v3::{
    TranscriptionAudioInput, TranscriptionCallOptions, TranscriptionResponseMetadata,
    TranscriptionResult, TranscriptionSegment,
};
