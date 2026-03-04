//! Model registry — resolve models by `"provider:model-name"` string IDs.

mod model_registry;

pub use model_registry::{CustomProvider, ModelRegistry};
