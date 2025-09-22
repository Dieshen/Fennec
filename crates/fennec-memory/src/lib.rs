//! # Fennec Memory Service
//!
//! The fennec-memory crate provides comprehensive memory management for Fennec,
//! including AGENTS.md configuration loading, conversation transcript storage,
//! command plan tracking, user notes management, and advanced search functionality.
//!
//! ## Features
//!
//! - **AGENTS.md Loading**: Loads configuration from global and repo-specific locations
//! - **Transcript Management**: Stores and manages conversation history with context tracking
//! - **Command Plan Tracking**: Manages planning sessions with step-by-step execution
//! - **User Notes Management**: Categorized notes with tagging and cross-referencing
//! - **Search & Retrieval**: Full-text search across all memory components
//! - **File Watching**: Automatic reloading when AGENTS.md files change
//! - **Memory Injection**: Provides relevant context for AI prompts
//! - **Session Management**: Tracks active conversations and their context
//! - **Timeline Tracking**: Complete activity timeline for sessions
//!
//! ## Usage
//!
//! ```rust,no_run
//! use fennec_core::{
//!     session::Session,
//!     transcript::MessageRole,
//! };
//! use fennec_memory::create_memory_service;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Create memory service with default configuration
//!     let memory = create_memory_service().await?;
//!
//!     // Start tracking a session
//!     let session = Session::new();
//!     memory.start_session(session.clone()).await?;
//!
//!     // Add messages to the conversation
//!     memory
//!         .add_message(
//!             session.id,
//!             MessageRole::User,
//!             "I'm working on an AI CLI in Rust".to_string(),
//!         )
//!         .await?;
//!     memory
//!         .add_message(
//!             session.id,
//!             MessageRole::Assistant,
//!             "Great! Remember to enforce sandbox policies for safety.".to_string(),
//!         )
//!         .await?;
//!
//!     // Retrieve memory injection for the current session
//!     let injection = memory.get_memory_injection(session.id, None).await?;
//!     println!("Guidance items: {}", injection.guidance.len());
//!     println!("Transcript matches: {}", injection.conversation_history.len());
//!
//!     Ok(())
//! }
//! ```

pub mod agents;
pub mod cline_files;
pub mod context;
pub mod files;
pub mod integration;
pub mod notes;
pub mod plans;
pub mod service;
pub mod transcript;

// Re-export main types for convenience
pub use service::{
    AdvancedSearchCriteria, ConversationContext, EnhancedSearchResults, MemoryConfig, MemoryError,
    MemoryInjection, MemorySearchResults, MemoryService, MemoryType, ScoringStrategy,
    SearchMetadata, SessionFilter, SessionMemory, TimeFilter, UnifiedSearchMetadata,
    UnifiedSearchResult,
};

pub use transcript::{
    ConversationContext as TranscriptConversationContext, ConversationContextUpdate,
    ExecutionResult, MemoryTranscript, SegmentType, TimelineEvent, TimelineEventType,
    TranscriptMetadata, TranscriptSearchFilters, TranscriptSearchResult, TranscriptSegment,
    TranscriptStore,
};

pub use agents::{AgentSection, AgentsConfig, AgentsService, GuidanceMatch, MatchType};

pub use files::{
    MatchLocation, MemoryFile, MemoryFileMetadata, MemoryFileSearchResult, MemoryFileService,
    MemoryFileType,
};

pub use plans::{
    CommandAssociation, CommandPlan, ExecutionResult as PlanExecutionResult, PlanMatchLocation,
    PlanPriority, PlanSearchResult, PlanStatus, PlanStep, PlanStore, PlanTemplate, StepStatus,
};

pub use notes::{
    NoteCategory, NoteMatchLocation, NoteMetadata, NotePriority, NoteSearchFilters,
    NoteSearchResult, NotesStore, UserNote,
};

pub use context::{
    ContentClassification, ContextBundle, ContextConfig, ContextDiscoveryStrategy, ContextEngine,
    ContextImportance, ContextItem, ContextItemMetadata, ContextRequest, ContextSizeConstraints,
    ContextSizeInfo, ContextSummary, ContextUseCase,
};

pub use integration::{
    CommandContextInjection, ContextInjectionService, ContextReceiver, ContextRequirements,
    EnhancedMemoryInjection, ProviderContextInjection, SessionInitContextInjection,
    SimpleCommandIntegration, SimpleProviderIntegration,
};

pub use cline_files::{
    Achievement, ActiveContextContent, ClineFileContent, ClineFileMetadata, ClineFileType,
    ClineMemoryFile, ClineMemoryFileService, CompletedTask, MemoryEvent, ProgressContent,
    ProjectBriefContent, ProjectStatus, SessionSummary, TemplateEngine, VersionEntry,
};

/// Result type alias for memory operations
pub type Result<T> = std::result::Result<T, anyhow::Error>;

/// Quick setup function for creating a memory service with default configuration
pub async fn create_memory_service() -> Result<MemoryService> {
    MemoryService::new().await
}

/// Create a memory service with custom configuration
pub async fn create_memory_service_with_config(config: MemoryConfig) -> Result<MemoryService> {
    MemoryService::with_config(config).await
}

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_library_integration() {
        // Test that we can create a memory service
        let memory_service = create_memory_service().await;
        assert!(memory_service.is_ok());

        // Test that we can create memory file service
        let file_service = MemoryFileService::new();
        assert!(file_service.is_ok());
    }

    #[test]
    fn test_version_constant() {
        assert!(!VERSION.is_empty());
    }
}
