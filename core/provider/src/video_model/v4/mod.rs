//! v4 Video Model interface — async-native.

pub mod video_model_v4;

// Re-export all types from v3 (unchanged)
pub use super::v3::video_model_v3::{
    VideoCallOptions, VideoData, VideoGenerateResult, VideoImageInput, VideoResponseMetadata,
};
