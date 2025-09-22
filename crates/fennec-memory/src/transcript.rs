use anyhow::{Context, Result};
use directories::ProjectDirs;
use fennec_core::transcript::{Message, MessageRole, Transcript};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;
use tracing::{debug, info};
use uuid::Uuid;

/// Extended transcript with memory-specific metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryTranscript {
    /// Core transcript data from fennec-core
    pub transcript: Transcript,
    /// Tags for categorization and search
    pub tags: Vec<String>,
    /// Summary generated for quick reference
    pub summary: Option<String>,
    /// Key topics extracted from the conversation
    pub topics: Vec<String>,
    /// Metadata about the conversation
    pub metadata: TranscriptMetadata,
    /// Conversation context tracking
    pub conversation_context: ConversationContext,
    /// Command executions within this conversation
    pub command_executions: Vec<CommandExecution>,
    /// Transcript segments for better organization
    pub segments: Vec<TranscriptSegment>,
}

/// Metadata about a conversation transcript
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptMetadata {
    /// Session this transcript belongs to
    pub session_id: Uuid,
    /// When the transcript was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the transcript was last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Total message count
    pub message_count: usize,
    /// Approximate token count (for context management)
    pub estimated_tokens: usize,
    /// Whether this transcript is active (current session)
    pub is_active: bool,
}

/// Conversation context extracted from the transcript
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConversationContext {
    /// User's primary intent or goal in this conversation
    pub user_intent: Option<String>,
    /// Summary of AI responses and assistance provided
    pub ai_response_summary: Option<String>,
    /// Technologies and frameworks discussed
    pub technologies_mentioned: Vec<String>,
    /// Decisions made during the conversation
    pub decisions_made: Vec<String>,
    /// Problems encountered and discussed
    pub problems_encountered: Vec<String>,
    /// Solutions found or implemented
    pub solutions_found: Vec<String>,
    /// Key insights discovered
    pub insights: Vec<String>,
    /// Current working directory or project context
    pub project_context: Option<String>,
    /// Files mentioned or modified
    pub files_mentioned: Vec<String>,
}

/// Command execution record within a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecution {
    /// Unique identifier for this command execution
    pub id: Uuid,
    /// The command that was executed
    pub command: String,
    /// When the command was executed
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Result of the command execution
    pub result: ExecutionResult,
    /// Command output if successful
    pub output: Option<String>,
    /// Error message if failed
    pub error: Option<String>,
    /// How long the command took to execute
    pub duration: Option<Duration>,
    /// Exit code of the command
    pub exit_code: Option<i32>,
    /// Working directory when command was executed
    pub working_directory: Option<String>,
    /// Environment variables that were set
    pub environment: HashMap<String, String>,
    /// Message ID that triggered this command (if any)
    pub triggered_by_message: Option<Uuid>,
}

/// Result of a command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Whether the execution was successful
    pub success: bool,
    /// Summary of what was accomplished
    pub summary: String,
    /// Detailed output or error information
    pub details: Option<String>,
    /// Files created or modified
    pub files_affected: Vec<String>,
    /// Any follow-up actions suggested
    pub follow_up_actions: Vec<String>,
}

/// A segment of a conversation for better organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    /// Unique identifier for this segment
    pub id: Uuid,
    /// Start message ID of this segment
    pub start_message_id: Uuid,
    /// End message ID of this segment (optional if ongoing)
    pub end_message_id: Option<Uuid>,
    /// Human-readable title for this segment
    pub title: String,
    /// Summary of what happened in this segment
    pub summary: String,
    /// Context specific to this segment
    pub context: ConversationContext,
    /// Key outcomes from this segment
    pub key_outcomes: Vec<String>,
    /// Type of activity in this segment
    pub segment_type: SegmentType,
    /// When this segment was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Estimated tokens in this segment
    pub estimated_tokens: usize,
}

