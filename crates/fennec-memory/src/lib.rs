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

    #[tokio::test]
    async fn test_create_memory_service() {
        let service = create_memory_service().await;
        assert!(service.is_ok());
    }

    #[tokio::test]
    async fn test_create_memory_service_with_default_config() {
        let config = MemoryConfig::default();
        let service = create_memory_service_with_config(config).await;
        assert!(service.is_ok());
    }

    #[tokio::test]
    async fn test_create_memory_service_with_custom_config() {
        let mut config = MemoryConfig::default();
        config.max_messages_in_memory = 5000;
        let service = create_memory_service_with_config(config).await;
        assert!(service.is_ok());
    }

    #[test]
    fn test_version_constant() {
        assert!(!VERSION.is_empty());
        assert!(VERSION.contains('.'));
    }

    #[test]
    fn test_version_format() {
        // Version should be in semver format
        let parts: Vec<&str> = VERSION.split('.').collect();
        assert!(parts.len() >= 2, "Version should have at least major.minor");
    }

    #[tokio::test]
    async fn test_memory_file_service_creation() {
        let file_service = MemoryFileService::new();
        assert!(file_service.is_ok());
    }

    #[tokio::test]
    async fn test_transcript_store_creation() {
        let store = TranscriptStore::new();
        assert!(store.is_ok());
    }

    #[tokio::test]
    async fn test_plan_store_creation() {
        let store = PlanStore::new();
        assert!(store.is_ok());
    }

    #[tokio::test]
    async fn test_notes_store_creation() {
        let store = NotesStore::new();
        assert!(store.is_ok());
    }

    #[tokio::test]
    async fn test_cline_memory_file_service_creation() {
        let service = ClineMemoryFileService::new();
        assert!(service.is_ok());
    }

    #[test]
    fn test_result_type_alias() {
        let ok_result: Result<i32> = Ok(42);
        assert!(ok_result.is_ok());
        assert_eq!(ok_result.unwrap(), 42);

        let err_result: Result<i32> = Err(anyhow::anyhow!("test error"));
        assert!(err_result.is_err());
    }

    #[test]
    fn test_memory_config_default() {
        let config = MemoryConfig::default();
        assert!(config.max_messages_in_memory > 0);
        assert!(config.max_search_results > 0);
    }

    #[test]
    fn test_memory_config_custom() {
        let mut config = MemoryConfig::default();
        config.max_messages_in_memory = 5000;
        config.max_search_results = 50;
        assert_eq!(config.max_messages_in_memory, 5000);
        assert_eq!(config.max_search_results, 50);
    }

    #[test]
    fn test_exports_available() {
        // Test that key types are exported and available
        let _ = MemoryType::Transcripts;
        let _ = MemoryType::Guidance;
        let _ = MemoryType::MemoryFiles;
    }

    #[test]
    fn test_note_category_values() {
        let _ = NoteCategory::Insight;
        let _ = NoteCategory::Decision;
        let _ = NoteCategory::Reminder;
        let _ = NoteCategory::Learning;
        let _ = NoteCategory::Issue;
    }

    #[test]
    fn test_plan_status_values() {
        let _ = PlanStatus::Draft;
        let _ = PlanStatus::Ready;
        let _ = PlanStatus::Completed;
    }

    #[test]
    fn test_plan_priority_values() {
        let _ = PlanPriority::Low;
        let _ = PlanPriority::Medium;
        let _ = PlanPriority::High;
        let _ = PlanPriority::Critical;
    }

    #[test]
    fn test_memory_file_type_values() {
        let _ = MemoryFileType::ProjectContext;
        let _ = MemoryFileType::DebuggingPatterns;
    }

    #[test]
    fn test_context_importance_values() {
        let _ = ContextImportance::Critical;
        let _ = ContextImportance::High;
        let _ = ContextImportance::Medium;
        let _ = ContextImportance::Low;
    }

    #[test]
    fn test_cline_file_type_values() {
        let _ = ClineFileType::ActiveContext;
        let _ = ClineFileType::ProjectBrief;
        let _ = ClineFileType::Progress;
    }

    #[test]
    fn test_note_priority_values() {
        let _ = NotePriority::Low;
        let _ = NotePriority::Medium;
        let _ = NotePriority::High;
        let _ = NotePriority::Critical;
    }

    #[test]
    fn test_step_status_values() {
        let _ = StepStatus::Pending;
        let _ = StepStatus::InProgress;
        let _ = StepStatus::Completed;
        let _ = StepStatus::Failed;
        let _ = StepStatus::Skipped;
    }
}
