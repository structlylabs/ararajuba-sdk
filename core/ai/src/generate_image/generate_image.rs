//! The `generate_image` high-level function.

use crate::error::Error;
use crate::types::call_warning::CallWarning;
use ararajuba_provider::image_model::v4::image_model_v4::ImageModelV4;
use ararajuba_provider::image_model::v4::{ImageCallOptions, ImageGenerateResult};
use std::collections::HashMap;

/// Options for `generate_image()`.
pub struct GenerateImageOptions {
    /// The image model to use.
    pub model: Box<dyn ImageModelV4>,
    /// Text prompt describing the desired image.
    pub prompt: String,
    /// Number of images to generate (default: 1).
    pub n: u32,
    /// Image size, e.g. "1024x1024".
    pub size: Option<String>,
    /// Aspect ratio, e.g. "16:9".
    pub aspect_ratio: Option<String>,
    /// Random seed.
    pub seed: Option<u64>,
    /// Provider-specific options.
    pub provider_options: Option<HashMap<String, HashMap<String, serde_json::Value>>>,
    /// Additional headers.
    pub headers: Option<HashMap<String, String>>,
}

/// Result of `generate_image()`.
pub struct GenerateImageResult {
    /// Generated images as base64-encoded strings.
    pub images: Vec<String>,
    /// Warnings from the model.
    pub warnings: Vec<CallWarning>,
    /// Provider-specific response metadata.
    pub response: ararajuba_provider::image_model::v4::ImageResponseMetadata,
}

/// Generate images from a text prompt using an image model.
pub async fn generate_image(options: GenerateImageOptions) -> Result<GenerateImageResult, Error> {
    let _span = tracing::info_span!(
        "generate_image",
        n = options.n,
    )
    .entered();

    let call_options = ImageCallOptions {
        prompt: Some(options.prompt),
        n: options.n,
        size: options.size,
        aspect_ratio: options.aspect_ratio,
        seed: options.seed,
        files: None,
        mask: None,
        provider_options: options.provider_options.unwrap_or_default(),
        headers: options.headers,
    };

    let result: ImageGenerateResult = options.model.do_generate(&call_options).await?;

    Ok(GenerateImageResult {
        images: result.images,
        warnings: result.warnings.into_iter().map(CallWarning::from).collect(),
        response: result.response,
    })
}
