//! The `generate_object` function.

use super::options::{GenerateObjectFinishEvent, GenerateObjectOptions};
use crate::error::Error;
use crate::types::call_warning::CallWarning;
use crate::types::finish_reason::FinishReason;
use ararajuba_provider::language_model::v4::call_options::CallOptions;
use ararajuba_provider::language_model::v4::content::Content;
use ararajuba_provider::language_model::v4::usage::Usage;

/// Result of `generate_object()`.
pub struct GenerateObjectResult {
    /// The parsed JSON object.
    pub object: serde_json::Value,
    /// Token usage.
    pub usage: Usage,
    /// Finish reason.
    pub finish_reason: FinishReason,
    /// Warnings.
    pub warnings: Vec<CallWarning>,
}

/// Generate a structured JSON object from a language model.
pub async fn generate_object(options: GenerateObjectOptions) -> Result<GenerateObjectResult, Error> {
    let _span = tracing::info_span!(
        "generate_object",
        model = %options.model.model_id(),
        provider = %options.model.provider(),
    )
    .entered();

    let messages = build_messages(&options);
    let response_format = options.output.response_format();

    let call_options = CallOptions {
        prompt: messages,
        max_output_tokens: options.call_settings.max_output_tokens,
        temperature: options.call_settings.temperature,
        stop_sequences: options.call_settings.stop_sequences.clone(),
        top_p: options.call_settings.top_p,
        top_k: options.call_settings.top_k,
        presence_penalty: options.call_settings.presence_penalty,
        frequency_penalty: options.call_settings.frequency_penalty,
        response_format: Some(response_format),
        seed: options.call_settings.seed,
        tools: None,
        tool_choice: None,
        include_raw_chunks: None,
        headers: options.call_settings.headers.clone(),
        provider_options: None,
        cancellation_token: options.call_settings.cancellation_token.clone(),
    };

    let result = options.model.do_generate(&call_options).await?;

    // Extract text from result
    let text: String = result
        .content
        .iter()
        .filter_map(|c| match c {
            Content::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();

    // First attempt to parse
    let object = match options.output.parse_complete(&text) {
        Ok(obj) => obj,
        Err(parse_err) => {
            // Try repair_text if available
            if let Some(ref repair) = options.repair_text {
                let schema = options.output.json_schema();
                let err_msg = parse_err.to_string();
                if let Some(repaired) = repair(text, schema, err_msg).await {
                    options.output.parse_complete(&repaired)?
                } else {
                    if let Some(ref on_err) = options.on_error {
                        on_err(&parse_err);
                    }
                    return Err(parse_err);
                }
            } else {
                if let Some(ref on_err) = options.on_error {
                    on_err(&parse_err);
                }
                return Err(parse_err);
            }
        }
    };

    let warnings: Vec<CallWarning> = result
        .warnings
        .iter()
        .cloned()
        .map(CallWarning::from)
        .collect();

    let gen_result = GenerateObjectResult {
        object: object.clone(),
        usage: result.usage.clone(),
        finish_reason: FinishReason::from_provider(&result.finish_reason.unified),
        warnings,
    };

    // Fire on_finish callback
    if let Some(ref cb) = options.on_finish {
        cb(&GenerateObjectFinishEvent {
            object,
            usage: result.usage,
        });
    }

    Ok(gen_result)
}

fn build_messages(
    options: &GenerateObjectOptions,
) -> Vec<ararajuba_provider::language_model::v4::prompt::Message> {
    use ararajuba_provider::language_model::v4::content_part::TextPart;
    use ararajuba_provider::language_model::v4::prompt::{Message, UserContentPart};

    let mut messages = Vec::new();

    if let Some(ref system) = options.system {
        messages.push(Message::System {
            content: system.clone(),
            provider_options: None,
        });
    }

    if let Some(ref prompt) = options.prompt {
        messages.push(Message::User {
            content: vec![UserContentPart::Text(TextPart {
                text: prompt.clone(),
                provider_options: None,
            })],
            provider_options: None,
        });
    }

    if let Some(ref msgs) = options.messages {
        messages.extend(msgs.clone());
    }

    messages
}
