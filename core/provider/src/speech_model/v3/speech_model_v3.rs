//! The SpeechModel trait and associated types.

use crate::errors::Error;
use crate::shared::{Headers, ProviderMetadata, ProviderOptions, Warning};
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};

/// Options passed to `do_generate` for speech models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechCallOptions {
    /// The text to synthesize into speech.
    pub text: String,
    /// Voice identifier (provider-specific, e.g. "alloy", "nova").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,
    /// Output audio format (e.g. "mp3", "opus", "wav").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
    /// Speech speed multiplier (1.0 = normal).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f64>,
    /// Additional instructions for the speech model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    /// Provider-specific options.
    pub provider_options: ProviderOptions,
    /// Additional headers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Headers>,
}

/// Result of a speech generation call.
#[derive(Debug, Clone)]
pub struct SpeechGenerateResult {
    /// The generated audio data.
    pub audio: AudioData,
    /// Warnings.
    pub warnings: Vec<Warning>,
    /// Provider-specific metadata.
    pub provider_metadata: Option<ProviderMetadata>,
    /// Response metadata.
    pub response: SpeechResponseMetadata,
}

/// Audio data output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AudioData {
    /// Base64-encoded audio.
    #[serde(rename = "base64")]
    Base64 {
        data: String,
        media_type: String,
    },
    /// Raw audio bytes.
    #[serde(rename = "binary")]
    Binary {
        #[serde(with = "serde_bytes_compat")]
        data: Vec<u8>,
        media_type: String,
    },
}

/// Helper for Vec<u8> serde.
mod serde_bytes_compat {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(data: &[u8], s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::Error;
        use std::io::Write;
        let mut buf = Vec::new();
        let engine = base64_compat_engine();
        {
            let mut encoder = Base64Writer { engine: &engine, buf: &mut buf };
            encoder.write_all(data).map_err(S::Error::custom)?;
        }
        String::from_utf8(buf).map_err(S::Error::custom)?.serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(d)?;
        // Simple base64 decode using lookup table
        base64_decode(&s).map_err(serde::de::Error::custom)
    }

    // Minimal base64 helpers (avoiding external dependency in spec crate)
    struct Base64Writer<'a> { engine: &'a [u8; 64], buf: &'a mut Vec<u8> }

    impl std::io::Write for Base64Writer<'_> {
        fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
            for chunk in data.chunks(3) {
                let b0 = chunk[0] as usize;
                let b1 = if chunk.len() > 1 { chunk[1] as usize } else { 0 };
                let b2 = if chunk.len() > 2 { chunk[2] as usize } else { 0 };
                self.buf.push(self.engine[b0 >> 2]);
                self.buf.push(self.engine[((b0 & 3) << 4) | (b1 >> 4)]);
                if chunk.len() > 1 {
                    self.buf.push(self.engine[((b1 & 0xf) << 2) | (b2 >> 6)]);
                } else {
                    self.buf.push(b'=');
                }
                if chunk.len() > 2 {
                    self.buf.push(self.engine[b2 & 0x3f]);
                } else {
                    self.buf.push(b'=');
                }
            }
            Ok(data.len())
        }
        fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
    }

    fn base64_compat_engine() -> [u8; 64] {
        let mut table = [0u8; 64];
        for (i, &c) in b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/".iter().enumerate() {
            table[i] = c;
        }
        table
    }

    fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
        let input = input.as_bytes();
        let mut out = Vec::with_capacity(input.len() * 3 / 4);
        let decode = |c: u8| -> Result<u8, String> {
            match c {
                b'A'..=b'Z' => Ok(c - b'A'),
                b'a'..=b'z' => Ok(c - b'a' + 26),
                b'0'..=b'9' => Ok(c - b'0' + 52),
                b'+' => Ok(62),
                b'/' => Ok(63),
                _ => Err(format!("Invalid base64 char: {c}")),
            }
        };
        let filtered: Vec<u8> = input.iter().copied().filter(|&c| c != b'=' && c != b'\n' && c != b'\r').collect();
        for chunk in filtered.chunks(4) {
            if chunk.len() >= 2 {
                let a = decode(chunk[0])?;
                let b = decode(chunk[1])?;
                out.push((a << 2) | (b >> 4));
                if chunk.len() >= 3 {
                    let c = decode(chunk[2])?;
                    out.push(((b & 0xf) << 4) | (c >> 2));
                    if chunk.len() >= 4 {
                        let d = decode(chunk[3])?;
                        out.push(((c & 3) << 6) | d);
                    }
                }
            }
        }
        Ok(out)
    }
}

/// Response metadata for speech generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechResponseMetadata {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub model_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Headers>,
}

/// Trait for text-to-speech models.
pub trait SpeechModel: Send + Sync {
    fn specification_version(&self) -> &'static str {
        "v3"
    }

    fn provider(&self) -> &str;
    fn model_id(&self) -> &str;

    /// Generate speech audio from text.
    fn do_generate<'a>(
        &'a self,
        options: &'a SpeechCallOptions,
    ) -> BoxFuture<'a, Result<SpeechGenerateResult, Error>>;
}
