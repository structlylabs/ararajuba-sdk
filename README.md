# Ararajuba

> **Ararajuba** (*Guaruba*) is a golden parakeet endemic to the Amazon rainforest in Brazil. Known for its striking yellow plumage with green flight feathers, it is a symbol of Brazilian biodiversity and one of the country's most iconic birds.

A unified Rust SDK for working with large language models.

## Features

- **`generate_text`** / **`stream_text`** — text generation with multi-step tool execution
- **`generate_object`** / **`stream_object`** — structured JSON output with schema validation
- **`embed`** / **`embed_many`** — text embeddings
- **`generate_image`** / **`generate_speech`** / **`generate_video`** / **`transcribe`** / **`rerank`** — multimodal generation
- **Tool system** — static and dynamic tools, approval workflows, multi-step agentic loops
- **Middleware** — intercept and transform model calls (`wrap_language_model`, `wrap_embedding_model`, etc.)
- **Agent** — stateful multi-call agent with tool loop and UI streaming
- **Chat** — pluggable chat framework with transport abstraction
- **MCP Client** — Model Context Protocol integration (stdio, HTTP, SSE transports)
- **UI streaming** — SSE-based protocol compatible with Vercel AI SDK frontend hooks
- **Telemetry** — tracing integration for observability

## Architecture

```
core/
├── provider          # Traits: LanguageModel, EmbeddingModel, ImageModel, etc.
├── provider-utils    # HTTP, SSE parsing, retry, API key loading
├── ai                # High-level API: generate_text, tools, middleware, agent, UI
└── mcp               # MCP client with stdio/HTTP/SSE transports

providers/
├── openai-compatible # Base crate for OpenAI-compatible APIs
├── openai            # OpenAI (GPT-4o, o1/o3/o4, GPT-5, embeddings, images, speech, transcription)
├── anthropic         # Anthropic (Claude 3.5/4/4.5/4.6, thinking, caching)
├── google            # Google (Gemini 2/2.5/3, embeddings, Imagen, Veo)
└── deepseek          # DeepSeek (chat, reasoner)

tools/
└── coding            # File system, git, shell, diagnostics tools for coding agents
```

## Installation

```toml
[dependencies]
ararajuba = { version = "0.1", features = ["openai", "anthropic"] }
```

Available features: `openai`, `anthropic`, `google`, `deepseek`, `mcp`, `coding-tools`, `full`.

## Quick Start

```rust
use ararajuba::{generate_text, GenerateTextOptions, Prompt};
use ararajuba::openai::openai;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let result = generate_text(GenerateTextOptions {
        model: &openai("gpt-4o"),
        prompt: Prompt::simple("Explain quantum computing in one sentence."),
        ..Default::default()
    }).await?;

    println!("{}", result.text);
    Ok(())
}
```

### Streaming

```rust
use ararajuba::{stream_text, GenerateTextOptions, Prompt};
use ararajuba::anthropic::anthropic;
use tokio_stream::StreamExt;

let result = stream_text(GenerateTextOptions {
    model: &anthropic("claude-sonnet-4-6"),
    prompt: Prompt::simple("Write a poem about Rust."),
    ..Default::default()
}).await?;

let mut stream = result.text_stream;
while let Some(chunk) = stream.next().await {
    print!("{}", chunk);
}
```

### Structured Output

```rust
use ararajuba::{generate_object, GenerateObjectOptions, object_output, json_schema};
use ararajuba::google::google;

let result = generate_object(GenerateObjectOptions {
    model: &google("gemini-2.5-flash"),
    prompt: Prompt::simple("Generate a user profile."),
    output: object_output(json_schema!({
        "type": "object",
        "properties": {
            "name": { "type": "string" },
            "age": { "type": "integer" }
        },
        "required": ["name", "age"]
    })),
    ..Default::default()
}).await?;

println!("{}", result.object); // {"name": "Alice", "age": 30}
```

### Tools

