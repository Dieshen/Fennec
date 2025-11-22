# Phase 3 Enhancement Roadmap

## Overview

Phase 3 focuses on UI/UX improvements, advanced code analysis features, and provider extensibility. With Phase 2 delivering 17 production-ready commands and comprehensive feature parity, Phase 3 will enhance the user experience and extend Fennec's capabilities beyond Rust-specific workflows.

## Phase 2 Summary ✅

**Completed:**
- 17 production-ready commands
- 145 passing tests (132 unit + 12 integration + 1 doc)
- Full-text and symbol-aware search
- Project indexing with dependency graphs
- File operations (create, rename, delete)
- Hunk approval and undo/redo system
- Auto-suggest fixes and test watching
- Git integration (PR summaries, commit templates)
- Quick action workflow templates

## Phase 3 Goals

### Primary Objectives
1. **Enhance UI/UX** - File tree browser, better navigation
2. **Advanced Code Analysis** - Call graphs, usage search, semantic search
3. **Provider Extensibility** - Anthropic (Claude) support, MCP integration
4. **Multi-Language Support** - Extend beyond Rust to Python, TypeScript, Go, etc.

### Success Metrics
- File tree browser integrated into TUI
- Call graph visualization working for Rust codebases
- Anthropic provider functional with streaming support
- At least 3 additional languages supported in commands
- Test coverage maintained > 80%
- Performance: Index build < 5s for medium projects (< 100k LOC)

---

## Sprint 5: UI/UX Enhancements (Week 1)

### 1. File Tree Browser Component

**Goal**: Add interactive file tree navigation to TUI

**Architecture**:
```rust
pub struct FileTreeBrowser {
    root: PathBuf,
    current_node: FileNode,
    expanded: HashSet<PathBuf>,
    selected: Option<PathBuf>,
}

pub struct FileNode {
    path: PathBuf,
    name: String,
    is_dir: bool,
    children: Vec<FileNode>,
    metadata: FileMetadata,
}
```

**Features**:
- Keyboard navigation (arrows, enter, space)
- Expand/collapse directories
- File type icons (Rust, TOML, Markdown, etc.)
- Search/filter in tree
- Show/hide hidden files
- Git status indicators (modified, untracked, etc.)
- Quick file open (opens in editor)

**Integration**:
- Add new TUI pane for file tree
- Connect to existing preview panel
- Sync with current file context
- Persist expansion state

**Testing**:
- Unit tests for tree building
- Integration tests for navigation
- Performance tests (large directories)

---

### 2. Multi-File Edit Coordination

**Goal**: Atomic multi-file changes with rollback support

**Architecture**:
```rust
pub struct MultiFileEdit {
    pub id: Uuid,
    pub edits: Vec<FileEdit>,
    pub status: EditStatus,
}

pub struct FileEdit {
    pub path: PathBuf,
    pub operation: EditOperation,
    pub before_hash: String,
    pub after_hash: String,
}

pub enum EditOperation {
    Modify { hunks: Vec<Hunk> },
    Create { content: Vec<u8> },
    Delete,
    Rename { to: PathBuf },
}
```

**Features**:
- Group related edits into transaction
- Preview all changes together
- Atomic apply (all or nothing)
- Automatic rollback on failure
- Conflict detection across files
- Dependency-aware ordering

**Commands**:
- `multi-edit start` - Begin multi-file transaction
- `multi-edit add <file>` - Add file to transaction
- `multi-edit preview` - Show all pending changes
- `multi-edit apply` - Apply all changes atomically
- `multi-edit cancel` - Discard transaction

**Testing**:
- Transaction atomicity tests
- Rollback tests
- Conflict detection tests
- Performance tests

---

### 3. Command History with Rerun

**Goal**: Persistent command history with replay

**Architecture**:
```rust
pub struct CommandHistory {
    commands: Vec<HistoryEntry>,
    max_size: usize,
    storage: Box<dyn HistoryStorage>,
}

pub struct HistoryEntry {
    pub id: Uuid,
    pub command: String,
    pub args: Value,
    pub timestamp: DateTime<Utc>,
    pub result: CommandResult,
    pub duration_ms: u64,
}
```

