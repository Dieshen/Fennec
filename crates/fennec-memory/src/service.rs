use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{watch, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use fennec_core::{
    session::Session,
    transcript::{MessageRole, Transcript},
    FennecError,
};

use crate::{
    agents::{AgentsConfig, AgentsService, GuidanceMatch},
    transcript::{TranscriptSearchResult, TranscriptStore},
};

/// Core memory service that orchestrates all memory functionality
#[derive(Debug)]
pub struct MemoryService {
    /// Agents configuration service
    agents_service: Arc<AgentsService>,
    /// Transcript storage service
    transcript_store: Arc<RwLock<TranscriptStore>>,
    /// Active sessions being tracked
    active_sessions: Arc<RwLock<HashMap<Uuid, SessionMemory>>>,
    /// Configuration for memory behavior
    config: MemoryConfig,
}

/// Configuration for memory service behavior
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Maximum number of messages to keep in memory per session
    pub max_messages_in_memory: usize,
    /// Whether to automatically generate summaries
    pub auto_generate_summaries: bool,
    /// Context window size for guidance injection
    pub guidance_context_window: usize,
    /// Maximum number of search results to return
    pub max_search_results: usize,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_messages_in_memory: 1000,
            auto_generate_summaries: true,
            guidance_context_window: 50,
            max_search_results: 10,
        }
    }
}

/// Memory data for an active session
#[derive(Debug, Clone)]
pub struct SessionMemory {
    /// Session information
    pub session: Session,
    /// Current transcript
    pub transcript: Transcript,
    /// Conversation context extracted from recent messages
    pub context: ConversationContext,
    /// Whether this session has been modified since last save
    pub is_dirty: bool,
}

/// Conversation context extracted from messages
#[derive(Debug, Clone)]
pub struct ConversationContext {
    /// Key topics discussed in recent messages
    pub recent_topics: Vec<String>,
    /// Current task or goal being worked on
    pub current_task: Option<String>,
    /// Technologies/frameworks mentioned
    pub technologies: Vec<String>,
    /// Error patterns or issues discussed
    pub error_patterns: Vec<String>,
}

impl Default for ConversationContext {
    fn default() -> Self {
        Self {
            recent_topics: Vec::new(),
            current_task: None,
            technologies: Vec::new(),
            error_patterns: Vec::new(),
        }
    }
}

/// Memory injection data for AI prompts
#[derive(Debug, Clone)]
pub struct MemoryInjection {
    /// Relevant guidance from AGENTS.md
    pub guidance: Vec<GuidanceMatch>,
    /// Relevant conversation history
    pub conversation_history: Vec<TranscriptSearchResult>,
    /// Current session context
    pub session_context: ConversationContext,
    /// Total estimated tokens for context management
    pub estimated_tokens: usize,
}

impl MemoryService {
    /// Create a new memory service
    pub async fn new() -> Result<Self> {
        let agents_service = Arc::new(AgentsService::new().await?);
        let transcript_store = Arc::new(RwLock::new(TranscriptStore::new()?));
        let active_sessions = Arc::new(RwLock::new(HashMap::new()));
        let config = MemoryConfig::default();

        info!("Memory service initialized");

        Ok(Self {
            agents_service,
            transcript_store,
            active_sessions,
            config,
        })
    }

    /// Create a new memory service with custom configuration
    pub async fn with_config(config: MemoryConfig) -> Result<Self> {
        let mut service = Self::new().await?;
        service.config = config;
        Ok(service)
    }

    /// Start tracking a session
    pub async fn start_session(&self, session: Session) -> Result<()> {
        let session_id = session.id;
        debug!("Starting memory tracking for session: {}", session_id);

        // Load existing transcript if available
        let transcript = {
            let mut store = self.transcript_store.write().await;
            store
                .load_transcript(session_id)
                .await?
                .map(|mt| mt.transcript)
                .unwrap_or_else(|| Transcript::new(session_id))
        };

        let session_memory = SessionMemory {
            session,
            transcript,
            context: ConversationContext::default(),
            is_dirty: false,
        };

        {
            let mut sessions = self.active_sessions.write().await;
            sessions.insert(session_id, session_memory);
        }

        info!("Started memory tracking for session: {}", session_id);
        Ok(())
    }

    /// Stop tracking a session and persist any changes
    pub async fn stop_session(&self, session_id: Uuid) -> Result<()> {
        debug!("Stopping memory tracking for session: {}", session_id);

        let session_memory = {
            let mut sessions = self.active_sessions.write().await;
            sessions.remove(&session_id)
        };

        if let Some(memory) = session_memory {
            if memory.is_dirty {
                // Save transcript
                let mut store = self.transcript_store.write().await;
                store
                    .update_transcript(session_id, memory.transcript)
                    .await?;
            }
        }

        info!("Stopped memory tracking for session: {}", session_id);
        Ok(())
    }

