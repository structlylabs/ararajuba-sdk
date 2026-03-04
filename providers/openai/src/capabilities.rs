//! OpenAI model capability detection.
//!
//! Determines model capabilities based on model ID prefix matching.
//! This mirrors `openai-language-model-capabilities.ts` from the TS SDK.

/// Capabilities of an OpenAI language model.
#[derive(Debug, Clone)]
pub struct OpenAIModelCapabilities {
    /// Whether this is a reasoning model (o-series, gpt-5).
    pub is_reasoning_model: bool,
    /// How system messages should be sent.
    pub system_message_mode: SystemMessageMode,
    /// Whether the model supports flex processing (service_tier).
    pub supports_flex_processing: bool,
    /// Whether the model supports priority processing (service_tier).
    pub supports_priority_processing: bool,
    /// Whether the model supports non-reasoning parameters when effort=none.
    pub supports_non_reasoning_params: bool,
}

/// How to handle system messages for a model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SystemMessageMode {
    /// Send as role="system" (standard models).
    System,
    /// Send as role="developer" (reasoning models).
    Developer,
    /// Remove system messages entirely.
    Remove,
}

/// Get the capabilities for an OpenAI model based on its ID.
pub fn get_openai_model_capabilities(model_id: &str) -> OpenAIModelCapabilities {
    let is_reasoning = is_reasoning_model(model_id);
    let is_gpt5_chat = model_id.starts_with("gpt-5-chat");

    OpenAIModelCapabilities {
        is_reasoning_model: is_reasoning,
        system_message_mode: if is_reasoning {
            SystemMessageMode::Developer
        } else {
            SystemMessageMode::System
        },
        supports_flex_processing: model_id.starts_with("o3")
            || model_id.starts_with("o4-mini")
            || (model_id.starts_with("gpt-5") && !is_gpt5_chat),
        supports_priority_processing: model_id.starts_with("gpt-4")
            || model_id.starts_with("gpt-5-mini")
            || (model_id.starts_with("gpt-5")
                && !model_id.starts_with("gpt-5-nano")
                && !is_gpt5_chat)
            || model_id.starts_with("o3")
            || model_id.starts_with("o4-mini"),
        supports_non_reasoning_params: model_id.starts_with("gpt-5.1")
            || model_id.starts_with("gpt-5.2"),
    }
}

/// Check if a model is a reasoning model by prefix.
fn is_reasoning_model(model_id: &str) -> bool {
    model_id.starts_with("o1")
        || model_id.starts_with("o3")
        || model_id.starts_with("o4-mini")
        || (model_id.starts_with("gpt-5") && !model_id.starts_with("gpt-5-chat"))
}

/// Check if a model is a search-preview model.
pub fn is_search_model(model_id: &str) -> bool {
    model_id.starts_with("gpt-4o-search-preview")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regular_model() {
        let caps = get_openai_model_capabilities("gpt-4o");
        assert!(!caps.is_reasoning_model);
        assert_eq!(caps.system_message_mode, SystemMessageMode::System);
    }

    #[test]
    fn test_o3_reasoning_model() {
        let caps = get_openai_model_capabilities("o3-mini");
        assert!(caps.is_reasoning_model);
        assert_eq!(caps.system_message_mode, SystemMessageMode::Developer);
        assert!(caps.supports_flex_processing);
        assert!(caps.supports_priority_processing);
    }

    #[test]
    fn test_gpt5_reasoning() {
        let caps = get_openai_model_capabilities("gpt-5");
        assert!(caps.is_reasoning_model);
        assert_eq!(caps.system_message_mode, SystemMessageMode::Developer);
        assert!(caps.supports_flex_processing);
        assert!(caps.supports_priority_processing);
    }

    #[test]
    fn test_gpt5_chat_not_reasoning() {
        let caps = get_openai_model_capabilities("gpt-5-chat");
        assert!(!caps.is_reasoning_model);
        assert_eq!(caps.system_message_mode, SystemMessageMode::System);
        assert!(!caps.supports_flex_processing);
    }

    #[test]
    fn test_gpt5_1_non_reasoning_params() {
        let caps = get_openai_model_capabilities("gpt-5.1");
        assert!(caps.supports_non_reasoning_params);
    }

    #[test]
    fn test_search_model() {
        assert!(is_search_model("gpt-4o-search-preview"));
        assert!(is_search_model("gpt-4o-search-preview-2025-01-13"));
        assert!(!is_search_model("gpt-4o"));
    }

    #[test]
    fn test_o4_mini() {
        let caps = get_openai_model_capabilities("o4-mini");
        assert!(caps.is_reasoning_model);
        assert!(caps.supports_flex_processing);
    }
}
