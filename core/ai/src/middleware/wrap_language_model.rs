//! Language model middleware — wraps a model to transform params/results.
//!
//! This module implements the full TS SDK middleware pattern with
//! `wrap_generate` / `wrap_stream` closures that receive both `do_generate`
//! and `do_stream` from the inner model, enabling cross-mode interception
//! (e.g., `simulate_streaming_middleware` can call `do_generate` when
//! `do_stream` is invoked).
//!
//! Supports chaining: pass a `Vec<LanguageModelMiddleware>` and they are
//! applied in reverse order (first middleware in the vec wraps outermost).

use ararajuba_provider::errors::Error;
use ararajuba_provider::language_model::v4::call_options::CallOptions;
use ararajuba_provider::language_model::v4::generate_result::GenerateResult;
use ararajuba_provider::language_model::v4::stream_result::StreamResult;
use ararajuba_provider::LanguageModelV4;
use async_trait::async_trait;
use futures::future::BoxFuture;
use std::sync::Arc;

/// Reference to the wrapped model, passed to middleware closures.
pub struct MiddlewareModelRef {
    pub provider: String,
    pub model_id: String,
}

/// Type alias for the doGenerate function passed to wrapGenerate/wrapStream.
pub type DoGenerate = Arc<
    dyn Fn(CallOptions) -> BoxFuture<'static, Result<GenerateResult, Error>> + Send + Sync,
>;

/// Type alias for the doStream function passed to wrapGenerate/wrapStream.
pub type DoStream = Arc<
    dyn Fn(CallOptions) -> BoxFuture<'static, Result<StreamResult, Error>> + Send + Sync,
>;

/// Middleware that can intercept and transform language model calls.
///
/// All fields are optional. When not provided, the default behavior is
/// pass-through to the inner model.
pub struct LanguageModelMiddleware {
    /// Override the provider name reported by the model.
    pub override_provider: Option<Box<dyn Fn(&MiddlewareModelRef) -> String + Send + Sync>>,

    /// Override the model ID reported by the model.
    pub override_model_id: Option<Box<dyn Fn(&MiddlewareModelRef) -> String + Send + Sync>>,

    /// Transform call options before they are sent to the model.
    /// Async to allow middleware that fetches external config.
    pub transform_params: Option<
        Box<
            dyn Fn(CallOptions, MiddlewareModelRef) -> BoxFuture<'static, Result<CallOptions, Error>>
                + Send
                + Sync,
        >,
    >,

    /// Wrap the generate call. Receives `doGenerate`, `doStream`, params, and model ref.
    /// The middleware can call either function, transform params/results, etc.
    pub wrap_generate: Option<
        Box<
            dyn Fn(
                    DoGenerate,
                    DoStream,
                    CallOptions,
                    MiddlewareModelRef,
                ) -> BoxFuture<'static, Result<GenerateResult, Error>>
                + Send
                + Sync,
        >,
    >,

    /// Wrap the stream call. Receives `doGenerate`, `doStream`, params, and model ref.
    pub wrap_stream: Option<
        Box<
            dyn Fn(
                    DoGenerate,
                    DoStream,
                    CallOptions,
                    MiddlewareModelRef,
                ) -> BoxFuture<'static, Result<StreamResult, Error>>
                + Send
                + Sync,
        >,
    >,
}

impl Default for LanguageModelMiddleware {
    fn default() -> Self {
        Self {
            override_provider: None,
            override_model_id: None,
            transform_params: None,
            wrap_generate: None,
            wrap_stream: None,
        }
    }
}

/// Wrap a language model with a single middleware.
pub fn wrap_language_model(
    model: Box<dyn LanguageModelV4>,
    middleware: LanguageModelMiddleware,
) -> Box<dyn LanguageModelV4> {
    Box::new(WrappedLanguageModel {
        model: Arc::from(model),
        middleware: Arc::new(middleware),
    })
}

