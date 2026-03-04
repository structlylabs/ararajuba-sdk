//! The `embed` and `embed_many` functions.

use super::options::{EmbedManyOptions, EmbedOptions};
use super::result::{EmbedManyResult, EmbedResult};
use crate::error::Error;
use ararajuba_provider::embedding_model::v4::call_options::EmbeddingCallOptions;

/// Embed a single text value.
pub async fn embed(options: EmbedOptions) -> Result<EmbedResult, Error> {
    let _span = tracing::info_span!(
        "embed",
        model = %options.model.model_id(),
        provider = %options.model.provider(),
    )
    .entered();

    let call_options = EmbeddingCallOptions {
        values: vec![options.value],
        provider_options: None,
        headers: options.call_settings.headers.clone(),
    };

    let result = options.model.do_embed(&call_options).await?;

    let embedding = result
        .embeddings
        .into_iter()
        .next()
        .ok_or_else(|| Error::Other {
            message: "No embedding returned".to_string(),
        })?;

    Ok(EmbedResult {
        embedding,
        usage: result.usage.map(|u| u.tokens),
    })
}

/// Embed multiple text values, automatically batching if needed.
pub async fn embed_many(options: EmbedManyOptions) -> Result<EmbedManyResult, Error> {
    let _span = tracing::info_span!(
        "embed_many",
        model = %options.model.model_id(),
        provider = %options.model.provider(),
        count = options.values.len(),
    )
    .entered();

    let max_per_call = options.model.max_embeddings_per_call();
    let values = options.values;

    if values.is_empty() {
        return Ok(EmbedManyResult {
            embeddings: Vec::new(),
            usage: None,
        });
    }

    // If no limit or all values fit in one call
    if max_per_call.is_none() || values.len() <= max_per_call.unwrap() {
        let call_options = EmbeddingCallOptions {
            values,
            provider_options: None,
            headers: options.call_settings.headers.clone(),
        };
        let result = options.model.do_embed(&call_options).await?;
        return Ok(EmbedManyResult {
            embeddings: result.embeddings,
            usage: result.usage.map(|u| u.tokens),
        });
    }

    // Batch the values
    let max = max_per_call.unwrap();
    let mut all_embeddings = Vec::new();
    let mut total_tokens: u64 = 0;

    for chunk in values.chunks(max) {
        let call_options = EmbeddingCallOptions {
            values: chunk.to_vec(),
            provider_options: None,
            headers: options.call_settings.headers.clone(),
        };
        let result = options.model.do_embed(&call_options).await?;
        all_embeddings.extend(result.embeddings);
        if let Some(usage) = result.usage {
            total_tokens += usage.tokens;
        }
    }

    Ok(EmbedManyResult {
        embeddings: all_embeddings,
        usage: if total_tokens > 0 {
            Some(total_tokens)
        } else {
            None
        },
    })
}
