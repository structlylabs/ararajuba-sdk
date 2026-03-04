//! The v4 ImageModel trait — async-native.

use super::ImageCallOptions;
use super::ImageGenerateResult;
use crate::errors::Error;
use async_trait::async_trait;

/// v4 Image model trait.
#[async_trait]
pub trait ImageModelV4: Send + Sync {
    fn specification_version(&self) -> &'static str {
        "v4"
    }

    fn provider(&self) -> &str;
    fn model_id(&self) -> &str;

    /// Maximum number of images per call, if limited.
    fn max_images_per_call(&self) -> Option<usize>;

    /// Generate images.
    async fn do_generate(&self, options: &ImageCallOptions) -> Result<ImageGenerateResult, Error>;
}
