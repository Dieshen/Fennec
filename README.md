# Fennec AI Assistant

A powerful, secure, and extensible TUI AI assistant for developers. Fennec brings AI-powered development workflows directly to your terminal with enterprise-grade security, comprehensive audit trails, and intelligent memory management.

## ✨ Key Features

### 🖥️ **Interactive Terminal Interface**
- **Clean, responsive TUI** built with ratatui for optimal developer experience
- **Multi-pane layout** with chat, preview, and status panels
- **Command palette** with fuzzy search and keyboard shortcuts
- **Real-time streaming** responses with syntax highlighting

### 🤖 **AI-Powered Development Commands**
- **`plan`** - Generate structured implementation plans and task breakdowns
- **`edit`** - Make precise file edits with intelligent diff previews
- **`run`** - Execute shell commands safely within sandbox constraints
- **`diff`** - Show detailed file changes and git-style diffs
- **`summarize`** - Create session summaries and memory updates
- **Enhanced commands** with depth and type controls for specialized workflows

### 🔒 **Enterprise-Grade Security**
- **Three-tier sandbox model**: `read-only`, `workspace-write`, `danger-full-access`
- **Approval workflows** with risk assessment and confirmation prompts
- **Path traversal protection** and command filtering
- **Capability-based permissions** system with fine-grained controls

### 🧠 **Intelligent Memory System**
- **AGENTS.md integration** for repository-specific guidelines
- **Cline-style memory files** (`projectbrief.md`, `activeContext.md`, `progress.md`)
- **Session transcripts** with full conversation history
- **Git history awareness** for contextual code understanding

### 📝 **Comprehensive Audit & Compliance**
- **Complete audit trails** in structured JSON format
- **Command-level tracking** with approval status and outcomes
- **Security event logging** with risk classification
- **Session management** with pause/resume capabilities

### 🔌 **Extensible Provider Architecture**
- **OpenAI** integration with streaming Chat Completions
- **Provider abstraction** ready for Anthropic, OpenRouter, Ollama
- **Configurable** model selection and parameters
- **Rate limiting** and retry logic built-in

## 🚀 Quick Start

### Prerequisites

- **Rust 1.70+** (2021 edition)
- **Git** for version control
- **OpenAI API key** (or other supported provider)

### Installation

```bash
# Clone the repository
git clone https://github.com/fennec-ai/fennec.git
cd fennec

# Build Fennec (optimized release)
cargo build --release

# Install system-wide (optional)
cargo install --path crates/fennec-cli
```

### Initial Setup

1. **Configure API credentials:**
   ```bash
   # Set OpenAI API key
   export OPENAI_API_KEY="sk-your-key-here"

   # Or use .env file (recommended)
   echo "OPENAI_API_KEY=sk-your-key-here" > .env
   ```

2. **Create configuration (optional):**
   ```bash
   # Create user config directory
   mkdir -p ~/.config/fennec

   # Copy and customize configuration
   cp config/example.toml ~/.config/fennec/config.toml
   ```

3. **Initialize project workspace:**
   ```bash
   # Navigate to your project
   cd /path/to/your/project

   # Copy AGENTS.md template for repository guidelines
   cp /path/to/fennec/AGENTS.md ./AGENTS.md

   # Edit AGENTS.md with your project specifics
   ```

### Basic Usage

```bash
# Start Fennec in current directory
fennec

# Start in specific project directory
fennec --cd /path/to/project

# Use read-only mode for safe exploration
fennec --sandbox read-only

# Enable approval prompts for enhanced security
fennec --ask-for-approval

# Use custom configuration file
fennec --config ./project-fennec.toml

# Enable verbose logging for debugging
fennec --verbose
```

### First Session

Once Fennec starts, try these commands:

```
plan "Add user authentication to my web app"
edit src/main.rs "Add error handling to the login function"
run "cargo test"
diff
summarize
```

## 🏗️ Architecture