**Features**:
- Persistent history across sessions
- Fuzzy search in history
- Rerun previous commands
- Edit and rerun
- History statistics (most used, etc.)
- Export history to file

**Commands**:
- `history [--search <query>]` - Enhanced history command
- `rerun <id>` - Rerun command by ID
- `rerun-last` - Rerun last command
- `history-export <file>` - Export history

**Storage**:
- SQLite database for persistence
- In-memory cache for current session
- Configurable retention policy

**Testing**:
- Persistence tests
- Search tests
- Rerun tests

---

### 4. File Attachment to Context

**Goal**: Explicitly attach files to conversation context

**Architecture**:
```rust
pub struct ContextManager {
    attached_files: HashMap<PathBuf, AttachedFile>,
    auto_attach: bool,
    max_files: usize,
}

pub struct AttachedFile {
    pub path: PathBuf,
    pub content_hash: String,
    pub attached_at: DateTime<Utc>,
    pub reason: AttachmentReason,
}

pub enum AttachmentReason {
    UserExplicit,
    AutoRelevant,
    ReferencedInCode,
}
```

**Features**:
- Explicit file attachment
- Auto-attach relevant files (imported, referenced)
- Show attached files in UI
- Detach files
- Attachment history
- Smart context pruning

**Commands**:
- `attach <file>` - Attach file to context
- `detach <file>` - Remove from context
- `context-list` - Show attached files
- `context-clear` - Clear all attachments

**UI**:
- Show attached files in status bar
- Visual indicator for auto-attached files
- Quick attach/detach shortcuts

**Testing**:
- Attachment tests
- Auto-attach tests
- Context limit tests

---

## Sprint 6: Advanced Code Analysis (Week 2)

### 5. Call Graph Generation

**Goal**: Visualize function call relationships

**Architecture**:
```rust
pub struct CallGraph {
    nodes: HashMap<FunctionId, CallGraphNode>,
    edges: HashMap<FunctionId, Vec<FunctionId>>,
}

pub struct CallGraphNode {
    pub function: Symbol,
    pub callers: Vec<FunctionId>,
    pub callees: Vec<FunctionId>,
    pub depth: usize,
}
```

**Features**:
- Build call graph from AST
- Find all callers of function
- Find all callees of function
- Transitive call chains
- Cycle detection
- Export to GraphViz DOT format
- ASCII art visualization

**Command**:
```rust
pub struct CallGraphArgs {
    pub symbol: String,
    pub direction: GraphDirection, // Callers, Callees, Both
    pub max_depth: Option<usize>,
    pub format: OutputFormat, // Text, Dot, Json
}
```

**Implementation**:
- Extend `symbols.rs` with call graph analysis
- Use `syn` for AST traversal
- Build caller/callee relationships
- Format output for visualization

**Testing**:
- Graph building tests
- Cycle detection tests
- Output format tests

---

### 6. Usage Search

**Goal**: Find all usages of a symbol

**Architecture**:
```rust
pub struct UsageFinder {
    symbol_index: Arc<SymbolIndex>,
    workspace: PathBuf,
}

pub struct SymbolUsage {
    pub symbol: Symbol,
    pub usage_type: UsageType,
    pub location: Location,
    pub context: String,
}

pub enum UsageType {
    Definition,
    Import,
    FunctionCall,
    FieldAccess,
    TypeAnnotation,
    MacroInvocation,
}
```

**Features**:
- Find all references to symbol
- Categorize usage types
- Show usage context (surrounding lines)
- Filter by usage type
- Show call hierarchy
- Export to file

**Command**:
```rust
pub struct FindUsagesArgs {
    pub symbol: String,
    pub usage_types: Vec<UsageType>,
    pub include_tests: bool,
    pub format: OutputFormat,
}
```

**Implementation**:
- AST-based reference finding
- Text-based fallback for non-Rust
- Integration with symbol index
- Context extraction

**Testing**:
- Usage finding tests
- Type categorization tests
- Edge case tests (macros, generics)