/// Wrap a language model with multiple middleware layers.
///
/// The first middleware in the vec is the outermost (applied first on input,
/// last on output). This matches the TS SDK convention.
pub fn wrap_language_model_chain(
    model: Box<dyn LanguageModelV4>,
    middleware: Vec<LanguageModelMiddleware>,
) -> Box<dyn LanguageModelV4> {
    // Apply in reverse: last middleware wraps closest to model.
    let mut current: Box<dyn LanguageModelV4> = model;
    for mw in middleware.into_iter().rev() {
        current = wrap_language_model(current, mw);
    }
    current
}

/// A language model wrapped with middleware.
pub struct WrappedLanguageModel {
    model: Arc<dyn LanguageModelV4>,
    middleware: Arc<LanguageModelMiddleware>,
}

impl WrappedLanguageModel {
    fn model_ref(&self) -> MiddlewareModelRef {
        MiddlewareModelRef {
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
impl LanguageModelV4 for WrappedLanguageModel {
    fn provider(&self) -> &str {
        // We can't return &str with a computed string, so we leak.
        // This is acceptable for middleware (long-lived, few instances).
        Box::leak(self.effective_provider().into_boxed_str())
    }

    fn model_id(&self) -> &str {
        Box::leak(self.effective_model_id().into_boxed_str())
    }

    async fn do_generate(
        &self,
        options: &CallOptions,
    ) -> Result<GenerateResult, Error> {
        let model = Arc::clone(&self.model);
        let middleware = Arc::clone(&self.middleware);
        let options = options.clone();

        let model_ref = MiddlewareModelRef {
                provider: model.provider().to_string(),
                model_id: model.model_id().to_string(),
            };

            // Transform params.
            let params = match &middleware.transform_params {
                Some(transform) => {
                    let mr = MiddlewareModelRef {
                        provider: model_ref.provider.clone(),
                        model_id: model_ref.model_id.clone(),
                    };
                    transform(options, mr).await?
                }
                None => options,
            };

            // Build doGenerate and doStream closures.
            let model_for_gen = Arc::clone(&model);
            let do_generate: DoGenerate = Arc::new(move |opts: CallOptions| {
                let m = Arc::clone(&model_for_gen);
                Box::pin(async move { m.do_generate(&opts).await })
                    as BoxFuture<'static, Result<GenerateResult, Error>>
            });

            let model_for_stream = Arc::clone(&model);
            let do_stream: DoStream = Arc::new(move |opts: CallOptions| {
                let m = Arc::clone(&model_for_stream);
                Box::pin(async move { m.do_stream(&opts).await })
                    as BoxFuture<'static, Result<StreamResult, Error>>
            });

            // Wrap generate.
            match &middleware.wrap_generate {
                Some(wrap) => wrap(do_generate, do_stream, params, model_ref).await,
                None => do_generate(params).await,
            }
    }

    async fn do_stream(
        &self,
        options: &CallOptions,
    ) -> Result<StreamResult, Error> {
        let model = Arc::clone(&self.model);
        let middleware = Arc::clone(&self.middleware);
        let options = options.clone();

        let model_ref = MiddlewareModelRef {
                provider: model.provider().to_string(),
                model_id: model.model_id().to_string(),
            };

            // Transform params.
            let params = match &middleware.transform_params {
                Some(transform) => {
                    let mr = MiddlewareModelRef {
                        provider: model_ref.provider.clone(),
                        model_id: model_ref.model_id.clone(),
                    };
                    transform(options, mr).await?
                }
                None => options,
            };

            // Build doGenerate and doStream closures.
            let model_for_gen = Arc::clone(&model);
            let do_generate: DoGenerate = Arc::new(move |opts: CallOptions| {
                let m = Arc::clone(&model_for_gen);
                Box::pin(async move { m.do_generate(&opts).await })
                    as BoxFuture<'static, Result<GenerateResult, Error>>
            });

            let model_for_stream = Arc::clone(&model);
            let do_stream: DoStream = Arc::new(move |opts: CallOptions| {
                let m = Arc::clone(&model_for_stream);
                Box::pin(async move { m.do_stream(&opts).await })
                    as BoxFuture<'static, Result<StreamResult, Error>>
            });

            // Wrap stream.
            match &middleware.wrap_stream {
                Some(wrap) => wrap(do_generate, do_stream, params, model_ref).await,
                None => do_stream(params).await,
            }
    }
}
