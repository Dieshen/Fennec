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
    cline_files::{Achievement, ClineFileType, ClineMemoryFileService, MemoryEvent, ProjectStatus},
    files::MemoryFileService,
    transcript::{TranscriptSearchResult, TranscriptStore},
};

/// Core memory service that orchestrates all memory functionality
#[derive(Debug)]
pub struct MemoryService {
    /// Agents configuration service
    agents_service: Arc<AgentsService>,
    /// Transcript storage service
    transcript_store: Arc<RwLock<TranscriptStore>>,
    /// Memory file service
    memory_file_service: Arc<RwLock<MemoryFileService>>,
    /// Cline-style memory file service
    cline_memory_service: Arc<RwLock<ClineMemoryFileService>>,
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

/// Advanced search criteria for context-aware memory retrieval
#[derive(Debug, Clone)]
pub struct AdvancedSearchCriteria {
    /// Primary search query
    pub query: String,
    /// Session-based filtering
    pub session_filter: Option<SessionFilter>,
    /// Time-based filtering
    pub time_filter: Option<TimeFilter>,
    /// Memory type preferences
    pub memory_types: Vec<MemoryType>,
    /// Relevance scoring strategy
    pub scoring_strategy: ScoringStrategy,
    /// Maximum number of results
    pub limit: Option<usize>,
    /// Minimum relevance score threshold
    pub min_score: Option<f64>,
}

/// Session-based filtering options
#[derive(Debug, Clone)]
pub enum SessionFilter {
    /// Only current session
    CurrentSession(Uuid),
    /// Exclude current session
    ExcludeCurrentSession(Uuid),
    /// Specific sessions only
    SpecificSessions(Vec<Uuid>),
    /// Cross-session (all sessions)
    CrossSession,
}

/// Time-based filtering options
#[derive(Debug, Clone)]
pub enum TimeFilter {
    /// Last N hours
    LastHours(u32),
    /// Last N days
    LastDays(u32),
    /// Since specific timestamp
    Since(chrono::DateTime<chrono::Utc>),
    /// Between two timestamps
    Between {
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    },
    /// Recent vs historical (split based on configured threshold)
    Recent,
    /// Historical only
    Historical,
}

/// Memory types that can be searched
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MemoryType {
    Transcripts,
    Guidance,
    MemoryFiles,
}

/// Relevance scoring strategies
#[derive(Debug, Clone)]
pub enum ScoringStrategy {
    /// Simple fuzzy matching (current default)
    FuzzyMatch,
    /// Weighted scoring considering multiple factors
    Weighted {
        text_relevance_weight: f64,
        recency_weight: f64,
        session_relevance_weight: f64,
    },
    /// Context-aware scoring based on current conversation
    ContextAware {
        conversation_context: ConversationContext,
    },
    /// Combined scoring using multiple strategies
    Combined(Vec<ScoringStrategy>),
}

/// Unified search result combining all memory types
#[derive(Debug, Clone)]
pub struct UnifiedSearchResult {
    /// Type of memory source
    pub memory_type: MemoryType,
    /// Unique identifier for this result
    pub id: String,
    /// Title or name of the result
    pub title: String,
    /// Content preview or excerpt
    pub content_preview: String,
    /// Full content (if requested)
    pub full_content: Option<String>,
    /// Relevance score (0.0 to 1.0)
    pub relevance_score: f64,
    /// Timestamp of creation/update
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Associated session ID (if applicable)
    pub session_id: Option<Uuid>,
    /// Additional metadata specific to memory type
    pub metadata: UnifiedSearchMetadata,
}

/// Metadata specific to different memory types
#[derive(Debug, Clone)]
pub enum UnifiedSearchMetadata {
    Transcript {
        message_count: usize,
        summary: Option<String>,
        tags: Vec<String>,
    },
    Guidance {
        section_title: String,
        match_type: crate::agents::MatchType,
    },
    MemoryFile {
        file_type: crate::files::MemoryFileType,
        tags: Vec<String>,
        related_sessions: Vec<Uuid>,
    },
}

/// Enhanced search results with context and scoring information
#[derive(Debug, Clone)]
pub struct EnhancedSearchResults {
    /// Found results
    pub results: Vec<UnifiedSearchResult>,
    /// Search criteria used
    pub criteria: AdvancedSearchCriteria,
    /// Search execution metadata
    pub search_metadata: SearchMetadata,
}

/// Metadata about search execution
#[derive(Debug, Clone)]
pub struct SearchMetadata {
    /// Total results found (before limit)
    pub total_found: usize,
    /// Number of results returned
    pub returned_count: usize,
    /// Time taken to execute search
    pub execution_time_ms: u64,
    /// Sources searched
    pub sources_searched: Vec<MemoryType>,
    /// Scoring strategy used
    pub scoring_strategy: ScoringStrategy,
}

impl MemoryService {
    /// Create a new memory service
    pub async fn new() -> Result<Self> {
        let agents_service = Arc::new(AgentsService::new().await?);
        let transcript_store = Arc::new(RwLock::new(TranscriptStore::new()?));
        let memory_file_service = Arc::new(RwLock::new(MemoryFileService::new()?));
        let cline_memory_service = Arc::new(RwLock::new(ClineMemoryFileService::new()?));
        let active_sessions = Arc::new(RwLock::new(HashMap::new()));
        let config = MemoryConfig::default();

        info!("Memory service initialized with Cline-style memory files");

        Ok(Self {
            agents_service,
            transcript_store,
            memory_file_service,
            cline_memory_service,
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

    /// Enhanced context-aware search across all memory types
    pub async fn search_advanced(
        &self,
        criteria: AdvancedSearchCriteria,
    ) -> Result<EnhancedSearchResults> {
        let start_time = std::time::Instant::now();
        debug!("Advanced search with criteria: {:?}", criteria);

        let mut all_results = Vec::new();
        let mut sources_searched = Vec::new();

        // Search each memory type if requested
        for memory_type in &criteria.memory_types {
            sources_searched.push(memory_type.clone());

            match memory_type {
                MemoryType::Transcripts => {
                    let transcript_results = self.search_transcripts_advanced(&criteria).await?;
                    all_results.extend(transcript_results);
                }
                MemoryType::Guidance => {
                    let guidance_results = self.search_guidance_advanced(&criteria);
                    all_results.extend(guidance_results);
                }
                MemoryType::MemoryFiles => {
                    let memory_file_results = self.search_memory_files_advanced(&criteria).await?;
                    all_results.extend(memory_file_results);
                }
            }
        }

        // Apply scoring strategy
        self.apply_scoring_strategy(&mut all_results, &criteria);

        // Apply time filtering
        if let Some(ref time_filter) = criteria.time_filter {
            all_results = self.apply_time_filter(all_results, time_filter);
        }

        // Apply session filtering
        if let Some(ref session_filter) = criteria.session_filter {
            all_results = self.apply_session_filter(all_results, session_filter);
        }

        // Apply minimum score threshold
        if let Some(min_score) = criteria.min_score {
            all_results.retain(|result| result.relevance_score >= min_score);
        }

        // Sort by relevance score (highest first)
        all_results.sort_by(|a, b| b.relevance_score.total_cmp(&a.relevance_score));

        let total_found = all_results.len();

        // Apply limit
        if let Some(limit) = criteria.limit {
            all_results.truncate(limit);
        }

        let execution_time = start_time.elapsed();

        let search_metadata = SearchMetadata {
            total_found,
            returned_count: all_results.len(),
            execution_time_ms: execution_time.as_millis() as u64,
            sources_searched,
            scoring_strategy: criteria.scoring_strategy.clone(),
        };

        Ok(EnhancedSearchResults {
            results: all_results,
            criteria,
            search_metadata,
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

    /// Search transcripts with advanced criteria
    async fn search_transcripts_advanced(
        &self,
        criteria: &AdvancedSearchCriteria,
    ) -> Result<Vec<UnifiedSearchResult>> {
        let store = self.transcript_store.read().await;
        let transcript_results = store.search_transcripts(&criteria.query, None).await?;

        let mut unified_results = Vec::new();
        for result in transcript_results {
            unified_results.push(UnifiedSearchResult {
                memory_type: MemoryType::Transcripts,
                id: result.session_id.to_string(),
                title: format!("Session {}", result.session_id),
                content_preview: self.generate_content_preview(&result.matching_messages),
                full_content: None, // Will be populated if needed
                relevance_score: self.normalize_fuzzy_score(result.score),
                timestamp: result.metadata.updated_at,
                session_id: Some(result.session_id),
                metadata: UnifiedSearchMetadata::Transcript {
                    message_count: result.metadata.message_count,
                    summary: result.summary,
                    tags: Vec::new(), // TODO: Add tags from transcript metadata
                },
            });
        }

        Ok(unified_results)
    }

    /// Search guidance with advanced criteria
    fn search_guidance_advanced(
        &self,
        criteria: &AdvancedSearchCriteria,
    ) -> Vec<UnifiedSearchResult> {
        let guidance_matches = self.agents_service.search_guidance(&criteria.query);

        let mut unified_results = Vec::new();
        for guidance in guidance_matches {
            unified_results.push(UnifiedSearchResult {
                memory_type: MemoryType::Guidance,
                id: format!("guidance_{}", guidance.section_title.replace(' ', "_")),
                title: guidance.section_title.clone(),
                content_preview: self.truncate_content(&guidance.content, 200),
                full_content: Some(guidance.content.clone()),
                relevance_score: self.normalize_fuzzy_score(guidance.score),
                timestamp: chrono::Utc::now(), // Guidance doesn't have timestamps
                session_id: None,
                metadata: UnifiedSearchMetadata::Guidance {
                    section_title: guidance.section_title,
                    match_type: guidance.match_type,
                },
            });
        }

        unified_results
    }

    /// Search memory files with advanced criteria
    async fn search_memory_files_advanced(
        &self,
        criteria: &AdvancedSearchCriteria,
    ) -> Result<Vec<UnifiedSearchResult>> {
        let mut file_service = self.memory_file_service.write().await;
        let file_results = file_service
            .search_memory_files(&criteria.query, None)
            .await?;

        let mut unified_results = Vec::new();
        for result in file_results {
            unified_results.push(UnifiedSearchResult {
                memory_type: MemoryType::MemoryFiles,
                id: result.id.to_string(),
                title: result.name,
                content_preview: result.content_preview,
                full_content: None, // Would need to load full file if needed
                relevance_score: self.normalize_fuzzy_score(result.score),
                timestamp: result.updated_at,
                session_id: None, // Memory files can have multiple related sessions
                metadata: UnifiedSearchMetadata::MemoryFile {
                    file_type: result.file_type,
                    tags: Vec::new(), // Would need to load from full file
                    related_sessions: Vec::new(), // Would need to load from full file
                },
            });
        }

        Ok(unified_results)
    }

    /// Apply scoring strategy to search results
    fn apply_scoring_strategy(
        &self,
        results: &mut [UnifiedSearchResult],
        criteria: &AdvancedSearchCriteria,
    ) {
        match &criteria.scoring_strategy {
            ScoringStrategy::FuzzyMatch => {
                // Already applied during individual searches
            }
            ScoringStrategy::Weighted {
                text_relevance_weight,
                recency_weight,
                session_relevance_weight,
            } => {
                let now = chrono::Utc::now();
                let current_session = self.get_current_session_from_criteria(criteria);

                for result in results.iter_mut() {
                    let text_score = result.relevance_score;

                    // Calculate recency score (more recent = higher score)
                    let hours_old = (now - result.timestamp).num_hours() as f64;
                    let recency_score = 1.0 / (1.0 + hours_old / 24.0); // Decay over days

                    // Calculate session relevance score
                    let session_score = if let Some(current_session) = current_session {
                        if result.session_id == Some(current_session) {
                            1.0
                        } else {
                            0.5 // Related sessions get partial score
                        }
                    } else {
                        0.8 // Default score when no session context
                    };

                    // Combine scores with weights
                    result.relevance_score = text_score * text_relevance_weight
                        + recency_score * recency_weight
                        + session_score * session_relevance_weight;
                }
            }
            ScoringStrategy::ContextAware {
                conversation_context,
            } => {
                for result in results.iter_mut() {
                    let mut context_bonus = 0.0;

                    // Boost score if content relates to current technologies
                    for tech in &conversation_context.technologies {
                        if result
                            .content_preview
                            .to_lowercase()
                            .contains(&tech.to_lowercase())
                        {
                            context_bonus += 0.2;
                        }
                    }

                    // Boost score if content relates to current task
                    if let Some(ref task) = conversation_context.current_task {
                        if result
                            .content_preview
                            .to_lowercase()
                            .contains(&task.to_lowercase())
                        {
                            context_bonus += 0.3;
                        }
                    }

                    // Boost score if content relates to recent topics
                    for topic in &conversation_context.recent_topics {
                        if result
                            .content_preview
                            .to_lowercase()
                            .contains(&topic.to_lowercase())
                        {
                            context_bonus += 0.1;
                        }
                    }

                    result.relevance_score = (result.relevance_score + context_bonus).min(1.0);
                }
            }
            ScoringStrategy::Combined(strategies) => {
                // Apply each strategy and average the scores
                let original_scores: Vec<f64> = results.iter().map(|r| r.relevance_score).collect();

                for strategy in strategies {
                    let mut temp_criteria = criteria.clone();
                    temp_criteria.scoring_strategy = strategy.clone();
                    self.apply_scoring_strategy(results, &temp_criteria);
                }

                // Average with original scores
                for (i, result) in results.iter_mut().enumerate() {
                    result.relevance_score = (result.relevance_score + original_scores[i]) / 2.0;
                }
            }
        }
    }

    /// Apply time-based filtering
    fn apply_time_filter(
        &self,
        mut results: Vec<UnifiedSearchResult>,
        time_filter: &TimeFilter,
    ) -> Vec<UnifiedSearchResult> {
        let now = chrono::Utc::now();

        results.retain(|result| {
            match time_filter {
                TimeFilter::LastHours(hours) => {
                    let cutoff = now - chrono::Duration::hours(*hours as i64);
                    result.timestamp >= cutoff
                }
                TimeFilter::LastDays(days) => {
                    let cutoff = now - chrono::Duration::days(*days as i64);
                    result.timestamp >= cutoff
                }
                TimeFilter::Since(timestamp) => result.timestamp >= *timestamp,
                TimeFilter::Between { start, end } => {
                    result.timestamp >= *start && result.timestamp <= *end
                }
                TimeFilter::Recent => {
                    // Consider recent as last 24 hours
                    let cutoff = now - chrono::Duration::hours(24);
                    result.timestamp >= cutoff
                }
                TimeFilter::Historical => {
                    // Consider historical as older than 24 hours
                    let cutoff = now - chrono::Duration::hours(24);
                    result.timestamp < cutoff
                }
            }
        });

        results
    }

    /// Apply session-based filtering
    fn apply_session_filter(
        &self,
        mut results: Vec<UnifiedSearchResult>,
        session_filter: &SessionFilter,
    ) -> Vec<UnifiedSearchResult> {
        results.retain(|result| {
            match session_filter {
                SessionFilter::CurrentSession(session_id) => result.session_id == Some(*session_id),
                SessionFilter::ExcludeCurrentSession(session_id) => {
                    result.session_id != Some(*session_id)
                }
                SessionFilter::SpecificSessions(session_ids) => result
                    .session_id
                    .map_or(false, |id| session_ids.contains(&id)),
                SessionFilter::CrossSession => {
                    true // Include all sessions
                }
            }
        });

        results
    }

    /// Helper to get current session from criteria
    fn get_current_session_from_criteria(&self, criteria: &AdvancedSearchCriteria) -> Option<Uuid> {
        match &criteria.session_filter {
            Some(SessionFilter::CurrentSession(session_id)) => Some(*session_id),
            _ => None,
        }
    }

    /// Normalize fuzzy match scores to 0.0-1.0 range
    fn normalize_fuzzy_score(&self, fuzzy_score: i64) -> f64 {
        // Fuzzy scores can vary widely, normalize to 0.0-1.0
        // This is a rough approximation - in production you'd want to calibrate this
        let normalized = fuzzy_score as f64 / 1000.0;
        normalized.min(1.0).max(0.0)
    }

    /// Generate content preview from messages
    fn generate_content_preview(&self, messages: &[fennec_core::transcript::Message]) -> String {
        if messages.is_empty() {
            return String::new();
        }

        // Take the first few messages and create a preview
        let preview_text: String = messages
            .iter()
            .take(3)
            .map(|m| m.content.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        self.truncate_content(&preview_text, 200)
    }

    /// Truncate content to specified length
    fn truncate_content(&self, content: &str, max_length: usize) -> String {
        if content.len() <= max_length {
            content.to_string()
        } else {
            format!("{}...", &content[..max_length])
        }
    }

    // ================================
    // Cline-style Memory File Methods
    // ================================

    /// Initialize Cline-style memory files for a project
    pub async fn initialize_project_memory(&self, project_id: Uuid) -> Result<()> {
        let mut cline_service = self.cline_memory_service.write().await;
        cline_service.initialize_project(project_id).await?;
        info!("Initialized Cline memory files for project: {}", project_id);
        Ok(())
    }

    /// Get project brief as markdown
    pub async fn get_project_brief(&self, project_id: Uuid) -> Result<Option<String>> {
        let mut cline_service = self.cline_memory_service.write().await;
        cline_service
            .render_to_markdown(project_id, ClineFileType::ProjectBrief)
            .await
    }

    /// Get active context as markdown
    pub async fn get_active_context(&self, project_id: Uuid) -> Result<Option<String>> {
        let mut cline_service = self.cline_memory_service.write().await;
        cline_service
            .render_to_markdown(project_id, ClineFileType::ActiveContext)
            .await
    }

    /// Get progress tracking as markdown
    pub async fn get_progress(&self, project_id: Uuid) -> Result<Option<String>> {
        let mut cline_service = self.cline_memory_service.write().await;
        cline_service
            .render_to_markdown(project_id, ClineFileType::Progress)
            .await
    }

    /// Update project goals
    pub async fn update_project_goals(&self, project_id: Uuid, goals: Vec<String>) -> Result<()> {
        let event = MemoryEvent::ProjectGoalUpdated { project_id, goals };
        let mut cline_service = self.cline_memory_service.write().await;
        cline_service.handle_event(event).await?;
        Ok(())
    }

    /// Update project status
    pub async fn update_project_status(
        &self,
        project_id: Uuid,
        status: ProjectStatus,
    ) -> Result<()> {
        let event = MemoryEvent::ProjectStatusChanged { project_id, status };
        let mut cline_service = self.cline_memory_service.write().await;
        cline_service.handle_event(event).await?;
        Ok(())
    }

    /// Mark a task as completed
    pub async fn complete_task(
        &self,
        project_id: Uuid,
        session_id: Option<Uuid>,
        task: String,
        outcome: String,
    ) -> Result<()> {
        let event = MemoryEvent::TaskCompleted {
            project_id,
            session_id,
            task,
            outcome,
        };
        let mut cline_service = self.cline_memory_service.write().await;
        cline_service.handle_event(event).await?;
        Ok(())
    }

    /// Record an achievement
    pub async fn record_achievement(
        &self,
        project_id: Uuid,
        session_id: Option<Uuid>,
        achievement: Achievement,
    ) -> Result<()> {
        let event = MemoryEvent::AchievementReached {
            project_id,
            session_id,
            achievement,
        };
        let mut cline_service = self.cline_memory_service.write().await;
        cline_service.handle_event(event).await?;
        Ok(())
    }

    /// Archive a project's memory files
    pub async fn archive_project(&self, project_id: Uuid) -> Result<std::path::PathBuf> {
        let mut cline_service = self.cline_memory_service.write().await;
        cline_service.archive_project(project_id).await
    }

    /// Create backup of project files
    pub async fn backup_project(&self, project_id: Uuid) -> Result<std::path::PathBuf> {
        let cline_service = self.cline_memory_service.read().await;
        cline_service.backup_files(project_id).await
    }

    /// List all projects with memory files
    pub async fn list_projects(&self) -> Result<Vec<Uuid>> {
        let cline_service = self.cline_memory_service.read().await;
        cline_service.list_projects().await
    }

    /// Emit memory event to update Cline files (internal method)
    #[allow(dead_code)]
    async fn emit_memory_event(&self, event: MemoryEvent) -> Result<()> {
        let mut cline_service = self.cline_memory_service.write().await;
        cline_service.handle_event(event).await?;
        Ok(())
    }

    /// Get session's project ID (would need to be tracked separately in real implementation)
    pub fn get_session_project_id(&self, _session_id: Uuid) -> Option<Uuid> {
        // For now, return None - in a real implementation, you'd need to track
        // which sessions belong to which projects
        // This could be stored in the SessionMemory struct or a separate mapping
        warn!("Session to project mapping not implemented - returning None");
        None
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