/// Types of conversation segments
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SegmentType {
    /// Planning and discussion phase
    Planning,
    /// Implementation and coding
    Implementation,
    /// Debugging and troubleshooting
    Debugging,
    /// Learning and exploration
    Learning,
    /// Review and testing
    Review,
    /// General conversation
    General,
}

/// Filters for searching transcripts
#[derive(Debug, Clone, Default)]
pub struct TranscriptSearchFilters {
    /// Filter by session ID
    pub session_id: Option<Uuid>,
    /// Filter by date range
    pub date_range: Option<(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>,
    /// Filter by technologies mentioned
    pub technologies: Option<Vec<String>>,
    /// Filter by segment type
    pub segment_type: Option<SegmentType>,
    /// Only include active transcripts
    pub active_only: bool,
    /// Maximum number of results
    pub limit: Option<usize>,
}

/// Storage service for managing conversation transcripts
#[derive(Debug)]
pub struct TranscriptStore {
    /// Base directory for storing transcripts
    storage_dir: PathBuf,
    /// In-memory cache of recent transcripts
    cache: HashMap<Uuid, MemoryTranscript>,
    /// Maximum cache size
    max_cache_size: usize,
}

impl TranscriptStore {
    /// Create a new transcript store
    pub fn new() -> Result<Self> {
        let storage_dir = Self::get_storage_dir()?;

        // Ensure storage directory exists
        std::fs::create_dir_all(&storage_dir).with_context(|| {
            format!(
                "Failed to create storage directory: {}",
                storage_dir.display()
            )
        })?;

        Ok(Self {
            storage_dir,
            cache: HashMap::new(),
            max_cache_size: 100, // Keep up to 100 transcripts in memory
        })
    }

    /// Get the storage directory for transcripts
    fn get_storage_dir() -> Result<PathBuf> {
        let proj_dirs =
            ProjectDirs::from("", "", "fennec").context("Failed to get project directories")?;

        Ok(proj_dirs.data_dir().join("transcripts"))
    }

    /// Store a transcript
    pub async fn store_transcript(&mut self, transcript: MemoryTranscript) -> Result<()> {
        let session_id = transcript.metadata.session_id;

        // Write to disk
        self.write_transcript_to_disk(&transcript).await?;

        // Update cache
        self.cache.insert(session_id, transcript);

        // Manage cache size
        if self.cache.len() > self.max_cache_size {
            self.evict_oldest_from_cache();
        }

        info!("Stored transcript for session: {}", session_id);
        Ok(())
    }

    /// Load a transcript by session ID
    pub async fn load_transcript(&mut self, session_id: Uuid) -> Result<Option<MemoryTranscript>> {
        // Check cache first
        if let Some(transcript) = self.cache.get(&session_id) {
            debug!("Found transcript in cache: {}", session_id);
            return Ok(Some(transcript.clone()));
        }

        // Load from disk
        match self.load_transcript_from_disk(session_id).await? {
            Some(transcript) => {
                debug!("Loaded transcript from disk: {}", session_id);
                self.cache.insert(session_id, transcript.clone());
                Ok(Some(transcript))
            }
            None => {
                debug!("Transcript not found: {}", session_id);
                Ok(None)
            }
        }
    }

    /// Update an existing transcript
    pub async fn update_transcript(
        &mut self,
        session_id: Uuid,
        transcript: Transcript,
    ) -> Result<()> {
        let memory_transcript = match self.load_transcript(session_id).await? {
            Some(mut mt) => {
                // Update the core transcript while preserving memory metadata
                mt.transcript = transcript;
                mt.metadata.updated_at = chrono::Utc::now();
                mt.metadata.message_count = mt.transcript.messages.len();
                mt.metadata.estimated_tokens = Self::estimate_tokens(&mt.transcript);
                mt
            }
            None => {
                // Create new memory transcript
                MemoryTranscript {
                    transcript: transcript.clone(),
                    tags: Vec::new(),
                    summary: None,
                    topics: Vec::new(),
                    metadata: TranscriptMetadata {
                        session_id,
                        created_at: chrono::Utc::now(),
                        updated_at: chrono::Utc::now(),
                        message_count: transcript.messages.len(),
                        estimated_tokens: Self::estimate_tokens(&transcript),
                        is_active: true,
                    },
                    conversation_context: ConversationContext::default(),
                    command_executions: Vec::new(),
                    segments: Vec::new(),
                }
            }
        };

        self.store_transcript(memory_transcript).await
    }

