//! OpenAI chat language model — wraps `OpenAICompatibleChatLanguageModel`
//! with OpenAI-specific reasoning model handling and option mapping.

use crate::capabilities::{
    get_openai_model_capabilities, is_search_model, SystemMessageMode,
};
use crate::chat::options::{parse_openai_options, LogprobsOption};
use async_trait::async_trait;
use ararajuba_provider::errors::Error;
use ararajuba_provider::language_model::v4::call_options::CallOptions;
use ararajuba_provider::language_model::v4::generate_result::GenerateResult;
use ararajuba_provider::language_model::v4::language_model_v4::LanguageModelV4;
use ararajuba_provider::language_model::v4::stream_result::StreamResult;
use ararajuba_provider::shared::Warning;
use ararajuba_openai_compatible::{ChatModelConfig, OpenAICompatibleChatLanguageModel};
use serde_json::{json, Value};

/// An OpenAI chat language model.
///
/// Wraps [`OpenAICompatibleChatLanguageModel`] with OpenAI-specific capabilities:
/// - Reasoning model detection and parameter stripping
/// - System → developer message mode for reasoning models
/// - OpenAI-specific provider options (logit_bias, logprobs, service_tier, etc.)
/// - Search model parameter stripping
pub struct OpenAIChatLanguageModel {
    model_id: String,
    provider_name: String,
    inner: OpenAICompatibleChatLanguageModel,
}

impl OpenAIChatLanguageModel {
    pub fn new(model_id: String, config: ChatModelConfig) -> Self {
        let provider_name = config.provider.clone();
        Self {
            model_id: model_id.clone(),
            provider_name,
            inner: OpenAICompatibleChatLanguageModel::new(model_id, config),
        }
    }

