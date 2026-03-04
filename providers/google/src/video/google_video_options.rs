//! Google Video model options.

use serde::{Deserialize, Serialize};

/// Google-specific options for Video (Veo) models.
///
/// Pass under `provider_options["google"]` as JSON:
/// ```json
/// {
///     "aspect_ratio": "16:9",
///     "person_generation": "allow_adult",
///     "number_of_videos": 2,
///     "duration_seconds": 8
/// }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GoogleVideoOptions {
    /// Aspect ratio: "9:16" | "16:9".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,

    /// Person generation policy: "dont_allow" | "allow_adult".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub person_generation: Option<String>,

    /// Number of videos to generate (1–4).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number_of_videos: Option<u32>,

    /// Duration in seconds: 5–8 (veo-2) or 6–8 (veo-3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<u32>,
}
