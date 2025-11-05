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
| Codebase indexing | Yes (automatic) | Partial (git awareness) | 🟡 | HIGH | Need full indexing |
| Resume sessions | Yes | Yes (transcripts) | ✅ | - | Working |

**Priority Items:**
1. Add file tree browser component
2. Implement automatic codebase indexing
3. Enhance session resume with full state

---

## 2. Codebase Awareness & Navigation

| Feature | Claude Code | Fennec | Status | Priority | Notes |
|---------|-------------|---------|--------|----------|-------|
| Project graph | Yes (searchable) | No | ❌ | HIGH | Critical for navigation |
| File tree browsing | Yes | No | ❌ | HIGH | Need UI component |
| Quick file open | Yes | No | ❌ | MEDIUM | Command palette |
| Symbol-aware search | Yes | No | ❌ | HIGH | AST parsing needed |
| Dependency awareness | Yes (manifests) | Partial (Cargo.toml) | 🟡 | MEDIUM | Expand to all deps |
| Contextual previews | Yes | Yes (preview panel) | ✅ | - | Working |
| File attachment | Yes | No | ❌ | MEDIUM | Attach specific files |

**Priority Items:**
1. Implement symbol-aware search (functions, types, traits)
2. Build project graph with dependencies
3. Add file tree UI component
4. Add quick file open command

---

## 3. Planning & Workflow Automation

| Feature | Claude Code | Fennec | Status | Priority | Notes |
|---------|-------------|---------|--------|----------|-------|
| Multi-step plans | Yes | Yes | ✅ | - | Working via plan command |
| Step-by-step approval | Yes | Yes | ✅ | - | Approval manager |
| Quick actions | Yes | No | ❌ | MEDIUM | Scaffold prompts |
| Plan editing | Yes (re-order, edit) | No | ❌ | LOW | Interactive plan modification |
| Partial plan approval | Yes | Partial | 🟡 | MEDIUM | Can approve individually |
| Iterative refinement | Yes | Yes | ✅ | - | Working |

**Priority Items:**
1. Add quick action templates
2. Implement interactive plan editing
3. Add plan persistence and resume

---

## 4. Editing & Refactoring Capabilities

| Feature | Claude Code | Fennec | Status | Priority | Notes |
|---------|-------------|---------|--------|----------|-------|
| Multi-file edits | Yes | Partial | 🟡 | HIGH | Currently single file focused |
| Side-by-side diffs | Yes | Yes (preview panel) | ✅ | - | Working |
| Individual hunk approval | Yes | No | ❌ | HIGH | Accept/reject per hunk |
| Create files | Yes | Partial | 🟡 | HIGH | Need explicit command |
| Rename files | Yes | No | ❌ | MEDIUM | Need rename command |
| Move files | Yes | No | ❌ | MEDIUM | Need move command |
| Delete files | Yes | No | ❌ | MEDIUM | Need delete command |
| Large-scale refactors | Yes | Partial | 🟡 | MEDIUM | Multi-file coordination |
| Generate documentation | Yes | Yes (summarize) | ✅ | - | Working |
| Code comments | Yes | Yes | ✅ | - | Working |
| Language migration | Yes | No | ❌ | LOW | Complex feature |

**Priority Items:**
1. Implement individual hunk approval/rejection
2. Add file creation command
3. Add file rename/move/delete commands
4. Enhance multi-file edit coordination

---

## 5. Execution & Tooling

| Feature | Claude Code | Fennec | Status | Priority | Notes |
|---------|-------------|---------|--------|----------|-------|
| Embedded terminal | Yes (streaming output) | Yes (via run command) | ✅ | - | Working |
| Command streaming | Yes | Yes | ✅ | - | Working |
| Auto-suggest fixes | Yes | No | ❌ | HIGH | Parse errors & suggest |
| Auto-rerun tests | Yes | No | ❌ | HIGH | After applying fix |
| Environment awareness | Yes | Partial | 🟡 | MEDIUM | Better detection |
| Project scripts | Yes | Yes | ✅ | - | Working |
| Formatter integration | Yes | Yes (cargo fmt) | ✅ | - | Working |
| Linter integration | Yes | Yes (cargo clippy) | ✅ | - | Working |
| Build pipelines | Yes | Yes | ✅ | - | Working |

**Priority Items:**
1. Implement auto-suggest fixes from command output
2. Add auto-rerun after fixes applied
3. Better environment detection (virtualenv, npm, etc.)

---

## 6. Testing & Quality Assurance

| Feature | Claude Code | Fennec | Status | Priority | Notes |
|---------|-------------|---------|--------|----------|-------|
| Detect test frameworks | Yes | Partial | 🟡 | MEDIUM | Cargo test only |
| Author tests | Yes | Yes | ✅ | - | Via AI |
| Update tests | Yes | Yes | ✅ | - | Via AI |
| Run relevant tests | Yes | Partial | 🟡 | HIGH | Need smart selection |
| Interpret failures | Yes | Partial | 🟡 | HIGH | Better error parsing |
| Suggest fixes | Yes | No | ❌ | HIGH | From test failures |
| Regression tests | Yes | Yes | ✅ | - | Via AI |