```rust
use ararajuba::{generate_text, GenerateTextOptions, tool, ToolSet, Prompt};
use ararajuba::openai::openai;

let tools = ToolSet::from([
    tool("weather")
        .description("Get current weather for a location")
        .schema(json_schema!({
            "type": "object",
            "properties": { "location": { "type": "string" } },
            "required": ["location"]
        }))
        .execute(|input| async move {
            Ok(format!("72°F and sunny in {}", input["location"]))
        })
        .build(),
]);

let result = generate_text(GenerateTextOptions {
    model: &openai("gpt-4o"),
    prompt: Prompt::simple("What's the weather in Tokyo?"),
    tools,
    max_steps: 5,
    ..Default::default()
}).await?;
```

### MCP Integration

```rust
use ararajuba::mcp::{create_mcp_client, StdioTransportConfig};
use ararajuba::{generate_text, GenerateTextOptions, Prompt};
use ararajuba::anthropic::anthropic;

let mcp = create_mcp_client(StdioTransportConfig {
    command: "npx".into(),
    args: vec!["@anthropic/mcp-server-github".into()],
    ..Default::default()
}).await?;

let tools = mcp.tools().await?;

let result = generate_text(GenerateTextOptions {
    model: &anthropic("claude-sonnet-4-6"),
    prompt: Prompt::simple("List open issues in my repo"),
    tools,
    max_steps: 5,
    ..Default::default()
}).await?;
```

### Middleware

```rust
use ararajuba::{wrap_language_model, extract_reasoning_middleware};
use ararajuba::anthropic::anthropic;

let model = wrap_language_model(
    anthropic("claude-sonnet-4-6"),
    extract_reasoning_middleware(),
);
```

## Providers

| Provider | Language | Embedding | Image | Speech | Transcription | Video |
|----------|----------|-----------|-------|--------|---------------|-------|
| **OpenAI** | ✅ | ✅ | ✅ (dall-e-2/3, gpt-image-1) | ✅ (tts-1, tts-1-hd) | ✅ (whisper-1) | — |
| **Anthropic** | ✅ | — | — | — | — | — |
| **Google** | ✅ | ✅ | ✅ (Imagen 4.0) | — | — | ✅ (Veo 2/3) |
| **DeepSeek** | ✅ | — | — | — | — | — |

## Crates

| Crate | Description | Tests |
|-------|-------------|-------|
| `ararajuba` | Facade crate with feature-gated re-exports | 1 |
| `ararajuba-core` | High-level API, tools, middleware, agent, chat, UI streaming, telemetry | 123 |
| `ararajuba-provider` | Traits for all model types (language, embedding, image, speech, transcription, reranking, video) | 7 |
| `ararajuba-provider-utils` | HTTP, SSE parsing, retry, API key loading, JSON repair | 18 |
| `ararajuba-mcp` | MCP client with 3 transports (stdio, HTTP, SSE) | 34 |
| `ararajuba-openai-compatible` | Base for OpenAI-compatible providers | 22 |
| `ararajuba-openai` | OpenAI provider (language, embedding, image, speech, transcription) | 25 |
| `ararajuba-anthropic` | Anthropic provider | 20 |
| `ararajuba-google` | Google Generative AI provider (language, embedding, image, video) | 28 |
| `ararajuba-deepseek` | DeepSeek provider | 8 |
| `ararajuba-tools-coding` | File system, git, shell, diagnostics tools | 21 |

## v4 Provider Interface

The v4 provider interface uses Rust-native idioms, diverging from the v3 design (based on Vercel AI SDK):

- **Async nativo** — `async fn` instead of `BoxFuture<'_>` (leveraging Rust 2024 AFIT)
- **Typed streams** — separate `content`, `tool_calls`, and `metadata` streams instead of a single enum with 15+ variants
- **Capability traits** — `SupportsReasoning`, `SupportsCaching`, `SupportsToolCalling`, `SupportsImages` instead of runtime flags
- **Typed provider options** — `AnthropicOptions`, `OpenAIOptions` instead of untyped `serde_json::Value`
- **Typed provider errors** — `AnthropicError`, `OpenAIError` with structured variants (`RateLimited`, `ContentFiltered`, etc.)
- **Cancellation via Drop** — streams cancel the underlying request on drop; `CancellationToken` available as opt-in

## Protocol Compatibility

The UI streaming protocol (`UIMessageStream`, SSE format) is compatible with `@ai-sdk/react` frontend hooks (`useChat`, `useCompletion`, `useObject`). Any frontend consuming the standard SSE protocol works with Ararajuba as backend.

## License

Apache-2.0
