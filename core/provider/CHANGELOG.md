# Changelog — ararajuba-provider

All notable changes to the `ararajuba-provider` crate (formerly `core-provider`) will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.2] - 2026-03-07 

### Changed
- organization name structlylabs -> atlantichq


## [0.2.1] — 2026-03-04

### Changed
- **Renamed** from `core-provider` to `ararajuba-provider`

## [0.2.0] — 2026-03-04

### Added

#### Provider Interface v4

- **Async-native traits** (`#[async_trait]`) for all 7 model types — replaces `BoxFuture` with idiomatic `async fn`
  - `LanguageModelV4` trait with `async fn do_generate()` and `async fn do_stream()`
  - `EmbeddingModelV4`, `ImageModelV4`, `SpeechModelV4`, `TranscriptionModelV4`, `RerankingModelV4`, `VideoModelV4`
  - All v4 traits live under `<model_type>/v4/` modules alongside existing v3

- **Typed streams** (`StreamResultV4`) — replaces single `BoxStream<StreamPart>` (15+ variants) with 3 focused streams:
  - `ContentDelta` — `Text`, `Reasoning`, `File`
  - `ToolCallDelta` — `Start`, `InputDelta`, `Complete`
  - `MetadataDelta` — `Usage`, `FinishReason`, `ProviderMetadata`
  - `split_merged_stream()` helper to fan-out a single merged stream into the 3 typed channels

- **Capability traits** for compile-time feature detection:
  - `SupportsReasoning`, `SupportsCaching`, `SupportsToolCalling`, `SupportsImages`, `SupportsStructuredOutput`, `SupportsComputerUse`

- **Typed provider options** on v4 `CallOptions`:
  - `set_provider_options<T: Serialize>()` / `get_provider_options<T: Deserialize>()` — type-safe access with serde roundtrip
  - `From<v3::CallOptions>` conversion for backward compatibility

- **Drop-based cancellation** (`AbortHandle`):
  - `StreamResultV4` holds an `AbortHandle` that cancels the underlying request on `Drop`
  - Automatic cleanup when stream is dropped — no manual cancellation required

- **Typed errors** (`ProviderError`):
  - `ProviderError` struct wrapping `Box<dyn Error + Send + Sync>` with `downcast::<T>()` support
  - `Error::Provider(ProviderError)` variant added to the error enum

#### Provider Trait v4 Methods

- `Provider::language_model_v4()` — returns `Box<dyn LanguageModelV4>` with auto-bridge from v3 via `V3LanguageModelAdapter`
- `Provider::embedding_model_v4()`, `image_model_v4()`, `speech_model_v4()`, `transcription_model_v4()`, `reranking_model_v4()`, `video_model_v4()` — all default to `None`
- `V3LanguageModelAdapter` — wraps any `Box<dyn LanguageModel>` (v3) as `LanguageModelV4` automatically

## [0.1.0] — 2026-03-04

### Added

#### Provider Trait
- `Provider` trait — unified entry point for obtaining models by ID

#### Language Model (v3)
- `LanguageModel` trait with `do_generate` and `do_stream` methods
- `CallOptions` — prompt, tools, tool choice, response format, provider options
- `GenerateResult` — text, tool calls, finish reason, usage, warnings, provider metadata
- `StreamResult` — streaming chunks via `BoxStream`
- Full prompt type system: `Message` (System, User, Assistant, Tool), content parts (`TextPart`, `FilePart`, `ToolCallPart`, `ToolResultPart`, `ReasoningPart`)
- `DataContent` enum (Text / Bytes) for binary data
- `ToolResultOutput` tagged enum (Text, Json, ExecutionDenied, ErrorText, ErrorJson, Content)
- `ToolDefinition`, `ToolChoice` (Auto, None, Required, Specific)
- `ResponseFormat` (Text, Json, JsonSchema)

#### Embedding Model (v3)
- `EmbeddingModel` trait with `do_embed` method
- `EmbedOptions`, `EmbedResult`, `EmbeddingUsage`

#### Image Model (v3)
- `ImageModel` trait with `do_generate` method
- `ImageGenerateOptions`, `ImageGenerateResult`, `ImageUsage`

#### Speech Model (v3)
- `SpeechModel` trait with `do_generate` method
- `SpeechGenerateOptions`, `SpeechGenerateResult`

#### Transcription Model (v3)
- `TranscriptionModel` trait with `do_generate` method
- `TranscribeOptions`, `TranscribeResult`

#### Reranking Model (v3)
- `RerankingModel` trait with `do_rerank` method
- `RerankOptions`, `RerankResult`, `RerankUsage`

#### Video Model (v3)
- `VideoModel` trait with `do_generate` method
- `VideoGenerateOptions`, `VideoGenerateResult`

#### Shared Types
- `ProviderOptions` (`HashMap<String, Value>`)
- `ProviderMetadata` (`HashMap<String, Value>`)
- `Warning` enum
- `JSONValue`, `JSONObject`, `JSONArray` type aliases

#### Error Types
- `Error` enum with `ApiCallError`, `TypeValidationError`, `UnsupportedFunctionalityError`
