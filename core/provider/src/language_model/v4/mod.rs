//! v4 Language Model interface — Rust-native with async traits, typed streams,
//! capability traits, and Drop-based cancellation.

pub mod call_options;
pub mod capabilities;
pub mod compat;
pub mod language_model_v4;
pub mod stream_result;

// Re-export shared types from v3 (unchanged between versions)
pub use super::v3::content;
pub use super::v3::content_part;
pub use super::v3::finish_reason;
pub use super::v3::generate_result;
pub use super::v3::prompt;
pub use super::v3::stream_part;
pub use super::v3::tool;
pub use super::v3::tool_choice;
pub use super::v3::usage;
