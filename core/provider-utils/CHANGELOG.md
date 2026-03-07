# Changelog — ararajuba-provider-utils

All notable changes to the `ararajuba-provider-utils` crate (formerly `core-provider-utils`) will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.2] - 2026-03-07 

### Changed
- organization name structlylabs -> atlantichq

## [0.1.1] — 2026-03-04

### Changed
- **Renamed** from `core-provider-utils` to `ararajuba-provider-utils`

## [0.1.0] — 2026-03-04

### Added

#### API Key Management
- `load_api_key()` — load from explicit value or environment variable
- Validation: errors on missing or empty keys

#### HTTP Utilities
- `RetryConfig` — configurable retry with exponential backoff
- `HttpClient` trait abstraction for making requests
- `retryable_request()` — auto-retry with configurable strategy
- Retryable status code detection (429, 5xx)

#### Header Utilities
- `normalize_headers()` — lowercase normalization
- `combine_headers()` — merge header maps with override semantics

#### ID Generation
- `generate_id()` — URL-safe random IDs with configurable length
- `create_id_generator()` — factory with custom prefix

#### JSON Parsing
- `parse_json()` / `safe_parse_json()` — secure JSON parsing (no raw `JSON.parse`)
- `repair_json()` — fix incomplete JSON by closing open structures

#### User Agent
- `with_user_agent_suffix()` — build user-agent strings for provider HTTP clients