    /// Add a message to a session
    pub async fn add_message(
        &self,
        session_id: Uuid,
        role: MessageRole,
        content: String,
    ) -> Result<()> {
        // Update active session if it exists
        {
            let mut sessions = self.active_sessions.write().await;
            if let Some(session_memory) = sessions.get_mut(&session_id) {
                session_memory
                    .transcript
                    .add_message(role.clone(), content.clone());
                session_memory.is_dirty = true;

                // Update conversation context
                self.update_conversation_context(&mut session_memory.context, &content);

                // Trim transcript if it's getting too large
                if session_memory.transcript.messages.len() > self.config.max_messages_in_memory {
                    let excess = session_memory.transcript.messages.len()
                        - self.config.max_messages_in_memory;
                    session_memory.transcript.messages.drain(0..excess);
                }
            }
        }

        // Also update persistent storage
        {
            let mut store = self.transcript_store.write().await;
            store.add_message(session_id, role, content).await?;
        }

        debug!("Added message to session: {}", session_id);
        Ok(())
    }

    /// Get memory injection data for AI prompts
    pub async fn get_memory_injection(
        &self,
        session_id: Uuid,
        query: Option<&str>,
    ) -> Result<MemoryInjection> {
        debug!("Generating memory injection for session: {}", session_id);

        let session_context = {
            let sessions = self.active_sessions.read().await;
            sessions
                .get(&session_id)
                .map(|sm| sm.context.clone())
                .unwrap_or_default()
        };

        // Get relevant guidance from AGENTS.md
        let guidance = if let Some(query) = query {
            self.agents_service.search_guidance(query)
        } else {
            // Use session context to find relevant guidance
            let mut guidance = Vec::new();
            for topic in &session_context.recent_topics {
                guidance.extend(self.agents_service.search_guidance(topic));
            }
            for tech in &session_context.technologies {
                guidance.extend(self.agents_service.search_guidance(tech));
            }
            guidance
        };

        // Get relevant conversation history
        let conversation_history = if let Some(query) = query {
            let store = self.transcript_store.write().await;
            store
                .search_transcripts(query, Some(self.config.max_search_results))
                .await?
        } else {
            // Search based on current session context
            let mut results = Vec::new();
            let store = self.transcript_store.write().await;

            for topic in &session_context.recent_topics {
                let search_results = store.search_transcripts(topic, Some(3)).await?;
                results.extend(search_results);
            }

            // Deduplicate and limit results
            results.sort_by(|a, b| b.score.cmp(&a.score));
            results.truncate(self.config.max_search_results);
            results
        };

        // Estimate tokens
        let estimated_tokens = self.estimate_injection_tokens(&guidance, &conversation_history);

        Ok(MemoryInjection {
            guidance: guidance.into_iter().take(5).collect(), // Limit guidance items
            conversation_history,
            session_context,
            estimated_tokens,
        })
    }

    /// Search through all memory
    pub async fn search(&self, query: &str, limit: Option<usize>) -> Result<MemorySearchResults> {
        debug!("Searching memory with query: {}", query);

        // Search guidance
        let guidance_matches = self.agents_service.search_guidance(query);

        // Search transcripts
        let store = self.transcript_store.write().await;
        let transcript_matches = store.search_transcripts(query, limit).await?;

        Ok(MemorySearchResults {
            guidance_matches,
            transcript_matches,
            query: query.to_string(),
        })
    }

    /// Get all available guidance sections
    pub fn get_available_guidance(&self) -> Vec<String> {
        self.agents_service.get_all_guidance()
    }

    /// Get specific guidance section
    pub fn get_guidance_section(&self, title: &str) -> Option<crate::agents::AgentSection> {
        self.agents_service.get_guidance_section(title)
    }

    /// List all stored sessions
    pub async fn list_sessions(&self) -> Result<Vec<crate::transcript::TranscriptMetadata>> {
        let store = self.transcript_store.read().await;
        store.list_transcripts().await
    }

    /// Delete a session and its transcript
    pub async fn delete_session(&self, session_id: Uuid) -> Result<()> {
        // Remove from active sessions
        {
            let mut sessions = self.active_sessions.write().await;
            sessions.remove(&session_id);
        }

        // Delete from storage
        {
            let mut store = self.transcript_store.write().await;
            store.delete_transcript(session_id).await?;
        }

        info!("Deleted session: {}", session_id);
        Ok(())
    }

    /// Subscribe to AGENTS.md configuration changes
    pub fn subscribe_to_agents_config(&self) -> watch::Receiver<Option<AgentsConfig>> {
        self.agents_service.subscribe()
    }

    /// Get current agents configuration
    pub fn get_agents_config(&self) -> Option<AgentsConfig> {
        self.agents_service.get_config()
    }

    /// Force reload of AGENTS.md configuration
    pub async fn reload_agents_config(&self) -> Result<()> {
        // Note: In the current implementation, AgentsService loads config in constructor
        // A full implementation would add a reload method to AgentsService
        warn!("Manual config reload not yet implemented - use file watching instead");
        Ok(())
    }

