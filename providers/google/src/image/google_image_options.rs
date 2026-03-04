//! Google Image model options.

use serde::{Deserialize, Serialize};

/// Google-specific options for Image (Imagen) models.
///
/// Pass under `provider_options["google"]` as JSON:
/// ```json
/// {
///     "aspect_ratio": "16:9",
///     "number_of_images": 2,
///     "image_size": "1K",
///     "negative_prompt": "blurry, low quality",
///     "person_generation": "allow_adult"
/// }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GoogleImageOptions {
    /// Aspect ratio: "1:1" | "3:4" | "4:3" | "9:16" | "16:9".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,

    /// Number of images to generate (1–4, default 4).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number_of_images: Option<u32>,

    /// Output image size: "512" | "1K" | "2K" | "4K".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_size: Option<String>,

    /// Negative prompt — things to avoid in the image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,

    /// Person generation policy: "dont_allow" | "allow_adult" | "allow_all".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub person_generation: Option<String>,
}
