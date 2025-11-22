# Claude Code vs Fennec Feature Comparison

This document provides a comprehensive comparison of features between Claude Code and Fennec to identify gaps and prioritize enhancements.

## Status Legend
- ✅ **Implemented** - Feature fully implemented
- 🟡 **Partial** - Feature partially implemented or basic version exists
- ❌ **Missing** - Feature not yet implemented
- 🚫 **Out of Scope** - Feature intentionally excluded (e.g., multi-agent)

---

## 1. Access & Workspace Setup

| Feature | Claude Code | Fennec | Status | Priority | Notes |
|---------|-------------|---------|--------|----------|-------|
| Split-pane interface | Yes (chat, file tree, diff, terminal) | Yes (chat, preview, status) | 🟡 | HIGH | Need file tree pane |
| Local folder import | Yes (upload/sync) | Yes (--cd flag) | ✅ | - | Working |
| Project-level context | Yes | Yes (AGENTS.md, memory) | ✅ | - | Working |
| Codebase indexing | Yes (automatic) | Yes (index command) | ✅ | - | **Phase 2 ✅** |
| Resume sessions | Yes | Yes (transcripts) | ✅ | - | Working |

**Remaining Priority Items:**
1. Add file tree browser component (UI enhancement)

---

## 2. Codebase Awareness & Navigation

| Feature | Claude Code | Fennec | Status | Priority | Notes |
|---------|-------------|---------|--------|----------|-------|
| Project graph | Yes (searchable) | Yes (index command) | ✅ | - | **Phase 2 ✅** |
| File tree browsing | Yes | No | ❌ | HIGH | Need UI component |
| Quick file open | Yes | No | ❌ | MEDIUM | Command palette |
| Symbol-aware search | Yes | Yes (find-symbol) | ✅ | - | **Phase 2 ✅** |
| Dependency awareness | Yes (manifests) | Yes (dependency_graph) | ✅ | - | **Phase 2 ✅** |
| Contextual previews | Yes | Yes (preview panel) | ✅ | - | Working |
| File attachment | Yes | No | ❌ | MEDIUM | Attach specific files |

**Remaining Priority Items:**
1. Add file tree UI component
2. Add quick file open command
3. Implement file attachment to context

---

## 3. Planning & Workflow Automation

| Feature | Claude Code | Fennec | Status | Priority | Notes |
|---------|-------------|---------|--------|----------|-------|
| Multi-step plans | Yes | Yes | ✅ | - | Working via plan command |
| Step-by-step approval | Yes | Yes | ✅ | - | Approval manager |
| Quick actions | Yes | Yes (quick-action) | ✅ | - | **Phase 2 ✅** (8 templates) |
| Plan editing | Yes (re-order, edit) | No | ❌ | LOW | Interactive plan modification |
| Partial plan approval | Yes | Yes | ✅ | - | Working |
| Iterative refinement | Yes | Yes | ✅ | - | Working |

**Remaining Priority Items:**
1. Implement interactive plan editing (re-order, modify steps)
2. Add plan persistence and resume capability

---

## 4. Editing & Refactoring Capabilities

| Feature | Claude Code | Fennec | Status | Priority | Notes |
|---------|-------------|---------|--------|----------|-------|
| Multi-file edits | Yes | Partial | 🟡 | HIGH | Sequential edits work |
| Side-by-side diffs | Yes | Yes (preview panel) | ✅ | - | Working |
| Individual hunk approval | Yes | Yes (hunks module) | ✅ | - | **Phase 2 ✅** |
| Create files | Yes | Yes (create command) | ✅ | - | **Phase 2 ✅** |
| Rename files | Yes | Yes (rename command) | ✅ | - | **Phase 2 ✅** |
| Move files | Yes | Partial (via rename) | 🟡 | MEDIUM | Works within directories |
| Delete files | Yes | Yes (delete command) | ✅ | - | **Phase 2 ✅** |
| Large-scale refactors | Yes | Partial | 🟡 | MEDIUM | Multi-file coordination |
| Generate documentation | Yes | Yes (summarize + AI) | ✅ | - | Working |
| Code comments | Yes | Yes (via AI) | ✅ | - | Working |
| Language migration | Yes | No | 🚫 | - | Out of scope |

**Remaining Priority Items:**
1. Enhance multi-file edit coordination (atomic multi-file changes)
2. Improve move command for cross-directory moves

---

## 5. Execution & Tooling

