//! The ImageModel trait and associated types.

use crate::errors::Error;
use crate::shared::{Headers, ProviderMetadata, ProviderOptions, Warning};
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};

/// Options passed to `do_generate` for image models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageCallOptions {
    /// The text prompt describing the desired image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// Number of images to generate.
    pub n: u32,
    /// Image size, e.g. "1024x1024".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    /// Aspect ratio, e.g. "16:9".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
    /// Random seed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    /// Input files for image-to-image generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<ImageFile>>,
    /// Mask file for inpainting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask: Option<ImageFile>,
    /// Provider-specific options.
    pub provider_options: ProviderOptions,
    /// Additional headers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Headers>,
}

/// An image file input.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ImageFile {
    #[serde(rename = "file")]
    File {
        media_type: String,
        /// Base64-encoded data.
        data: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        provider_options: Option<ProviderMetadata>,
    },
    #[serde(rename = "url")]
    Url {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        provider_options: Option<ProviderMetadata>,
    },
}

/// Result of an image generation call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenerateResult {
    /// Generated images as base64-encoded strings.
    pub images: Vec<String>,
    /// Warnings.
    pub warnings: Vec<Warning>,
    /// Provider-specific metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_metadata: Option<ProviderMetadata>,
    /// Response metadata.
    pub response: ImageResponseMetadata,
    /// Usage information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<ImageUsage>,
}

/// Response metadata for image generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageResponseMetadata {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub model_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Headers>,
}

/// Usage for image generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUsage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u64>,
}

/// Trait for image generation models.
pub trait ImageModel: Send + Sync {
    fn specification_version(&self) -> &'static str {
        "v3"
    }

    fn provider(&self) -> &str;
    fn model_id(&self) -> &str;

    /// Maximum number of images per call, if limited.
    fn max_images_per_call(&self) -> Option<usize>;

    /// Generate images.
    fn do_generate<'a>(
        &'a self,
        options: &'a ImageCallOptions,
    ) -> BoxFuture<'a, Result<ImageGenerateResult, Error>>;
}