    /// Transform CallOptions for OpenAI-specific requirements.
    fn transform_options(&self, options: &CallOptions) -> (CallOptions, Vec<Warning>) {
        let mut warnings = Vec::new();
        let mut modified = options.clone();
        let caps = get_openai_model_capabilities(&self.model_id);
        let openai_opts = parse_openai_options(
            options.provider_options.as_ref(),
            &self.provider_name,
        );

        // Determine if we should treat as reasoning model
        let is_reasoning = openai_opts.force_reasoning.unwrap_or(caps.is_reasoning_model);
        let reasoning_effort = openai_opts.reasoning_effort.as_deref();

        // Determine system message mode
        let system_mode = openai_opts
            .system_message_mode
            .as_deref()
            .map(|s| match s {
                "developer" => SystemMessageMode::Developer,
                "remove" => SystemMessageMode::Remove,
                _ => SystemMessageMode::System,
            })
            .unwrap_or_else(|| {
                if is_reasoning {
                    SystemMessageMode::Developer
                } else {
                    caps.system_message_mode
                }
            });

        // Transform system messages based on mode
        match system_mode {
            SystemMessageMode::Developer => {
                // Convert system messages to developer role — done via provider_options
                // The compatible base will see the role hint
                let po = modified.provider_options.get_or_insert_with(Default::default);
                let openai_po = po.entry(self.provider_name.clone()).or_default();
                openai_po.insert(
                    "systemMessageMode".into(),
                    Value::String("developer".into()),
                );
            }
            SystemMessageMode::Remove => {
                // Strip system messages from prompt
                modified.prompt.retain(|m| {
                    !matches!(m, ararajuba_provider::language_model::v4::prompt::Message::System { .. })
                });
                if options.prompt.iter().any(|m| {
                    matches!(m, ararajuba_provider::language_model::v4::prompt::Message::System { .. })
                }) {
                    warnings.push(Warning::Unsupported {
                        feature: "system messages".into(),
                        details: Some(format!(
                            "System messages removed for model {}",
                            self.model_id
                        )),
                    });
                }
            }
            SystemMessageMode::System => {
                // Default — do nothing
            }
        }

        // Reasoning model parameter stripping
        if is_reasoning {
            let allows_non_reasoning = reasoning_effort == Some("none")
                && caps.supports_non_reasoning_params;

            if !allows_non_reasoning {
                if modified.temperature.is_some() {
                    modified.temperature = None;
                    warnings.push(Warning::Unsupported {
                        feature: "temperature".into(),
                        details: Some("Removed for reasoning model".into()),
                    });
                }
                if modified.top_p.is_some() {
                    modified.top_p = None;
                    warnings.push(Warning::Unsupported {
                        feature: "top_p".into(),
                        details: Some("Removed for reasoning model".into()),
                    });
                }
                if modified.frequency_penalty.is_some() {
                    modified.frequency_penalty = None;
                    warnings.push(Warning::Unsupported {
                        feature: "frequency_penalty".into(),
                        details: Some("Removed for reasoning model".into()),
                    });
                }
                if modified.presence_penalty.is_some() {
                    modified.presence_penalty = None;
                    warnings.push(Warning::Unsupported {
                        feature: "presence_penalty".into(),
                        details: Some("Removed for reasoning model".into()),
                    });
                }
            }

            // Reasoning models use max_completion_tokens instead of max_tokens.
            // We pass this via provider_options so the compatible base puts it in the body.
            if let Some(max) = modified.max_output_tokens.take() {
                let po = modified.provider_options.get_or_insert_with(Default::default);
                let openai_po = po.entry(self.provider_name.clone()).or_default();
                openai_po.insert(
                    "max_completion_tokens".into(),
                    json!(max),
                );
            }

            // Also pass max_completion_tokens from options if set
            if let Some(mct) = openai_opts.max_completion_tokens {
                let po = modified.provider_options.get_or_insert_with(Default::default);
                let openai_po = po.entry(self.provider_name.clone()).or_default();
                openai_po.insert(
                    "max_completion_tokens".into(),
                    json!(mct),
                );
            }
        }

        // Search model: strip temperature
        if is_search_model(&self.model_id) && modified.temperature.is_some() {
            modified.temperature = None;
            warnings.push(Warning::Unsupported {
                feature: "temperature".into(),
                details: Some("Removed for search model".into()),
            });
        }

        // Map OpenAI-specific options into provider_options for the base
        {
            let po = modified.provider_options.get_or_insert_with(Default::default);
            let openai_po = po.entry(self.provider_name.clone()).or_default();

            if let Some(ref lb) = openai_opts.logit_bias {
                openai_po.insert("logit_bias".into(), json!(lb));
            }
            if let Some(ref lp) = openai_opts.logprobs {
                match lp {
                    LogprobsOption::Enabled(true) => {
                        openai_po.insert("logprobs".into(), json!(true));
                        openai_po.insert("top_logprobs".into(), json!(0));
                    }
                    LogprobsOption::TopN(n) => {
                        openai_po.insert("logprobs".into(), json!(true));
                        openai_po.insert("top_logprobs".into(), json!(n));
                    }
                    _ => {}
                }
            }
            if let Some(ptc) = openai_opts.parallel_tool_calls {
                openai_po.insert("parallel_tool_calls".into(), json!(ptc));
            }
            if let Some(ref user) = openai_opts.user {
                openai_po.insert("user".into(), json!(user));
            }
            if let Some(ref re) = openai_opts.reasoning_effort {
                openai_po.insert("reasoning_effort".into(), json!(re));
            }
            if let Some(store) = openai_opts.store {
                openai_po.insert("store".into(), json!(store));
            }
            if let Some(ref md) = openai_opts.metadata {
                openai_po.insert("metadata".into(), json!(md));
            }
            if let Some(ref pred) = openai_opts.prediction {
                openai_po.insert("prediction".into(), pred.clone());
            }
            if let Some(ref st) = openai_opts.service_tier {
                openai_po.insert("service_tier".into(), json!(st));
            }
            if let Some(ref tv) = openai_opts.text_verbosity {
                openai_po.insert("verbosity".into(), json!(tv));
            }
            if let Some(ref pck) = openai_opts.prompt_cache_key {
                openai_po.insert("prompt_cache_key".into(), json!(pck));
            }
            if let Some(ref pcr) = openai_opts.prompt_cache_retention {
                openai_po.insert("prompt_cache_retention".into(), json!(pcr));
            }
            if let Some(ref si) = openai_opts.safety_identifier {
                openai_po.insert("safety_identifier".into(), json!(si));
            }
        }

        (modified, warnings)
    }
}

#[async_trait]
impl LanguageModelV4 for OpenAIChatLanguageModel {
    fn provider(&self) -> &str {
        self.inner.provider()
    }

    fn model_id(&self) -> &str {
        self.inner.model_id()
    }

    async fn do_generate(&self, options: &CallOptions) -> Result<GenerateResult, Error> {
        let (modified_options, extra_warnings) = self.transform_options(options);
        let mut result = self.inner.do_generate(&modified_options).await?;
        // Prepend transform warnings
        let mut all_warnings = extra_warnings;
        all_warnings.append(&mut result.warnings);
        result.warnings = all_warnings;
        Ok(result)
    }

