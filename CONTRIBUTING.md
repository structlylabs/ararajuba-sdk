# Contributing to Ararajuba

Thank you for your interest in contributing to Ararajuba! We appreciate your help in making this Rust AI SDK better for everyone.

## Getting Started

### Prerequisites

- **Rust** (edition 2024, nightly or stable with edition support)
- **Cargo** (latest stable)

### Setup

1. Fork the repository
2. Clone your fork:
   ```bash
   git clone https://github.com/<your-username>/ararajuba-sdk.git
   cd ararajuba-sdk
   ```
3. Build the workspace:
   ```bash
   cargo build --workspace
   ```
4. Run tests:
   ```bash
   cargo test --workspace
   ```

## Development Workflow

### Building

```bash
# Build all crates
cargo build --workspace

# Build a specific crate
cargo build -p ararajuba-openai
```

### Testing

```bash
# Run all tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p ararajuba-core

# Run a specific test
cargo test -p ararajuba-core -- test_name
```

### Workspace Structure

```
ararajuba/                  # Facade crate (user-facing)
core/
├── provider                # Trait definitions for all model types
├── provider-utils          # HTTP, SSE, retry, JSON utilities
├── ai                      # High-level API (generate_text, tools, middleware, etc.)
└── mcp                     # MCP client (stdio, HTTP, SSE transports)
providers/
├── openai-compatible       # Base for OpenAI-compatible APIs
├── openai                  # OpenAI provider
├── anthropic               # Anthropic provider
├── google                  # Google Generative AI provider
└── deepseek                # DeepSeek provider
tools/
└── coding                  # Coding agent tools (fs, git, shell, diagnostics)
```

## Pull Request Process

1. Create a descriptive branch name (e.g., `fix/streaming-error`, `feat/new-provider`)
2. Make your changes with clear, focused commits
3. Ensure all tests pass: `cargo test --workspace`
4. Ensure the code compiles without warnings: `cargo clippy --workspace`
5. Format your code: `cargo fmt --all`
6. Open a pull request with a clear description of the changes

### Commit Convention

Use conventional commit format:

```
fix(ararajuba-openai): handle rate limit responses correctly
feat(ararajuba-core): add retry middleware
docs: update README examples
```

### Adding a New Provider

1. Create a new crate under `providers/`
2. Implement the relevant traits from `ararajuba-provider` (e.g., `LanguageModelV4`, `EmbeddingModelV4`)
3. Add the provider as an optional dependency in `ararajuba/Cargo.toml`
4. Add a feature flag in the facade crate
5. Re-export in `ararajuba/src/lib.rs` behind `#[cfg(feature = "...")]`
6. Add tests

## Reporting Issues

- Use [GitHub Issues](https://github.com/structlylabs/ararajuba-sdk/issues) to report bugs
- Include Rust version (`rustc --version`), OS, and a minimal reproduction
- For security vulnerabilities, see [SECURITY.md](SECURITY.md)

## License

By contributing, you agree that your contributions will be licensed under the [Apache License 2.0](LICENSE).
