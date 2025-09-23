# Fennec Development Workflow

## Daily Development Commands

### Building and Testing
```bash
# Format code according to Rust standards
cargo fmt

# Check formatting in CI
cargo fmt --check

# Run comprehensive lints
cargo clippy --all-targets --all-features

# Run all tests (unit + integration)
cargo test

# Run ignored tests (network-dependent)
cargo test -- --ignored

# Run benchmarks
cargo bench

# Run smoke test
./scripts/smoke.sh
```

### Running Fennec
```bash
# Development build with default settings
cargo run --bin fennec

# With debug logging enabled
RUST_LOG=debug cargo run --bin fennec

# In specific directory
cargo run --bin fennec -- --cd /path/to/project

# With read-only sandbox for safe exploration
cargo run --bin fennec -- --sandbox read-only

# With approval prompts enabled
cargo run --bin fennec -- --ask-for-approval

# With custom configuration
cargo run --bin fennec -- --config ./config/example.toml
```

## Code Style and Standards

### Rust Guidelines
- **Rust 2021 Edition** with four-space indentation
- **snake_case** for modules and functions
- **PascalCase** for types and structs (e.g., `SessionManager`)
- **SCREAMING_SNAKE_CASE** for constants
- **Explicit error handling** with `Result<T, FennecError>`
- **Never `unwrap()`** in production code paths
- **Document public APIs** with `///` doc comments

### Error Handling Patterns
```rust
// Preferred: Explicit error types
pub fn load_config(path: &Path) -> Result<Config, ConfigError> {
    // Implementation
}

// For simple contexts: anyhow
pub fn parse_agents_file(content: &str) -> anyhow::Result<AgentsConfig> {
    // Implementation with .context() for additional info
}

// For structured errors: thiserror
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuration file not found: {path}")]
    NotFound { path: PathBuf },
    #[error("Invalid configuration format")]
    InvalidFormat(#[from] toml::de::Error),
}
```

### Testing Patterns
```rust
// Async tests with tokio
#[tokio::test]
async fn test_session_management() {
    // Test implementation
}

// Network tests (marked ignored)
#[tokio::test]
#[ignore = "requires network access"]
async fn test_openai_integration() {
    // Test implementation
}

// Use tempfile for filesystem tests
#[test]
fn test_memory_persistence() {
    let temp_dir = tempfile::tempdir().unwrap();
    // Test with temporary directory
}
```

## Git Workflow

### Conventional Commits
Follow Conventional Commits v1.0.0 standard:

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

**Common Types**:
- `feat`: New feature (MINOR version bump)
- `fix`: Bug fix (PATCH version bump)
- `refactor`: Code restructuring without behavior change
- `perf`: Performance improvement
- `docs`: Documentation only changes
- `test`: Adding or correcting tests
- `build`: Build system or dependency changes
- `ci`: CI/CD configuration changes
- `chore`: Maintenance tasks
- `style`: Code formatting changes

**Examples**:
```bash
# Feature commit
git commit -m "feat(memory): add fuzzy search for context retrieval"

# Bug fix with issue reference
git commit -m "fix(tui): prevent panic on terminal resize

Fixes issue where rapid terminal resizing could cause
the TUI to panic due to layout calculation race condition.

Fixes: #123"

# Breaking change
git commit -m "feat(provider)!: change streaming API interface

BREAKING CHANGE: StreamingProvider trait now requires
async fn stream_response() instead of sync implementation"
```

