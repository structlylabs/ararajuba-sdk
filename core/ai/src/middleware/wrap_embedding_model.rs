//! Embedding model middleware — wraps an `EmbeddingModelV4` to intercept calls.

use ararajuba_provider::embedding_model::v4::call_options::EmbeddingCallOptions;
use ararajuba_provider::embedding_model::v4::embedding_model_v4::EmbeddingModelV4;
use ararajuba_provider::embedding_model::v4::result::EmbeddingResult;
use ararajuba_provider::errors::Error;
use async_trait::async_trait;
use futures::future::BoxFuture;
use std::sync::Arc;

/// Reference to the wrapped embedding model.
pub struct EmbeddingMiddlewareModelRef {
    pub provider: String,
    pub model_id: String,
}

/// Type alias for the doEmbed function passed to middleware.
pub type DoEmbed = Arc<
    dyn Fn(EmbeddingCallOptions) -> BoxFuture<'static, Result<EmbeddingResult, Error>>
        + Send
        + Sync,
>;

/// Middleware for embedding models.
pub struct EmbeddingModelMiddleware {
    /// Override the provider name.
    pub override_provider:
        Option<Box<dyn Fn(&EmbeddingMiddlewareModelRef) -> String + Send + Sync>>,
    /// Override the model ID.
    pub override_model_id:
        Option<Box<dyn Fn(&EmbeddingMiddlewareModelRef) -> String + Send + Sync>>,
    /// Transform call options before the model sees them.
    pub transform_params: Option<
        Box<
            dyn Fn(
                    EmbeddingCallOptions,
                    EmbeddingMiddlewareModelRef,
                ) -> BoxFuture<'static, Result<EmbeddingCallOptions, Error>>
                + Send
                + Sync,
        >,
    >,
    /// Wrap the embed call.
    pub wrap_embed: Option<
        Box<
            dyn Fn(
                    DoEmbed,
                    EmbeddingCallOptions,
                    EmbeddingMiddlewareModelRef,
                ) -> BoxFuture<'static, Result<EmbeddingResult, Error>>
                + Send
                + Sync,
        >,
    >,
}

impl Default for EmbeddingModelMiddleware {
    fn default() -> Self {
        Self {
            override_provider: None,
            override_model_id: None,
            transform_params: None,
            wrap_embed: None,
        }
    }
}

/// Wrap an embedding model with middleware.
pub fn wrap_embedding_model(
    model: Box<dyn EmbeddingModelV4>,
    middleware: EmbeddingModelMiddleware,
) -> Box<dyn EmbeddingModelV4> {
    Box::new(WrappedEmbeddingModel {
        model: Arc::from(model),
        middleware: Arc::new(middleware),
    })
}

struct WrappedEmbeddingModel {
    model: Arc<dyn EmbeddingModelV4>,
    middleware: Arc<EmbeddingModelMiddleware>,
}

impl WrappedEmbeddingModel {
    fn model_ref(&self) -> EmbeddingMiddlewareModelRef {
        EmbeddingMiddlewareModelRef {
            provider: self.model.provider().to_string(),
            model_id: self.model.model_id().to_string(),
        }
    }

    fn effective_provider(&self) -> String {
        match &self.middleware.override_provider {
            Some(f) => f(&self.model_ref()),
            None => self.model.provider().to_string(),
        }
    }

    fn effective_model_id(&self) -> String {
        match &self.middleware.override_model_id {
            Some(f) => f(&self.model_ref()),
            None => self.model.model_id().to_string(),
        }
    }
}

#[async_trait]
impl EmbeddingModelV4 for WrappedEmbeddingModel {
    fn provider(&self) -> &str {
        Box::leak(self.effective_provider().into_boxed_str())
    }

    fn model_id(&self) -> &str {
        Box::leak(self.effective_model_id().into_boxed_str())
    }

    fn max_embeddings_per_call(&self) -> Option<usize> {
        self.model.max_embeddings_per_call()
    }

    fn supports_parallel_calls(&self) -> bool {
        self.model.supports_parallel_calls()
    }

    async fn do_embed(
        &self,
        options: &EmbeddingCallOptions,
    ) -> Result<EmbeddingResult, Error> {
        let model = Arc::clone(&self.model);
        let middleware = Arc::clone(&self.middleware);
        let options = options.clone();

        let model_ref = EmbeddingMiddlewareModelRef {
            provider: model.provider().to_string(),
            model_id: model.model_id().to_string(),
        };

        let params = match &middleware.transform_params {
            Some(transform) => {
                let mr = EmbeddingMiddlewareModelRef {
                    provider: model_ref.provider.clone(),
                    model_id: model_ref.model_id.clone(),
                };
                transform(options, mr).await?
            }
            None => options,
        };

        let model_for_embed = Arc::clone(&model);
        let do_embed: DoEmbed = Arc::new(move |opts| {
            let m = Arc::clone(&model_for_embed);
            Box::pin(async move { m.do_embed(&opts).await })
        });

        match &middleware.wrap_embed {
            Some(wrap) => wrap(do_embed, params, model_ref).await,
            None => do_embed(params).await,
        }
    }
}
