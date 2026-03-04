//! v4 Reranking Model interface — async-native.

pub mod reranking_model_v4;

// Re-export all types from v3 (unchanged)
pub use super::v3::reranking_model_v3::{
    RankedDocument, RerankCallOptions, RerankResponseMetadata, RerankResult, RerankUsage,
};
