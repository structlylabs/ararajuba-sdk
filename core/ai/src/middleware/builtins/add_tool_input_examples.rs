//! `addToolInputExamplesMiddleware` — injects example inputs into tool schemas.
//!
//! Some providers (notably Anthropic) perform better when tool schemas include
//! concrete `input_examples`. This middleware copies the `input_examples` field
//! from each provider-level tool definition into the tool schema's JSON before
//! sending it to the model.

use crate::middleware::wrap_language_model::{
    LanguageModelMiddleware, MiddlewareModelRef,
};
use ararajuba_provider::language_model::v4::call_options::CallOptions;

/// Create a middleware that copies `input_examples` from `FunctionTool` into each
/// tool's JSON Schema under a top-level `"examples"` key.
pub fn add_tool_input_examples_middleware() -> LanguageModelMiddleware {
    LanguageModelMiddleware {
        transform_params: Some(Box::new(
            |mut opts: CallOptions, _model_ref: MiddlewareModelRef| {
                Box::pin(async move {
                    if let Some(ref mut tools) = opts.tools {
                        for tool in tools.iter_mut() {
                            if let ararajuba_provider::language_model::v4::tool::Tool::Function(
                                ft,
                            ) = tool
                            {
                                if let Some(ref examples) = ft.input_examples {
                                    if !examples.is_empty() {
                                        // Inject "examples" into the input_schema object.
                                        if let serde_json::Value::Object(ref mut map) =
                                            ft.input_schema
                                        {
                                            map.insert(
                                                "examples".to_string(),
                                                serde_json::Value::Array(
                                                    examples
                                                        .iter()
                                                        .map(|ex| ex.input.clone())
                                                        .collect(),
                                                ),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Ok(opts)
                })
            },
        )),
        ..LanguageModelMiddleware::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_middleware_creates_successfully() {
        let mw = add_tool_input_examples_middleware();
        assert!(mw.transform_params.is_some());
        assert!(mw.wrap_generate.is_none());
        assert!(mw.wrap_stream.is_none());
    }
}
