# Fennec Memory System Architecture

## Overview

Fennec implements an intelligent, persistent memory system that learns and adapts across development sessions. The memory system draws inspiration from Cline's memory bank approach while adding enterprise-grade features for team environments.

## Memory Sources and Integration

### AGENTS.md Integration
**Purpose**: Repository-specific guidelines and project context
**Location**: Project root (`./AGENTS.md`)
**Content**:
- Project structure and module organization
- Coding style and naming conventions
- Build, test, and development commands
- Testing guidelines and fixtures
- Security and configuration requirements

**Integration**: Automatically parsed and loaded at session start

### Cline-Style Memory Files
**Compatibility**: Full compatibility with existing Cline memory banks
**File Types**:
- `projectbrief.md` - High-level project overview and context
- `activeContext.md` - Current development context and focus
- `progress.md` - Development progress and milestone tracking
- Session transcripts - Full conversation history

**Location**: `.memory_bank/` or project-specific location

### Session Transcripts
**Purpose**: Complete conversation history with command outcomes
**Format**: Structured JSON with metadata
**Storage**: Local filesystem with optional cloud sync
**Retention**: Configurable retention policies

### Git Integration
**Features**:
- Repository history awareness
- Commit message context
- Branch and merge understanding
- File change tracking
- Author and timeline context

## Memory Architecture

### Memory Service (`fennec-memory`)
**Core Components**:
- **MemoryManager**: Central coordination and access
- **FileWatcher**: Real-time change detection
- **ContextRetrieval**: Smart context injection
- **SearchEngine**: Fuzzy search across all memory sources

**Key Traits**:
```rust
#[async_trait]
pub trait MemoryProvider {
    async fn store(&self, key: &str, content: &str) -> Result<()>;
    async fn retrieve(&self, key: &str) -> Result<Option<String>>;
    async fn search(&self, query: &str) -> Result<Vec<SearchResult>>;
    async fn update_context(&self, context: SessionContext) -> Result<()>;
}
```

### Context Management
**Session Context**:
```rust
pub struct SessionContext {
    pub session_id: Uuid,
    pub project_root: PathBuf,
    pub current_files: Vec<PathBuf>,
    pub git_branch: Option<String>,
    pub git_status: GitStatus,
    pub active_commands: Vec<String>,
    pub timestamp: DateTime<Utc>,
}
```

**Context Injection**: Smart context retrieval based on:
- Current working directory
- Active files and recent changes
- Git branch and commit history
- Previous session continuation
- Command history and patterns

### Memory Persistence
**Storage Strategy**:
- **Local Files**: Primary storage for session data
- **Structured Format**: JSON for metadata, Markdown for content
- **Version Control**: Git integration for memory history
- **Backup Strategy**: Configurable backup and sync options

**File Organization**:
```
.memory_bank/
├── sessions/           # Session-specific transcripts
│   ├── 2025-09-23/    # Date-organized sessions
│   └── current.json   # Active session state
├── context/           # Contextual memory files
│   ├── projectbrief.md
│   ├── activeContext.md
│   └── progress.md
├── search/            # Search indices and caches
└── config/            # Memory system configuration
```

## Intelligent Features

### Contextual Retrieval
**Smart Context Selection**:
- **File-based**: Relevant code context for current files
- **Task-based**: Previous work on similar tasks
- **Project-based**: Overall project understanding
- **Temporal**: Recent session continuity

**Context Ranking Algorithm**:
1. **Recency**: Recently accessed or modified content
2. **Relevance**: Similarity to current task or files
3. **Importance**: User-marked important content
4. **Frequency**: Commonly referenced patterns

### Fuzzy Search Capabilities
**Search Features**:
- **Content Search**: Full-text search across all memory
- **Semantic Search**: Meaning-based content matching
- **File Search**: Find files by name or path patterns
- **Command Search**: Previous command usage patterns

