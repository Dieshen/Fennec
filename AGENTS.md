# Repository Guidelines

## Project Structure & Module Organization
- `src/cli/` holds the TUI front end: layout primitives, input handling, and session routing for single vs. multi-chat panels.
- `src/agents/` contains reusable agent behaviors inspired by Claude Code, Codex, and AutoGen (prompt templates, tool adapters, orchestration traits).
- `src/core/` provides shared domain logic (state machine, transcript store, capability registry).
- `assets/themes/` defines colors, keymaps, and layout presets for the TUI.
- `tests/` covers integration scenarios; colocate unit tests in the same Rust module with `#[cfg(test)]` blocks.
- `examples/` documents runnable demo flows; keep matching configuration files in `config/`.

## Build, Test, and Development Commands
- `cargo fmt` formats Rust code; use `cargo fmt -- --check` in CI.
- `cargo clippy --all-targets --all-features` enforces lint rules tuned for async + TUI code.
- `cargo test` runs unit and integration tests; add `-- --ignored` for long-running chat orchestration cases.
- `cargo run --bin fennec --features tui` launches the interactive client against local backends.
- `scripts/smoke.sh` (bash) should exercise a minimal single-chat session; extend as new workflows land.

## Coding Style & Naming Conventions
- Follow Rust 2021 edition defaults with four-space indentation and SnakeCase for modules/functions.
- Use PascalCase for types and Agent structs (e.g., `ClaudeLikeAgent`).
- Expose async entry points with explicit error types (`Result<T, CliError>`); never `unwrap()` in production paths.
- Document non-obvious orchestration decisions with `///` doc comments above public items.

## Testing Guidelines
- Rely on `tokio::test` for async scenarios; gate network-dependent tests with `#[ignore]` and document prerequisites.
- Structure fixtures under `tests/fixtures/` (JSON prompts, mock tool responses) and load via `include_str!` for determinism.
- Maintain coverage of core chat flows (message routing, tool invocation retries, transcript persistence) before merging.

## Commit & Pull Request Guidelines
- Use Conventional Commits (`feat:`, `fix:`, `chore:`) and keep each change atomic.
- PRs must describe the agent capability added, affected commands, and screenshots/gif of the TUI when UI changes occur.
- Link tracking issues and note follow-up tasks; request review from agent-runtime and TUI owners.

## Security & Configuration Tips
- Never hardcode API keys; load from `.env` or OS keyring and document required variables in `config/example.env`.
- Validate all tool outputs before acting (limit filesystem writes, sanitize shell commands).
- Record sensitive logs behind `debug` features only; default builds should avoid emitting PII.