| Feature | Claude Code | Fennec | Status | Priority | Notes |
|---------|-------------|---------|--------|----------|-------|
| Embedded terminal | Yes (streaming output) | Yes (via run command) | ✅ | - | Working |
| Command streaming | Yes | Yes | ✅ | - | Working |
| Auto-suggest fixes | Yes | Yes (fix-errors) | ✅ | - | **Phase 2 ✅** |
| Auto-rerun tests | Yes | Yes (test-watch) | ✅ | - | **Phase 2 ✅** |
| Environment awareness | Yes | Partial | 🟡 | MEDIUM | Better detection |
| Project scripts | Yes | Yes | ✅ | - | Working |
| Formatter integration | Yes | Yes (cargo fmt) | ✅ | - | Working |
| Linter integration | Yes | Yes (cargo clippy) | ✅ | - | Working |
| Build pipelines | Yes | Yes | ✅ | - | Working |

**Remaining Priority Items:**
1. Better environment detection (virtualenv, npm, node, etc.)
2. Extend fix-errors to non-Rust languages

---

## 6. Testing & Quality Assurance

| Feature | Claude Code | Fennec | Status | Priority | Notes |
|---------|-------------|---------|--------|----------|-------|
| Detect test frameworks | Yes | Partial | 🟡 | MEDIUM | Cargo test only |
| Author tests | Yes | Yes | ✅ | - | Via AI + quick-action |
| Update tests | Yes | Yes | ✅ | - | Via AI |
| Run relevant tests | Yes | Yes (test-watch) | ✅ | - | **Phase 2 ✅** |
| Interpret failures | Yes | Yes (fix-errors) | ✅ | - | **Phase 2 ✅** |
| Suggest fixes | Yes | Yes (fix-errors) | ✅ | - | **Phase 2 ✅** |
| Regression tests | Yes | Yes | ✅ | - | Via AI |

**Remaining Priority Items:**
1. Extend test framework detection beyond Cargo (pytest, jest, etc.)
2. Enhanced test failure interpretation with AI analysis

---

## 7. Code Understanding & Explanation

| Feature | Claude Code | Fennec | Status | Priority | Notes |
|---------|-------------|---------|--------|----------|-------|
| Explain code | Yes | Yes (AI + quick-action) | ✅ | - | Working |
| Call graphs | Yes | No | ❌ | MEDIUM | AST analysis |
| Config explanations | Yes | Yes | ✅ | - | Via AI |
| PR summaries | Yes | Yes (pr-summary) | ✅ | - | **Phase 2 ✅** |
| Change set analysis | Yes | Yes (diff + index) | ✅ | - | Working |
| Dependency audits | Yes | No | ❌ | MEDIUM | cargo-audit integration |
| Security pattern detection | Yes | Yes (quick-action) | ✅ | - | **Phase 2 ✅** (security-review template) |

**Remaining Priority Items:**
1. Add call graph generation (visualize function dependencies)
2. Integrate cargo-audit for dependency vulnerability checking
3. Enhanced security pattern detection with static analysis

---

## 8. Search & Analysis Tools

| Feature | Claude Code | Fennec | Status | Priority | Notes |
|---------|-------------|---------|--------|----------|-------|
| Full-text search | Yes | Yes (search command) | ✅ | - | **Phase 2 ✅** (regex, case-insensitive) |
| Semantic search | Yes | No | ❌ | MEDIUM | AI-powered search |
| Symbol search | Yes | Yes (find-symbol) | ✅ | - | **Phase 2 ✅** (AST-based) |
| Usage search | Yes | No | ❌ | MEDIUM | Find all symbol usages |
| Similar implementations | Yes | No | ❌ | LOW | AI-powered |
| Cross-repo search | Yes | No | 🚫 | - | Out of scope |

**Remaining Priority Items:**
1. Add semantic search (AI-powered code understanding)
2. Add usage finding (find all usages of symbol)
3. Similar implementation search (AI-powered pattern matching)

---

## 9. Safety, Controls & Review

| Feature | Claude Code | Fennec | Status | Priority | Notes |
|---------|-------------|---------|--------|----------|-------|
| User approval required | Yes | Yes | ✅ | - | Working |
| High-impact previews | Yes | Yes | ✅ | - | Working |
| Action log | Yes | Yes (history command) | ✅ | - | **Phase 2 ✅** |
| Undo operations | Yes | Yes (undo/redo) | ✅ | - | **Phase 2 ✅** |
| Rerun steps | Yes | Partial | 🟡 | MEDIUM | Command history |
| Project sandbox | Yes | Yes | ✅ | - | Working |
| No external network | Yes | Yes (except API) | ✅ | - | Working |

**Remaining Priority Items:**
1. Add command history with rerun capability
2. Persistent action log across sessions

---

## 10. Collaboration & Sharing