**Priority Items:**
1. Implement smart test selection (run relevant tests only)
2. Better test failure interpretation
3. Auto-suggest fixes from test failures

---

## 7. Code Understanding & Explanation

| Feature | Claude Code | Fennec | Status | Priority | Notes |
|---------|-------------|---------|--------|----------|-------|
| Explain code | Yes | Yes | ✅ | - | Via AI |
| Call graphs | Yes | No | ❌ | MEDIUM | AST analysis |
| Config explanations | Yes | Yes | ✅ | - | Via AI |
| PR summaries | Yes | Partial | 🟡 | MEDIUM | Need git integration |
| Change set analysis | Yes | Yes (diff) | ✅ | - | Working |
| Dependency audits | Yes | No | ❌ | MEDIUM | cargo-audit integration |
| Security pattern detection | Yes | Partial | 🟡 | MEDIUM | Basic checks |

**Priority Items:**
1. Add call graph generation
2. Enhance PR summary generation
3. Integrate cargo-audit for dependency checking

---

## 8. Search & Analysis Tools

| Feature | Claude Code | Fennec | Status | Priority | Notes |
|---------|-------------|---------|--------|----------|-------|
| Full-text search | Yes | No | ❌ | HIGH | Need search command |
| Semantic search | Yes | No | ❌ | MEDIUM | AI-powered search |
| Symbol search | Yes | No | ❌ | HIGH | Functions, types, traits |
| Usage search | Yes | No | ❌ | HIGH | Find all usages |
| Similar implementations | Yes | No | ❌ | LOW | AI-powered |
| Cross-repo search | Yes | No | ❌ | LOW | Out of scope for MVP |

**Priority Items:**
1. Implement full-text search command
2. Add symbol search (AST-based)
3. Add usage finding functionality

---

## 9. Safety, Controls & Review

| Feature | Claude Code | Fennec | Status | Priority | Notes |
|---------|-------------|---------|--------|----------|-------|
| User approval required | Yes | Yes | ✅ | - | Working |
| High-impact previews | Yes | Yes | ✅ | - | Working |
| Action log | Yes | No | ❌ | HIGH | History tracking |
| Undo operations | Yes | No | ❌ | HIGH | Rollback changes |
| Rerun steps | Yes | Partial | 🟡 | MEDIUM | Command history |
| Project sandbox | Yes | Yes | ✅ | - | Working |
| No external network | Yes | Yes (except API) | ✅ | - | Working |

**Priority Items:**
1. Implement comprehensive action log
2. Add undo/redo functionality
3. Add command history with rerun

---

## 10. Collaboration & Sharing

| Feature | Claude Code | Fennec | Status | Priority | Notes |
|---------|-------------|---------|--------|----------|-------|
| Export transcripts | Yes | Yes | ✅ | - | JSON format |
| Export diffs | Yes | Yes | ✅ | - | Working |
| Export patches | Yes | Yes | ✅ | - | Git format |
| Git integration | Yes | Yes | ✅ | - | Working |
| Commit messages | Yes | Partial | 🟡 | MEDIUM | AI-generated suggestions |
| Ready-to-commit patches | Yes | Yes | ✅ | - | Working |

**Priority Items:**
1. Enhance commit message generation
2. Add PR description generation

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

## Summary: High-Priority Missing Features

### Critical (Implement First)
1. **File Tree Browser** - Essential navigation component
2. **Symbol-Aware Search** - Find functions, types, traits
3. **Individual Hunk Approval** - Accept/reject specific changes
4. **Action Log & Undo** - History tracking and rollback
5. **Full-Text Search** - Project-wide code search

### High Priority (Implement Soon)
6. **File Operations Commands** - Create, rename, move, delete
7. **Auto-Suggest Fixes** - Parse errors and suggest corrections
8. **Auto-Rerun Tests** - After applying fixes
9. **Smart Test Selection** - Run only relevant tests
10. **Project Graph** - Searchable dependency and symbol graph

### Medium Priority (Enhancements)
11. **Quick Actions** - Template-based workflows
12. **Call Graph Generation** - Visualize function calls
13. **Enhanced PR Summaries** - Better git integration
14. **Anthropic Provider** - Claude model support
15. **MCP Integration** - External tool access

---

## Implementation Phases

### Phase 1: Core Navigation & Search (Week 1-2)
- File tree browser component
- Full-text search command
- Symbol-aware search
- Project indexing

### Phase 2: Enhanced Editing (Week 3-4)
- Individual hunk approval
- File operations (create, rename, move, delete)
- Multi-file edit coordination
- Action log and undo

### Phase 3: Smart Testing & Execution (Week 5-6)
- Auto-suggest fixes from errors
- Auto-rerun tests after fixes
- Smart test selection
- Better error interpretation

### Phase 4: Advanced Features (Week 7-8)
- Call graph generation
- Quick action templates
- Enhanced git integration
- Anthropic provider support

---

## Intentionally Excluded Features

These features are out of scope for Fennec's current design:
- Multi-agent orchestration (explicitly excluded by user)
- Cross-repository search (single project focus)
- Language migration (complex, low ROI)
- Web-based interface (terminal-first design)
