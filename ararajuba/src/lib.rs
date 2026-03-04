//! # Ararajuba
//!
//! A unified Rust SDK for working with large language models.
//!
//! This crate re-exports [`ararajuba_core`] (always available) and
//! feature-gated provider crates.
//!
//! # Usage
//!
//! ```toml
//! [dependencies]
//! ararajuba = { version = "0.1", features = ["openai", "anthropic"] }
//! ```
//!
//! ```rust,ignore
//! use ararajuba::{generate_text, GenerateTextOptions, Prompt};
//! use ararajuba::openai::openai;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let result = generate_text(GenerateTextOptions {
//!         model: openai().language_model_v4("gpt-4o").unwrap(),
//!         prompt: Prompt::simple("Hello!"),
//!         ..Default::default()
//!     }).await?;
//!
//!     println!("{}", result.text);
//!     Ok(())
//! }
//! ```

// Re-export core (always available)
pub use ararajuba_core::*;
pub use ararajuba_provider as provider;

// Re-export providers (feature-gated)
#[cfg(feature = "openai")]
pub use ararajuba_openai as openai;

#[cfg(feature = "anthropic")]
pub use ararajuba_anthropic as anthropic;

#[cfg(feature = "google")]
pub use ararajuba_google as google;

#[cfg(feature = "deepseek")]
pub use ararajuba_deepseek as deepseek;

#[cfg(feature = "mcp")]
pub use ararajuba_mcp as mcp;

#[cfg(feature = "coding-tools")]
pub use ararajuba_tools_coding as coding_tools;
