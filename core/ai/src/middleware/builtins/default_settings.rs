//! `default_settings_middleware` — applies default `CallSettings` values.

use crate::middleware::wrap_language_model::LanguageModelMiddleware;
use ararajuba_provider::language_model::v4::call_options::CallOptions;

/// Default settings to apply when the caller doesn't specify them.
pub struct DefaultSettings {
    pub max_output_tokens: Option<u32>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub top_k: Option<u32>,
    pub presence_penalty: Option<f64>,
    pub frequency_penalty: Option<f64>,
    pub stop_sequences: Option<Vec<String>>,
    pub seed: Option<u64>,
}

impl Default for DefaultSettings {
    fn default() -> Self {
        Self {
            max_output_tokens: None,
            temperature: None,
            top_p: None,
            top_k: None,
            presence_penalty: None,
            frequency_penalty: None,
            stop_sequences: None,
            seed: None,
        }
    }
}

/// Creates middleware that applies default call settings when the caller
/// doesn't explicitly set them.
pub fn default_settings_middleware(defaults: DefaultSettings) -> LanguageModelMiddleware {
    LanguageModelMiddleware {
        transform_params: Some(Box::new(move |mut opts: CallOptions, _model_ref| {
            let max_output_tokens = defaults.max_output_tokens;
            let temperature = defaults.temperature;
            let top_p = defaults.top_p;
            let top_k = defaults.top_k;
            let presence_penalty = defaults.presence_penalty;
            let frequency_penalty = defaults.frequency_penalty;
            let stop_sequences = defaults.stop_sequences.clone();
            let seed = defaults.seed;

            Box::pin(async move {
                if opts.max_output_tokens.is_none() {
                    opts.max_output_tokens = max_output_tokens;
                }
                if opts.temperature.is_none() {
                    opts.temperature = temperature;
                }
                if opts.top_p.is_none() {
                    opts.top_p = top_p;
                }
                if opts.top_k.is_none() {
                    opts.top_k = top_k;
                }
                if opts.presence_penalty.is_none() {
                    opts.presence_penalty = presence_penalty;
                }
                if opts.frequency_penalty.is_none() {
                    opts.frequency_penalty = frequency_penalty;
                }
                if opts.stop_sequences.is_none() {
                    opts.stop_sequences = stop_sequences;
                }
                if opts.seed.is_none() {
                    opts.seed = seed;
                }
                Ok(opts)
            })
        })),
        ..LanguageModelMiddleware::default()
    }
}
