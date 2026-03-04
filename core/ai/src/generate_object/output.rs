//! Output trait and factories for generate_object.

use crate::error::Error;
use ararajuba_provider::language_model::v4::call_options::ResponseFormat;

/// Trait for defining how model output should be parsed.
pub trait Output: Send + Sync {
    /// The response format to request from the model.
    fn response_format(&self) -> ResponseFormat;
    /// Parse a complete text response into a JSON value.
    fn parse_complete(&self, text: &str) -> Result<serde_json::Value, Error>;
    /// Attempt to parse a partial text response (for streaming).
    fn parse_partial(&self, text: &str) -> Option<serde_json::Value>;
    /// Return the JSON Schema used by this output (for repair purposes).
    ///
    /// Returns `serde_json::Value::Null` if no schema is applicable.
    fn json_schema(&self) -> serde_json::Value {
        serde_json::Value::Null
    }
}

/// An output that produces a typed JSON object matching a schema.
struct ObjectOutput {
    schema: serde_json::Value,
}

impl Output for ObjectOutput {
    fn response_format(&self) -> ResponseFormat {
        ResponseFormat::Json {
            schema: Some(self.schema.clone()),
            name: None,
            description: None,
        }
    }

    fn parse_complete(&self, text: &str) -> Result<serde_json::Value, Error> {
        serde_json::from_str(text).map_err(|e| Error::OutputParse {
            message: format!("Failed to parse object output: {e}"),
        })
    }

    fn parse_partial(&self, text: &str) -> Option<serde_json::Value> {
        let repaired = ararajuba_provider_utils::parsing::json_repair::repair_incomplete_json(text);
        serde_json::from_str(&repaired).ok()
    }

    fn json_schema(&self) -> serde_json::Value {
        self.schema.clone()
    }
}

/// An output that produces a JSON array of elements matching a schema.
struct ArrayOutput {
    element_schema: serde_json::Value,
}

impl Output for ArrayOutput {
    fn response_format(&self) -> ResponseFormat {
        let array_schema = serde_json::json!({
            "type": "array",
            "items": self.element_schema,
        });
        ResponseFormat::Json {
            schema: Some(array_schema),
            name: None,
            description: None,
        }
    }

    fn parse_complete(&self, text: &str) -> Result<serde_json::Value, Error> {
        let value: serde_json::Value =
            serde_json::from_str(text).map_err(|e| Error::OutputParse {
                message: format!("Failed to parse array output: {e}"),
            })?;
        if !value.is_array() {
            return Err(Error::OutputParse {
                message: "Expected array output".to_string(),
            });
        }
        Ok(value)
    }

    fn parse_partial(&self, text: &str) -> Option<serde_json::Value> {
        let repaired = ararajuba_provider_utils::parsing::json_repair::repair_incomplete_json(text);
        serde_json::from_str(&repaired).ok()
    }

    fn json_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "array",
            "items": self.element_schema,
        })
    }
}

/// An output that produces unstructured JSON.
struct JsonOutput;

impl Output for JsonOutput {
    fn response_format(&self) -> ResponseFormat {
        ResponseFormat::Json {
            schema: None,
            name: None,
            description: None,
        }
    }

    fn parse_complete(&self, text: &str) -> Result<serde_json::Value, Error> {
        serde_json::from_str(text).map_err(|e| Error::OutputParse {
            message: format!("Failed to parse JSON output: {e}"),
        })
    }

    fn parse_partial(&self, text: &str) -> Option<serde_json::Value> {
        let repaired = ararajuba_provider_utils::parsing::json_repair::repair_incomplete_json(text);
        serde_json::from_str(&repaired).ok()
    }
}

/// Create an output that produces a typed JSON object matching a schema.
pub fn object_output(schema: serde_json::Value) -> Box<dyn Output> {
    Box::new(ObjectOutput { schema })
}

/// Create an output that produces a JSON array of elements matching a schema.
pub fn array_output(element_schema: serde_json::Value) -> Box<dyn Output> {
    Box::new(ArrayOutput { element_schema })
}

/// Create an output that produces unstructured JSON.
pub fn json_output() -> Box<dyn Output> {
    Box::new(JsonOutput)
}

/// An output that forces the model to pick exactly one value from a fixed list.
struct ChoiceOutput {
    options: Vec<String>,
}

impl Output for ChoiceOutput {
    fn response_format(&self) -> ResponseFormat {
        // Build a JSON schema that constrains the output to an enum.
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "choice": {
                    "type": "string",
                    "enum": self.options,
                }
            },
            "required": ["choice"],
            "additionalProperties": false,
        });
        ResponseFormat::Json {
            schema: Some(schema),
            name: Some("choice".to_string()),
            description: Some("Pick exactly one option from the provided list.".to_string()),
        }
    }

    fn parse_complete(&self, text: &str) -> Result<serde_json::Value, Error> {
        let value: serde_json::Value =
            serde_json::from_str(text).map_err(|e| Error::OutputParse {
                message: format!("Failed to parse choice output: {e}"),
            })?;

        // Extract the "choice" field.
        let choice = value
            .get("choice")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::OutputParse {
                message: "Expected JSON with a 'choice' field".to_string(),
            })?;

        // Validate the choice is one of the allowed options.
        if !self.options.iter().any(|o| o == choice) {
            return Err(Error::OutputParse {
                message: format!(
                    "Model returned '{}' which is not one of the allowed choices: {:?}",
                    choice, self.options
                ),
            });
        }

        Ok(serde_json::Value::String(choice.to_string()))
    }

    fn parse_partial(&self, text: &str) -> Option<serde_json::Value> {
        let repaired = ararajuba_provider_utils::parsing::json_repair::repair_incomplete_json(text);
        let value: serde_json::Value = serde_json::from_str(&repaired).ok()?;
        let choice = value.get("choice")?.as_str()?;
        Some(serde_json::Value::String(choice.to_string()))
    }
}

/// Create an output that forces the model to pick exactly one value from a fixed list.
///
/// # Example
/// ```ignore
/// let output = choice(vec!["positive".into(), "negative".into(), "neutral".into()]);
/// let result = generate_object(GenerateObjectOptions {
///     output,
///     ..
/// }).await?;
/// // result.object is a JSON string like "positive"
/// ```
pub fn choice(options: Vec<String>) -> Box<dyn Output> {
    Box::new(ChoiceOutput { options })
}
