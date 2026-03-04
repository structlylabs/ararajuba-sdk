//! Image model middleware — wraps an `ImageModelV4` to intercept calls.

use ararajuba_provider::errors::Error;
use ararajuba_provider::image_model::v4::image_model_v4::ImageModelV4;
use ararajuba_provider::image_model::v4::{ImageCallOptions, ImageGenerateResult};
use async_trait::async_trait;
use futures::future::BoxFuture;
use std::sync::Arc;

/// Reference to the wrapped image model.
pub struct ImageMiddlewareModelRef {
    pub provider: String,
    pub model_id: String,
}

/// Type alias for the doGenerate function passed to middleware.
pub type DoGenerateImage = Arc<
    dyn Fn(ImageCallOptions) -> BoxFuture<'static, Result<ImageGenerateResult, Error>>
        + Send
        + Sync,
>;

/// Middleware for image models.
pub struct ImageModelMiddleware {
    /// Override the provider name.
    pub override_provider:
        Option<Box<dyn Fn(&ImageMiddlewareModelRef) -> String + Send + Sync>>,
    /// Override the model ID.
    pub override_model_id:
        Option<Box<dyn Fn(&ImageMiddlewareModelRef) -> String + Send + Sync>>,
    /// Transform call options before the model sees them.
    pub transform_params: Option<
        Box<
            dyn Fn(
                    ImageCallOptions,
                    ImageMiddlewareModelRef,
                ) -> BoxFuture<'static, Result<ImageCallOptions, Error>>
                + Send
                + Sync,
        >,
    >,
    /// Wrap the generate call.
    pub wrap_generate: Option<
        Box<
            dyn Fn(
                    DoGenerateImage,
                    ImageCallOptions,
                    ImageMiddlewareModelRef,
                ) -> BoxFuture<'static, Result<ImageGenerateResult, Error>>
                + Send
                + Sync,
        >,
    >,
}

impl Default for ImageModelMiddleware {
    fn default() -> Self {
        Self {
            override_provider: None,
            override_model_id: None,
            transform_params: None,
            wrap_generate: None,
        }
    }
}

/// Wrap an image model with middleware.
pub fn wrap_image_model(
    model: Box<dyn ImageModelV4>,
    middleware: ImageModelMiddleware,
) -> Box<dyn ImageModelV4> {
    Box::new(WrappedImageModel {
        model: Arc::from(model),
        middleware: Arc::new(middleware),
    })
}

struct WrappedImageModel {
    model: Arc<dyn ImageModelV4>,
    middleware: Arc<ImageModelMiddleware>,
}

impl WrappedImageModel {
    fn model_ref(&self) -> ImageMiddlewareModelRef {
        ImageMiddlewareModelRef {
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
impl ImageModelV4 for WrappedImageModel {
    fn provider(&self) -> &str {
        Box::leak(self.effective_provider().into_boxed_str())
    }

    fn model_id(&self) -> &str {
        Box::leak(self.effective_model_id().into_boxed_str())
    }

    fn max_images_per_call(&self) -> Option<usize> {
        self.model.max_images_per_call()
    }

    async fn do_generate(
        &self,
        options: &ImageCallOptions,
    ) -> Result<ImageGenerateResult, Error> {
        let model = Arc::clone(&self.model);
        let middleware = Arc::clone(&self.middleware);
        let options = options.clone();

        let model_ref = ImageMiddlewareModelRef {
            provider: model.provider().to_string(),
            model_id: model.model_id().to_string(),
        };

        let params = match &middleware.transform_params {
            Some(transform) => {
                let mr = ImageMiddlewareModelRef {
                    provider: model_ref.provider.clone(),
                    model_id: model_ref.model_id.clone(),
                };
                transform(options, mr).await?
            }
            None => options,
        };

        let model_for_gen = Arc::clone(&model);
        let do_generate: DoGenerateImage = Arc::new(move |opts| {
            let m = Arc::clone(&model_for_gen);
            Box::pin(async move { m.do_generate(&opts).await })
        });

        match &middleware.wrap_generate {
            Some(wrap) => wrap(do_generate, params, model_ref).await,
            None => do_generate(params).await,
        }
    }
}
