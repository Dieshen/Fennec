# Fennec AI Assistant

A powerful, secure, and extensible TUI AI assistant for developers. Fennec brings AI-powered development workflows directly to your terminal with enterprise-grade security, comprehensive audit trails, and intelligent memory management.

## ✨ Key Features

### 🖥️ **Interactive Terminal Interface**
- **Clean, responsive TUI** built with ratatui for optimal developer experience
- **Multi-pane layout** with chat, preview, and status panels
- **Command palette** with fuzzy search and keyboard shortcuts
- **Real-time streaming** responses with syntax highlighting

### 🤖 **AI-Powered Development Commands**
- **17 built-in commands** covering planning, file ops, search, testing, and git workflows
- **`plan`** - Structured implementation plans with complexity assessment
- **`edit`** - Multi-strategy file editing (search/replace, line range, append/prepend)
- **`search`** & **`find-symbol`** - Full-text and AST-based code navigation
- **`test-watch`** - Auto-rerun tests with file watching and smart selection
- **`fix-errors`** - Parse compiler errors and auto-suggest fixes
- **`index`** - Project analysis with dependency graphs and impact analysis
- **`quick-action`** - 8 workflow templates for common development tasks

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

## 🛠️ Built-in Commands

Fennec includes **17 production-ready commands** covering the complete development workflow:

### Planning & Analysis
- **`plan`** - Generate structured implementation plans with complexity assessment and task breakdowns
- **`diff`** - Show detailed file changes with syntax-highlighted git-style diffs and unified format
- **`summarize`** - Create session summaries with configurable depth and output destinations
- **`summarize_enhanced`** - Advanced summarization with memory integration and progress tracking

### File Operations
- **`create`** - Create new files and directories with parent directory support
- **`edit`** - Make precise file edits with multiple strategies (search/replace, line range, append, prepend)
- **`rename`** - Rename files and directories with conflict detection
- **`delete`** - Delete files and directories with safety checks and recursive options

### Code Search & Navigation
- **`search`** - Full-text search across files with regex support, case-insensitive mode, and context lines
- **`find-symbol`** - Symbol-aware search for Rust functions, structs, traits, and enums using AST parsing

### Execution & Testing
- **`run`** - Execute shell commands safely within sandbox constraints with timeout and output capture
- **`test-watch`** - Auto-rerun tests on file changes with smart test selection and real-time status

### Git Integration
- **`pr-summary`** - Generate comprehensive PR summaries from git history and changed files
- **`commit-template`** - Create conventional commit messages with scope detection and breaking change analysis

### Advanced Features
- **`fix-errors`** - Parse compiler errors and auto-suggest fixes with confidence scoring
- **`index`** - Analyze project structure with dependency graphs, symbol indexing, and impact analysis
- **`quick-action`** - Execute pre-defined workflow templates for common development tasks

### Undo/Redo System
- **`undo`** - Revert file operations with state restoration
- **`redo`** - Reapply undone operations
- **`history`** - View action log with detailed operation history

### Command Categories

**Read-Only Commands** (safe for exploration):
- `plan`, `diff`, `search`, `find-symbol`, `summarize`, `summarize_enhanced`, `index`

**Write Commands** (modify workspace):
- `create`, `edit`, `rename`, `delete`, `undo`, `redo`

**Execution Commands** (run code/tools):
- `run`, `test-watch`, `fix-errors`, `pr-summary`, `commit-template`

**Advanced Workflows**:
- `quick-action` - 8 built-in templates including "fix-error", "add-tests", "document-function", "optimize-code", "security-review"

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

🎯 **Production Ready** - Fennec has completed MVP and Phase 2 feature development:

**Phase 1 - MVP** (Completed):
- ✅ **Milestone 0** - Project scaffold and workspace setup
- ✅ **Milestone 1** - Core conversation loop and TUI
- ✅ **Milestone 2** - Editing and sandbox enforcement
- ✅ **Milestone 3** - Memory and summaries
- ✅ **Milestone 4** - Hardening and release readiness

**Phase 2 - Enhanced Features** (Completed):
- ✅ **Sprint 1** - Search command and file operations (create, rename, delete)
- ✅ **Sprint 2** - Hunk approval, action log, and undo/redo system
- ✅ **Sprint 3** - Symbol search, git integration, error fixes, test watching
- ✅ **Sprint 4** - Project indexing, quick action templates

**Test Coverage**: 145 tests passing (132 unit + 12 integration + 1 doc test)

See [docs/MVP.md](docs/MVP.md) and [docs/PHASE2_ROADMAP.md](docs/PHASE2_ROADMAP.md) for complete development roadmaps.

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