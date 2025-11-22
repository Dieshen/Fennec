# Phase 2 Enhancement Roadmap

## Overview

This document outlines the remaining work to bring Fennec closer to Claude Code feature parity, building on Phase 1's foundation.

## Phase 1 Completed ✅

- **Feature Comparison Document**: Comprehensive analysis of 11 categories
- **File Tree Browser Component**: Production-ready UI component with full navigation
- **Rust Best Practices Plugin**: Claude Code plugin with team-shareable standards
- **Telemetry Fix**: Added missing `gauge` macro import
- **Documentation**: Implementation summary and architecture notes

## Phase 2 Priority Tasks

### 1. Full-Text Search Command (HIGH PRIORITY) ⚠️

**Status**: Created but compilation errors need fixing

**Issue**: The search command uses incorrect error handling patterns. Need to:
- Use `CommandError` instead of `FennecError` throughout
- Properly map errors using CommandError variants:
  - `CommandError::InvalidArguments(String)` for validation
  - `CommandError::ExecutionFailed(String, String, Option<i32>)` for runtime errors
- Ensure all `?` operators work with the command error type

**Files**:
- `crates/fennec-commands/src/search.rs` (needs error handling fix)
- Pattern: Follow `diff.rs` or `edit.rs` for correct error handling

**Features to Include**:
- Full-text search across files
- Regex pattern support
- Case-insensitive option
- File pattern filtering (*.rs, *.toml, etc.)
- Filename-only search mode
- Context lines before/after matches
- Cancellation support
- Max results limiting

**Testing Requirements**:
- Unit tests for pattern matching
- Integration test with temp directory
- Cancellation handling test
- Regex validation test

### 2. File Operations Commands (HIGH PRIORITY)

**Commands to Implement**:

#### `create` Command
```rust
pub struct CreateArgs {
    pub path: PathBuf,
    pub content: Option<String>,
    pub is_directory: bool,
}
```
- Create files or directories
- Handle parent directory creation
- Proper error messages if path exists
- Sandbox validation

#### `rename` Command
```rust
pub struct RenameArgs {
    pub from: PathBuf,
    pub to: PathBuf,
}
```
- Rename files or directories
- Validate both paths exist/don't exist as appropriate
- Handle cross-directory moves
- Update any internal references

#### `move` Command
```rust
pub struct MoveArgs {
    pub from: PathBuf,
    pub to: PathBuf,
}
```
- Move files between directories
- Preserve file metadata
- Handle conflicts (overwrite flag)
- Atomic operations where possible

