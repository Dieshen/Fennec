# Contributing to Fennec

## Development Setup

### Prerequisites
- Rust 1.70+ (2021 edition)
- Git

### Initial Setup
```bash
# Clone the repository
git clone https://github.com/fennec-ai/fennec.git
cd fennec

# Build the workspace
cargo build

# Run tests
cargo test

# Run smoke test
./scripts/smoke.sh
```

## Coding Standards

### Rust Style
- Follow Rust 2021 edition defaults with four-space indentation
- Use snake_case for modules and functions
- Use PascalCase for types and structs (e.g., `SessionManager`)
- Expose async entry points with explicit error types (`Result<T, FennecError>`)
- Never `unwrap()` in production paths
- Document non-obvious decisions with `///` doc comments above public items

### Error Handling
- Use `Result<T, FennecError>` for all fallible operations
- Prefer `anyhow` for simple error context
- Use `thiserror` for structured error types
- Never ignore errors silently

### Testing Guidelines
- Use `tokio::test` for async scenarios
- Gate network-dependent tests with `#[ignore]` and document prerequisites
- Structure fixtures under `tests/fixtures/` using `include_str!` for determinism
- Maintain coverage of core chat flows before merging

### Security Guidelines
- Never hardcode API keys or secrets
- Load credentials from `.env` or OS keyring
- Validate all tool outputs before acting
- Limit filesystem writes and sanitize shell commands
- Record sensitive logs behind `debug` features only

## Development Commands

### Building and Testing
```bash
# Format code
cargo fmt

# Check formatting (CI)
cargo fmt --check

# Run lints
cargo clippy --all-targets --all-features

# Run all tests
cargo test

# Run tests with ignored network tests
cargo test -- --ignored

# Run smoke test
./scripts/smoke.sh
```

### Running Fennec
```bash
# Development build
cargo run --bin fennec

# With debug logging
RUST_LOG=debug cargo run --bin fennec

# With custom config
cargo run --bin fennec -- --config ./config/example.toml
```

## Project Structure

### Crate Organization
- `fennec-cli/` - Main CLI binary and command-line interface
- `fennec-core/` - Shared domain logic, types, and traits
- `fennec-tui/` - Terminal UI components and layout
- `fennec-orchestration/` - Session management and agent coordination
- `fennec-memory/` - Memory management and persistence
- `fennec-provider/` - LLM provider implementations
- `fennec-security/` - Sandbox, audit, and approval systems
- `fennec-commands/` - Command implementations (plan, edit, run, etc.)

### Module Guidelines
- Colocate unit tests in the same module with `#[cfg(test)]` blocks
- Place integration tests in `tests/` directory
- Keep configuration files in `config/` with examples
- Document runnable flows in `examples/`

## Git Workflow

### Commit Guidelines
- Use Conventional Commits format: `feat:`, `fix:`, `chore:`, etc.
- Keep each change atomic and focused
- Reference issues when applicable
- Never mention AI tools in commit messages

### Pull Request Guidelines
- Describe the capability added or issue fixed
- Include screenshots/gif for UI changes
- Link to tracking issues
- Note any follow-up tasks required
- Request review from appropriate maintainers

## Architecture Decisions

### MVP Focus Areas
1. **Single-chat experience** - Perfect the core workflow before multi-chat
2. **Provider abstraction** - Support OpenAI with extensible design
3. **Command safety** - Sandbox and approval from day one
4. **Memory integration** - AGENTS.md and Cline-style memory files
5. **Testing culture** - Comprehensive test coverage

### Code Organization Principles
- Separate concerns clearly between crates
- Use traits for extensibility (providers, commands)
- Prefer composition over inheritance
- Keep async boundaries explicit
- Design for testability from the start

## Release Process

### Version Management
- Follow semantic versioning
- Update all workspace crates together
- Tag releases with `v` prefix (e.g., `v0.1.0`)

### Pre-release Checklist
- [ ] All tests pass locally and in CI
- [ ] Smoke test passes
- [ ] Documentation is up to date
- [ ] CHANGELOG.md is updated
- [ ] Version numbers are bumped consistently

## Getting Help

- Check existing issues and documentation first
- For bugs, provide minimal reproduction steps
- For features, describe the user problem being solved
- Include relevant log output with `RUST_LOG=debug`

## Code of Conduct

- Be respectful and inclusive
- Focus on technical merit
- Provide constructive feedback
- Help newcomers get started
- Maintain a professional environment