| Feature | Claude Code | Fennec | Status | Priority | Notes |
|---------|-------------|---------|--------|----------|-------|
| Export transcripts | Yes | Yes | ✅ | - | JSON format |
| Export diffs | Yes | Yes | ✅ | - | Working |
| Export patches | Yes | Yes | ✅ | - | Git format |
| Git integration | Yes | Yes | ✅ | - | Working |
| Commit messages | Yes | Yes (commit-template) | ✅ | - | **Phase 2 ✅** (conventional commits) |
| Ready-to-commit patches | Yes | Yes | ✅ | - | Working |

**Remaining Priority Items:**
1. Enhanced PR description generation with AI analysis
2. Export artifacts in multiple formats (HTML, PDF, etc.)

---

## 11. Extensibility & Integrations

| Feature | Claude Code | Fennec | Status | Priority | Notes |
|---------|-------------|---------|--------|----------|-------|
| Multiple AI models | Yes | Partial | 🟡 | MEDIUM | OpenAI only currently |
| MCP server support | Yes | No | ❌ | MEDIUM | Model Context Protocol |
| Plugin system | No (built-in) | No | ❌ | LOW | Future enhancement |
| External tool hooks | Yes (MCP) | Partial | 🟡 | MEDIUM | Limited |

**Priority Items:**
1. Add Anthropic provider support
2. Implement MCP server integration
3. Design plugin architecture

---

## Summary: Phase 2 Achievements ✅

### Phase 2 Complete - 13 Major Features Delivered
1. ✅ **Full-Text Search** - Project-wide code search with regex (search command)
2. ✅ **Symbol-Aware Search** - Find functions, types, traits (find-symbol command)
3. ✅ **Individual Hunk Approval** - Accept/reject specific changes (hunks module)
4. ✅ **Action Log & Undo** - History tracking and rollback (undo/redo/history commands)
5. ✅ **File Operations Commands** - Create, rename, delete (create/rename/delete commands)
6. ✅ **Auto-Suggest Fixes** - Parse errors and suggest corrections (fix-errors command)
7. ✅ **Auto-Rerun Tests** - File watching with smart test selection (test-watch command)
8. ✅ **Project Graph & Indexing** - Dependency graph and symbol cross-referencing (index command)
9. ✅ **Quick Actions** - 8 template-based workflows (quick-action command)
10. ✅ **Enhanced PR Summaries** - Git history analysis (pr-summary command)
11. ✅ **Commit Message Generation** - Conventional commits (commit-template command)
12. ✅ **Security Pattern Detection** - Security review template (quick-action security-review)
13. ✅ **Impact Analysis** - Affected symbols and tests (index impact mode)

**Commands Delivered:** 17 production-ready commands with 145 passing tests

---

## Remaining Features for Phase 3

### High Priority (UI & UX Enhancements)
1. **File Tree Browser** - TUI component for file navigation
2. **Multi-File Edit Coordination** - Atomic multi-file changes
3. **Command History with Rerun** - Persistent command replay
4. **File Attachment to Context** - Explicitly attach files to conversation

### Medium Priority (Advanced Analysis)
5. **Call Graph Generation** - Visualize function dependencies
6. **Usage Search** - Find all usages of a symbol
7. **Semantic Search** - AI-powered code understanding
8. **Dependency Audit Integration** - cargo-audit for vulnerability checking
9. **Enhanced Environment Detection** - Better detection for non-Rust projects

### Medium Priority (Provider & Integration)
10. **Anthropic Provider** - Claude model support
11. **MCP Server Integration** - Model Context Protocol
12. **Multi-Language Support** - Extend commands beyond Rust
13. **Interactive Plan Editing** - Re-order and modify plan steps

### Lower Priority (Advanced Features)
14. **Persistent Action Log** - Cross-session history
15. **Enhanced Export Formats** - HTML, PDF output
16. **Similar Implementation Search** - AI-powered pattern matching

---

## Implementation Phases

### Phase 1: MVP Foundation ✅ (COMPLETED)
- Core conversation loop and TUI
- Basic editing and sandbox enforcement
- Memory and session management
- Provider integration (OpenAI)

### Phase 2: Feature Parity ✅ (COMPLETED)
- **Sprint 1** ✅: Search and file operations
- **Sprint 2** ✅: Hunk approval, action log, undo/redo
- **Sprint 3** ✅: Symbol search, git integration, error fixes, test watching
- **Sprint 4** ✅: Project indexing, quick action templates

### Phase 3: UI/UX & Advanced Features (NEXT)
- **Sprint 5**: File tree browser, multi-file coordination
- **Sprint 6**: Call graphs, usage search, semantic search
- **Sprint 7**: Anthropic provider, MCP integration
- **Sprint 8**: Polish, performance, multi-language support

---

## Intentionally Excluded Features

These features are out of scope for Fennec's current design:
- Multi-agent orchestration (explicitly excluded by user)
- Cross-repository search (single project focus)
- Language migration (complex, low ROI)
- Web-based interface (terminal-first design)
