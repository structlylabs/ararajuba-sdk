//! The v4 SpeechModel trait — async-native.

use super::SpeechCallOptions;
use super::SpeechGenerateResult;
use crate::errors::Error;
use async_trait::async_trait;

/// v4 Speech (text-to-speech) model trait.
#[async_trait]
pub trait SpeechModelV4: Send + Sync {
    fn specification_version(&self) -> &'static str {
        "v4"
    }

    fn provider(&self) -> &str;
    fn model_id(&self) -> &str;

    /// Generate speech audio from text.
    async fn do_generate(
        &self,
        options: &SpeechCallOptions,
    ) -> Result<SpeechGenerateResult, Error>;
}
