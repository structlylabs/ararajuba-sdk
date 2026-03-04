//! The `generate_video` high-level function.

use crate::error::Error;
use crate::types::call_warning::CallWarning;
use ararajuba_provider::video_model::v4::video_model_v4::VideoModelV4;
use ararajuba_provider::video_model::v4::{
    VideoCallOptions, VideoData, VideoGenerateResult, VideoImageInput,
    VideoResponseMetadata,
};
use std::collections::HashMap;

/// Options for `generate_video()`.
pub struct GenerateVideoOptions {
    /// The video model to use.
    pub model: Box<dyn VideoModelV4>,
    /// Text prompt describing the desired video.
    pub prompt: Option<String>,
    /// Input image for image-to-video generation.
    pub image: Option<VideoImageInput>,
    /// Desired video size, e.g. "1280x720".
    pub size: Option<String>,
    /// Aspect ratio, e.g. "16:9".
    pub aspect_ratio: Option<String>,
    /// Desired duration in seconds.
    pub duration_seconds: Option<f64>,
    /// Random seed.
    pub seed: Option<u64>,
    /// Provider-specific options.
    pub provider_options: Option<HashMap<String, HashMap<String, serde_json::Value>>>,
    /// Additional headers.
    pub headers: Option<HashMap<String, String>>,
}

/// Result of `generate_video()`.
pub struct GenerateVideoResult {
    /// The generated video.
    pub video: VideoData,
    /// Warnings from the model.
    pub warnings: Vec<CallWarning>,
    /// Response metadata.
    pub response: VideoResponseMetadata,
}

/// Generate a video using a video model.
pub async fn generate_video(options: GenerateVideoOptions) -> Result<GenerateVideoResult, Error> {
    let _span = tracing::info_span!(
        "generate_video",
        aspect_ratio = options.aspect_ratio.as_deref().unwrap_or("default"),
        duration_seconds = ?options.duration_seconds,
    )
    .entered();

    let call_options = VideoCallOptions {
        prompt: options.prompt,
        image: options.image,
        size: options.size,
        aspect_ratio: options.aspect_ratio,
        duration_seconds: options.duration_seconds,
        seed: options.seed,
        provider_options: options.provider_options.unwrap_or_default(),
        headers: options.headers,
    };

    let result: VideoGenerateResult = options.model.do_generate(&call_options).await?;

    Ok(GenerateVideoResult {
        video: result.video,
        warnings: result.warnings.into_iter().map(CallWarning::from).collect(),
        response: result.response,
    })
}
