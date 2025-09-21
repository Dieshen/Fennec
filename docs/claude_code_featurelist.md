# Claude Code Feature List

## Access & Workspace Setup
- Accessible from Claude web and Claude Desktop under the **Claude Code / Projects** workspace with a split-pane interface (chat, file tree, diff view, terminal).
- Supports importing projects from local folders (upload or sync) with automatic indexing of the entire codebase.
- Maintains project-level context and conversation history so sessions can be resumed without re-uploading code.

## Codebase Awareness & Navigation
- Builds a searchable project graph: understands directory layout, dependency manifests, tests, and configuration files.
- Provides file tree browsing, quick file open, symbol-aware search, and contextual previews before edits.
- Lets users attach files or snippets from the project into a message for targeted guidance.

## Planning & Workflow Automation
- Generates explicit multi-step plans before making changes; each step shows the intended command or edit for approval.
- Offers quick actions (e.g., **Implement feature**, **Fix failing test**, **Explain code**) that scaffold prompts and plans.
- Supports iterative refinement—plans can be edited, re-ordered, or partially approved before execution.

## Editing & Refactoring Capabilities
- Performs multi-file edits with side-by-side diffs; each hunk can be reviewed, accepted, or rejected individually.
- Can create, rename, move, and delete files or directories with confirmation prompts.
- Handles large-scale refactors (interface changes, dependency upgrades) while keeping related files in sync.
- Generates documentation (docstrings, README sections) and code comments on request.
- Can migrate code between languages or frameworks when supplied with target constraints.

## Execution & Tooling
- Embedded terminal runs shell commands (`npm test`, `cargo build`, `pytest`, etc.) with streaming output.
- Recognizes failing commands and can suggest/rerun fixes automatically (e.g., rerun tests after applying a patch).
- Integrates with project environments (virtualenv, npm, poetry, cargo) by respecting existing scripts and lockfiles.
- Offers task-specific commands such as running formatters, linters, database migrations, or build pipelines.

## Testing & Quality Assurance
- Detects existing test frameworks and can author or update unit/integration tests alongside code changes.
- Suggests running relevant test suites after modifications and interprets failures to propose fixes.
- Can synthesize regression tests from bug descriptions or failing stack traces.

## Code Understanding & Explanation
- Explains unfamiliar code sections, call graphs, or configuration files with inline references to source lines.
- Summarizes pull requests or change sets, highlighting risk areas and follow-up tasks.
- Performs dependency or API audits (e.g., identify usage of deprecated functions, find insecure patterns).

## Search & Analysis Tools
- Full-text and semantic search across the project to locate symbols, usages, or similar implementations.
- Supports semantic and full-text search prompts (e.g., "show files touching authentication middleware") across large repositories.

## Safety, Controls & Review
- Every command or edit requires user approval; high-impact actions show both plan and diff previews.
- Maintains an action log so users can backtrack, undo, or rerun prior steps.
- Operates within the project sandbox—no network or system access outside the workspace without explicit commands.

## Collaboration & Sharing
- Allows exporting chat transcripts, diffs, and generated patches for external review or Git commits.
- Integrates with Git workflows by producing ready-to-commit patches and commit message suggestions.

## Extensibility & Integrations
- Works with Claude 3.5 Sonnet and Haiku models for code generation, explanation, and tooling.
- Hooks into MCP (Model Context Protocol) servers for external tool access (e.g., additional linters, database shells).
