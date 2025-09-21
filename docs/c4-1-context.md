# Fennec C4 Level 1 — System Context

## Purpose
Outline how the Fennec AI CLI fits into a developer's workflow, the actors that interact with it, and the high-level responsibilities of the system.

## Scope
- Single-chat experience (initial milestone)
- Planned multi-chat orchestration and helper subagents (planner, reviewer, executor roles)
- Support for OpenAI, OpenRouter, Anthropic, and Ollama providers
- Integration with MCP servers (Codex, Claude Code) and local file system operations

## Primary Actors
- **Developer Operator**: Launches the CLI, issues commands, navigates the TUI.
- **LLM Providers**: External APIs supplying language model completions.
- **Local Tooling**: Git repository, filesystem, and optional Ollama runtime.
- **MCP Servers**: External procedure hosts exposing additional tools and automation.

## System Responsibilities
- Provide a performant TUI for interactive agent sessions.
- Manage a unified memory layer combining `AGENTS.md`, `CLAUDE.md`, `.memory_bank`, and git history insights.
- Execute built-in and custom commands safely, surfacing previews and guardrails.
- Coordinate helper subagents and route conversational turns across single or multi-chat modes.

## External Dependencies & Trust Boundaries
- Network calls to hosted LLM APIs (OpenAI, Anthropic, OpenRouter).
- Local socket or HTTP access to Ollama.
- MCP protocol communication with Codex and Claude Code servers.
- Local file system read/write restricted by sandbox policies.

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

## Open Questions
- How will credentials be supplied (env vars, keyring, config files)?
- What telemetry or analytics, if any, will the system emit?
- Are there future external systems (issue trackers, knowledge bases) to anticipate?
