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
//! use fennec_memory::{MemoryService, MemoryConfig, NoteCategory, PlanStatus};
//! use fennec_core::session::Session;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Create memory service with default configuration
//!     let memory = MemoryService::new().await?;
//!
//!     // Start tracking a session
//!     let session = Session::new();
//!     memory.start_session(session.clone()).await?;
//!
//!     // Add messages to the conversation
//!     memory.add_message(
//!         session.id,
//!         fennec_core::transcript::MessageRole::User,
//!         "I need to implement a Rust web API".to_string()
//!     ).await?;
//!
//!     // Create a plan for the implementation
//!     let plan_id = memory.create_plan(
//!         session.id,
//!         "Rust Web API Implementation".to_string(),
//!         "Build a REST API using Axum framework".to_string()
//!     ).await?;
//!
//!     // Add steps to the plan
//!     let step_id = memory.add_plan_step(
//!         plan_id,
//!         "Set up project structure with Cargo.toml".to_string(),
//!         vec![]
//!     ).await?;
//!
//!     // Create a note for important decisions
//!     let note_id = memory.create_note(
//!         Some(session.id),
//!         "Technology Choice".to_string(),
//!         "Decided on Axum for its async performance and ecosystem".to_string(),
//!         NoteCategory::Decision
//!     ).await?;
//!
//!     // Link the note to the plan
//!     memory.link_note_to_plan(note_id, plan_id).await?;
//!
//!     // Get comprehensive session memory summary
//!     let summary = memory.get_session_memory_summary(session.id).await?;
//!     println!("Session has {} plans and {} notes", summary.plans_count, summary.notes_count);
//!
//!     // Get relevant memory injection for AI prompts
//!     let injection = memory.get_memory_injection(
//!         session.id,
//!         Some("rust web api")
//!     ).await?;
//!
//!     println!("Found {} guidance matches", injection.guidance.len());
//!     println!("Found {} conversation matches", injection.conversation_history.len());
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
