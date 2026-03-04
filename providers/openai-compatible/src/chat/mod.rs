//! OpenAI-compatible chat model module.

pub mod chat_model;
pub mod convert_messages;
pub mod finish_reason;
pub mod prepare_tools;
pub mod usage;

pub use chat_model::{ChatModelConfig, OpenAICompatibleChatLanguageModel};
