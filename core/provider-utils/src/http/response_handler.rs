//! Response handler types and factories.

use ararajuba_provider::errors::Error;
use futures::future::BoxFuture;
use futures::stream::BoxStream;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::sync::Arc;

/// Options passed to a response handler.
pub struct ResponseHandlerOptions {
    pub url: String,
    pub request_body: Option<serde_json::Value>,
    pub response: reqwest::Response,
}

/// The result of a response handler.
pub struct HandlerResult<T> {
    pub value: T,
    pub raw_value: Option<serde_json::Value>,
    pub response_headers: Option<HashMap<String, String>>,
}

/// A boxed response handler function.
pub type ResponseHandler<T> = Arc<
    dyn Fn(ResponseHandlerOptions) -> BoxFuture<'static, Result<HandlerResult<T>, Error>>
        + Send
        + Sync,
>;

/// Extract headers from a reqwest response into a HashMap.
fn extract_headers(response: &reqwest::Response) -> HashMap<String, String> {
    response
        .headers()
        .iter()
        .filter_map(|(k, v)| {
            v.to_str()
                .ok()
                .map(|val| (k.as_str().to_string(), val.to_string()))
        })
        .collect()
}

/// Create a response handler that parses the response body as JSON.
pub fn create_json_response_handler<T>(
    validate: impl Fn(serde_json::Value) -> Result<T, Error> + Send + Sync + 'static,
) -> ResponseHandler<T>
where
    T: DeserializeOwned + Send + 'static,
{
    let validate = Arc::new(validate);
    Arc::new(move |opts: ResponseHandlerOptions| {
        let validate = Arc::clone(&validate);
        let url = opts.url;
        let response = opts.response;
        Box::pin(async move {
            let headers = extract_headers(&response);
            let text = response.text().await.map_err(|e| Error::Http {
                message: e.to_string(),
            })?;

            if text.is_empty() {
                return Err(Error::EmptyResponseBody);
            }

            let raw_value: serde_json::Value =
                serde_json::from_str(&text).map_err(|_| Error::JsonParse {
                    message: format!("Failed to parse JSON response from {url}"),
                    text: text.clone(),
                })?;

            let value = validate(raw_value.clone())?;

            Ok(HandlerResult {
                value,
                raw_value: Some(raw_value),
                response_headers: Some(headers),
            })
        }) as BoxFuture<'static, Result<HandlerResult<T>, Error>>
    })
}

/// Create a response handler that parses the response as a server-sent event stream.
pub fn create_event_source_response_handler<T>(
    parse_chunk: Arc<dyn Fn(serde_json::Value) -> Result<T, Error> + Send + Sync + 'static>,
) -> ResponseHandler<BoxStream<'static, Result<T, Error>>>
where
    T: Send + 'static,
{
    Arc::new(move |opts: ResponseHandlerOptions| {
        let parse_chunk = Arc::clone(&parse_chunk);
        let response = opts.response;
        Box::pin(async move {
            let headers = extract_headers(&response);
            let byte_stream = response.bytes_stream();

            let stream = crate::parsing::parse_event_stream::parse_event_stream(
                byte_stream,
                parse_chunk,
            );

            Ok(HandlerResult {
                value: stream,
                raw_value: None,
                response_headers: Some(headers),
            })
        })
            as BoxFuture<
                'static,
                Result<HandlerResult<BoxStream<'static, Result<T, Error>>>, Error>,
            >
    })
}

/// Create a response handler for error responses.
pub fn create_json_error_response_handler(
    parse_error: impl Fn(serde_json::Value) -> Error + Send + Sync + 'static,
) -> ResponseHandler<Error> {
    let parse_error = Arc::new(parse_error);
    Arc::new(move |opts: ResponseHandlerOptions| {
        let parse_error = Arc::clone(&parse_error);
        let response = opts.response;
        Box::pin(async move {
            let headers = extract_headers(&response);
            let text = response.text().await.map_err(|e| Error::Http {
                message: e.to_string(),
            })?;

            let raw_value: serde_json::Value =
                serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
            let error = parse_error(raw_value.clone());

            Ok(HandlerResult {
                value: error,
                raw_value: Some(raw_value),
                response_headers: Some(headers),
            })
        }) as BoxFuture<'static, Result<HandlerResult<Error>, Error>>
    })
}
