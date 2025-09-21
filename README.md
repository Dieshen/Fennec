# Fennec AI Assistant

An interactive TUI AI assistant for developers, inspired by Claude Code, Codex CLI, and Cline Memory Bank.

## Features

- 🖥️ **Interactive Terminal UI** - Clean, responsive TUI built with ratatui
- 🤖 **AI-Powered Commands** - Plan, edit, run, diff, and summarize with AI assistance
- 🔒 **Sandbox Security** - Three-tier security model with approval workflows
- 🧠 **Persistent Memory** - AGENTS.md integration and Cline-style memory files
- 📝 **Audit Logging** - Complete audit trail of all privileged actions
- 🔌 **Provider Agnostic** - Support for OpenAI with extensible architecture

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/fennec-ai/fennec.git
cd fennec

# Build Fennec
cargo build --release

# Install locally (optional)
cargo install --path crates/fennec-cli
```

### Configuration

1. **Set up API key:**
   ```bash
   export OPENAI_API_KEY="sk-your-key-here"
   ```

2. **Create config file (optional):**
   ```bash
   mkdir -p ~/.config/fennec
   cp config/example.toml ~/.config/fennec/config.toml
   # Edit config.toml as needed
   ```

3. **Initialize workspace:**
   ```bash
   # Copy the AGENTS.md template to your project
   cp AGENTS.md /path/to/your/project/
   ```

### Usage

```bash
# Start Fennec in interactive mode
fennec

# Specify working directory
fennec --cd /path/to/project

# Set sandbox level
fennec --sandbox read-only

# Use custom config
fennec --config ./custom-config.toml
```

## Architecture

Fennec is built as a modular Rust workspace:

- **`fennec-cli`** - Main CLI binary and argument parsing
- **`fennec-core`** - Shared types, traits, and domain logic
- **`fennec-tui`** - Terminal user interface components
- **`fennec-orchestration`** - Session management and coordination
- **`fennec-memory`** - Memory persistence and AGENTS.md integration
- **`fennec-provider`** - LLM provider implementations
- **`fennec-security`** - Sandbox, audit, and approval systems
- **`fennec-commands`** - Core command implementations

## Core Commands

- **`plan`** - Generate and preview execution plans
- **`edit`** - Make file edits with diff previews
- **`run`** - Execute shell commands safely
- **`diff`** - Show changes and file differences
- **`summarize`** - Create session summaries for memory

## Security Model

Fennec implements a three-tier security model:

1. **`read-only`** - Can only read files and analyze code
2. **`workspace-write`** - Can modify files within the project directory
3. **`danger-full-access`** - Full system access (use with caution)

All actions require explicit approval and are logged for audit.

## Memory System

Fennec integrates with existing project memory systems:

- **AGENTS.md** - Repository guidelines and coding standards
- **Cline Memory Bank** - Persistent project context and progress
- **Session Transcripts** - Conversation history and command outcomes

## Development

### Prerequisites

- Rust 1.70+ (2021 edition)
- Git

### Development Setup

```bash
# Clone and build
git clone https://github.com/fennec-ai/fennec.git
cd fennec
cargo build

# Run tests
cargo test

# Run smoke test
./scripts/smoke.sh

# Development run
cargo run --bin fennec
```

### Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed development guidelines.

### Project Status

🚧 **MVP Development** - Currently implementing core features:

- ✅ **Milestone 0** - Project scaffold and workspace setup
- 🔄 **Milestone 1** - Core conversation loop and TUI
- ⏳ **Milestone 2** - Editing and sandbox enforcement
- ⏳ **Milestone 3** - Memory and summaries
- ⏳ **Milestone 4** - Hardening and release readiness

See [docs/MVP.md](docs/MVP.md) for the complete roadmap.

## Inspiration

Fennec draws inspiration from several excellent projects:

- **[Claude Code](https://claude.ai/code)** - Interactive coding experience and diff workflows
- **[Codex CLI](https://github.com/microsoft/codex-cli)** - Sandbox security and approval patterns
- **[Cline Memory Bank](https://github.com/cline/cline)** - Persistent memory and project context

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.