---

### 7. Semantic Search

**Goal**: AI-powered code understanding and search

**Architecture**:
```rust
pub struct SemanticSearch {
    provider: Arc<dyn LlmProvider>,
    symbol_index: Arc<SymbolIndex>,
    embeddings_cache: Option<EmbeddingsCache>,
}

pub struct SemanticSearchResult {
    pub file: PathBuf,
    pub symbol: Option<Symbol>,
    pub relevance_score: f32,
    pub explanation: String,
}
```

**Features**:
- Natural language code search
- "Find functions that handle authentication"
- "Show error handling patterns"
- Optional embeddings for faster search
- Explanation of why results match
- Interactive refinement

**Command**:
```rust
pub struct SemanticSearchArgs {
    pub query: String,
    pub max_results: usize,
    pub use_embeddings: bool,
}
```

**Implementation**:
- Use LLM for semantic understanding
- Optional embeddings for performance
- Combine with symbol index for precision
- Interactive result exploration

**Testing**:
- Query understanding tests
- Relevance scoring tests
- Performance benchmarks

---

### 8. Dependency Audit Integration

**Goal**: Security vulnerability checking

**Architecture**:
```rust
pub struct DependencyAuditor {
    audit_db: PathBuf,
    last_update: DateTime<Utc>,
}

pub struct AuditResult {
    pub package: String,
    pub version: String,
    pub vulnerabilities: Vec<Vulnerability>,
}

pub struct Vulnerability {
    pub id: String,
    pub severity: Severity,
    pub description: String,
    pub patched_versions: Vec<String>,
}
```

**Features**:
- Run cargo-audit automatically
- Parse and display vulnerabilities
- Suggest version upgrades
- Show dependency tree for vulnerable deps
- Integration with `index` command

**Command**:
```rust
pub struct AuditArgs {
    pub fix: bool, // Auto-update to safe versions
    pub severity_threshold: Severity,
}
```

**Implementation**:
- Execute cargo-audit
- Parse JSON output
- Display formatted results
- Optional automatic fixes

**Testing**:
- Audit parsing tests
- Fix suggestion tests
- Integration tests

---

## Sprint 7: Provider & Integration (Week 3)

### 9. Anthropic Provider Support

**Goal**: Add Claude model support

**Implementation Files**:
- `crates/fennec-provider/src/anthropic.rs`
- `crates/fennec-provider/src/anthropic_stream.rs`

**Architecture**:
```rust
pub struct AnthropicProvider {
    api_key: String,
    base_url: String,
    client: Client,
}

impl LlmProvider for AnthropicProvider {
    async fn chat_completion_stream(
        &self,
        messages: Vec<Message>,
        config: CompletionConfig,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<CompletionChunk>>>>, ProviderError>;
}
```

**Features**:
- Messages API support
- Streaming responses
- Tool use (function calling)
- Vision support (images)
- Context caching
- Model selection (Sonnet, Opus, Haiku)

**Configuration**:
```toml
[provider.anthropic]
api_key = "${ANTHROPIC_API_KEY}"
model = "claude-3-5-sonnet-20241022"
max_tokens = 4096
temperature = 0.7
```

**Testing**:
- API integration tests
- Streaming tests
- Tool use tests
- Error handling tests

---

### 10. MCP Server Integration

**Goal**: Model Context Protocol support

**Architecture**:
```rust
pub struct McpClient {
    server_url: String,
    tools: HashMap<String, McpTool>,
}

pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}
```

**Features**:
- Connect to MCP servers
- Discover available tools
- Execute tool calls
- Handle tool responses
- Error handling and retries

**Configuration**:
```toml
[[mcp.servers]]
name = "filesystem"
url = "http://localhost:8080"
enabled = true
```

**Implementation**:
- HTTP client for MCP protocol
- Tool discovery and registration
- Integration with command system
- Error handling

**Testing**:
- Connection tests
- Tool execution tests
- Error handling tests

---

### 11. Multi-Language Support

**Goal**: Extend commands beyond Rust

