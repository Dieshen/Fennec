use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, info};
use uuid::Uuid;

/// Cline-style memory files for preserving context and knowledge
/// This module provides a foundation for Milestone 3 implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFile {
    /// Unique identifier for this memory file
    pub id: Uuid,
    /// Human-readable name/title
    pub name: String,
    /// File content in markdown format
    pub content: String,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// When this file was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When this file was last modified
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Associated session IDs that contributed to this memory
    pub related_sessions: Vec<Uuid>,
    /// Memory file type
    pub file_type: MemoryFileType,
}

/// Types of memory files supported
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryFileType {
    /// Project-specific context and knowledge
    ProjectContext,
    /// Debugging patterns and solutions
    DebuggingPatterns,
    /// Code patterns and best practices
    CodePatterns,
    /// Architecture decisions and rationale
    Architecture,
    /// Learning notes and documentation
    Learning,
    /// General knowledge base entries
    Knowledge,
    /// Task templates and workflows
    Templates,
}

/// Service for managing Cline-style memory files
#[derive(Debug)]
pub struct MemoryFileService {
    /// Base directory for storing memory files
    storage_dir: PathBuf,
    /// In-memory cache of recently accessed files
    cache: HashMap<Uuid, MemoryFile>,
    /// Maximum cache size
    max_cache_size: usize,
}

impl MemoryFileService {
    /// Create a new memory file service
    pub fn new() -> Result<Self> {
        let storage_dir = Self::get_storage_dir()?;

        // Ensure storage directory exists
        std::fs::create_dir_all(&storage_dir).with_context(|| {
            format!(
                "Failed to create memory files directory: {}",
                storage_dir.display()
            )
        })?;

        Ok(Self {
            storage_dir,
            cache: HashMap::new(),
            max_cache_size: 50,
        })
    }

    /// Get the storage directory for memory files
    fn get_storage_dir() -> Result<PathBuf> {
        let proj_dirs =
            ProjectDirs::from("", "", "fennec").context("Failed to get project directories")?;

        Ok(proj_dirs.data_dir().join("memory_files"))
    }

    /// Create a new memory file
    pub async fn create_memory_file(
        &mut self,
        name: String,
        content: String,
        file_type: MemoryFileType,
        tags: Vec<String>,
    ) -> Result<Uuid> {
        let now = chrono::Utc::now();
        let id = Uuid::new_v4();

        let memory_file = MemoryFile {
            id,
            name,
            content,
            tags,
            created_at: now,
            updated_at: now,
            related_sessions: Vec::new(),
            file_type,
        };

        self.save_memory_file(&memory_file).await?;
        let name = memory_file.name.clone();
        self.cache.insert(id, memory_file);

        info!("Created memory file: {} ({})", id, name);
        Ok(id)
    }

