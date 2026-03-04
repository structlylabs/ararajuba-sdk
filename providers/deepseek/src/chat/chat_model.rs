//! DeepSeek chat language model — wraps `OpenAICompatibleChatLanguageModel`
//! with DeepSeek-specific thinking/reasoning handling.

use async_trait::async_trait;
use ararajuba_provider::errors::Error;
use ararajuba_provider::language_model::v4::call_options::CallOptions;
use ararajuba_provider::language_model::v4::generate_result::GenerateResult;
use ararajuba_provider::language_model::v4::language_model_v4::LanguageModelV4;
use ararajuba_provider::language_model::v4::stream_result::StreamResult;
use ararajuba_provider::shared::Warning;
use ararajuba_openai_compatible::{ChatModelConfig, OpenAICompatibleChatLanguageModel};
use std::collections::HashMap;

/// A DeepSeek chat language model.
///
/// Wraps [`OpenAICompatibleChatLanguageModel`] with DeepSeek-specific features:
/// - `thinking` config for reasoning models (deepseek-reasoner)
/// - Prompt cache token tracking
/// - `insufficient_system_resource` finish reason handling
pub struct DeepSeekChatLanguageModel {
    model_id: String,
    provider_name: String,
    inner: OpenAICompatibleChatLanguageModel,
}

impl DeepSeekChatLanguageModel {
    pub fn new(model_id: String, config: ChatModelConfig) -> Self {
        let provider_name = config.provider.clone();
        Self {
            model_id: model_id.clone(),
            provider_name,
            inner: OpenAICompatibleChatLanguageModel::new(model_id, config),
        }
    }

    /// Transform CallOptions for DeepSeek-specific requirements.
    fn transform_options(&self, options: &CallOptions) -> (CallOptions, Vec<Warning>) {
        let mut warnings = Vec::new();
        let mut modified = options.clone();

        // Parse DeepSeek-specific provider options
        let ds_opts = options
            .provider_options
            .as_ref()
            .and_then(|po| po.get("deepseek"));

        // Build extra options to pass through
        let mut extra = HashMap::new();

        // Thinking config
        if let Some(opts) = ds_opts {
            if let Some(thinking) = opts.get("thinking") {
                extra.insert("thinking".to_string(), thinking.clone());
            }
        }

        // For reasoner models, warn about unsupported params
        let is_reasoner = self.model_id.contains("reasoner");
        if is_reasoner {
            if options.temperature.is_some() {
                warnings.push(Warning::Unsupported {
                    feature: "temperature".into(),
                    details: Some("Not supported for deepseek-reasoner".into()),
                });
                modified.temperature = None;
            }
            if options.top_p.is_some() {
                warnings.push(Warning::Unsupported {
                    feature: "top_p".into(),
                    details: Some("Not supported for deepseek-reasoner".into()),
                });
                modified.top_p = None;
            }
        }

        // Pass extra options through provider_options for the base model
        if !extra.is_empty() {
            let po = modified.provider_options.get_or_insert_with(HashMap::new);
            let ds_pass = po.entry("deepseek".to_string()).or_insert_with(HashMap::new);
            for (k, v) in extra {
                ds_pass.insert(k, v);
            }
        }

        (modified, warnings)
    }
}

#[async_trait]
impl LanguageModelV4 for DeepSeekChatLanguageModel {
    fn provider(&self) -> &str {
        &self.provider_name
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }

    async fn do_generate(
        &self,
        options: &CallOptions,
    ) -> Result<GenerateResult, Error> {
        let (modified, extra_warnings) = self.transform_options(options);
        let mut result = self.inner.do_generate(&modified).await?;
        result.warnings.extend(extra_warnings);
        Ok(result)
    }

    async fn do_stream(
        &self,
        options: &CallOptions,
    ) -> Result<StreamResult, Error> {
        let (modified, _extra_warnings) = self.transform_options(options);
        self.inner.do_stream(&modified).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_model(model_id: &str) -> DeepSeekChatLanguageModel {
        DeepSeekChatLanguageModel::new(
            model_id.into(),
            ChatModelConfig {
                provider: "deepseek.chat".into(),
                url: "https://api.deepseek.com/chat/completions".into(),
                headers: HashMap::new(),
                include_usage: true,
                supports_structured_outputs: false,
                fetch: None,
            },
        )
    }

    #[test]
    fn test_provider_info() {
        let model = make_model("deepseek-chat");
        assert_eq!(model.provider(), "deepseek.chat");
        assert_eq!(model.model_id(), "deepseek-chat");
    }

    #[test]
    fn test_reasoner_strips_temperature() {
        let model = make_model("deepseek-reasoner");
        let options = CallOptions {
            prompt: vec![],
            temperature: Some(0.7),
            top_p: Some(0.9),
            ..Default::default()
        };
        let (modified, warnings) = model.transform_options(&options);
        assert!(modified.temperature.is_none());
        assert!(modified.top_p.is_none());
        assert_eq!(warnings.len(), 2);
    }

    #[test]
    fn test_non_reasoner_keeps_temperature() {
        let model = make_model("deepseek-chat");
        let options = CallOptions {
            prompt: vec![],
            temperature: Some(0.7),
            ..Default::default()
        };
        let (modified, warnings) = model.transform_options(&options);
        assert_eq!(modified.temperature, Some(0.7));
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_thinking_config_passthrough() {
        let model = make_model("deepseek-reasoner");
        let mut po = HashMap::new();
        let mut ds = HashMap::new();
        ds.insert("thinking".to_string(), json!({"type": "enabled"}));
        po.insert("deepseek".to_string(), ds);

        let options = CallOptions {
            prompt: vec![],
            provider_options: Some(po),
            ..Default::default()
        };
        let (modified, _) = model.transform_options(&options);
        let ds_opts = modified.provider_options.unwrap().get("deepseek").unwrap().clone();
        assert_eq!(ds_opts.get("thinking").unwrap(), &json!({"type": "enabled"}));
    }
}

// ---------------------------------------------------------------------------
// Capability trait implementations
// ---------------------------------------------------------------------------

use ararajuba_provider::capabilities::{
    ImageFormat, ReasoningConfig, SupportsImages, SupportsReasoning, SupportsStructuredOutput,
    SupportsToolCalling,
};

impl SupportsReasoning for DeepSeekChatLanguageModel {
    fn reasoning_config(&self) -> ReasoningConfig {
        let is_reasoner = self.model_id.contains("reasoner");
        ReasoningConfig {
            enabled: is_reasoner,
            default_effort: None,
            max_reasoning_tokens: None,
        }
    }
}

impl SupportsToolCalling for DeepSeekChatLanguageModel {
    fn max_tools(&self) -> Option<usize> {
        None
    }

    fn supports_parallel_calls(&self) -> bool {
        true
    }
}

impl SupportsImages for DeepSeekChatLanguageModel {
    fn supported_image_formats(&self) -> Vec<ImageFormat> {
        vec![
            ImageFormat::Jpeg,
            ImageFormat::Png,
            ImageFormat::Gif,
            ImageFormat::Webp,
        ]
    }
}

impl SupportsStructuredOutput for DeepSeekChatLanguageModel {
    fn supports_json_mode(&self) -> bool {
        true
    }

    fn supports_json_schema(&self) -> bool {
        false // DeepSeek doesn't support strict JSON schema
    }
}
