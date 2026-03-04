# Changelog — ararajuba-core

All notable changes to the `ararajuba-core` crate (formerly `core-ai`) will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.0] — 2026-03-04

### Changed
- **Renamed** from `core-ai` to `ararajuba-core`

## [1.0.0] — 2026-03-04

### Added

#### Core AI Functions
- `generate_text()` — multi-step text generation with tool loop
- `stream_text()` — multi-step streaming text generation with tool execution via tokio channels
- `generate_object()` — structured output generation with JSON schema validation
- `stream_object()` — streaming structured output
- `embed()` / `embed_many()` — embedding generation
- `generate_image()` — image generation
- `generate_speech()` — speech synthesis
- `transcribe()` — audio transcription
- `rerank()` — document reranking
- `generate_video()` — video generation

#### Error System
- 25+ granular error variants: `NoContentGenerated`, `NoObjectGenerated`, `NoImageGenerated`, `NoSpeechGenerated`, `NoTranscriptGenerated`, `NoVideoGenerated`, `InvalidToolInput`, `InvalidToolApproval`, `ToolCallNotFoundForApproval`, `MissingToolResults`, `ToolNotFound`, `ToolCallRepair`, `InvalidStreamPart`, `UnsupportedModelVersion`, `UIMessageStream`, `InvalidDataContent`, `InvalidMessageRole`, `MessageConversion`, `RetryError`, `DownloadError`, `InvalidArgument`, `NoSuchProvider`, `NoModelVariants`, `ProviderError`

#### Callbacks & Events
- 7 event structs: `StartEvent`, `StepStartEvent`, `ToolCallStartEvent`, `ToolCallFinishEvent`, `FinishEvent`, `ChunkEvent`, `ErrorEvent`
- Full callback wiring on `GenerateTextOptions`: `on_start`, `on_step_start`, `on_step_finish`, `on_tool_call_start`, `on_tool_call_finish`, `on_finish`, `on_chunk`, `on_error`, `on_tool_approval`, `on_preliminary_tool_result`
- `PrepareStepFn` — dynamic tool/settings adjustment per step

#### Tool System
- `tool()` builder — define tools with JSON schema, description, and async execute function
- `ToolSet` — collection of named tools with iteration support
- `ToolCall`, `ToolResult` — structured call/result types
- `ToolApprovalRequest` / `ToolApprovalResponse` — human-in-the-loop approval
- `dynamic_tool()` builder — tools without compile-time schema
- `DynamicToolCall`, `DynamicToolResult`, `TypedToolCall` — dynamic tool types
- Stop conditions: `has_tool_call()`, `step_count_is()`

#### Middleware
- `wrap_language_model()` / `wrap_language_model_chain()` — language model middleware
- `wrap_embedding_model()` — embedding model middleware
- `wrap_image_model()` — image model middleware
- `wrap_provider()` — provider-level middleware (intercept model creation)
- Built-in middleware: `extract_json`, `extract_reasoning`, `add_tool_input_examples`

#### Registry
- `ModelRegistry` — register providers, resolve models by `provider:model-id` format
- `CustomProvider` — create custom providers with factory methods for all 7 model types (language, embedding, image, speech, transcription, reranking, video)

#### Agent
- `Agent` struct — stateful wrapper with conversation history, `call()` and `stream()` methods
- `AgentSettings` — id, version, model, system prompt, tools, max_steps, callbacks
- `AGENT_VERSION` constant (`agent-v1`)
- All callbacks: `on_start`, `on_step_start`, `on_step_finish`, `on_tool_call_start`, `on_tool_call_finish`, `on_finish`
- `create_agent_ui_stream()` — stream agent output as UI message chunks
- `create_agent_ui_stream_response()` — wrap agent stream in SSE response
- `pipe_agent_ui_stream_to_response()` — pipe agent stream to HTTP response

#### Chat Framework
- `Chat` struct — full chat session management with message history, streaming, cancellation
- `ChatTransport` trait — abstract transport interface (`send_messages`, `reconnect_to_stream`)
- `DirectChatTransport` — in-process transport that calls an Agent directly
- `ChatStatus` enum (Submitted, Streaming, Ready, Error)
- `ChatRequestOptions`, `ChatFinishInfo`, `ChatTrigger`
- `OnChatError`, `OnChatFinish` callbacks
- `IdGenerator` type alias, `default_id_generator()` (UUID v4)
- Serial job execution via `SerialJobExecutor`

#### UI Types & Message System
- `UIMessage` — full message type with role, parts, metadata, created_at
- 9 UI part types: `TextUIPart`, `ReasoningUIPart`, `ToolUIPart`, `DynamicToolUIPart`, `FileUIPart`, `DataUIPart`, `SourceUrlUIPart`, `SourceDocumentUIPart`, `StepStartUIPart`
- `ToolInvocationState` enum (InputStreaming, InputAvailable, ApprovalRequested, ApprovalResponded, OutputAvailable, OutputError, OutputDenied)
- Type predicates: `is_text_ui_part()`, `is_reasoning_ui_part()`, `is_tool_ui_part()`, `is_file_ui_part()`, `is_data_ui_part()`, etc.
- `get_tool_name()`, `get_text_from_ui_message()` helpers

#### UI Message Stream
- `UIMessageChunk` enum — 34-variant tagged SSE chunk type
- `create_ui_message_stream()` — create async stream of UI chunks
- `chunks_to_sse()` — convert chunks to Server-Sent Events format

#### UI Validation & Conversion
- `validate_ui_messages()` / `safe_validate_ui_messages()` — message validation
- `convert_to_model_messages()` — convert UIMessages to provider Message format
- `convert_file_list_to_file_ui_parts()` — convert file paths to FileUIPart

#### UI Reader & Response
- `read_ui_message_stream()` — parse SSE stream into UIMessageChunks
- `split_sse_and_parse()` — SSE line splitting and JSON parsing
- `SseResponse` — HTTP response wrapper for SSE streams
- `create_ui_message_stream_response()` — build SSE response from chunk stream
- `pipe_ui_message_stream_to_response()` — pipe stream to existing response
- `create_text_stream_response()` / `pipe_text_stream_to_response()` — plain text streaming

#### Schema Utilities
- `Schema` struct — JSON Schema wrapper with optional validation
- `json_schema()` — create Schema from raw JSON Schema value
- `as_schema()` — normalize optional Schema
- `ValidateFn`, `ValidationResult` types

#### Telemetry
- `TelemetrySettings` — configuration for OpenTelemetry integration
- `record_generation_span()`, `record_step_span()`, `record_tool_call_span()`, `record_response_attributes()`

#### Utilities
- `cosine_similarity()` / `cosine_similarity_f32()` — vector similarity with 8 tests
- `parse_partial_json()` — repair incomplete JSON by closing open structures
- `consume_stream()` / `collect_stream()` — stream consumption helpers
- `simulate_readable_stream()` — create simulated streams with delays
- `is_deep_equal_data()` — deep JSON value equality comparison
- `get_text_from_data_url()` / `get_mime_type_from_data_url()` — data URL parsing
- `SerialJobExecutor` — mutex-based serial job executor
- `prune_messages()` — message pruning with strategies (RemoveOldest, KeepEnds, KeepLast)
- `smooth_stream()` — smooth text stream output with configurable chunking (Character, Word, Line)

#### Shared Types
- `CallSettings` — temperature, top_p, top_k, max_tokens, stop_sequences, seed, etc.
- `CallWarning` — structured warning type
- `FinishReason` enum (Stop, Length, ContentFilter, ToolCalls, Error, Other, Unknown)