#### `delete` Command
```rust
pub struct DeleteArgs {
    pub path: PathBuf,
    pub recursive: bool,
    pub confirm: bool,
}
```
- Delete files or directories
- Require confirmation for directories
- Recursive deletion option
- Safety checks (don't delete .git, etc.)

**Implementation Notes**:
- All commands need `WriteFile` capability
- WorkspaceWrite sandbox level minimum
- Proper preview generation
- Rollback support for undo system

### 3. Individual Hunk Approval (HIGH PRIORITY)

**Goal**: Allow accepting/rejecting specific parts of a diff

**Architecture**:
```rust
pub struct Hunk {
    pub id: String,
    pub start_line: usize,
    pub end_line: usize,
    pub old_content: Vec<String>,
    pub new_content: Vec<String>,
    pub status: HunkStatus, // Pending, Accepted, Rejected
}

pub enum HunkStatus {
    Pending,
    Accepted,
    Rejected,
}
```

**Features**:
- Split diffs into discrete hunks
- Interactive UI for hunk selection
- Apply only accepted hunks
- Preview final result
- Undo hunk acceptance

**UI Components**:
- Hunk list with status indicators
- Keyboard shortcuts (a=accept, r=reject, space=toggle)
- Diff preview with highlighting
- Summary of accepted/rejected changes

**Integration**:
- Extend `DiffCommand` with hunk splitting
- New `ApplyHunksCommand` for selective application
- Update `EditCommand` to support hunk mode

### 4. Action Log and Undo System (HIGH PRIORITY)

**Architecture**:
```rust
pub struct ActionLog {
    actions: Vec<Action>,
    current_index: usize,
}

pub struct Action {
    pub id: Uuid,
    pub command: String,
    pub timestamp: DateTime<Utc>,
    pub state_before: ActionState,
    pub state_after: ActionState,
    pub reversible: bool,
}

pub enum ActionState {
    FileCreated { path: PathBuf },
    FileModified { path: PathBuf, content_hash: String },
    FileDeleted { path: PathBuf, content: Vec<u8> },
    FileMoved { from: PathBuf, to: PathBuf },
    // ... other action types
}
```

**Features**:
- Track all file modifications
- Store before/after states
- Undo stack with redo support
- Action history UI
- Persistent across sessions (optional)

**Commands**:
- `undo` - Revert last action
- `redo` - Reapply undone action
- `history` - Show action log
- `clear-history` - Clear undo stack

**Storage**:
- In-memory for current session
- Optional SQLite persistence
- Configurable max history size
- Automatic cleanup of old actions

### 5. Symbol-Aware Search (MEDIUM PRIORITY)

**Goal**: Search for Rust symbols (functions, structs, traits, enums)

**Dependencies**:
- `syn` crate for Rust parsing
- AST traversal and symbol extraction

**Features**:
```rust
pub enum SymbolType {
    Function,
    Struct,
    Enum,
    Trait,
    Type,
    Const,
    Module,
}

pub struct Symbol {
    pub name: String,
    pub symbol_type: SymbolType,
    pub path: PathBuf,
    pub line: usize,
    pub visibility: Visibility,
}
```

**Commands**:
- `find-symbol <name>` - Find symbol by name
- `find-usages <symbol>` - Find all usages
- `find-implementations <trait>` - Find trait impls
- `go-to-definition <symbol>` - Navigate to definition

**Indexing**:
- Build symbol index on workspace load
- Incremental updates on file changes
- Fast lookup with HashMap
- File watcher integration

### 6. Enhanced Git Integration (MEDIUM PRIORITY)

**Features**:

#### PR Summary Generation
- Analyze changed files
- Extract commit messages
- Identify affected components
- Generate risk assessment
- Suggest reviewers based on CODEOWNERS

#### Commit Message Templates
- Conventional commits format
- Scope detection from changed files
- Breaking change detection
- Co-author attribution

#### Change Set Analysis
- Impact analysis (files, functions affected)
- Test coverage changes
- Dependency updates
- Documentation changes

### 7. Auto-Suggest Fixes (MEDIUM PRIORITY)

**Goal**: Parse errors and suggest fixes

**Error Sources**:
- Compiler errors (`cargo build`)
- Test failures (`cargo test`)
- Linter warnings (`cargo clippy`)
- Format issues (`cargo fmt --check`)

**Features**:
```rust
pub struct ErrorSuggestion {
    pub error_message: String,
    pub file: PathBuf,
    pub line: usize,
    pub suggested_fix: String,
    pub confidence: f32,
    pub auto_apply: bool,
}
```

**Patterns**:
- Missing imports → Add use statement
- Type mismatches → Add type conversion
- Unused variables → Add `#[allow(unused)]` or use
- Missing trait bounds → Add where clause
- Borrow checker errors → Add clone/borrow

**Integration**:
- Hook into `RunCommand` output parsing
- Interactive suggestion UI
- One-click apply
- Batch apply multiple fixes

### 8. Auto-Rerun Tests (MEDIUM PRIORITY)

**Goal**: Automatically rerun tests after applying fixes

**Features**:
- File watcher on source files
- Smart test selection (only affected tests)
- Background execution
- Real-time status updates
- Failure highlighting

**Architecture**:
```rust
pub struct TestRunner {
    watcher: FileWatcher,
    last_run: HashMap<PathBuf, TestResult>,
    pending_runs: Vec<TestTarget>,
}

pub struct TestTarget {
    pub test_name: String,
    pub affected_by: Vec<PathBuf>,
}
```

**UI**:
- Test status in status bar
- Pass/fail indicators
- Notification on test completion
- Test output panel

### 9. Project Graph and Indexing ✅ (COMPLETED - Sprint 4)

**Status**: Implemented in `crates/fennec-commands/src/` as:
- `dependency_graph.rs` - Cargo.toml parsing and dependency analysis
- `project_index.rs` - Comprehensive project indexing
- `index.rs` - Command interface for project analysis

**Implemented Features**:
- ✅ Dependency graph (Cargo.toml analysis with cycle detection)
- ✅ Symbol cross-references (integrated with SymbolIndex)
- ✅ Module hierarchy (recursive directory scanning)
- ✅ Impact analysis (affected symbols, packages, tests)
- ✅ Project statistics (packages, symbols, modules)
- ✅ Multiple analysis modes (stats, deps, symbols, impact, modules)

**Command Usage**:
```bash
# Project statistics
index --analysis-type stats

# Dependency graph
index --analysis-type deps --detailed

# Symbol index summary
index --analysis-type symbols --detailed

# Impact analysis for file
index --analysis-type impact --file-path src/main.rs

# Module hierarchy
index --analysis-type modules
```

### 10. Quick Action Templates ✅ (COMPLETED - Sprint 4)

**Status**: Implemented in `crates/fennec-commands/src/quick_actions.rs`

**Implemented Templates** (8 built-in workflows):
- ✅ "fix-error" - Suggest fixes for errors at current location
- ✅ "add-tests" - Generate unit tests for functions
- ✅ "document-function" - Add comprehensive doc comments
- ✅ "add-error-handling" - Add Result<T, E> patterns
- ✅ "optimize-code" - Performance optimization suggestions
- ✅ "refactor-pattern" - Apply design patterns
- ✅ "explain-code" - Detailed code explanations
- ✅ "security-review" - Security vulnerability analysis

**Command Usage**:
```bash
# List all available quick actions
quick-action --list

# Execute specific action
quick-action --action-id fix-error --context file=main.rs line=42 error="type mismatch"
```

**Features**:
- ✅ Context-aware template substitution
- ✅ Template variables with mustache-style syntax
- ✅ Required context validation
- ✅ Tag-based categorization
- ✅ Extensible template system

## Implementation Order

### Sprint 1 ✅ (COMPLETED)
1. ✅ Fix and complete search command
2. ✅ Implement file operations (create, rename, delete)
3. ✅ Basic testing for both

### Sprint 2 ✅ (COMPLETED)
4. ✅ Individual hunk approval
5. ✅ Action log infrastructure
6. ✅ Undo/redo commands

### Sprint 3 ✅ (COMPLETED)
7. ✅ Symbol-aware search (find-symbol command)
8. ✅ Enhanced git integration (pr-summary, commit-template commands)
9. ✅ Auto-suggest fixes (fix-errors command)
10. ✅ Auto-rerun tests (test-watch command)

### Sprint 4 ✅ (COMPLETED)
11. ✅ Project indexing (index command with dependency graph)
12. ✅ Quick action templates (quick-action command with 8 templates)
13. ⏳ Polish and documentation (in progress)

## Testing Strategy

### Unit Tests
- Each command has comprehensive tests
- Error condition coverage
- Edge case handling

### Integration Tests
- Multi-command workflows
- File system operations
- Cancellation scenarios

### Performance Tests
- Search performance on large codebases
- Index build time
- Memory usage monitoring

## Documentation Updates

### User Documentation
- Command reference with examples
- Workflow guides
- Troubleshooting section

### Developer Documentation
- Architecture decision records
- API documentation
- Contributing guide updates

## Metrics for Success

- ✅ All commands pass `cargo test`
- ✅ No `cargo clippy` warnings
- ✅ Code coverage > 80%
- ✅ Documentation complete
- ✅ Performance benchmarks met
- ✅ User acceptance testing passed

## Risks and Mitigations

### Risk: Complexity of AST Parsing
- **Mitigation**: Start with basic symbol extraction, iterate
- **Fallback**: Text-based search as alternative

### Risk: Performance with Large Codebases
- **Mitigation**: Implement incremental indexing early
- **Fallback**: Lazy loading, background processing

### Risk: Undo System Complexity
- **Mitigation**: Start with simple file-based undo
- **Fallback**: Git-based rollback option

## Next Session Immediate Tasks

1. **Fix search command errors**:
   - Replace all `FennecError::Command(Box::new(...))` with `CommandError::*` variants
   - Test compilation and fix any remaining type errors
   - Write unit tests

2. **Implement create command**:
   - Simple file/directory creation
   - Basic validation
   - Tests

3. **Start on hunk approval**:
   - Diff splitting logic
   - Hunk data structure
   - Basic tests

## Notes

- Keep Phase 2 focused on high-impact features
- Maintain code quality standards from Phase 1
- Document as you go
- Test early and often
- Get working features merged quickly rather than perfect features slowly
