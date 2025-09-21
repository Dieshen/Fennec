//! # Fennec Memory Service
//!
//! The fennec-memory crate provides comprehensive memory management for Fennec,
//! including AGENTS.md configuration loading, conversation transcript storage,
//! search functionality, and Cline-style memory files.
//!
//! ## Features
//!
//! - **AGENTS.md Loading**: Loads configuration from global and repo-specific locations
//! - **Transcript Management**: Stores and manages conversation history
//! - **Search & Retrieval**: Full-text search across all memory components  
//! - **File Watching**: Automatic reloading when AGENTS.md files change
//! - **Memory Injection**: Provides relevant context for AI prompts
//! - **Session Management**: Tracks active conversations and their context
//! - **Cline-style Memory Files**: Foundation for persistent knowledge base (Milestone 3)
//!
//! ## Usage
//!
//! ```rust,no_run
//! use fennec_memory::{MemoryService, MemoryConfig};
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
//!         "How do I implement async functions in Rust?".to_string()
//!     ).await?;
//!     
//!     // Get relevant memory injection for AI prompts
//!     let injection = memory.get_memory_injection(
//!         session.id,
//!         Some("async rust")
//!     ).await?;
//!     
//!     // Use the guidance and conversation history to enhance AI prompts
//!     println!("Found {} guidance matches", injection.guidance.len());
//!     println!("Found {} conversation matches", injection.conversation_history.len());
//!     
//!     Ok(())
//! }
//! ```

pub mod agents;
pub mod files;
pub mod service;
pub mod transcript;

// Re-export main types for convenience
pub use service::{
    ConversationContext, MemoryConfig, MemoryError, MemoryInjection, MemorySearchResults,
    MemoryService, SessionMemory,
};

pub use transcript::{
    MemoryTranscript, TranscriptMetadata, TranscriptSearchResult, TranscriptStore,
};

pub use agents::{AgentSection, AgentsConfig, AgentsService, GuidanceMatch, MatchType};

pub use files::{
    MatchLocation, MemoryFile, MemoryFileMetadata, MemoryFileSearchResult, MemoryFileService,
    MemoryFileType,
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
