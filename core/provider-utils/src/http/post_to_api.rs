//! POST requests to AI provider APIs.

use super::response_handler::{ResponseHandler, ResponseHandlerOptions};
use super::retry::{RetryConfig, with_retry};
use ararajuba_provider::errors::Error;
use std::collections::HashMap;
use std::sync::Arc;
use futures::future::BoxFuture;
use tokio_util::sync::CancellationToken;

/// Options for `post_json_to_api`.
pub struct PostJsonOptions<T> {
    pub url: String,
    pub headers: Option<HashMap<String, String>>,
    pub body: serde_json::Value,
    pub successful_response_handler: ResponseHandler<T>,
    pub failed_response_handler: ResponseHandler<Error>,
    /// Optional custom fetch function (for testing / middleware).
    pub fetch: Option<
        Arc<
            dyn Fn(reqwest::Request) -> BoxFuture<'static, reqwest::Result<reqwest::Response>>
                + Send
                + Sync,
        >,
    >,
    /// Optional retry configuration.
    pub retry: Option<RetryConfig>,
    /// Optional cancellation token.
    pub cancellation_token: Option<CancellationToken>,
}

/// Send a JSON POST request to an API endpoint.
pub async fn post_json_to_api<T: Send + 'static>(options: PostJsonOptions<T>) -> Result<T, Error> {
    let url = options.url;
    let headers = options.headers;
    let body = options.body;
    let successful_response_handler = options.successful_response_handler;
    let failed_response_handler = options.failed_response_handler;
    let fetch = options.fetch;
    let retry_config = options.retry.unwrap_or_default();
    let cancellation_token = options.cancellation_token;

    let url_clone = url.clone();
    let execute = move || {
        let url = url_clone.clone();
        let headers = headers.clone();
        let body = body.clone();
        let successful_response_handler = Arc::clone(&successful_response_handler);
        let failed_response_handler = Arc::clone(&failed_response_handler);
        let fetch = fetch.clone();
        let cancellation_token = cancellation_token.clone();

        Box::pin(async move {
            // Check cancellation before making the request.
            if let Some(ref token) = cancellation_token {
                if token.is_cancelled() {
                    return Err(Error::Other {
                        message: "Operation cancelled".to_string(),
                    });
                }
            }

            let client = reqwest::Client::new();
            let mut builder = client
                .post(&url)
                .header("content-type", "application/json");

            if let Some(ref headers) = headers {
                for (key, value) in headers {
                    builder = builder.header(key, value);
                }
            }

            builder = builder.json(&body);

            let response = if let Some(ref fetch) = fetch {
                let request = builder.build().map_err(|e| Error::Http {
                    message: e.to_string(),
                })?;
                fetch(request).await.map_err(|e| Error::Http {
                    message: e.to_string(),
                })?
            } else {
                builder.send().await.map_err(|e| Error::Http {
                    message: e.to_string(),
                })?
            };

            let status = response.status();

            if status.is_success() {
                let handler_opts = ResponseHandlerOptions {
                    url: url.clone(),
                    request_body: Some(body),
                    response,
                };
                let result = (successful_response_handler)(handler_opts).await?;
                Ok(result.value)
            } else {
                let status_code = status.as_u16();
                let handler_opts = ResponseHandlerOptions {
                    url: url.clone(),
                    request_body: Some(body),
                    response,
                };

                match (failed_response_handler)(handler_opts).await {
                    Ok(result) => Err(result.value),
                    Err(_) => Err(Error::ApiCallError {
                        message: format!("API call failed with status {status_code}"),
                        url,
                        status_code: Some(status_code),
                        response_body: None,
                        is_retryable: status_code == 429 || status_code >= 500,
                        data: None,
                    }),
                }
            }
        }) as BoxFuture<'static, Result<T, Error>>
    };

    with_retry(retry_config, execute).await
}
