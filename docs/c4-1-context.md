# Fennec C4 Level 1 — System Context

## Purpose
Outline how the Fennec AI CLI fits into a developer's workflow, the actors that interact with it, and the high-level responsibilities of the system.

## Scope
- **Production Ready**: Single-chat experience with comprehensive TUI (✅ Implemented)
- **Provider Support**: OpenAI integration with extensible architecture for Anthropic, OpenRouter, Ollama (✅ OpenAI Complete)
- **Security Model**: Three-tier sandbox system with approval workflows and audit trails (✅ Implemented)
- **Memory System**: AGENTS.md integration, Cline-style memory files, and session persistence (✅ Implemented)
- **Command System**: Plan, edit, run, diff, summarize with preview and confirmation flows (✅ Implemented)
- **Future Roadmap**: Multi-chat orchestration, helper subagents, and MCP server integration

## Primary Actors
- **Developer Operator**: Launches the CLI, issues commands, navigates the TUI.
- **LLM Providers**: External APIs supplying language model completions.
- **Local Tooling**: Git repository, filesystem, and optional Ollama runtime.
- **MCP Servers**: External procedure hosts exposing additional tools and automation.

## System Responsibilities
- **TUI Interface**: Provide responsive terminal interface with ratatui and crossterm (✅ fennec-tui crate)
- **Memory Management**: Unified memory layer with AGENTS.md, Cline-style files, and session transcripts (✅ fennec-memory crate)
- **Command Execution**: Safe command execution with sandbox policies and approval workflows (✅ fennec-commands + fennec-security)
- **Provider Integration**: Streaming LLM integration with OpenAI Chat Completions API (✅ fennec-provider crate)
- **Session Orchestration**: Session management with audit logging and context preservation (✅ fennec-orchestration crate)
- **Security Enforcement**: Three-tier sandbox with capability-based permissions and audit trails (✅ fennec-security crate)

## External Dependencies & Trust Boundaries

### Implemented Dependencies
- **OpenAI API**: HTTPS requests to api.openai.com with API key authentication (✅ fennec-provider)
- **Local Filesystem**: Read/write operations constrained by sandbox policies and workspace boundaries (✅ fennec-security)
- **Git Integration**: Repository analysis and history awareness for context (✅ fennec-memory)
- **Environment Configuration**: .env files, OS keyring, and user config directories (✅ fennec-core)

### Future Dependencies
- **Anthropic API**: Claude API integration for additional provider choice
- **Ollama**: Local LLM runtime for offline functionality
- **OpenRouter**: Multi-provider API gateway
- **MCP Servers**: Model Context Protocol for external tool integration

## Quality Attributes
- Low-latency interactive feedback in the terminal UI.
- Secure handling of credentials and tool outputs, with audit logging for privileged actions.
- Offline functionality when Ollama is present; graceful degradation otherwise.
- Guardrails on command execution (diff previews, confirmations, capability-aware sandboxing).

## Mermaid Diagram
```mermaid
graph TD
    Developer[Developer Operator] -->|Commands & Navigation| FennecCore[Fennec Core]
    subgraph Fennec[ Fennec AI CLI ]
        FennecCore
        Subagents[Helper Subagents\n(Planner/Reviewer/Executor)]
    end
    FennecCore -->|Orchestrates| Subagents
    FennecCore -->|LLM Requests| Providers[Hosted LLM Providers\n(OpenAI, Anthropic, OpenRouter)]
    FennecCore -->|Local API| Ollama[Ollama Runtime]
    FennecCore -->|File Ops & Git| Tooling[Local Tooling\n(Git, Filesystem)]
    FennecCore -->|MCP Protocol| MCP[MCP Servers\n(Codex, Claude Code)]
    Ollama -.Optional Offline Path.- FennecCore
```

## Reference Material
- [Claude Code Feature Inventory](./claude_code_featurelist.md) — highlights end-user expectations for planning, edits, and reviews.
- [Codex CLI Feature Inventory](./codex_featurelist.md) — captures sandboxing, approval, and configuration patterns we aim to match.
- [Cline Memory Bank Notes](./cline_memory_bank.md) — informs unified memory design and recall behavior.

## Implementation Notes

### Completed Features (MVP Ready)
- **Credential Management**: Environment variables (.env), OS keyring support planned, TOML configuration files
- **Telemetry**: Structured tracing with configurable levels, audit logging in JSON Lines format
- **Command Registry**: Extensible system with built-in commands (plan, edit, run, diff, summarize, enhanced-summarize)
- **Security Model**: PolicyResult enum with Allow/Deny/RequireApproval, risk-based command classification
- **Memory Architecture**: Adapters for AGENTS.md, Cline-style files, Git history, with search capabilities

### Architecture Decisions Made
- **Rust Workspace**: Modular design with 8 crates for clear separation of concerns
- **Async Runtime**: Tokio for all async operations with streaming LLM responses
- **Error Handling**: `thiserror` for structured errors, `anyhow` for error context
- **Configuration**: TOML files with environment variable substitution
- **TUI Framework**: ratatui with crossterm for cross-platform terminal support
- **Security**: Capability-based permissions with sandbox policy enforcement

### Technical Debt and Improvements
- Add support for additional LLM providers (Anthropic, Ollama)
- Implement MCP server integration for external tools
- Add semantic search with embeddings for memory system
- Implement multi-chat orchestration and subagent coordination
- Add plugin system for custom commands and workflows
