//! Tool definition and builder.

use futures::future::BoxFuture;
use futures::stream::BoxStream;
use std::sync::Arc;

/// A tool that the AI model can call during generation.
#[derive(Clone)]
pub struct ToolDef {
    /// Tool name.
    pub name: String,
    /// Human-readable description.
    pub description: Option<String>,
    /// JSON Schema defining the tool's input parameters.
    pub input_schema: serde_json::Value,
    /// The execute function. If `None`, the tool call will be returned to the caller.
    pub execute: Option<
        Arc<
            dyn Fn(serde_json::Value) -> BoxFuture<'static, Result<serde_json::Value, String>>
                + Send
                + Sync,
        >,
    >,
    /// Streaming execute function (async generator equivalent).
    ///
    /// Returns a stream of `Result<Value, String>` items. Every item except the
    /// last is treated as a **preliminary** result (useful for UI updates). The
    /// last item becomes the **final** tool result fed back to the model.
    ///
    /// If both `execute` and `execute_streaming` are set, `execute_streaming`
    /// takes priority.
    pub execute_streaming: Option<
        Arc<
            dyn Fn(serde_json::Value) -> BoxStream<'static, Result<serde_json::Value, String>>
                + Send
                + Sync,
        >,
    >,
    /// Optional approval function for human-in-the-loop.
    pub needs_approval: Option<Arc<dyn Fn(&serde_json::Value) -> bool + Send + Sync>>,
}

impl std::fmt::Debug for ToolDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolDef")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("input_schema", &self.input_schema)
            .field("has_execute", &self.execute.is_some())
            .field("has_execute_streaming", &self.execute_streaming.is_some())
            .field("has_needs_approval", &self.needs_approval.is_some())
            .finish()
    }
}

/// Builder for creating a `ToolDef`.
pub struct ToolBuilder {
    name: String,
    description: Option<String>,
    input_schema: serde_json::Value,
    execute: Option<
        Arc<
            dyn Fn(serde_json::Value) -> BoxFuture<'static, Result<serde_json::Value, String>>
                + Send
                + Sync,
        >,
    >,
    execute_streaming: Option<
        Arc<
            dyn Fn(serde_json::Value) -> BoxStream<'static, Result<serde_json::Value, String>>
                + Send
                + Sync,
        >,
    >,
    needs_approval: Option<Arc<dyn Fn(&serde_json::Value) -> bool + Send + Sync>>,
}

impl ToolBuilder {
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn input_schema(mut self, schema: serde_json::Value) -> Self {
        self.input_schema = schema;
        self
    }

    pub fn execute<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(serde_json::Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<serde_json::Value, String>> + Send + 'static,
    {
        self.execute = Some(Arc::new(move |input| Box::pin(f(input))));
        self
    }

    /// Set a streaming execute function (async generator equivalent).
    ///
    /// The stream should yield `Ok(value)` items. All but the last yield are
    /// treated as preliminary results. The last yield becomes the final result.
    pub fn execute_streaming<F, S>(mut self, f: F) -> Self
    where
        F: Fn(serde_json::Value) -> S + Send + Sync + 'static,
        S: futures::Stream<Item = Result<serde_json::Value, String>> + Send + 'static,
    {
        self.execute_streaming = Some(Arc::new(move |input| Box::pin(f(input))));
        self
    }

    pub fn needs_approval<F>(mut self, f: F) -> Self
    where
        F: Fn(&serde_json::Value) -> bool + Send + Sync + 'static,
    {
        self.needs_approval = Some(Arc::new(f));
        self
    }

    pub fn build(self) -> ToolDef {
        ToolDef {
            name: self.name,
            description: self.description,
            input_schema: self.input_schema,
            execute: self.execute,
            execute_streaming: self.execute_streaming,
            needs_approval: self.needs_approval,
        }
    }
}

/// Create a new tool builder.
pub fn tool(name: impl Into<String>) -> ToolBuilder {
    ToolBuilder {
        name: name.into(),
        description: None,
        input_schema: serde_json::json!({}),
        execute: None,
        execute_streaming: None,
        needs_approval: None,
    }
}