    async fn do_stream(&self, options: &CallOptions) -> Result<StreamResult, Error> {
        let (modified_options, _extra_warnings) = self.transform_options(options);
        // Warnings are emitted inside the stream via StreamStart
        self.inner.do_stream(&modified_options).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_model(model_id: &str) -> OpenAIChatLanguageModel {
        OpenAIChatLanguageModel::new(
            model_id.into(),
            ChatModelConfig {
                provider: "openai.chat".into(),
                url: "https://api.openai.com/v1/chat/completions".into(),
                headers: HashMap::new(),
                include_usage: true,
                supports_structured_outputs: true,
                fetch: None,
            },
        )
    }

    #[test]
    fn test_regular_model_no_stripping() {
        let model = make_model("gpt-4o");
        let options = CallOptions {
            prompt: vec![],
            temperature: Some(0.7),
            top_p: Some(0.9),
            frequency_penalty: Some(0.1),
            presence_penalty: Some(0.2),
            ..Default::default()
        };
        let (modified, warnings) = model.transform_options(&options);
        assert!(warnings.is_empty());
        assert_eq!(modified.temperature, Some(0.7));
        assert_eq!(modified.top_p, Some(0.9));
    }

    #[test]
    fn test_reasoning_model_strips_params() {
        let model = make_model("o3-mini");
        let options = CallOptions {
            prompt: vec![],
            temperature: Some(0.7),
            top_p: Some(0.9),
            frequency_penalty: Some(0.1),
            presence_penalty: Some(0.2),
            max_output_tokens: Some(1000),
            ..Default::default()
        };
        let (modified, warnings) = model.transform_options(&options);
        assert!(warnings.len() >= 4); // temperature, top_p, frequency, presence
        assert!(modified.temperature.is_none());
        assert!(modified.top_p.is_none());
        assert!(modified.frequency_penalty.is_none());
        assert!(modified.presence_penalty.is_none());
        assert!(modified.max_output_tokens.is_none()); // converted to max_completion_tokens
    }

    #[test]
    fn test_gpt5_is_reasoning() {
        let model = make_model("gpt-5");
        let options = CallOptions {
            prompt: vec![],
            temperature: Some(0.5),
            ..Default::default()
        };
        let (modified, warnings) = model.transform_options(&options);
        assert!(modified.temperature.is_none());
        assert!(!warnings.is_empty());
    }

    #[test]
    fn test_search_model_strips_temperature() {
        let model = make_model("gpt-4o-search-preview");
        let options = CallOptions {
            prompt: vec![],
            temperature: Some(0.7),
            ..Default::default()
        };
        let (modified, warnings) = model.transform_options(&options);
        assert!(modified.temperature.is_none());
        assert_eq!(warnings.len(), 1);
    }
}

// ---------------------------------------------------------------------------
// Capability trait implementations
// ---------------------------------------------------------------------------

use ararajuba_provider::capabilities::{
    CacheConfig, ImageFormat, ReasoningConfig, SupportsCaching, SupportsImages, SupportsReasoning,
    SupportsStructuredOutput, SupportsToolCalling,
};

impl SupportsReasoning for OpenAIChatLanguageModel {
    fn reasoning_config(&self) -> ReasoningConfig {
        let caps = get_openai_model_capabilities(&self.model_id);
        ReasoningConfig {
            enabled: caps.is_reasoning_model,
            default_effort: Some("medium".to_string()),
            max_reasoning_tokens: None,
        }
    }
}

impl SupportsCaching for OpenAIChatLanguageModel {
    fn cache_config(&self) -> CacheConfig {
        CacheConfig {
            supports_auto_cache: true,
            supports_cache_control: false,
            max_cache_tokens: None,
        }
    }
}

impl SupportsToolCalling for OpenAIChatLanguageModel {
    fn max_tools(&self) -> Option<usize> {
        Some(128)
    }

    fn supports_parallel_calls(&self) -> bool {
        true
    }

    fn supports_strict_schemas(&self) -> bool {
        true
    }
}

impl SupportsImages for OpenAIChatLanguageModel {
    fn supported_image_formats(&self) -> Vec<ImageFormat> {
        vec![
            ImageFormat::Jpeg,
            ImageFormat::Png,
            ImageFormat::Gif,
            ImageFormat::Webp,
        ]
    }
}

impl SupportsStructuredOutput for OpenAIChatLanguageModel {
    fn supports_json_mode(&self) -> bool {
        true
    }

    fn supports_json_schema(&self) -> bool {
        true
    }
}