### Branch Strategy
- **main/master**: Production-ready code
- **feature/**: New features (`feature/memory-search`)
- **fix/**: Bug fixes (`fix/tui-resize-panic`)
- **refactor/**: Code restructuring (`refactor/provider-abstraction`)

### Pull Request Guidelines
**Required Information**:
- Clear description of changes and motivation
- Screenshots/GIFs for UI changes
- Link to tracking issues
- Breaking change documentation
- Test coverage for new functionality

**PR Template**:
```markdown
## Summary
Brief description of changes

## Changes Made
- [ ] Feature/fix implemented
- [ ] Tests added/updated
- [ ] Documentation updated
- [ ] Breaking changes documented

## Testing
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Smoke test passes
- [ ] Manual testing completed

## Screenshots (if UI changes)
[Include relevant screenshots or GIFs]

## Breaking Changes
[Document any breaking changes]

Fixes: #issue_number
```

## Development Setup

### Prerequisites Checklist
- [ ] Rust 1.70+ installed
- [ ] Git configured
- [ ] OpenAI API key available (for testing)
- [ ] Terminal that supports TUI (most modern terminals)

### Initial Setup
```bash
# Clone repository
git clone https://github.com/fennec-ai/fennec.git
cd fennec

# Set up environment
echo "OPENAI_API_KEY=sk-your-key-here" > .env

# Build workspace
cargo build

# Run tests to verify setup
cargo test

# Run smoke test
./scripts/smoke.sh
```

### Development Environment
```bash
# Recommended environment variables
export RUST_LOG=debug                    # Enable debug logging
export RUST_BACKTRACE=1                  # Stack traces on panic
export FENNEC_CONFIG=./config/dev.toml   # Development config

# Optional: Enable faster builds
export CARGO_INCREMENTAL=1
export RUSTC_WRAPPER=sccache  # If sccache is installed
```

## Testing Strategy

### Test Categories
1. **Unit Tests**: Fast, isolated component tests
2. **Integration Tests**: Cross-component functionality
3. **Smoke Tests**: End-to-end workflow validation
4. **Performance Tests**: Benchmarks and load testing

### Test Organization
```
tests/
├── integration/           # Cross-crate integration tests
│   ├── tui_tests.rs      # TUI component integration
│   ├── memory_tests.rs   # Memory system integration
│   └── provider_tests.rs # Provider integration
├── fixtures/             # Test data and mocks
│   ├── agents_examples/  # Sample AGENTS.md files
│   ├── memory_files/     # Memory file examples
│   └── responses/        # Mock LLM responses
└── smoke/                # End-to-end smoke tests
    └── basic_workflow.rs # Complete workflow test
```

### Test Data Management
- **Fixtures**: Use `include_str!` for deterministic test data
- **Mocking**: Use `mockall` for external service mocking
- **Temporary Files**: Use `tempfile` for filesystem tests
- **Environment**: Use `serial_test` for tests requiring isolation

## Performance Optimization

### Development Profile
```toml
[profile.dev]
opt-level = 0      # No optimization for faster builds
debug = true       # Full debug info
incremental = true # Incremental compilation
```

### Release Profile
```toml
[profile.release]
opt-level = 3         # Full optimization
lto = true           # Link-time optimization
codegen-units = 1    # Single codegen unit
panic = "abort"      # Abort on panic
strip = true         # Strip symbols
```

### Performance Monitoring
- **Benchmarks**: Use `criterion` for performance regression detection
- **Profiling**: Use `perf` or `flamegraph` for performance analysis
- **Memory**: Use `valgrind` or `heaptrack` for memory analysis

## Debugging Workflows

### Common Debugging Commands
```bash
# Debug build with symbols
cargo build

# Run with debug logging
RUST_LOG=debug cargo run --bin fennec

# Run with backtrace on panic
RUST_BACKTRACE=full cargo run --bin fennec

# Debug specific module
RUST_LOG=fennec_memory=trace cargo run --bin fennec
```

### Debugging TUI Issues
```rust
// Use debug logging in TUI components
tracing::debug!("Panel state: {:?}", panel_state);

// Conditional debugging
#[cfg(debug_assertions)]
eprintln!("Debug: {}", debug_info);
```

### Memory Debugging
```bash
# Check for memory leaks
valgrind --tool=memcheck --leak-check=full target/debug/fennec

# Profile memory usage
heaptrack target/debug/fennec
```

## Release Process

### Pre-release Checklist
- [ ] All tests pass locally and in CI
- [ ] Smoke test passes with clean environment
- [ ] Documentation is up to date
- [ ] CHANGELOG.md is updated with changes
- [ ] Version numbers are consistent across workspace
- [ ] Security audit completed (if applicable)

### Version Management
```bash
# Update all workspace versions
cargo workspaces version --all patch|minor|major

# Create release tag
git tag -a v0.2.0 -m "Release version 0.2.0"

# Push tag to trigger release
git push origin v0.2.0
```

### Release Notes Template
```markdown
# Fennec v0.2.0

## 🎉 New Features
- Feature description with usage example

## 🐛 Bug Fixes
- Bug fix description and impact

## ⚡ Performance Improvements
- Performance improvement details

## 🔒 Security Updates
- Security fix details (if any)

## 📖 Documentation
- Documentation improvements

## 🔧 Internal Changes
- Development and maintenance updates

## Migration Guide
[Include if breaking changes]
```

---

*This workflow ensures consistent, high-quality development practices across the Fennec project.*