    /// Add tags to a session
    pub async fn add_session_tags(&self, session_id: Uuid, tags: Vec<String>) -> Result<()> {
        let mut store = self.transcript_store.write().await;
        store.add_tags(session_id, tags).await
    }

    /// Set summary for a session
    pub async fn set_session_summary(&self, session_id: Uuid, summary: String) -> Result<()> {
        let mut store = self.transcript_store.write().await;
        store.set_summary(session_id, summary).await
    }

    /// Get session memory if active
    pub async fn get_session_memory(&self, session_id: Uuid) -> Option<SessionMemory> {
        let sessions = self.active_sessions.read().await;
        sessions.get(&session_id).cloned()
    }

    /// Update conversation context based on message content
    fn update_conversation_context(&self, context: &mut ConversationContext, content: &str) {
        // Simple keyword extraction - in a real implementation you'd want more sophisticated NLP
        let content_lower = content.to_lowercase();

        // Extract technologies
        let tech_keywords = [
            "rust",
            "python",
            "javascript",
            "typescript",
            "react",
            "vue",
            "go",
            "java",
            "c++",
            "docker",
            "kubernetes",
        ];
        for tech in tech_keywords {
            if content_lower.contains(tech) && !context.technologies.contains(&tech.to_string()) {
                context.technologies.push(tech.to_string());
            }
        }

        // Extract task indicators
        let task_indicators = [
            "implement",
            "fix",
            "debug",
            "create",
            "build",
            "deploy",
            "test",
        ];
        for indicator in task_indicators {
            if content_lower.contains(indicator) {
                // Extract potential task description (simplified)
                if let Some(task_start) = content_lower.find(indicator) {
                    let task_text =
                        &content[task_start..std::cmp::min(task_start + 100, content.len())];
                    context.current_task = Some(task_text.to_string());
                    break;
                }
            }
        }

        // Extract error patterns
        if content_lower.contains("error")
            || content_lower.contains("exception")
            || content_lower.contains("failed")
        {
            context.error_patterns.push(content.to_string());
            if context.error_patterns.len() > 10 {
                context.error_patterns.remove(0);
            }
        }

        // Update recent topics (simplified - just use first few words)
        let words: Vec<&str> = content.split_whitespace().take(5).collect();
        if !words.is_empty() {
            let topic = words.join(" ");
            context.recent_topics.push(topic);
            if context.recent_topics.len() > 20 {
                context.recent_topics.remove(0);
            }
        }
    }

    /// Estimate token count for memory injection
    fn estimate_injection_tokens(
        &self,
        guidance: &[GuidanceMatch],
        conversation_history: &[TranscriptSearchResult],
    ) -> usize {
        let guidance_tokens: usize = guidance.iter().map(|g| g.content.len() / 4).sum();

        let conversation_tokens: usize = conversation_history
            .iter()
            .map(|ch| {
                ch.matching_messages
                    .iter()
                    .map(|m| m.content.len() / 4)
                    .sum::<usize>()
            })
            .sum();

        guidance_tokens + conversation_tokens
    }
}

/// Combined search results from memory
#[derive(Debug)]
pub struct MemorySearchResults {
    pub guidance_matches: Vec<GuidanceMatch>,
    pub transcript_matches: Vec<TranscriptSearchResult>,
    pub query: String,
}

/// Error types specific to memory operations
#[derive(thiserror::Error, Debug)]
pub enum MemoryError {
    #[error("Session not found: {session_id}")]
    SessionNotFound { session_id: Uuid },

    #[error("Storage error: {message}")]
    Storage { message: String },

    #[error("Configuration error: {message}")]
    Configuration { message: String },

    #[error("Search error: {message}")]
    Search { message: String },
}

impl From<MemoryError> for FennecError {
    fn from(err: MemoryError) -> Self {
        FennecError::Memory {
            message: err.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fennec_core::session::Session;

    #[tokio::test]
    async fn test_memory_service_creation() {
        let service = MemoryService::new().await;
        assert!(service.is_ok());
    }

    #[tokio::test]
    async fn test_session_lifecycle() {
        let service = MemoryService::new().await.unwrap();
        let session = Session::new();
        let session_id = session.id;

        // Start session
        service.start_session(session).await.unwrap();

        // Add message
        service
            .add_message(session_id, MessageRole::User, "Hello".to_string())
            .await
            .unwrap();

        // Get memory injection
        let injection = service
            .get_memory_injection(session_id, Some("test"))
            .await
            .unwrap();
        assert_eq!(injection.session_context.recent_topics.len(), 1);

        // Stop session
        service.stop_session(session_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_memory_search() {
        let service = MemoryService::new().await.unwrap();

        // Search should not fail even with no data
        let results = service.search("test query", Some(5)).await.unwrap();
        assert_eq!(results.query, "test query");
    }
}
