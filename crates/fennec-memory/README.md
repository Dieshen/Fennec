# Fennec Memory Service

The `fennec-memory` crate provides comprehensive memory management for Fennec, including configuration loading, conversation transcript storage, search functionality, and the foundation for Cline-style memory files.

## Features

### 🔧 AGENTS.md Configuration Loading
- Automatically loads configuration from global (`~/.fennec/AGENTS.md`) and repository-specific (`./AGENTS.md`) locations
- Parses markdown content into structured sections for easy access
- File watching for automatic reloading when configuration changes
- Fuzzy search across guidance content

### 💾 Transcript Storage and Management
- Persistent storage of conversation transcripts with metadata
- In-memory caching for performance
- Automatic conversation context extraction (topics, technologies, errors)
- Support for tags and summaries
- Efficient search across conversation history

### 🔍 Search and Retrieval
- Full-text search across all memory components
- Fuzzy matching for flexible queries
- Relevance scoring and result ranking
- Context-aware search based on current conversation

### 🧠 Memory Injection for AI Prompts
- Provides relevant context from AGENTS.md guidance
- Includes related conversation history
- Estimates token usage for context management
- Session-aware context extraction

### 📁 Cline-style Memory Files (Foundation)
- Framework for persistent knowledge base entries
- Support for different memory file types (project context, debugging patterns, etc.)
- Search and categorization capabilities
- Session association for context preservation

## Quick Start

```rust
use fennec_memory::{MemoryService, MemoryConfig};
use fennec_core::{session::Session, transcript::MessageRole};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create memory service
    let memory = MemoryService::new().await?;
    
    // Start tracking a session
    let session = Session::new();
    memory.start_session(session.clone()).await?;
    
    // Add messages
    memory.add_message(
        session.id,
        MessageRole::User,
        "How do I implement async functions in Rust?".to_string()
    ).await?;
    
    // Get memory injection for AI prompts
    let injection = memory.get_memory_injection(
        session.id,
        Some("async rust")
    ).await?;
    
    // Use guidance and conversation history...
    
    Ok(())
}
```

## Configuration

The memory service looks for `AGENTS.md` files in the following order:

1. Repository root: `./AGENTS.md`
2. Global configuration: `~/.fennec/AGENTS.md`

### AGENTS.md Format

The AGENTS.md file should be structured as a markdown document with sections:

```markdown
# Repository Guidelines

## Project Structure & Module Organization
- Guidelines for organizing code
- Module naming conventions

## Build, Test, and Development Commands
- Common development workflows
- Testing strategies

## Coding Style & Naming Conventions
- Code formatting rules
- Naming patterns
```

## Storage Locations

- **Transcripts**: `~/.local/share/fennec/transcripts/`
- **Memory Files**: `~/.local/share/fennec/memory_files/` (Milestone 3)
- **Configuration**: `~/.fennec/AGENTS.md`

## Memory Service Configuration

```rust
use fennec_memory::MemoryConfig;

let config = MemoryConfig {
    max_messages_in_memory: 1000,      // Max messages per session in memory
    auto_generate_summaries: true,      // Auto-generate conversation summaries
    guidance_context_window: 50,        // Context window for guidance search
    max_search_results: 10,            // Max search results to return
};

let memory = MemoryService::with_config(config).await?;
```

## API Overview

### Core Memory Service

- `MemoryService::new()` - Create with default configuration
- `start_session()` / `stop_session()` - Manage session tracking
- `add_message()` - Add messages to conversations
- `get_memory_injection()` - Get relevant context for AI prompts
- `search()` - Search across all memory
- `list_sessions()` - List stored sessions

### Transcript Management

- `TranscriptStore` - Persistent transcript storage
- `MemoryTranscript` - Extended transcript with metadata
- Search and retrieval capabilities
- Tagging and summarization support

### AGENTS.md Service

- `AgentsService` - Configuration loading and management
- `search_guidance()` - Search through guidance content
- File watching for automatic updates
- Structured section parsing

### Memory Files (Foundation)

- `MemoryFileService` - Cline-style memory file management
- Support for different file types and categories
- Search and association capabilities
- Foundation for Milestone 3 features

## Integration with Fennec Core

The memory service integrates seamlessly with `fennec-core` types:

- Uses `Session` and `Transcript` types from core
- Extends with memory-specific metadata
- Provides error types compatible with `FennecError`
- Maintains consistency with core session management

## Performance Considerations

- In-memory caching with configurable limits
- Lazy loading of transcript data
- Efficient search indexing
- Token estimation for context management
- Automatic cache eviction policies

## Future Roadmap (Milestone 3)

- Enhanced Cline-style memory files
- Automatic knowledge extraction
- Advanced conversation summarization
- Integration with AI providers for content generation
- Memory file templates and workflows

## Testing

Run the comprehensive test suite:

```bash
cargo test -p fennec-memory
```

See the `examples/memory_service_demo.rs` for a complete demonstration of memory service capabilities.