    /// Update an existing memory file
    pub async fn update_memory_file(&mut self, id: Uuid, content: String) -> Result<()> {
        let mut memory_file = self
            .load_memory_file(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Memory file not found: {}", id))?;

        memory_file.content = content;
        memory_file.updated_at = chrono::Utc::now();

        self.save_memory_file(&memory_file).await?;
        self.cache.insert(id, memory_file);

        info!("Updated memory file: {}", id);
        Ok(())
    }

    /// Load a memory file by ID
    pub async fn load_memory_file(&mut self, id: Uuid) -> Result<Option<MemoryFile>> {
        // Check cache first
        if let Some(file) = self.cache.get(&id) {
            debug!("Found memory file in cache: {}", id);
            return Ok(Some(file.clone()));
        }

        // Load from disk
        let file_path = self.get_file_path(id);
        if !file_path.exists() {
            return Ok(None);
        }

        let json = fs::read_to_string(&file_path)
            .await
            .with_context(|| format!("Failed to read memory file: {}", file_path.display()))?;

        let memory_file: MemoryFile = serde_json::from_str(&json).with_context(|| {
            format!("Failed to deserialize memory file: {}", file_path.display())
        })?;

        // Add to cache
        self.cache.insert(id, memory_file.clone());
        self.manage_cache_size();

        debug!("Loaded memory file from disk: {}", id);
        Ok(Some(memory_file))
    }

    /// List all memory files
    pub async fn list_memory_files(&self) -> Result<Vec<MemoryFileMetadata>> {
        let mut files = Vec::new();

        let mut dir = fs::read_dir(&self.storage_dir).await.with_context(|| {
            format!(
                "Failed to read memory files directory: {}",
                self.storage_dir.display()
            )
        })?;

        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(file_id_str) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(_file_id) = Uuid::parse_str(file_id_str) {
                        // Read just the metadata (not full content for efficiency)
                        if let Ok(json) = fs::read_to_string(&path).await {
                            if let Ok(memory_file) = serde_json::from_str::<MemoryFile>(&json) {
                                files.push(MemoryFileMetadata {
                                    id: memory_file.id,
                                    name: memory_file.name,
                                    file_type: memory_file.file_type,
                                    tags: memory_file.tags,
                                    created_at: memory_file.created_at,
                                    updated_at: memory_file.updated_at,
                                    content_length: memory_file.content.len(),
                                    related_sessions_count: memory_file.related_sessions.len(),
                                });
                            }
                        }
                    }
                }
            }
        }

        // Sort by updated_at (most recent first)
        files.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(files)
    }

    /// Search memory files by content, name, or tags
    pub async fn search_memory_files(
        &mut self,
        query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<MemoryFileSearchResult>> {
        let mut results = Vec::new();
        use fuzzy_matcher::FuzzyMatcher;
        let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();

        let files = self.list_memory_files().await?;

        for file_meta in files {
            let mut best_score = 0i64;
            let mut match_location = MatchLocation::None;

            // Load full file for content search
            if let Ok(Some(memory_file)) = self.load_memory_file(file_meta.id).await {
                // Search in name
                if let Some(score) = matcher.fuzzy_match(&memory_file.name, query) {
                    if score > best_score {
                        best_score = score;
                        match_location = MatchLocation::Name;
                    }
                }

                // Search in content
                if let Some(score) = matcher.fuzzy_match(&memory_file.content, query) {
                    if score > best_score {
                        best_score = score;
                        match_location = MatchLocation::Content;
                    }
                }

                // Search in tags
                for tag in &memory_file.tags {
                    if let Some(score) = matcher.fuzzy_match(tag, query) {
                        if score > best_score {
                            best_score = score;
                            match_location = MatchLocation::Tags;
                        }
                    }
                }

                if best_score > 0 {
                    results.push(MemoryFileSearchResult {
                        id: memory_file.id,
                        name: memory_file.name,
                        file_type: memory_file.file_type,
                        score: best_score,
                        match_location,
                        content_preview: self.generate_content_preview(&memory_file.content, query),
                        updated_at: memory_file.updated_at,
                    });
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

    /// Delete a memory file
    pub async fn delete_memory_file(&mut self, id: Uuid) -> Result<()> {
        // Remove from cache
        self.cache.remove(&id);

        // Remove from disk
        let file_path = self.get_file_path(id);
        if file_path.exists() {
            fs::remove_file(&file_path).await.with_context(|| {
                format!("Failed to delete memory file: {}", file_path.display())
            })?;
            info!("Deleted memory file: {}", id);
        }

        Ok(())
    }

    /// Associate a memory file with a session
    pub async fn associate_with_session(&mut self, file_id: Uuid, session_id: Uuid) -> Result<()> {
        let mut memory_file = self
            .load_memory_file(file_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Memory file not found: {}", file_id))?;

        if !memory_file.related_sessions.contains(&session_id) {
            memory_file.related_sessions.push(session_id);
            memory_file.updated_at = chrono::Utc::now();

            self.save_memory_file(&memory_file).await?;
            self.cache.insert(file_id, memory_file);
        }

        Ok(())
    }

    /// Get memory files associated with a session
    pub async fn get_session_memory_files(
        &mut self,
        session_id: Uuid,
    ) -> Result<Vec<MemoryFileMetadata>> {
        let all_files = self.list_memory_files().await?;

        let mut session_files = Vec::new();
        for file_meta in all_files {
            if let Ok(Some(memory_file)) = self.load_memory_file(file_meta.id).await {
                if memory_file.related_sessions.contains(&session_id) {
                    session_files.push(file_meta);
                }
            }
        }

        Ok(session_files)
    }

    /// Generate a content preview around search matches
    fn generate_content_preview(&self, content: &str, query: &str) -> String {
        // Find the first occurrence of the query (case insensitive)
        let content_lower = content.to_lowercase();
        let query_lower = query.to_lowercase();

        if let Some(pos) = content_lower.find(&query_lower) {
            // Extract context around the match
            let start = pos.saturating_sub(100);
            let end = std::cmp::min(pos + query.len() + 100, content.len());

            let mut preview = content[start..end].to_string();

            // Add ellipsis if we truncated
            if start > 0 {
                preview = format!("...{}", preview);
            }
            if end < content.len() {
                preview = format!("{}...", preview);
            }

            preview
        } else {
            // Just return the first 200 characters
            if content.len() > 200 {
                format!("{}...", &content[..200])
            } else {
                content.to_string()
            }
        }
    }

    /// Get the file path for a memory file
    fn get_file_path(&self, id: Uuid) -> PathBuf {
        self.storage_dir.join(format!("{}.json", id))
    }

    /// Save a memory file to disk
    async fn save_memory_file(&self, memory_file: &MemoryFile) -> Result<()> {
        let file_path = self.get_file_path(memory_file.id);
        let json =
            serde_json::to_string_pretty(memory_file).context("Failed to serialize memory file")?;

        fs::write(&file_path, json)
            .await
            .with_context(|| format!("Failed to write memory file: {}", file_path.display()))?;

        Ok(())
    }

    /// Manage cache size by removing oldest entries
    fn manage_cache_size(&mut self) {
        while self.cache.len() > self.max_cache_size {
            if let Some((oldest_id, _)) = self
                .cache
                .iter()
                .min_by_key(|(_, file)| file.updated_at)
                .map(|(id, file)| (*id, file.clone()))
            {
                self.cache.remove(&oldest_id);
                debug!("Evicted memory file from cache: {}", oldest_id);
            } else {
                break;
            }
        }
    }
}

/// Metadata about a memory file (without full content)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFileMetadata {
    pub id: Uuid,
    pub name: String,
    pub file_type: MemoryFileType,
    pub tags: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub content_length: usize,
    pub related_sessions_count: usize,
}

/// Result of searching memory files
#[derive(Debug, Clone)]
pub struct MemoryFileSearchResult {
    pub id: Uuid,
    pub name: String,
    pub file_type: MemoryFileType,
    pub score: i64,
    pub match_location: MatchLocation,
    pub content_preview: String,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Where a search match was found
#[derive(Debug, Clone)]
pub enum MatchLocation {
    Name,
    Content,
    Tags,
    None,
}

impl Default for MemoryFileService {
    fn default() -> Self {
        Self::new().expect("Failed to create default MemoryFileService")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_and_load_memory_file() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_owned();

        let mut service = MemoryFileService {
            storage_dir,
            cache: HashMap::new(),
            max_cache_size: 50,
        };

        let file_id = service
            .create_memory_file(
                "Test Memory".to_string(),
                "This is test content".to_string(),
                MemoryFileType::Knowledge,
                vec!["test".to_string()],
            )
            .await
            .unwrap();

        let loaded = service.load_memory_file(file_id).await.unwrap().unwrap();
        assert_eq!(loaded.name, "Test Memory");
        assert_eq!(loaded.content, "This is test content");
        assert_eq!(loaded.tags, vec!["test"]);
    }

    #[tokio::test]
    async fn test_search_memory_files() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_owned();

        let mut service = MemoryFileService {
            storage_dir,
            cache: HashMap::new(),
            max_cache_size: 50,
        };

        // Create a test file
        service
            .create_memory_file(
                "Rust Programming".to_string(),
                "Rust is a systems programming language".to_string(),
                MemoryFileType::Learning,
                vec!["rust".to_string(), "programming".to_string()],
            )
            .await
            .unwrap();

        // Search for it
        let results = service.search_memory_files("rust", Some(10)).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Rust Programming");
    }
}