**Search Implementation**:
```rust
pub struct SearchResult {
    pub content: String,
    pub source: MemorySource,
    pub score: f64,
    pub context: SearchContext,
    pub timestamp: DateTime<Utc>,
}

pub enum MemorySource {
    AgentsFile,
    SessionTranscript,
    MemoryFile,
    GitHistory,
    FileContent,
}
```

### Learning and Adaptation
**Pattern Recognition**:
- **Command Patterns**: Frequently used command sequences
- **File Patterns**: Common file access and modification patterns
- **Error Patterns**: Common mistakes and resolutions
- **Workflow Patterns**: Typical development workflows

**Adaptive Suggestions**:
- **Command Completion**: Smart command suggestions based on context
- **File Suggestions**: Relevant files for current task
- **Context Hints**: Helpful context from previous sessions
- **Workflow Automation**: Learned workflow pattern suggestions

## Configuration and Customization

### Memory Configuration
```toml
[memory]
# Enable memory system
enabled = true

# Memory storage location
storage_path = "./.memory_bank"

# Search and retrieval settings
search_limit = 50
context_window = 5000  # characters

# File watching
watch_changes = true
auto_update = true

# Retention policies
session_retention_days = 90
transcript_retention_days = 365

# Cline compatibility
cline_compatibility = true
cline_path = "./.cline"
```

### Privacy and Security
**Privacy Controls**:
- **Local Storage**: All memory stored locally by default
- **Selective Sync**: Choose what to sync to cloud storage
- **Content Filtering**: Exclude sensitive files and directories
- **Encryption**: Optional encryption for sensitive projects

**Security Features**:
- **Access Control**: Sandbox-aware memory access
- **Audit Integration**: Memory access audit trails
- **Safe Content**: Automatic filtering of credentials and secrets
- **Workspace Boundaries**: Memory isolated to project workspace

## Integration with Commands

### Memory-Aware Commands
**Enhanced Commands**:
- **`plan`**: Uses project history and patterns for better planning
- **`edit`**: Suggests edits based on previous similar changes
- **`summarize`**: Creates contextual summaries with memory integration
- **`diff`**: Shows changes in context of project evolution

**Memory API for Commands**:
```rust
#[async_trait]
pub trait MemoryAwareCommand {
    async fn execute_with_memory(
        &self,
        args: &[String],
        memory: &MemoryProvider,
        context: &SessionContext,
    ) -> Result<CommandResult>;
}
```

### Memory Updates
**Automatic Updates**:
- **File Changes**: Track file modifications and context
- **Command Execution**: Record command results and outcomes
- **Session Progress**: Update project progress and status
- **Error Resolution**: Learn from error patterns and solutions

**Manual Updates**:
- **Annotations**: User-added context and notes
- **Bookmarks**: Important code locations and decisions
- **Milestones**: Project milestone and achievement tracking
- **Learnings**: Captured insights and best practices

## Performance and Scalability

### Performance Optimizations
**Caching Strategy**:
- **Memory Cache**: Frequently accessed content in memory
- **Search Index**: Pre-built search indices for fast retrieval
- **Context Cache**: Cached context for common scenarios
- **File Cache**: Cached file content and metadata

**Lazy Loading**:
- **On-Demand**: Load memory content only when needed
- **Progressive**: Load most relevant content first
- **Background**: Background loading of secondary content
- **Streaming**: Stream large content progressively

### Scalability Considerations
**Large Projects**:
- **Selective Indexing**: Index only relevant files and content
- **Hierarchical Storage**: Tiered storage for different content types
- **Compression**: Compress older session data
- **Cleanup**: Automatic cleanup of old and irrelevant data

**Team Environments**:
- **Shared Memory**: Optional shared team memory
- **Conflict Resolution**: Handle concurrent memory updates
- **Merge Strategies**: Merge memory from different team members
- **Access Control**: Role-based memory access and modification

---

*This memory system provides the foundation for intelligent, context-aware development assistance that learns and improves over time.*