//! OpenAI audio transcription model — whisper-1.
//!
//! Implements `TranscriptionModel` against the OpenAI Audio Transcriptions API.
//!
//! **Endpoint:** `POST /v1/audio/transcriptions` (multipart/form-data)
//!
//! **Supported models:** whisper-1

use crate::transcription::openai_transcription_options::parse_openai_transcription_options;
use async_trait::async_trait;
use ararajuba_provider::errors::Error;
use ararajuba_provider::transcription_model::v4::{
    TranscriptionAudioInput, TranscriptionCallOptions,
    TranscriptionResponseMetadata, TranscriptionResult, TranscriptionSegment,
};
use ararajuba_provider::transcription_model::v4::transcription_model_v4::TranscriptionModelV4;
use reqwest::multipart::{Form, Part};
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;

/// An OpenAI audio transcription model.
pub struct OpenAITranscriptionModel {
    model_id: String,
    provider_name: String,
    url: String,
    headers: HashMap<String, String>,
    client: Client,
}

impl OpenAITranscriptionModel {
    pub fn new(
        model_id: String,
        provider_name: String,
        base_url: &str,
        headers: HashMap<String, String>,
    ) -> Self {
        Self {
            model_id,
            provider_name,
            url: format!(
                "{}/audio/transcriptions",
                base_url.trim_end_matches('/')
            ),
            headers,
            client: Client::new(),
        }
    }
}

#[async_trait]
impl TranscriptionModelV4 for OpenAITranscriptionModel {
    fn provider(&self) -> &str {
        &self.provider_name
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }

    async fn do_transcribe(
        &self,
        options: &TranscriptionCallOptions,
    ) -> Result<TranscriptionResult, Error> {
            let openai_opts = parse_openai_transcription_options(
                &options.provider_options,
                &self.provider_name,
            );
            let warnings = Vec::new();

            // Use verbose_json to get timestamps and segments
            let response_format = openai_opts
                .response_format
                .as_deref()
                .unwrap_or("verbose_json");

            // Build multipart form
            let audio_part = match &options.audio {
                TranscriptionAudioInput::Base64 { data, media_type } => {
                    let bytes = base64::Engine::decode(
                        &base64::engine::general_purpose::STANDARD,
                        data,
                    )
                    .map_err(|e| Error::InvalidArgument {
                        parameter: "audio".into(),
                        message: format!("Failed to decode base64 audio: {}", e),
                    })?;

                    let extension = media_type_to_extension(media_type);
                    Part::bytes(bytes)
                        .file_name(format!("audio.{}", extension))
                        .mime_str(media_type)
                        .map_err(|e| Error::Other {
                            message: format!("Invalid media type: {}", e),
                        })?
                }
                TranscriptionAudioInput::Url { url } => {
                    // Download the audio first
                    let response = self
                        .client
                        .get(url)
                        .send()
                        .await
                        .map_err(|e| Error::Http {
                            message: format!("Failed to download audio: {}", e),
                        })?;
                    let bytes = response.bytes().await.map_err(|e| Error::Http {
                        message: format!("Failed to read audio bytes: {}", e),
                    })?;

                    Part::bytes(bytes.to_vec())
                        .file_name("audio.mp3")
                        .mime_str("audio/mpeg")
                        .map_err(|e| Error::Other {
                            message: format!("Invalid media type: {}", e),
                        })?
                }
            };

            let mut form = Form::new()
                .part("file", audio_part)
                .text("model", self.model_id.clone())
                .text("response_format", response_format.to_string());

            // Language hint
            if let Some(ref language) = options.language {
                form = form.text("language", language.clone());
            }

            // Prompt/context
            if let Some(ref prompt) = options.prompt {
                form = form.text("prompt", prompt.clone());
            }

            // Temperature
            if let Some(temp) = openai_opts.temperature {
                form = form.text("temperature", temp.to_string());
            }

            // Timestamp granularities
            if let Some(ref granularities) = openai_opts.timestamp_granularities {
                for g in granularities {
                    form = form.text("timestamp_granularities[]", g.clone());
                }
            }

            // Build request (multipart — don't set content-type manually)
            let mut request = self.client.post(&self.url);
            for (k, v) in &self.headers {
                // Skip content-type header — multipart form sets its own
                if k.to_lowercase() != "content-type" {
                    request = request.header(k, v);
                }
            }
            if let Some(ref extra_headers) = options.headers {
                for (k, v) in extra_headers {
                    request = request.header(k, v);
                }
            }

            let response = request
                .multipart(form)
                .send()
                .await
                .map_err(|e| Error::Http {
                    message: e.to_string(),
                })?;

            let status = response.status();
            let response_text = response.text().await.map_err(|e| Error::Http {
                message: e.to_string(),
            })?;

            if !status.is_success() {
                return Err(Error::ApiCallError {
                    message: format!("Transcription failed: {}", status),
                    url: self.url.clone(),
                    status_code: Some(status.as_u16()),
                    response_body: Some(response_text),
                    is_retryable: status.as_u16() == 429 || status.as_u16() >= 500,
                    data: None,
                });
            }

            // Parse based on response format
            if response_format == "text" || response_format == "srt" || response_format == "vtt" {
                return Ok(TranscriptionResult {
                    text: response_text,
                    segments: None,
                    language: None,
                    duration_seconds: None,
                    warnings,
                    provider_metadata: None,
                    response: TranscriptionResponseMetadata {
                        timestamp: chrono::Utc::now(),
                        model_id: self.model_id.clone(),
                        headers: None,
                    },
                });
            }

            let raw: Value =
                serde_json::from_str(&response_text).map_err(|e| Error::JsonParse {
                    message: e.to_string(),
                    text: response_text.clone(),
                })?;

            let text = raw
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let language = raw
                .get("language")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let duration_seconds = raw.get("duration").and_then(|v| v.as_f64());

            // Parse segments (verbose_json only)
            let segments = raw
                .get("segments")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|s| {
                            let text = s.get("text")?.as_str()?.to_string();
                            let start = s.get("start")?.as_f64()?;
                            let end = s.get("end")?.as_f64()?;
                            Some(TranscriptionSegment { text, start, end })
                        })
                        .collect()
                });

            Ok(TranscriptionResult {
                text,
                segments,
                language,
                duration_seconds,
                warnings,
                provider_metadata: None,
                response: TranscriptionResponseMetadata {
                    timestamp: chrono::Utc::now(),
                    model_id: self.model_id.clone(),
                    headers: None,
                },
            })
    }
}

/// Map MIME type to file extension.
fn media_type_to_extension(media_type: &str) -> &str {
    match media_type {
        "audio/mpeg" | "audio/mp3" => "mp3",
        "audio/wav" => "wav",
        "audio/flac" => "flac",
        "audio/ogg" => "ogg",
        "audio/webm" => "webm",
        "audio/mp4" | "audio/m4a" => "m4a",
        _ => "mp3",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_transcription_model_metadata() {
        let model = OpenAITranscriptionModel::new(
            "whisper-1".into(),
            "openai.transcription".into(),
            "https://api.openai.com/v1",
            HashMap::new(),
        );
        assert_eq!(model.provider(), "openai.transcription");
        assert_eq!(model.model_id(), "whisper-1");
    }

    #[test]
    fn test_media_type_to_extension() {
        assert_eq!(media_type_to_extension("audio/mpeg"), "mp3");
        assert_eq!(media_type_to_extension("audio/wav"), "wav");
        assert_eq!(media_type_to_extension("audio/flac"), "flac");
    }
}
