use anyhow::{Context, Result};
use directories::ProjectDirs;
use fennec_core::transcript::{Message, MessageRole, Transcript};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
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
