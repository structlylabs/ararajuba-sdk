# Changelog — ararajuba-openai

## [0.2.2] — 2026-03-07

### Changed
- organization name structlylabs -> atlantichq

## 0.2.1 — 2026-03-04

### Changed
- **Renamed** from `openai` to `ararajuba-openai`

## 0.2.0 — 2026-03-04

### Added

#### Image Model (`OpenAIImageModel`)
- Implements `ImageModel` trait for DALL·E 2, DALL·E 3, and gpt-image-1
- Endpoint: `POST /v1/images/generations`
- Provider options: `quality` (standard/hd), `size`, `style` (vivid/natural), `response_format` (url/b64_json), `background`, `output_compression`, `moderation`

#### Speech Model (`OpenAISpeechModel`)
- Implements `SpeechModel` trait for tts-1 and tts-1-hd
- Endpoint: `POST /v1/audio/speech`
- Provider options: `voice` (alloy/echo/fable/onyx/nova/shimmer), `speed` (0.25–4.0), `response_format` (mp3/opus/aac/flac/wav/pcm), `instructions`

#### Transcription Model (`OpenAITranscriptionModel`)
- Implements `TranscriptionModel` trait for whisper-1
- Endpoint: `POST /v1/audio/transcriptions` (multipart/form-data)
- Provider options: `language`, `prompt`, `temperature`, `timestamp_granularities`, `response_format`

#### Embedding Model Enhancement
- Added `serde::Serialize/Deserialize` derives on `OpenAIEmbeddingOptions`
- Documented `dimensions` and `user` passthrough via `provider_options`

#### Responses API (`OpenAIResponsesLanguageModel`)
- Implements `LanguageModel` trait using the OpenAI Responses API (`POST /v1/responses`)
- Full prompt conversion: System → developer role, User/Assistant content parts, Tool → function_call_output
- Non-streaming `do_generate` with response parsing (output items: message, function_call, reasoning)
- Streaming `do_stream` with SSE event transformation (response.created, output_text.delta/done, reasoning_summary_text.delta/done, function_call_arguments.delta/done, output_item.added, response.completed)
- Responses-specific options: `previous_response_id`, `store`, `include`, `truncation`, `conversation`, `reasoning_summary`, `max_tool_calls`, `logprobs`, `top_logprobs`
- `OpenAIProvider::responses(model_id)` factory method
- 4 unit tests

#### Provider Registration
- `OpenAIProvider::image_model()` factory
- `OpenAIProvider::speech_model()` factory
- `OpenAIProvider::transcription_model()` factory

## 0.1.0 (unreleased)

### Added

- **Chat language model** (`OpenAIChatLanguageModel`)
  - Extends `openai-compatible` with OpenAI-specific capabilities
  - Structured output support, reasoning tokens
  - `o1` model series compatibility
- **Embedding model** (`OpenAIEmbeddingModel`)
  - Custom options: `dimensions`, `user`
- **Provider factory** (`openai()`, `create_openai()`)
  - Configurable: API key, base URL, organization, project
  - `language_model()` and `embedding_model()` factories
