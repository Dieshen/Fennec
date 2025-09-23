# Fennec Crate Structure

## Workspace Organization

Fennec is organized as a Rust workspace with 9 distinct crates, each with clear responsibilities and minimal coupling.

```
crates/
├── fennec-cli/           # Main CLI binary and application entry point
├── fennec-core/          # Shared domain types, traits, and core business logic
├── fennec-tui/           # Terminal user interface components and layout
├── fennec-orchestration/ # Session management and agent coordination
├── fennec-memory/        # Memory persistence and context management
├── fennec-provider/      # LLM provider implementations with streaming support
├── fennec-security/      # Sandbox policies, approval workflows, and audit system
├── fennec-commands/      # Built-in command implementations with registry
└── fennec-telemetry/     # Optional telemetry and metrics collection
```

## Crate Dependencies & Responsibilities

### fennec-cli
**Purpose**: Application entry point and command-line interface
**Key Responsibilities**:
- CLI argument parsing with clap
- Application bootstrap and configuration loading
- Main loop initialization and graceful shutdown
- Environment setup (logging, telemetry, etc.)

**Dependencies**: All other crates (orchestrates the entire application)

### fennec-core
**Purpose**: Shared domain logic and foundational types
**Key Responsibilities**:
- Core domain types (Message, Session, Tool, etc.)
- Shared traits and interfaces
- Error types and result handling
- Configuration structures
- Utilities and helper functions

**Dependencies**: Minimal - only external libraries, no internal crates
**Used by**: All other crates

### fennec-tui
**Purpose**: Terminal user interface components and layout management
**Key Responsibilities**:
- TUI layout primitives and components
- Input handling and keyboard shortcuts
- Event processing and state management
- Panel management (chat, preview, status)
- Theme and styling system

**Dependencies**: fennec-core, ratatui, crossterm
**Integration**: Receives events, sends commands to orchestration layer

### fennec-orchestration
**Purpose**: Session management and high-level coordination
**Key Responsibilities**:
- Session lifecycle management
- Agent coordination and workflow orchestration
- Message routing and processing
- State machine implementation
- Integration between TUI and backend services

**Dependencies**: fennec-core, fennec-memory, fennec-provider, fennec-security, fennec-commands

### fennec-memory
**Purpose**: Memory persistence, context management, and AGENTS.md integration
**Key Responsibilities**:
- Memory file management (projectbrief.md, activeContext.md, progress.md)
- AGENTS.md parsing and integration
- Session transcript persistence
- Context retrieval and search
- File watching and change detection

**Dependencies**: fennec-core, notify, fuzzy-matcher
**Features**: Cline-style memory compatibility

### fennec-provider
**Purpose**: LLM provider abstractions and implementations
**Key Responsibilities**:
- Provider trait definitions and abstractions
- OpenAI API integration with streaming support
- Request/response handling and error management
- Rate limiting and retry logic
- Future: Anthropic, OpenRouter, Ollama integrations

**Dependencies**: fennec-core, reqwest, tokio, futures
**Extensibility**: Plugin-ready architecture for new providers

### fennec-security
**Purpose**: Security sandbox, approval workflows, and comprehensive audit system
**Key Responsibilities**:
- Three-tier sandbox implementation (read-only, workspace-write, danger-full-access)
- Path traversal protection and command filtering
- Approval workflow management with risk assessment
- Comprehensive audit trail generation (JSON format)
- Capability-based permission system

**Dependencies**: fennec-core, ring (cryptography)
**Security Features**: Enterprise-grade controls and logging

### fennec-commands
**Purpose**: Built-in command implementations with extensible registry
**Key Responsibilities**:
- Core command implementations (plan, edit, run, diff, summarize)
- Command registry and capability declarations
- Tool integration and result processing
- Command validation and execution
- Extensible plugin architecture

**Dependencies**: fennec-core, fennec-security, similar (for diff)
**Extensibility**: Plugin-ready for custom commands

### fennec-telemetry (Optional)
**Purpose**: Telemetry and metrics collection (optional feature)
**Key Responsibilities**:
- Usage metrics and performance monitoring
- Error reporting and diagnostics
- Optional telemetry data collection
- Privacy-conscious implementation

**Dependencies**: fennec-core
**Features**: Disabled by default, opt-in only

## Dependency Flow

```
fennec-cli
    ├── fennec-orchestration
    │   ├── fennec-memory ────── fennec-core
    │   ├── fennec-provider ──── fennec-core
    │   ├── fennec-security ──── fennec-core
    │   ├── fennec-commands ──── fennec-core
    │   └── fennec-core
    ├── fennec-tui ──────────── fennec-core
    └── fennec-telemetry ────── fennec-core (optional)
```

## Key Design Patterns

### Separation of Concerns
- **UI layer** (fennec-tui) is completely separate from business logic
- **Core domain** (fennec-core) provides shared abstractions
- **Security** (fennec-security) is a cross-cutting concern applied consistently

### Provider Pattern
- **Abstractions** in fennec-core define provider contracts
- **Implementations** in fennec-provider handle specific LLM APIs
- **Extensibility** allows adding new providers without core changes

### Command Pattern
- **Registry** in fennec-commands manages available commands
- **Capabilities** declare what permissions each command needs
- **Execution** is mediated through security sandbox

### Event-Driven Architecture
- **TUI events** flow through orchestration layer
- **Provider responses** are streamed back through the same path
- **Memory updates** happen asynchronously

## Build Configuration

### Workspace Features
- **Default**: All standard features enabled
- **Telemetry**: Optional telemetry collection (fennec-telemetry)
- **Development**: Additional debugging and test utilities

### Performance Optimizations
- **Release profile**: Full optimization with LTO and strip
- **Async runtime**: Tokio with full feature set
- **Memory efficiency**: Careful resource management for long-running sessions

---

*This structure provides clear separation of concerns while maintaining flexibility for future extensions and modifications.*