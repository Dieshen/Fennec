# Fennec MVP Roadmap

## Guiding Principles
- Deliver a compelling single-chat experience before layering multi-chat and subagents.
- Reuse proven patterns from [Claude Code](./claude_code_featurelist.md), [Codex CLI](./codex_featurelist.md), and [Cline Memory Bank](./cline_memory_bank.md).
- Ship with security, auditability, and testing guardrails from day one.
- Keep documentation and implementation in lockstep; update C4 docs as decisions are made.

## Scope of MVP
1. Interactive TUI for a single conversation stream.
2. Provider abstraction with one fully working backend (OpenAI Chat Completions or local stub).
3. Core command loop: `plan`, `edit`, `run`, `diff`, `summarize` with preview + confirmation flow.
4. Sandbox + approval enforcement (`read-only`, `workspace-write`, `danger-full-access`) mirroring Codex defaults.
5. Unified in-memory transcript + lightweight `AGENTS.md` ingestion (global + repo root) stored in-memory.
6. Audit logging (JSON lines) for privileged actions and command outcomes.
7. Foundational tests (unit, integration, TUI snapshots) and smoke script.

## Milestone Breakdown

### Milestone 0 — Project Scaffold (Week 0)
- Initialize Cargo workspace, `Cargo.toml`, and base crates/modules (`tui`, `commands`, `orchestration`, `memory`, `provider`, `security`).
- Add key dependencies: `ratatui`, `crossterm`, `tokio`, `serde`, `serde_json`, `anyhow`, `thiserror`, `tracing`, `directories`.
- Set up tooling: `cargo fmt`, `cargo clippy`, `cargo test`; create `scripts/smoke.sh` stub.
- Establish CI placeholder (GitHub Action or script) running lint + tests.
- Document coding standards and testing expectations in `CONTRIBUTING.md` (if not already present).

**Exit Criteria**
- Repository builds with `cargo build`.
- `cargo fmt -- --check` and `cargo clippy` pass.
- Smoke script executes without errors (even if it only checks environment).

### Milestone 1 — Core Conversation Loop (Weeks 1-2)
- Implement TUI shell: layout, event loop, panes (chat, status, preview panel placeholder).
- Define `ProviderClient` trait and implement OpenAI backend (API key + Chat Completions).
- Build `SessionManager` for single-chat: handles message history, provider streaming, transcript storage.
- Implement `plan`, `edit`, `run`, `diff`, `summarize` command descriptors with preview scaffolding (dry-run output/diff simulation until file ops exist).
- Integrate simple `AGENTS.md` loader (global + repo root) and surface guidance in session initialization.

**Exit Criteria**
- User can start Fennec, provide prompts, receive responses from provider, and see transcript update in TUI.
- Commands return structured preview payloads even if they log "TODO" actions.
- AGENTS.md content appears in session banner/log.
- Unit tests cover SessionManager happy path and provider mocking.

### Milestone 2 — Editing & Sandbox Enforcement (Weeks 2-4)
- Implement file read/write operations with diff previews (inspired by Claude Code) and confirmation flows.
- Add command execution engine that applies edits after approval, persists backups if requested.
- Implement sandbox + approval policy matrix mirroring Codex behavior; add CLI flags (`--sandbox`, `--ask-for-approval`).
- Integrate audit logging (JSONL) capturing command id, capabilities, preview hash, approval state, result.
- Support `--cd` flag and verification of working directory.

**Exit Criteria**
- `plan` + `edit` commands produce diff previews, require explicit confirm; `run` executes shell command under sandbox.
- Audit log file created per session with entries for privileged actions.
- Sandbox policies enforce read-only vs workspace-write boundaries in tests.
- Smoke script performs simple edit -> confirm -> diff verification.

### Milestone 3 — Memory & Summaries (Weeks 4-5)
- Expand memory service to track transcripts, command plans, and user-provided notes per session.
- Adopt Cline-style files (`projectbrief.md`, `activeContext.md`, `progress.md`) for persistent storage (in-memory for MVP; file-backed optional).
- Implement `summarize` command that generates session summary and writes to memory file.
- Extend search capability to retrieve recent memory entries for context injection.

**Exit Criteria**
- Memory files update when summaries/notes are generated.
- `summarize` produces structured output saved to `progress.md` (or equivalent) and surfaced in TUI.
- Unit tests cover memory load/merge and summary persistence.

### Milestone 4 — Hardening & Readiness (Week 6)
- Finalize error handling strategy (`thiserror` enums, user-friendly TUI messages).
- Add telemetry toggles (`tracing` to file); document log rotation/retention plan.
- Flesh out integration tests simulating planner → edit → run; incorporate provider fakes and filesystem sandbox tests.
- Complete documentation: update C4 diagrams with implementation notes, `README`, usage examples, known limitations.
- Prepare release checklist (binary packaging plan, environment prerequisites, future milestones for subagents/MCP).

**Exit Criteria**
- All tests + smoke script pass in CI.
- Documentation aligns with shipped features and states what is deferred.
- MVP ready for early user testing and feedback.

## Deferred (Post-MVP) Items
- Multi-chat orchestration and subagent coordination.
- Semantic search + embeddings for memory index (Tantivy/Qdrant).
- Macro system & recipes (`config/commands.toml`).
- MCP client/server support.
- Additional providers (Anthropic, OpenRouter, Ollama).
- Persistent storage backend (`sled`/SQLite) and encryption at rest.

## Security & Compliance Checklist (MVP)
- Sandbox + approval policies implemented and configurable.
- Credentials sourced from env vars or OS keyring; never logged.
- Audit logs written per session with rotation strategy defined.
- Dependency supply-chain review documented (cargo audit, minimal optional features).
- Threat model stub covering sandbox bypass, provider misuse, and memory data leakage.

## Testing Strategy Snapshot
- **Unit tests**: Session manager, provider adapter mocks, command validation, memory loader.
- **Integration tests**: TUI harness executing plan→edit workflow, sandbox enforcement tests.
- **Snapshot tests**: Rendered TUI states for transcript + diff view.
- **Smoke script**: End-to-end CLI invocation verifying provider connectivity (mock/stub) and audit log creation.

## Milestone Governance
- Hold milestone kickoff/review meetings; adjust roadmap based on findings.
- Update C4 docs and MVP roadmap after each milestone completion.
- Track work items in issue tracker aligned with milestone structure.
