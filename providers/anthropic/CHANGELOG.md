# Changelog – `ararajuba-anthropic` (Anthropic Provider)

All notable changes to the Anthropic provider crate (formerly `anthropic`).

## 0.2.0

### Changed
- **Renamed** from `anthropic` to `ararajuba-anthropic`

## 0.1.0 – Initial Release

### Features

- **Anthropic Messages API language model** (`AnthropicMessagesLanguageModel`)
  - `do_generate()`: POST to `/messages`, parses text/thinking/tool_use content blocks,
    stop_reason, usage with cache tokens
  - `do_stream()`: SSE streaming with full Anthropic event lifecycle:
    `message_start` → `content_block_start` → `content_block_delta` → `content_block_stop`
    → `message_delta` → `message_stop`
  - Content block index-based state machine for streaming
  - Extended thinking support via provider options
  - Response format with JSON Schema via `output_config`
  - Warns on unsupported params: `frequency_penalty`, `presence_penalty`, `seed`
  - Temperature clamped to [0, 1] (Anthropic range)
  - `max_tokens` required (defaults to 4096)

- **Message conversion** (`convert_to_anthropic_messages_prompt`)
  - System messages extracted to top-level `system` field
  - User + Tool messages merged into single user blocks (Anthropic alternating requirement)
  - Assistant text trimmed of trailing whitespace
  - File parts mapped to `image`/`document` types with base64/URL sources
  - Reasoning parts mapped to `thinking` blocks with signature
  - Tool calls mapped to `tool_use` blocks

- **Tool preparation** (`prepare_anthropic_tools`)
  - Flat format: `{ name, description, input_schema }` (no wrapping `function` key)
  - `required` → `{ type: "any" }`
  - `none` → tools removed entirely
  - Tool-specific choice: `{ type: "tool", name }`

- **Finish reason mapping** (`map_anthropic_stop_reason`)
  - `end_turn`/`stop_sequence`/`pause_turn` → Stop
  - `tool_use` → ToolCalls (or Stop if JSON response tool)
  - `max_tokens`/`model_context_window_exceeded` → Length
  - `refusal` → ContentFilter

- **Usage conversion** (`convert_anthropic_usage`)
  - Cache-aware: `cache_creation_input_tokens`, `cache_read_input_tokens`
  - Total includes cache tokens

- **Error parsing** (`parse_anthropic_error`)
  - Parses `{ type: "error", error: { type, message } }` format

- **Provider factory** (`create_anthropic`, `anthropic`)
  - API key via `x-api-key` header (not Bearer)
  - `anthropic-version` header (default: `2023-06-01`)
  - Custom base URL via config or `ANTHROPIC_BASE_URL` env
  - No embedding model (returns `None`)

### Provider Options

- `thinking` – Extended thinking configuration
- `effort` – Output effort level
- `speed` – Generation speed
- `cacheControl` / `cache_control` – Prompt caching

### Test Coverage

- 17 unit tests + 1 doc-test
- Message conversion (system extraction, tool merging, text trimming, image handling)
- Tool preparation (flat format, required→any, none removes tools)
- Stop reason mapping (all variants)
- Usage conversion (with/without cache)
- Error parsing (standard, overloaded, empty)
- Provider factory (model creation, headers, custom URL, no embedding)
