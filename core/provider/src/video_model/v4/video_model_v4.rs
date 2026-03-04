//! The v4 VideoModel trait — async-native.

use super::VideoCallOptions;
use super::VideoGenerateResult;
use crate::errors::Error;
use async_trait::async_trait;

/// v4 Video generation model trait.
#[async_trait]
pub trait VideoModelV4: Send + Sync {
    fn specification_version(&self) -> &'static str {
        "v4"
    }

    fn provider(&self) -> &str;
    fn model_id(&self) -> &str;

    /// Generate a video.
    async fn do_generate(
        &self,
        options: &VideoCallOptions,
    ) -> Result<VideoGenerateResult, Error>;
}
