# Changelog — ararajuba-google

## 0.3.0 — 2026-03-04

### Changed
- **Renamed** from `google` to `ararajuba-google`

## 0.2.0 — 2026-03-04

### Added

#### Image Model (`GoogleImageModel`)
- Implements `ImageModel` trait for Imagen 4.0 models (imagen-4.0-generate-001, imagen-4.0-ultra-generate-001, imagen-4.0-fast-generate-001)
- Endpoint: `POST /v1beta/models/{model}:predict`
- Provider options: `number_of_images` (1–4), `aspect_ratio`, `image_size` (512/1K/2K/4K → pixel dimensions), `negative_prompt`, `person_generation`

#### Video Model (`GoogleVideoModel`)
- Implements `VideoModel` trait for Veo 2/3 models (veo-2.0-generate-001, veo-3.0-generate-001, veo-3.0-fast-generate-001, veo-3.1-generate)
- Endpoint: `POST /v1beta/models/{model}:predictLongRunning` with async polling
- Automatic polling: 5s intervals, up to 120 attempts (10 min timeout)
- Supports text-to-video and image-to-video (Base64 or URL input)
- Provider options: `aspect_ratio`, `person_generation`, `number_of_videos` (1–4), `duration_seconds`

#### Provider Registration
- `GoogleProvider::image_model()` factory
- `GoogleProvider::video_model()` factory

## 0.1.0 (unreleased)

### Added

- **Chat language model** (`GoogleChatLanguageModel`)
  - Implements `LanguageModel` trait for Gemini models
  - `do_generate()` and `do_stream()` via `POST /v1beta/models/{model}:generateContent` / `streamGenerateContent`
  - Function calling, JSON mode, structured output
  - Safety settings, system instruction, cached content
- **Embedding model** (`GoogleEmbeddingModel`)
  - Implements `EmbeddingModel` trait for text-embedding models
  - `do_embed()` via `POST /v1beta/models/{model}:batchEmbedContents`
- **Provider factory** (`google()`, `create_google()`)
  - Configurable: API key, base URL, custom headers/fetch
