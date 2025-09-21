# Codex CLI Feature List

## Installation & Access
- Distributed as an open-source CLI (`npm i -g @openai/codex`, `brew install codex`, or platform-specific binaries from GitHub releases).
- Runs locally with optional sign-in via ChatGPT plans (Plus/Pro/Team/Edu/Enterprise) or API key authentication for usage-based billing.
- Supports headless environments via device-code login flow and API key overrides.

## Session Management & Modes
- Launches an interactive TUI (`codex`) or accepts an initial prompt argument for immediate execution.
- Provides non-interactive automation with `codex exec` plus `--full-auto` for hands-free runs.
- Offers session persistence: `codex resume`, `--last`, or session-id resumption for both TUI and headless sessions.
- Supports conversation backtracking (Esc–Esc) to edit and replay prior prompts.

## Workspace Awareness & Memory
- Uses layered `AGENTS.md` files (`~/.codex`, repo root, subdirectories) to capture persistent guidance, similar to project memory.
- Exposes `@` search to quickly reference files from the workspace and attach targeted context into prompts.
- Accepts image attachments (`--image/-i`) and arbitrary file uploads to enrich prompts.

## Planning, Automation & Examples
- Handles plan-and-execute workflows, scaffolding code, installing dependencies, running commands, and presenting diffs for approval.
- Ships with curated example prompts covering refactors, migrations, security review, bulk renames, and exploratory analysis.
- Supports project bootstrapping via `--full-auto` tasks that generate runnable apps and commit-ready diffs.

## Editing, Execution & Tooling
- Executes shell commands within the workspace sandbox, streaming output in the TUI.
- Automatically reruns tests or follow-up commands when previous steps fail, providing suggested fixes.
- Provides shell completions (`codex completion bash|zsh|fish`) and `--cd/-C` flag to operate on alternate directories.
- Integrates with project environments, detecting package managers (npm, poetry, cargo) and scripts defined in the repo.

## Sandbox & Approvals
- Three sandbox levels: `read-only`, `workspace-write`, and `danger-full-access`, with configurable approval policies (`untrusted`, `on-request`, `on-failure`, `never`).
- Default `Auto` preset allows workspace edits but prompts for network or out-of-scope access; `/approvals` command toggles modes.
- Provides `codex debug seatbelt|landlock` subcommands to test sandbox behavior on macOS (Seatbelt) and Linux (Landlock/seccomp).

## Configuration & Customization
- Centralized `~/.codex/config.toml` (or `$CODEX_HOME`) plus command-line overrides (`--config key=value`).
- Allows per-profile presets (`[profiles.*]`), project trust levels, notification hooks, and TUI options.
- Flexible model routing: define custom `model_providers` for OpenAI, Azure, Ollama, Mistral, or any OpenAI-compatible API (custom headers, query params, retries, SSE timeouts).
- Supports optional tools such as web search (`tools.web_search = true`) and experimental instruction files.

## Integrations & MCP
- Acts as an MCP **client** by launching external MCP servers defined under `mcp_servers` (command, args, env, startup timeout).
- Can run as an MCP **server** via `codex mcp`, exposing tools (`codex`, `codex-reply`) for multi-agent frameworks or the OpenAI Agents SDK.

## Non-Interactive & CI
- Designed for pipelines: install globally, authenticate with API key, and run `codex exec` inside CI/CD jobs.
- Headless sessions can be resumed (`codex exec resume --last`) and accept prompts via stdin to continue automation rolls.

## Observability & Compliance
- Uses `RUST_LOG` for granular logging; interactive logs stored under `~/.codex/log/` and tail-able in real time.
- Offers Zero Data Retention (ZDR) mode via `disable_response_storage` for regulated organizations.
- Emits desktop notifications (supporting terminals) and can invoke external notification commands with structured JSON payloads.

## Safety & Governance
- Approval requests logged within the session; action history is retained for auditing.
- Network access disabled by default in workspace-write mode unless explicitly enabled in config.
- Provides high-autonomy flags (`--yolo`) for trusted environments, alongside strong defaults to minimize accidental damage.
