# Changelog — ararajuba-openai-compatible

## 0.3.0 — 2026-03-04

### Changed
- **Renamed** from `openai-compatible` to `ararajuba-openai-compatible`

## 0.2.0 — 2026-03-04

### Added

- **Embedding model `provider_options` passthrough**: `do_embed()` now merges `provider_options` key-value pairs into the request body JSON, enabling `dimensions`, `user`, and other provider-specific parameters to be sent to the `/embeddings` endpoint

## 0.1.0 (unreleased)

### Added

- **Chat language model** (`OpenAICompatibleChatLanguageModel`)
  - Implements `LanguageModel` trait from `core-provider`
  - `do_generate()`: POST to `/chat/completions`, parse choices/message/usage into `GenerateResult`
  - `do_stream()`: SSE streaming with state machine for text, reasoning, and tool call blocks
  - Response format support: text, json_object, json_schema (structured outputs)
  - Provider options pass-through (user, reasoningEffort, etc.)
  - Configurable: provider name, URL, headers, custom fetch, structured output support

- **Message conversion** (`convert_to_openai_compatible_chat_messages`)
  - System, User, Assistant, Tool message types
  - User: single text optimization vs. multi-part arrays (image_url, file, text)
  - Assistant: text/reasoning concatenation, tool call mapping
  - Tool results to individual messages with tool_call_id

- **Tool preparation** (`prepare_tools`)
  - Function tools → OpenAI format with name, parameters, description, strict
  - Provider tools → warning + skip
  - ToolChoice mapping: Auto, None, Required, specific Tool

- **Finish reason mapping** (`map_openai_compatible_finish_reason`)
  - stop → Stop, length → Length, content_filter → ContentFilter
  - function_call/tool_calls → ToolCalls, unknown → Other

- **Usage conversion** (`convert_openai_compatible_usage`)
  - Maps prompt_tokens, completion_tokens, cached_tokens, reasoning_tokens
  - InputTokens/OutputTokens breakdown

- **Embedding model** (`OpenAICompatibleEmbeddingModel`)
  - Implements `EmbeddingModel` trait from `core-provider`
  - POST to `/embeddings` with float encoding format
  - Index-sorted response parsing, usage extraction

- **Error parsing** (`parse_openai_compatible_error`)
  - Standard OpenAI error format: message, type, code

- **Provider factory** (`create_openai_compatible`)
  - Creates `Provider` with `language_model()` and `embedding_model()` factories
  - `openai_compatible_settings()` convenience with Bearer token auth
  - Configurable: base URL, headers, structured outputs, usage inclusion

- **17 unit tests** covering all modules