    /// Add a message to a transcript
    pub async fn add_message(
        &mut self,
        session_id: Uuid,
        role: MessageRole,
        content: String,
    ) -> Result<()> {
        let mut memory_transcript =
            self.load_transcript(session_id)
                .await?
                .unwrap_or_else(|| MemoryTranscript {
                    transcript: Transcript::new(session_id),
                    tags: Vec::new(),
                    summary: None,
                    topics: Vec::new(),
                    metadata: TranscriptMetadata {
                        session_id,
                        created_at: chrono::Utc::now(),
                        updated_at: chrono::Utc::now(),
                        message_count: 0,
                        estimated_tokens: 0,
                        is_active: true,
                    },
                    conversation_context: ConversationContext::default(),
                    command_executions: Vec::new(),
                    segments: Vec::new(),
                });

        memory_transcript.transcript.add_message(role, content);
        memory_transcript.metadata.updated_at = chrono::Utc::now();
        memory_transcript.metadata.message_count = memory_transcript.transcript.messages.len();
        memory_transcript.metadata.estimated_tokens =
            Self::estimate_tokens(&memory_transcript.transcript);

        self.store_transcript(memory_transcript).await
    }

    /// List all stored transcripts
    pub async fn list_transcripts(&self) -> Result<Vec<TranscriptMetadata>> {
        let mut transcripts = Vec::new();

        let mut dir = fs::read_dir(&self.storage_dir).await.with_context(|| {
            format!(
                "Failed to read storage directory: {}",
                self.storage_dir.display()
            )
        })?;

        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(session_id_str) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(session_id) = Uuid::parse_str(session_id_str) {
                        if let Ok(Some(transcript)) =
                            self.load_transcript_from_disk(session_id).await
                        {
                            transcripts.push(transcript.metadata);
                        }
                    }
                }
            }
        }

        // Sort by updated_at (most recent first)
        transcripts.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(transcripts)
    }

    /// Search transcripts by content
    pub async fn search_transcripts(
        &self,
        query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<TranscriptSearchResult>> {
        let mut results = Vec::new();
        use fuzzy_matcher::FuzzyMatcher;
        let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();

        let mut dir = fs::read_dir(&self.storage_dir).await.with_context(|| {
            format!(
                "Failed to read storage directory: {}",
                self.storage_dir.display()
            )
        })?;

        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(session_id_str) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(session_id) = Uuid::parse_str(session_id_str) {
                        if let Ok(Some(transcript)) =
                            self.load_transcript_from_disk(session_id).await
                        {
                            // Search in messages, summary, and topics
                            let mut best_score = 0i64;
                            let mut matching_messages = Vec::new();

                            // Search in messages
                            for message in &transcript.transcript.messages {
                                if let Some(score) = matcher.fuzzy_match(&message.content, query) {
                                    if score > best_score {
                                        best_score = score;
                                    }
                                    matching_messages.push(message.clone());
                                }
                            }

                            // Search in summary
                            if let Some(ref summary) = transcript.summary {
                                if let Some(score) = matcher.fuzzy_match(summary, query) {
                                    if score > best_score {
                                        best_score = score;
                                    }
                                }
                            }

                            // Search in topics
                            for topic in &transcript.topics {
                                if let Some(score) = matcher.fuzzy_match(topic, query) {
                                    if score > best_score {
                                        best_score = score;
                                    }
                                }
                            }

                            if best_score > 0 {
                                results.push(TranscriptSearchResult {
                                    session_id,
                                    metadata: transcript.metadata,
                                    score: best_score,
                                    matching_messages,
                                    summary: transcript.summary,
                                });
                            }
                        }
                    }
                }
            }
        }

        // Sort by score (highest first)
        results.sort_by(|a, b| b.score.cmp(&a.score));

        // Apply limit if specified
        if let Some(limit) = limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    /// Delete a transcript
    pub async fn delete_transcript(&mut self, session_id: Uuid) -> Result<()> {
        // Remove from cache
        self.cache.remove(&session_id);

        // Remove from disk
        let file_path = self.get_transcript_path(session_id);
        if file_path.exists() {
            fs::remove_file(&file_path).await.with_context(|| {
                format!("Failed to delete transcript file: {}", file_path.display())
            })?;
            info!("Deleted transcript for session: {}", session_id);
        }

        Ok(())
    }

    /// Get the file path for a transcript
    fn get_transcript_path(&self, session_id: Uuid) -> PathBuf {
        self.storage_dir.join(format!("{}.json", session_id))
    }

    /// Write transcript to disk
    async fn write_transcript_to_disk(&self, transcript: &MemoryTranscript) -> Result<()> {
        let file_path = self.get_transcript_path(transcript.metadata.session_id);
        let json =
            serde_json::to_string_pretty(transcript).context("Failed to serialize transcript")?;

        fs::write(&file_path, json)
            .await
            .with_context(|| format!("Failed to write transcript to: {}", file_path.display()))?;

        Ok(())
    }

    /// Load transcript from disk
    async fn load_transcript_from_disk(
        &self,
        session_id: Uuid,
    ) -> Result<Option<MemoryTranscript>> {
        let file_path = self.get_transcript_path(session_id);

        if !file_path.exists() {
            return Ok(None);
        }

        let json = fs::read_to_string(&file_path)
            .await
            .with_context(|| format!("Failed to read transcript from: {}", file_path.display()))?;

        let transcript: MemoryTranscript = serde_json::from_str(&json).with_context(|| {
            format!(
                "Failed to deserialize transcript from: {}",
                file_path.display()
            )
        })?;

        Ok(Some(transcript))
    }

    /// Evict oldest transcript from cache
    fn evict_oldest_from_cache(&mut self) {
        if let Some((oldest_id, _)) = self
            .cache
            .iter()
            .min_by_key(|(_, transcript)| transcript.metadata.updated_at)
            .map(|(id, transcript)| (*id, transcript.clone()))
        {
            self.cache.remove(&oldest_id);
            debug!("Evicted transcript from cache: {}", oldest_id);
        }
    }

    /// Estimate token count for a transcript (rough approximation)
    fn estimate_tokens(transcript: &Transcript) -> usize {
        transcript
            .messages
            .iter()
            .map(|msg| msg.content.len() / 4) // Rough estimation: 4 chars per token
            .sum()
    }

    /// Add tags to a transcript
    pub async fn add_tags(&mut self, session_id: Uuid, tags: Vec<String>) -> Result<()> {
        if let Some(mut transcript) = self.load_transcript(session_id).await? {
            for tag in tags {
                if !transcript.tags.contains(&tag) {
                    transcript.tags.push(tag);
                }
            }
            transcript.metadata.updated_at = chrono::Utc::now();
            self.store_transcript(transcript).await?;
        }
        Ok(())
    }

    /// Set summary for a transcript
    pub async fn set_summary(&mut self, session_id: Uuid, summary: String) -> Result<()> {
        if let Some(mut transcript) = self.load_transcript(session_id).await? {
            transcript.summary = Some(summary);
            transcript.metadata.updated_at = chrono::Utc::now();
            self.store_transcript(transcript).await?;
        }
        Ok(())
    }

    /// Add a command execution record to a transcript
    pub async fn add_command_execution(
        &mut self,
        session_id: Uuid,
        command: String,
        result: ExecutionResult,
        output: Option<String>,
        error: Option<String>,
        duration: Option<Duration>,
        exit_code: Option<i32>,
        triggered_by_message: Option<Uuid>,
    ) -> Result<Uuid> {
        let mut transcript = self
            .load_transcript(session_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Transcript not found: {}", session_id))?;

        let execution_id = Uuid::new_v4();
        let execution = CommandExecution {
            id: execution_id,
            command,
            timestamp: chrono::Utc::now(),
            result,
            output,
            error,
            duration,
            exit_code,
            working_directory: std::env::current_dir()
                .ok()
                .map(|p| p.to_string_lossy().to_string()),
            environment: std::env::vars().collect(),
            triggered_by_message,
        };

        transcript.command_executions.push(execution);
        transcript.metadata.updated_at = chrono::Utc::now();
        self.store_transcript(transcript).await?;

        Ok(execution_id)
    }

    /// Update conversation context
    pub async fn update_conversation_context(
        &mut self,
        session_id: Uuid,
        context_update: ConversationContextUpdate,
    ) -> Result<()> {
        let mut transcript = self
            .load_transcript(session_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Transcript not found: {}", session_id))?;

        let context = &mut transcript.conversation_context;

        if let Some(intent) = context_update.user_intent {
            context.user_intent = Some(intent);
        }

        if let Some(summary) = context_update.ai_response_summary {
            context.ai_response_summary = Some(summary);
        }

        context
            .technologies_mentioned
            .extend(context_update.technologies_mentioned);
        context.decisions_made.extend(context_update.decisions_made);
        context
            .problems_encountered
            .extend(context_update.problems_encountered);
        context
            .solutions_found
            .extend(context_update.solutions_found);
        context.insights.extend(context_update.insights);
        context
            .files_mentioned
            .extend(context_update.files_mentioned);

        if let Some(project_context) = context_update.project_context {
            context.project_context = Some(project_context);
        }

        transcript.metadata.updated_at = chrono::Utc::now();
        self.store_transcript(transcript).await?;

        Ok(())
    }

    /// Create a new conversation segment
    pub async fn create_segment(
        &mut self,
        session_id: Uuid,
        start_message_id: Uuid,
        title: String,
        segment_type: SegmentType,
    ) -> Result<Uuid> {
        let mut transcript = self
            .load_transcript(session_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Transcript not found: {}", session_id))?;

        let segment_id = Uuid::new_v4();
        let segment = TranscriptSegment {
            id: segment_id,
            start_message_id,
            end_message_id: None,
            title,
            summary: String::new(),
            context: ConversationContext::default(),
            key_outcomes: Vec::new(),
            segment_type,
            created_at: chrono::Utc::now(),
            estimated_tokens: 0,
        };

        transcript.segments.push(segment);
        transcript.metadata.updated_at = chrono::Utc::now();
        self.store_transcript(transcript).await?;

        Ok(segment_id)
    }

    /// End a conversation segment
    pub async fn end_segment(
        &mut self,
        session_id: Uuid,
        segment_id: Uuid,
        end_message_id: Uuid,
        summary: String,
        key_outcomes: Vec<String>,
    ) -> Result<()> {
        let mut transcript = self
            .load_transcript(session_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Transcript not found: {}", session_id))?;

        if let Some(segment) = transcript.segments.iter_mut().find(|s| s.id == segment_id) {
            segment.end_message_id = Some(end_message_id);
            segment.summary = summary;
            segment.key_outcomes = key_outcomes;

            // Calculate estimated tokens for this segment
            let start_idx = transcript
                .transcript
                .messages
                .iter()
                .position(|m| m.id == segment.start_message_id)
                .unwrap_or(0);
            let end_idx = transcript
                .transcript
                .messages
                .iter()
                .position(|m| m.id == end_message_id)
                .unwrap_or(transcript.transcript.messages.len());

            segment.estimated_tokens = transcript.transcript.messages[start_idx..=end_idx]
                .iter()
                .map(|m| m.content.len() / 4)
                .sum();
        }

        transcript.metadata.updated_at = chrono::Utc::now();
        self.store_transcript(transcript).await?;

        Ok(())
    }

    /// Search transcripts with advanced filters
    pub async fn search_transcripts_filtered(
        &mut self,
        query: &str,
        filters: TranscriptSearchFilters,
    ) -> Result<Vec<TranscriptSearchResult>> {
        let mut results = Vec::new();
        use fuzzy_matcher::FuzzyMatcher;
        let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();

        let mut dir = fs::read_dir(&self.storage_dir).await.with_context(|| {
            format!(
                "Failed to read storage directory: {}",
                self.storage_dir.display()
            )
        })?;

        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(session_id_str) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(session_id) = Uuid::parse_str(session_id_str) {
                        if let Ok(Some(transcript)) = self.load_transcript(session_id).await {
                            // Apply filters
                            if let Some(filter_session) = filters.session_id {
                                if transcript.metadata.session_id != filter_session {
                                    continue;
                                }
                            }

                            if let Some((start_date, end_date)) = filters.date_range {
                                if transcript.metadata.created_at < start_date
                                    || transcript.metadata.created_at > end_date
                                {
                                    continue;
                                }
                            }

                            if let Some(ref technologies) = filters.technologies {
                                if !technologies.iter().any(|tech| {
                                    transcript
                                        .conversation_context
                                        .technologies_mentioned
                                        .contains(tech)
                                }) {
                                    continue;
                                }
                            }

                            if let Some(ref segment_type) = filters.segment_type {
                                if !transcript
                                    .segments
                                    .iter()
                                    .any(|s| &s.segment_type == segment_type)
                                {
                                    continue;
                                }
                            }

                            if filters.active_only && !transcript.metadata.is_active {
                                continue;
                            }

                            // Perform search
                            let mut best_score = 0i64;
                            let mut matching_messages = Vec::new();

                            // Search in messages
                            for message in &transcript.transcript.messages {
                                if let Some(score) = matcher.fuzzy_match(&message.content, query) {
                                    if score > best_score {
                                        best_score = score;
                                    }
                                    matching_messages.push(message.clone());
                                }
                            }

                            // Search in conversation context
                            if let Some(ref intent) = transcript.conversation_context.user_intent {
                                if let Some(score) = matcher.fuzzy_match(intent, query) {
                                    if score > best_score {
                                        best_score = score;
                                    }
                                }
                            }

                            // Search in segments
                            for segment in &transcript.segments {
                                if let Some(score) = matcher.fuzzy_match(&segment.title, query) {
                                    if score > best_score {
                                        best_score = score;
                                    }
                                }
                                if let Some(score) = matcher.fuzzy_match(&segment.summary, query) {
                                    if score > best_score {
                                        best_score = score;
                                    }
                                }
                            }

                            // Search in command executions
                            for execution in &transcript.command_executions {
                                if let Some(score) = matcher.fuzzy_match(&execution.command, query)
                                {
                                    if score > best_score {
                                        best_score = score;
                                    }
                                }
                            }

                            if best_score > 0 {
                                results.push(TranscriptSearchResult {
                                    session_id,
                                    metadata: transcript.metadata,
                                    score: best_score,
                                    matching_messages,
                                    summary: transcript.summary,
                                });
                            }
                        }
                    }
                }
            }
        }

        // Sort by score (highest first)
        results.sort_by(|a, b| b.score.cmp(&a.score));

        // Apply limit if specified
        if let Some(limit) = filters.limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    /// Get timeline of activities for a session
    pub async fn get_session_timeline(&mut self, session_id: Uuid) -> Result<Vec<TimelineEvent>> {
        let transcript = self
            .load_transcript(session_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Transcript not found: {}", session_id))?;

        let mut events = Vec::new();

        // Add message events
        for message in &transcript.transcript.messages {
            events.push(TimelineEvent {
                timestamp: message.timestamp,
                event_type: TimelineEventType::Message {
                    role: message.role.clone(),
                    content_preview: if message.content.len() > 100 {
                        format!("{}...", &message.content[..100])
                    } else {
                        message.content.clone()
                    },
                },
            });
        }

        // Add command execution events
        for execution in &transcript.command_executions {
            events.push(TimelineEvent {
                timestamp: execution.timestamp,
                event_type: TimelineEventType::CommandExecution {
                    command: execution.command.clone(),
                    success: execution.result.success,
                    duration: execution.duration,
                },
            });
        }

        // Add segment events
        for segment in &transcript.segments {
            events.push(TimelineEvent {
                timestamp: segment.created_at,
                event_type: TimelineEventType::SegmentStart {
                    title: segment.title.clone(),
                    segment_type: segment.segment_type.clone(),
                },
            });
        }

        // Sort by timestamp
        events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        Ok(events)
    }
}

impl Default for TranscriptStore {
    fn default() -> Self {
        Self::new().expect("Failed to create default TranscriptStore")
    }
}

/// Result of a transcript search
#[derive(Debug, Clone)]
pub struct TranscriptSearchResult {
    pub session_id: Uuid,
    pub metadata: TranscriptMetadata,
    pub score: i64,
    pub matching_messages: Vec<Message>,
    pub summary: Option<String>,
}

/// Update structure for conversation context
#[derive(Debug, Clone, Default)]
pub struct ConversationContextUpdate {
    pub user_intent: Option<String>,
    pub ai_response_summary: Option<String>,
    pub technologies_mentioned: Vec<String>,
    pub decisions_made: Vec<String>,
    pub problems_encountered: Vec<String>,
    pub solutions_found: Vec<String>,
    pub insights: Vec<String>,
    pub project_context: Option<String>,
    pub files_mentioned: Vec<String>,
}

/// Timeline event for session activity tracking
#[derive(Debug, Clone)]
pub struct TimelineEvent {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event_type: TimelineEventType,
}

/// Types of events in a session timeline
#[derive(Debug, Clone)]
pub enum TimelineEventType {
    Message {
        role: MessageRole,
        content_preview: String,
    },
    CommandExecution {
        command: String,
        success: bool,
        duration: Option<Duration>,
    },
    SegmentStart {
        title: String,
        segment_type: SegmentType,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_store_and_load_transcript() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_owned();

        let mut store = TranscriptStore {
            storage_dir,
            cache: HashMap::new(),
            max_cache_size: 100,
        };

        let session_id = Uuid::new_v4();
        let transcript = Transcript::new(session_id);

        let memory_transcript = MemoryTranscript {
            transcript,
            tags: vec!["test".to_string()],
            summary: Some("Test summary".to_string()),
            topics: vec!["testing".to_string()],
            metadata: TranscriptMetadata {
                session_id,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                message_count: 0,
                estimated_tokens: 0,
                is_active: true,
            },
            conversation_context: ConversationContext::default(),
            command_executions: Vec::new(),
            segments: Vec::new(),
        };

        // Store transcript
        store
            .store_transcript(memory_transcript.clone())
            .await
            .unwrap();

        // Load transcript
        let loaded = store.load_transcript(session_id).await.unwrap().unwrap();
        assert_eq!(loaded.metadata.session_id, session_id);
        assert_eq!(loaded.tags, vec!["test"]);
        assert_eq!(loaded.summary, Some("Test summary".to_string()));
    }

    #[tokio::test]
    async fn test_add_message() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_owned();

        let mut store = TranscriptStore {
            storage_dir,
            cache: HashMap::new(),
            max_cache_size: 100,
        };

        let session_id = Uuid::new_v4();

        // Add a message
        store
            .add_message(session_id, MessageRole::User, "Hello".to_string())
            .await
            .unwrap();

        // Load and verify
        let transcript = store.load_transcript(session_id).await.unwrap().unwrap();
        assert_eq!(transcript.transcript.messages.len(), 1);
        assert_eq!(transcript.transcript.messages[0].content, "Hello");
    }
}
