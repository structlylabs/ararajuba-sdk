# Changelog

All notable changes to the Ararajuba AI SDK workspace will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] — 2026-03-04

### Changed

- **Crate rename**: all crates renamed from `core-*` / provider names to `ararajuba-*` prefix
  - `core-provider` → `ararajuba-provider`
  - `core-provider-utils` → `ararajuba-provider-utils`
  - `core-ai` → `ararajuba-core`
  - `mcp-client` → `ararajuba-mcp`
  - `openai-compatible` → `ararajuba-openai-compatible`
  - `openai` → `ararajuba-openai`
  - `anthropic` → `ararajuba-anthropic`
  - `google` → `ararajuba-google`
  - `deepseek` → `ararajuba-deepseek`
  - `tools-coding` → `ararajuba-tools-coding`

### Added

- **`ararajuba` facade crate** — single dependency for end users with feature-gated providers
  - Default features: `openai`, `anthropic`
  - `full` feature enables all providers + MCP + coding tools

## [0.2.0] — 2026-03-04

### Added

- **Provider Interface v4** — Rust-native async traits diverging from the v3 Vercel AI SDK port:
  - Async-native `#[async_trait]` for all 7 model types (replaces `BoxFuture`)
  - Typed streams: `ContentDelta`, `ToolCallDelta`, `MetadataDelta` (replaces 15+ variant `StreamPart`)
  - Capability traits: `SupportsReasoning`, `SupportsCaching`, `SupportsToolCalling`, `SupportsImages`, `SupportsStructuredOutput`, `SupportsComputerUse`
  - Typed provider options with `set_provider_options<T>()` / `get_provider_options<T>()`
  - Drop-based stream cancellation via `AbortHandle`
  - Typed errors with `ProviderError` + `downcast::<T>()` support
  - Auto v3→v4 bridge via `V3LanguageModelAdapter`

- **OpenAI media models**: Image (DALL·E 2/3, gpt-image-1), Speech (tts-1/tts-1-hd), Transcription (whisper-1)
- **OpenAI Responses API** (`POST /v1/responses`) — full streaming + non-streaming `LanguageModel` impl
- **OpenAI Embedding** — `dimensions`/`user` passthrough via `provider_options`
- **Google Image model** (Imagen 4.0) — `POST /v1beta/models/{model}:predict`
- **Google Video model** (Veo 2/3) — async polling via `predictLongRunning`

### Crates Updated

| Crate | Version | Changes |
|-------|---------|---------|
| `core-provider` | 0.2.0 | v4 traits, capabilities, typed streams/options/errors, cancellation |
| `openai-compatible` | 0.2.0 | Embedding `provider_options` passthrough |
| `openai` | 0.2.0 | Image, Speech, Transcription models, Responses API |
| `google` | 0.2.0 | Image model (Imagen), Video model (Veo) |

## [0.1.0] — 2026-03-04

### Added

- **Workspace**: Cargo workspace with 4 crates (`core-provider`, `core-provider-utils`, `core-ai`, `tools-coding`)
- Edition 2024, resolver 3, full Rust port of the Vercel AI SDK (`ai` npm package)
- 150 tests passing across all crates (104 core-ai, 18 provider-utils, 21 tools-coding, 7 doc-tests)
- Zero errors, zero warnings

### Crates

| Crate | Version | Description |
|-------|---------|-------------|
| `core-provider` | 1.0.0 | Provider trait interfaces (LanguageModel, EmbeddingModel, ImageModel, etc.) |
| `core-provider-utils` | 1.0.0 | Shared HTTP/JSON/retry utilities for provider implementations |
| `core-ai` | 1.0.0 | High-level AI functions, tools, middleware, UI, chat framework |
| `tools-coding` | 1.0.0 | Pre-built coding-agent tool set (filesystem, git, shell, analysis) |
