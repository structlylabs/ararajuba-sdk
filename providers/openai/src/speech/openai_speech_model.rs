//! OpenAI text-to-speech model — tts-1, tts-1-hd.
//!
//! Implements `SpeechModel` against the OpenAI Audio Speech API.
//!
//! **Endpoint:** `POST /v1/audio/speech`
//!
//! **Supported models:** tts-1, tts-1-hd
//!
//! **Voices:** alloy, ash, ballad, coral, echo, fable, nova, onyx, sage, shimmer

use crate::speech::openai_speech_options::parse_openai_speech_options;
use async_trait::async_trait;
use ararajuba_provider::errors::Error;
use ararajuba_provider::speech_model::v4::{
    AudioData, SpeechCallOptions, SpeechGenerateResult, SpeechResponseMetadata,
};
use ararajuba_provider::speech_model::v4::speech_model_v4::SpeechModelV4;
use reqwest::Client;
use serde_json::json;
use std::collections::HashMap;

/// An OpenAI text-to-speech model.
pub struct OpenAISpeechModel {
    model_id: String,
    provider_name: String,
    url: String,
    headers: HashMap<String, String>,
    client: Client,
}

impl OpenAISpeechModel {
    pub fn new(
        model_id: String,
        provider_name: String,
        base_url: &str,
        headers: HashMap<String, String>,
    ) -> Self {
        Self {
            model_id,
            provider_name,
            url: format!("{}/audio/speech", base_url.trim_end_matches('/')),
            headers,
            client: Client::new(),
        }
    }
}

#[async_trait]
impl SpeechModelV4 for OpenAISpeechModel {
    fn provider(&self) -> &str {
        &self.provider_name
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }

    async fn do_generate(
        &self,
        options: &SpeechCallOptions,
    ) -> Result<SpeechGenerateResult, Error> {
            let openai_opts =
                parse_openai_speech_options(&options.provider_options, &self.provider_name);
            let warnings = Vec::new();

            // Build request body
            let mut body = json!({
                "model": self.model_id,
                "input": options.text,
            });

            // Voice (default: "alloy")
            let voice = options.voice.as_deref().unwrap_or("alloy");
            body["voice"] = json!(voice);

            // Speed (0.25 - 4.0, default 1.0)
            if let Some(speed) = options.speed {
                body["speed"] = json!(speed.clamp(0.25, 4.0));
            }

            // Instructions (voice style guidance)
            if let Some(ref instructions) = options.instructions {
                body["instructions"] = json!(instructions);
            }

            // Response format
            let response_format = options
                .output_format
                .as_deref()
                .or(openai_opts.response_format.as_deref())
                .unwrap_or("mp3");
            body["response_format"] = json!(response_format);

            // Determine media type from format
            let media_type = match response_format {
                "mp3" => "audio/mpeg",
                "opus" => "audio/opus",
                "aac" => "audio/aac",
                "flac" => "audio/flac",
                "wav" => "audio/wav",
                "pcm" => "audio/pcm",
                _ => "audio/mpeg",
            };

            // Build request
            let mut request = self.client.post(&self.url);
            for (k, v) in &self.headers {
                request = request.header(k, v);
            }
            if let Some(ref extra_headers) = options.headers {
                for (k, v) in extra_headers {
                    request = request.header(k, v);
                }
            }

            let response = request
                .json(&body)
                .send()
                .await
                .map_err(|e| Error::Http {
                    message: e.to_string(),
                })?;

            let status = response.status();

            if !status.is_success() {
                let response_text = response.text().await.unwrap_or_default();
                return Err(Error::ApiCallError {
                    message: format!("Speech generation failed: {}", status),
                    url: self.url.clone(),
                    status_code: Some(status.as_u16()),
                    response_body: Some(response_text),
                    is_retryable: status.as_u16() == 429 || status.as_u16() >= 500,
                    data: None,
                });
            }

            // Response is raw audio bytes
            let bytes = response
                .bytes()
                .await
                .map_err(|e| Error::Http {
                    message: e.to_string(),
                })?;

            Ok(SpeechGenerateResult {
                audio: AudioData::Binary {
                    data: bytes.to_vec(),
                    media_type: media_type.to_string(),
                },
                warnings,
                provider_metadata: None,
                response: SpeechResponseMetadata {
                    timestamp: chrono::Utc::now(),
                    model_id: self.model_id.clone(),
                    headers: None,
                },
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_speech_model_metadata() {
        let model = OpenAISpeechModel::new(
            "tts-1".into(),
            "openai.speech".into(),
            "https://api.openai.com/v1",
            HashMap::new(),
        );
        assert_eq!(model.provider(), "openai.speech");
        assert_eq!(model.model_id(), "tts-1");
    }
}
