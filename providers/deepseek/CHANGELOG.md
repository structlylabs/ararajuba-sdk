# Changelog – `ararajuba-deepseek`

## [0.1.2] (unreleased)

### Changed
- organization name structlylabs -> atlantichq

## 0.1.1 (unreleased)

### Changed
- **Renamed** from `deepseek` to `ararajuba-deepseek`

## 0.1.0 (unreleased)

### Added
- `DeepSeekChatLanguageModel` — wraps `OpenAICompatibleChatLanguageModel` with
  DeepSeek-specific thinking/reasoning support.
- `DeepSeekProvider` with `create_deepseek()` / `deepseek()` convenience
  constructors.
- `thinking` provider option pass-through for reasoner models.
- Automatic parameter stripping (temperature, top_p) for `deepseek-reasoner`
  with unsupported-feature warnings.
- Bearer-token auth via `DEEPSEEK_API_KEY` environment variable.
- No embedding model (returns `None`).
- 8 unit tests + 1 doc-test.
