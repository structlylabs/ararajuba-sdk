//! The VideoModel trait and associated types.

use crate::errors::Error;
use crate::shared::{Headers, ProviderMetadata, ProviderOptions, Warning};
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};

/// Options passed to `do_generate` for video models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoCallOptions {
    /// Text prompt describing the desired video.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// Input image for image-to-video generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<VideoImageInput>,
    /// Desired video size, e.g. "1280x720".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    /// Aspect ratio, e.g. "16:9".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
    /// Desired duration in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<f64>,
    /// Random seed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    /// Provider-specific options.
    pub provider_options: ProviderOptions,
    /// Additional headers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Headers>,
}

/// Image input for video generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum VideoImageInput {
    /// Base64-encoded image.
    #[serde(rename = "base64")]
    Base64 { data: String, media_type: String },
    /// URL pointing to an image.
    #[serde(rename = "url")]
    Url { url: String },
}

/// Result of a video generation call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoGenerateResult {
    /// The generated video.
    pub video: VideoData,
    /// Warnings.
    pub warnings: Vec<Warning>,
    /// Provider-specific metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_metadata: Option<ProviderMetadata>,
    /// Response metadata.
    pub response: VideoResponseMetadata,
}

/// Video data output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum VideoData {
    /// Base64-encoded video data.
    #[serde(rename = "base64")]
    Base64 { data: String, media_type: String },
    /// URL pointing to the generated video.
    #[serde(rename = "url")]
    Url { url: String },
}

/// Response metadata for video generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoResponseMetadata {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub model_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Headers>,
}

/// Trait for video generation models.
pub trait VideoModel: Send + Sync {
    fn specification_version(&self) -> &'static str {
        "v3"
    }

    fn provider(&self) -> &str;
    fn model_id(&self) -> &str;

    /// Generate a video.
    fn do_generate<'a>(
        &'a self,
        options: &'a VideoCallOptions,
    ) -> BoxFuture<'a, Result<VideoGenerateResult, Error>>;
}
