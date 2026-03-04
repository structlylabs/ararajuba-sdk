# Security Policy

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

If you discover a security vulnerability in Ararajuba, please report it responsibly by emailing:

**security@structly.ai**

Please include:

- A description of the vulnerability
- Steps to reproduce the issue
- The potential impact
- Any suggested fixes (if applicable)

## Response Timeline

- **Acknowledgment**: Within 48 hours of your report
- **Initial assessment**: Within 5 business days
- **Resolution**: We aim to release a fix within 30 days for confirmed vulnerabilities

## Scope

This security policy applies to the latest released versions of all Ararajuba crates:

- `ararajuba`
- `ararajuba-core`
- `ararajuba-provider`
- `ararajuba-provider-utils`
- `ararajuba-mcp`
- `ararajuba-openai-compatible`
- `ararajuba-openai`
- `ararajuba-anthropic`
- `ararajuba-google`
- `ararajuba-deepseek`
- `ararajuba-tools-coding`

## Supported Versions

| Version | Supported |
|---------|-----------|
| Latest  | Yes       |

## Security Best Practices

When using Ararajuba in your applications:

- **API keys**: Never hardcode API keys. Use environment variables (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, etc.)
- **Tool execution**: When using the tool system, validate and sanitize tool inputs before execution
- **MCP transports**: Use HTTPS for HTTP/SSE MCP transports in production
- **Dependencies**: Keep Ararajuba and its dependencies up to date with `cargo update`

## Disclosure Policy

We follow a coordinated disclosure process:

1. The reporter submits the vulnerability privately
2. We confirm and assess the issue
3. We develop and test a fix
4. We release the fix and publish a security advisory
5. The reporter is credited (unless they prefer anonymity)

Thank you for helping keep Ararajuba and its users safe.
