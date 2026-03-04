//! Capability traits for v4 language models.
//!
//! These traits extend `LanguageModelV4` and advertise specific capabilities.
//! Consumers can use trait-object downcasting or generic bounds to check
//! capabilities at runtime or compile time.
//!
//! # Example
//! ```ignore
//! use ararajuba_provider::language_model::v4::capabilities::*;
//!
//! fn maybe_enable_reasoning(model: &dyn LanguageModelV4) {
//!     // Runtime check via Any downcast (trait objects)
//!     if let Some(reasoning) = model_supports_reasoning(model) {
//!         println!("Reasoning config: {:?}", reasoning.reasoning_config());
//!     }
//! }
//! ```

use super::language_model_v4::LanguageModelV4;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Capability: Reasoning (extended thinking / chain-of-thought)
// ---------------------------------------------------------------------------

/// Configuration for reasoning / extended thinking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningConfig {
    /// Whether reasoning is enabled by default.
    pub enabled: bool,
    /// Default effort level: "low", "medium", "high".
    pub default_effort: Option<String>,
    /// Maximum budget tokens for reasoning.
    pub max_reasoning_tokens: Option<u32>,
}

/// A language model that supports reasoning / extended thinking.
pub trait SupportsReasoning: LanguageModelV4 {
    /// Get the reasoning configuration for this model.
    fn reasoning_config(&self) -> ReasoningConfig;
}

// ---------------------------------------------------------------------------
// Capability: Caching (prompt caching, context caching)
// ---------------------------------------------------------------------------

/// Configuration for prompt/context caching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Whether automatic caching is supported.
    pub supports_auto_cache: bool,
    /// Whether explicit cache breakpoints are supported.
    pub supports_cache_control: bool,
    /// Maximum cacheable token window.
    pub max_cache_tokens: Option<u64>,
}

/// A language model that supports prompt/context caching.
pub trait SupportsCaching: LanguageModelV4 {
    /// Get the cache configuration.
    fn cache_config(&self) -> CacheConfig;
}

// ---------------------------------------------------------------------------
// Capability: Tool calling
// ---------------------------------------------------------------------------

/// A language model that supports tool/function calling.
pub trait SupportsToolCalling: LanguageModelV4 {
    /// Maximum number of tools that can be passed, if limited.
    fn max_tools(&self) -> Option<usize>;

    /// Whether the model supports parallel tool calls.
    fn supports_parallel_calls(&self) -> bool;

    /// Whether the model supports strict JSON Schema for tool inputs.
    fn supports_strict_schemas(&self) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------
// Capability: Image/file input
// ---------------------------------------------------------------------------

/// Supported image formats.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFormat {
    Jpeg,
    Png,
    Gif,
    Webp,
    Svg,
}

/// A language model that supports image/file inputs in prompts.
pub trait SupportsImages: LanguageModelV4 {
    /// Supported image input formats.
    fn supported_image_formats(&self) -> Vec<ImageFormat>;

    /// Maximum image dimensions, if limited.
    fn max_image_dimensions(&self) -> Option<(u32, u32)> {
        None
    }

    /// Maximum file size in bytes, if limited.
    fn max_file_size(&self) -> Option<u64> {
        None
    }
}

// ---------------------------------------------------------------------------
// Capability: Structured output (JSON mode / JSON Schema)
// ---------------------------------------------------------------------------

/// A language model that supports structured JSON output.
pub trait SupportsStructuredOutput: LanguageModelV4 {
    /// Whether the model supports JSON mode (unstructured JSON).
    fn supports_json_mode(&self) -> bool;

    /// Whether the model supports JSON Schema constraints.
    fn supports_json_schema(&self) -> bool;
}

// ---------------------------------------------------------------------------
// Capability: Computer use (browser, desktop, etc.)
// ---------------------------------------------------------------------------

/// A language model that supports computer use tools.
pub trait SupportsComputerUse: LanguageModelV4 {
    /// Available computer use tool types.
    fn computer_use_tools(&self) -> Vec<String>;
}
