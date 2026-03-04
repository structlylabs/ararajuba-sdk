//! The v4 TranscriptionModel trait — async-native.

use super::TranscriptionCallOptions;
use super::TranscriptionResult;
use crate::errors::Error;
use async_trait::async_trait;

/// v4 Transcription (speech-to-text) model trait.
#[async_trait]
pub trait TranscriptionModelV4: Send + Sync {
    fn specification_version(&self) -> &'static str {
        "v4"
    }

    fn provider(&self) -> &str;
    fn model_id(&self) -> &str;

    /// Transcribe audio to text.
    async fn do_transcribe(
        &self,
        options: &TranscriptionCallOptions,
    ) -> Result<TranscriptionResult, Error>;
}