**Languages to Support**:
- Python (pytest, mypy, black, ruff)
- TypeScript/JavaScript (jest, eslint, prettier)
- Go (go test, golint, gofmt)

**Extensions Needed**:

**Search Command**:
- Language-specific ignore patterns
- Syntax-aware search (if available)

**Symbol Search**:
- Python: AST parsing with `rustpython-parser`
- TypeScript: Use `swc` or `tree-sitter`
- Go: Use `go/parser` bindings

**Test Watch**:
- Detect test framework (pytest, jest, go test)
- Language-specific watch patterns
- Test command generation

**Fix Errors**:
- Parse Python tracebacks
- Parse TypeScript compiler errors
- Parse Go compiler errors

**Implementation**:
```rust
pub trait LanguageSupport {
    fn detect_language(path: &Path) -> Option<Language>;
    fn parse_symbols(&self, source: &str) -> Result<Vec<Symbol>>;
    fn parse_errors(&self, output: &str) -> Vec<CompilerError>;
    fn test_command(&self, workspace: &Path) -> String;
}
```

**Testing**:
- Language detection tests
- Symbol parsing tests per language
- Error parsing tests per language

---

## Sprint 8: Polish & Performance (Week 4)

### 12. Performance Optimization

**Areas**:
1. **Index Build Performance**
   - Parallel file processing
   - Incremental updates
   - Caching strategies

2. **Search Performance**
   - Optimize regex matching
   - Parallel file scanning
   - Result streaming

3. **Symbol Index Performance**
   - Lazy loading
   - Memory-efficient storage
   - Fast lookup data structures

4. **Memory Usage**
   - Reduce allocations
   - Streaming large files
   - LRU caches for hot data

**Benchmarks**:
- Index build: < 5s for 100k LOC
- Search: < 1s for 1M LOC
- Symbol lookup: < 100ms
- Memory: < 500MB for large projects

---

### 13. Enhanced Testing

**Areas**:
1. **Integration Tests**
   - End-to-end workflow tests
   - Multi-command scenarios
   - Error recovery tests

2. **Performance Tests**
   - Benchmark suite
   - Large codebase tests
   - Stress tests

3. **UI Tests**
   - TUI interaction tests
   - Keyboard navigation tests
   - Display tests

---

### 14. Documentation

**Updates Needed**:
1. **User Documentation**
   - Command reference updates
   - New feature guides
   - Workflow examples

2. **Developer Documentation**
   - Architecture documentation
   - Adding new providers guide
   - Adding new languages guide

3. **API Documentation**
   - Provider trait docs
   - Command trait docs
   - Core API docs

---

## Testing Strategy

### Unit Tests
- Maintain > 80% coverage
- Test all new modules
- Edge case coverage

### Integration Tests
- Multi-command workflows
- Provider integration
- MCP integration

### Performance Tests
- Index build benchmarks
- Search performance
- Memory profiling

---

## Documentation Updates

### User Documentation
- File tree browser guide
- Multi-file edit workflows
- Call graph usage
- Provider configuration

### Developer Documentation
- Provider implementation guide
- Language support guide
- MCP integration guide

---

## Risks and Mitigations

### Risk: UI Complexity
- **Mitigation**: Incremental rollout, feature flags
- **Fallback**: Keep existing TUI as option

### Risk: Provider Integration Complexity
- **Mitigation**: Abstract provider traits, thorough testing
- **Fallback**: OpenAI remains default

### Risk: Multi-Language Support Scope Creep
- **Mitigation**: Start with 3 languages, expand later
- **Fallback**: Rust-only mode

---

## Success Criteria

- ✅ File tree browser integrated and usable
- ✅ Call graph generation working for Rust
- ✅ Anthropic provider functional
- ✅ At least 3 languages supported
- ✅ All tests passing (target: 180+ tests)
- ✅ Performance benchmarks met
- ✅ Documentation complete

---

## Phase 4 Preview

Potential future enhancements:
- LSP integration for advanced code intelligence
- Real-time collaboration features
- Custom plugin system
- Cloud sync for settings and history
- Advanced refactoring tools
- Code generation templates