Fennec is architected as a modular Rust workspace with clear separation of concerns:

### Core Crates

- **`fennec-cli`** - Main CLI binary with argument parsing and application entry point
- **`fennec-core`** - Shared domain types, traits, and core business logic
- **`fennec-tui`** - Terminal user interface built with ratatui and crossterm
- **`fennec-orchestration`** - Session management, agent coordination, and workflow orchestration
- **`fennec-memory`** - Memory persistence, AGENTS.md integration, and context management
- **`fennec-provider`** - LLM provider implementations with streaming support
- **`fennec-security`** - Sandbox policies, approval workflows, and comprehensive audit system
- **`fennec-commands`** - Built-in command implementations with extensible registry

### Design Principles

- **Security by Design** - All operations are validated through sandbox policies
- **Provider Agnostic** - Clean abstractions for multiple LLM backends
- **Extensible Commands** - Plugin-ready command system with capability declarations
- **Audit Everything** - Complete traceability of all system actions
- **Memory Aware** - Context-aware operations with persistent learning

## 🛠️ Core Commands

### Planning & Analysis
- **`plan <task>`** - Generate structured implementation plans with step-by-step breakdowns
- **`diff [file]`** - Show file changes with syntax-highlighted git-style diffs

### File Operations
- **`edit <file> <instruction>`** - Make precise file edits with intelligent diff previews
- **File operations** - Built-in support for reading, writing, and analyzing code files

### Execution & Testing
- **`run <command>`** - Execute shell commands safely within sandbox constraints
- **Shell integration** - Secure command execution with approval workflows

### Memory & Documentation
- **`summarize [--depth <level>] [--type <summary|progress|brief>]`** - Create comprehensive session summaries
- **Memory management** - Automatic context preservation and recall

### Enhanced Commands
- **Enhanced summarize** with output destination control and depth settings
- **Configurable workflows** with macro support and command chaining

## 🔐 Security Model

Fennec implements a **three-tier security model** with comprehensive audit trails:

### Sandbox Levels

1. **`read-only`**
   - File reading and code analysis only
   - No write operations or command execution
   - Perfect for code exploration and learning

2. **`workspace-write`** (default)
   - Read/write access within project workspace
   - Limited shell command execution
   - Ideal for most development workflows

3. **`danger-full-access`**
   - Full system access with all capabilities
   - Requires explicit approval for dangerous operations
   - Use only when necessary and with caution

### Security Features

- **Path traversal protection** - Prevents access outside workspace boundaries
- **Command filtering** - Risk-based classification of shell commands
- **Approval workflows** - Interactive confirmation for high-risk operations
- **Capability system** - Fine-grained permission control per command
- **Audit logging** - Complete JSON audit trail of all privileged actions

## 🧠 Memory & Context System

Fennec provides intelligent, persistent memory that learns and adapts:

### Memory Sources

- **AGENTS.md** - Repository-specific guidelines, coding standards, and project context
- **Cline Memory Bank** - Compatible with existing `.memory_bank` files for project continuity
- **Session transcripts** - Full conversation history with command outcomes
- **Git integration** - Awareness of repository history and changes

### Memory Features

- **Contextual retrieval** - Smart context injection based on current task
- **Session persistence** - Conversations and state preserved across sessions
- **Memory files** - Structured storage in `projectbrief.md`, `activeContext.md`, `progress.md`
- **Search capabilities** - Fuzzy search across all memory sources

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

🎯 **Release Ready** - Fennec has completed MVP development and is ready for production use:

- ✅ **Milestone 0** - Project scaffold and workspace setup
- ✅ **Milestone 1** - Core conversation loop and TUI
- ✅ **Milestone 2** - Editing and sandbox enforcement
- ✅ **Milestone 3** - Memory and summaries
- ✅ **Milestone 4** - Hardening and release readiness

See [docs/MVP.md](docs/MVP.md) for the complete development roadmap and [docs/RELEASE.md](docs/RELEASE.md) for release notes.

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