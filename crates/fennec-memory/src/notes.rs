use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, info};
use uuid::Uuid;

/// User-provided note with categorization and cross-referencing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserNote {
    /// Unique identifier for this note
    pub id: Uuid,
    /// Session this note belongs to (optional for global notes)
    pub session_id: Option<Uuid>,
    /// Human-readable title for the note
    pub title: String,
    /// Note content in markdown format
    pub content: String,
    /// Category for organization
    pub category: NoteCategory,
    /// Tags for flexible categorization
    pub tags: Vec<String>,
    /// When this note was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When this note was last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Plans this note is linked to
    pub linked_plans: Vec<Uuid>,
    /// Commands this note references
    pub linked_commands: Vec<String>,
    /// Cross-references to other notes
    pub cross_references: Vec<Uuid>,
    /// Priority or importance level
    pub priority: NotePriority,
    /// Whether this note is pinned for quick access
    pub is_pinned: bool,
    /// Reminder date for follow-up (optional)
    pub reminder_date: Option<chrono::DateTime<chrono::Utc>>,
    /// Color coding for visual organization
    pub color: Option<String>,
}

/// Categories for organizing notes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NoteCategory {
    /// Insights discovered during work
    Insight,
    /// Decisions made and their rationale
    Decision,
    /// Reminders for future actions
    Reminder,
    /// Learning notes and documentation
    Learning,
    /// Issues and problems encountered
    Issue,
    /// Solutions and workarounds found
    Solution,
    /// References to external resources
    Reference,
    /// Meeting notes and discussions
    Meeting,
    /// Project-specific notes
    Project,
    /// General notes that don't fit other categories
    General,
}

/// Priority levels for notes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum NotePriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Storage service for managing user notes
#[derive(Debug)]
pub struct NotesStore {
    /// Base directory for storing notes
    storage_dir: PathBuf,
    /// In-memory cache of recent notes
    cache: HashMap<Uuid, UserNote>,
    /// Maximum cache size
    max_cache_size: usize,
}

impl NotesStore {
    /// Create a new notes store
    pub fn new() -> Result<Self> {
        let storage_dir = Self::get_storage_dir()?;

        // Ensure storage directory exists
        std::fs::create_dir_all(&storage_dir).with_context(|| {
            format!(
                "Failed to create notes directory: {}",
                storage_dir.display()
            )
        })?;

        Ok(Self {
            storage_dir,
            cache: HashMap::new(),
            max_cache_size: 200,
        })
    }

    /// Get the storage directory for notes
    fn get_storage_dir() -> Result<PathBuf> {
        let proj_dirs =
            ProjectDirs::from("", "", "fennec").context("Failed to get project directories")?;

        Ok(proj_dirs.data_dir().join("notes"))
    }

    /// Create a new note
    pub async fn create_note(
        &mut self,
        session_id: Option<Uuid>,
        title: String,
        content: String,
        category: NoteCategory,
    ) -> Result<Uuid> {
        let now = chrono::Utc::now();
        let note_id = Uuid::new_v4();

        let note = UserNote {
            id: note_id,
            session_id,
            title: title.clone(),
            content,
            category,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
            linked_plans: Vec::new(),
            linked_commands: Vec::new(),
            cross_references: Vec::new(),
            priority: NotePriority::Medium,
            is_pinned: false,
            reminder_date: None,
            color: None,
        };

        self.store_note(&note).await?;
        self.cache.insert(note_id, note);

        info!("Created note: {} ({})", note_id, title);
        Ok(note_id)
    }

    /// Load a note by ID
    pub async fn load_note(&mut self, note_id: Uuid) -> Result<Option<UserNote>> {
        // Check cache first
        if let Some(note) = self.cache.get(&note_id) {
            debug!("Found note in cache: {}", note_id);
            return Ok(Some(note.clone()));
        }

        // Load from disk
        let file_path = self.get_note_path(note_id);
        if !file_path.exists() {
            return Ok(None);
        }

        let json = fs::read_to_string(&file_path)
            .await
            .with_context(|| format!("Failed to read note from: {}", file_path.display()))?;

        let note: UserNote = serde_json::from_str(&json)
            .with_context(|| format!("Failed to deserialize note from: {}", file_path.display()))?;

        // Add to cache
        self.cache.insert(note_id, note.clone());
        self.manage_cache_size();

        debug!("Loaded note from disk: {}", note_id);
        Ok(Some(note))
    }

    /// Update an existing note
    pub async fn update_note(&mut self, note: UserNote) -> Result<()> {
        let note_id = note.id;
        let mut updated_note = note;
        updated_note.updated_at = chrono::Utc::now();

        self.store_note(&updated_note).await?;
        self.cache.insert(updated_note.id, updated_note);

        debug!("Updated note: {}", note_id);
        Ok(())
    }

    /// Update note content
    pub async fn update_note_content(&mut self, note_id: Uuid, content: String) -> Result<()> {
        let mut note = self
            .load_note(note_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Note not found: {}", note_id))?;

        note.content = content;
        self.update_note(note).await
    }

    /// Add tags to a note
    pub async fn add_tags(&mut self, note_id: Uuid, tags: Vec<String>) -> Result<()> {
        let mut note = self
            .load_note(note_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Note not found: {}", note_id))?;

        for tag in tags {
            if !note.tags.contains(&tag) {
                note.tags.push(tag);
            }
        }

        self.update_note(note).await
    }

    /// Remove tags from a note
    pub async fn remove_tags(&mut self, note_id: Uuid, tags: Vec<String>) -> Result<()> {
        let mut note = self
            .load_note(note_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Note not found: {}", note_id))?;

        note.tags.retain(|tag| !tags.contains(tag));
        self.update_note(note).await
    }

    /// Link a note to a plan
    pub async fn link_to_plan(&mut self, note_id: Uuid, plan_id: Uuid) -> Result<()> {
        let mut note = self
            .load_note(note_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Note not found: {}", note_id))?;

        if !note.linked_plans.contains(&plan_id) {
            note.linked_plans.push(plan_id);
            self.update_note(note).await?;
        }

        Ok(())
    }

    /// Link a note to a command
    pub async fn link_to_command(&mut self, note_id: Uuid, command: String) -> Result<()> {
        let mut note = self
            .load_note(note_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Note not found: {}", note_id))?;

        if !note.linked_commands.contains(&command) {
            note.linked_commands.push(command);
            self.update_note(note).await?;
        }

        Ok(())
    }

    /// Add cross-reference between notes
    pub async fn add_cross_reference(&mut self, note_id: Uuid, reference_id: Uuid) -> Result<()> {
        // Add reference from first note to second
        {
            let mut note = self
                .load_note(note_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Note not found: {}", note_id))?;

            if !note.cross_references.contains(&reference_id) {
                note.cross_references.push(reference_id);
                self.update_note(note).await?;
            }
        }

        // Add bidirectional reference (from second note to first)
        {
            let mut reference_note = self
                .load_note(reference_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Reference note not found: {}", reference_id))?;

            if !reference_note.cross_references.contains(&note_id) {
                reference_note.cross_references.push(note_id);
                self.update_note(reference_note).await?;
            }
        }

        Ok(())
    }

    /// Remove cross-reference between notes
    pub async fn remove_cross_reference(
        &mut self,
        note_id: Uuid,
        reference_id: Uuid,
    ) -> Result<()> {
        // Remove reference from first note
        {
            let mut note = self
                .load_note(note_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Note not found: {}", note_id))?;

            note.cross_references.retain(|&id| id != reference_id);
            self.update_note(note).await?;
        }

        // Remove bidirectional reference
        if let Ok(Some(mut reference_note)) = self.load_note(reference_id).await {
            reference_note.cross_references.retain(|&id| id != note_id);
            self.update_note(reference_note).await?;
        }

        Ok(())
    }

    /// Set note priority
    pub async fn set_priority(&mut self, note_id: Uuid, priority: NotePriority) -> Result<()> {
        let mut note = self
            .load_note(note_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Note not found: {}", note_id))?;

        note.priority = priority;
        self.update_note(note).await
    }

    /// Pin or unpin a note
    pub async fn set_pinned(&mut self, note_id: Uuid, is_pinned: bool) -> Result<()> {
        let mut note = self
            .load_note(note_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Note not found: {}", note_id))?;

        note.is_pinned = is_pinned;
        self.update_note(note).await
    }

    /// Set reminder date for a note
    pub async fn set_reminder(
        &mut self,
        note_id: Uuid,
        reminder_date: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<()> {
        let mut note = self
            .load_note(note_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Note not found: {}", note_id))?;

        note.reminder_date = reminder_date;
        self.update_note(note).await
    }

    /// List all notes for a session
    pub async fn list_session_notes(&mut self, session_id: Uuid) -> Result<Vec<NoteMetadata>> {
        let mut notes = Vec::new();

        let mut dir = fs::read_dir(&self.storage_dir).await.with_context(|| {
            format!(
                "Failed to read notes directory: {}",
                self.storage_dir.display()
            )
        })?;

        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(note_id_str) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(note_id) = Uuid::parse_str(note_id_str) {
                        if let Ok(Some(note)) = self.load_note(note_id).await {
                            if note.session_id == Some(session_id) {
                                notes.push(NoteMetadata::from_note(&note));
                            }
                        }
                    }
                }
            }
        }

        // Sort by updated time (most recent first)
        notes.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(notes)
    }

    /// List notes by category
    pub async fn list_notes_by_category(
        &mut self,
        category: NoteCategory,
    ) -> Result<Vec<NoteMetadata>> {
        let mut notes = Vec::new();

        let mut dir = fs::read_dir(&self.storage_dir).await.with_context(|| {
            format!(
                "Failed to read notes directory: {}",
                self.storage_dir.display()
            )
        })?;

        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(note_id_str) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(note_id) = Uuid::parse_str(note_id_str) {
                        if let Ok(Some(note)) = self.load_note(note_id).await {
                            if note.category == category {
                                notes.push(NoteMetadata::from_note(&note));
                            }
                        }
                    }
                }
            }
        }

        // Sort by updated time (most recent first)
        notes.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(notes)
    }

    /// List pinned notes
    pub async fn list_pinned_notes(&mut self) -> Result<Vec<NoteMetadata>> {
        let mut notes = Vec::new();

        let mut dir = fs::read_dir(&self.storage_dir).await.with_context(|| {
            format!(
                "Failed to read notes directory: {}",
                self.storage_dir.display()
            )
        })?;

        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(note_id_str) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(note_id) = Uuid::parse_str(note_id_str) {
                        if let Ok(Some(note)) = self.load_note(note_id).await {
                            if note.is_pinned {
                                notes.push(NoteMetadata::from_note(&note));
                            }
                        }
                    }
                }
            }
        }

        // Sort by priority (highest first), then by updated time
        notes.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then(b.updated_at.cmp(&a.updated_at))
        });
        Ok(notes)
    }

    /// Get notes with upcoming reminders
    pub async fn get_upcoming_reminders(&mut self, within_hours: u32) -> Result<Vec<NoteMetadata>> {
        let mut notes = Vec::new();
        let cutoff_time = chrono::Utc::now() + chrono::Duration::hours(within_hours as i64);

        let mut dir = fs::read_dir(&self.storage_dir).await.with_context(|| {
            format!(
                "Failed to read notes directory: {}",
                self.storage_dir.display()
            )
        })?;

        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(note_id_str) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(note_id) = Uuid::parse_str(note_id_str) {
                        if let Ok(Some(note)) = self.load_note(note_id).await {
                            if let Some(reminder_date) = note.reminder_date {
                                if reminder_date <= cutoff_time {
                                    notes.push(NoteMetadata::from_note(&note));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Sort by reminder date (earliest first)
        notes.sort_by(|a, b| a.reminder_date.cmp(&b.reminder_date));
        Ok(notes)
    }

    /// Search notes by title, content, or tags
    pub async fn search_notes(
        &mut self,
        query: &str,
        filters: NoteSearchFilters,
    ) -> Result<Vec<NoteSearchResult>> {
        let mut results = Vec::new();
        use fuzzy_matcher::FuzzyMatcher;
        let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();

        let mut dir = fs::read_dir(&self.storage_dir).await.with_context(|| {
            format!(
                "Failed to read notes directory: {}",
                self.storage_dir.display()
            )
        })?;

        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(note_id_str) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(note_id) = Uuid::parse_str(note_id_str) {
                        if let Ok(Some(note)) = self.load_note(note_id).await {
                            // Apply filters
                            if let Some(ref category_filter) = filters.category {
                                if note.category != *category_filter {
                                    continue;
                                }
                            }

                            if let Some(ref session_filter) = filters.session_id {
                                if note.session_id != Some(*session_filter) {
                                    continue;
                                }
                            }

                            if filters.pinned_only && !note.is_pinned {
                                continue;
                            }

                            if let Some(ref tag_filter) = filters.tags {
                                if !tag_filter.iter().any(|tag| note.tags.contains(tag)) {
                                    continue;
                                }
                            }

                            let mut best_score = 0i64;
                            let mut match_location = NoteMatchLocation::None;

                            // Search in title
                            if let Some(score) = matcher.fuzzy_match(&note.title, query) {
                                if score > best_score {
                                    best_score = score;
                                    match_location = NoteMatchLocation::Title;
                                }
                            }

                            // Search in content
                            if let Some(score) = matcher.fuzzy_match(&note.content, query) {
                                if score > best_score {
                                    best_score = score;
                                    match_location = NoteMatchLocation::Content;
                                }
                            }

                            // Search in tags
                            for tag in &note.tags {
                                if let Some(score) = matcher.fuzzy_match(tag, query) {
                                    if score > best_score {
                                        best_score = score;
                                        match_location = NoteMatchLocation::Tags;
                                    }
                                }
                            }

                            if best_score > 0 {
                                results.push(NoteSearchResult {
                                    note_id: note.id,
                                    session_id: note.session_id,
                                    title: note.title,
                                    category: note.category,
                                    priority: note.priority,
                                    is_pinned: note.is_pinned,
                                    score: best_score,
                                    match_location,
                                    content_preview: self
                                        .generate_content_preview(&note.content, query),
                                    created_at: note.created_at,
                                    updated_at: note.updated_at,
                                    tags: note.tags,
                                });
                            }
                        }
                    }
                }
            }
        }

        // Sort by score (highest first), then by priority
        results.sort_by(|a, b| b.score.cmp(&a.score).then(b.priority.cmp(&a.priority)));

        // Apply limit if specified
        if let Some(limit) = filters.limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    /// Delete a note
    pub async fn delete_note(&mut self, note_id: Uuid) -> Result<()> {
        // Remove cross-references from other notes
        if let Ok(Some(note)) = self.load_note(note_id).await {
            for &ref_id in &note.cross_references {
                if let Ok(Some(mut ref_note)) = self.load_note(ref_id).await {
                    ref_note.cross_references.retain(|&id| id != note_id);
                    self.update_note(ref_note).await?;
                }
            }
        }

        // Remove from cache
        self.cache.remove(&note_id);

        // Remove from disk
        let file_path = self.get_note_path(note_id);
        if file_path.exists() {
            fs::remove_file(&file_path)
                .await
                .with_context(|| format!("Failed to delete note file: {}", file_path.display()))?;
            info!("Deleted note: {}", note_id);
        }

        Ok(())
    }

    /// Generate a content preview around search matches
    fn generate_content_preview(&self, content: &str, query: &str) -> String {
        let content_lower = content.to_lowercase();
        let query_lower = query.to_lowercase();

        if let Some(pos) = content_lower.find(&query_lower) {
            let start = pos.saturating_sub(100);
            let end = std::cmp::min(pos + query.len() + 100, content.len());

            let mut preview = content[start..end].to_string();

            if start > 0 {
                preview = format!("...{}", preview);
            }
            if end < content.len() {
                preview = format!("{}...", preview);
            }

            preview
        } else {
            if content.len() > 200 {
                format!("{}...", &content[..200])
            } else {
                content.to_string()
            }
        }
    }

    /// Get the file path for a note
    fn get_note_path(&self, note_id: Uuid) -> PathBuf {
        self.storage_dir.join(format!("{}.json", note_id))
    }

    /// Store a note to disk
    async fn store_note(&self, note: &UserNote) -> Result<()> {
        let file_path = self.get_note_path(note.id);
        let json = serde_json::to_string_pretty(note).context("Failed to serialize note")?;

        fs::write(&file_path, json)
            .await
            .with_context(|| format!("Failed to write note to: {}", file_path.display()))?;

        Ok(())
    }

    /// Manage cache size by removing oldest entries
    fn manage_cache_size(&mut self) {
        while self.cache.len() > self.max_cache_size {
            if let Some((oldest_id, _)) = self
                .cache
                .iter()
                .min_by_key(|(_, note)| note.updated_at)
                .map(|(id, note)| (*id, note.clone()))
            {
                self.cache.remove(&oldest_id);
                debug!("Evicted note from cache: {}", oldest_id);
            } else {
                break;
            }
        }
    }
}

/// Metadata about a note (without full content)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteMetadata {
    pub id: Uuid,
    pub session_id: Option<Uuid>,
    pub title: String,
    pub category: NoteCategory,
    pub tags: Vec<String>,
    pub priority: NotePriority,
    pub is_pinned: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub reminder_date: Option<chrono::DateTime<chrono::Utc>>,
    pub content_length: usize,
    pub linked_plans_count: usize,
    pub linked_commands_count: usize,
    pub cross_references_count: usize,
}

impl NoteMetadata {
    fn from_note(note: &UserNote) -> Self {
        Self {
            id: note.id,
            session_id: note.session_id,
            title: note.title.clone(),
            category: note.category.clone(),
            tags: note.tags.clone(),
            priority: note.priority.clone(),
            is_pinned: note.is_pinned,
            created_at: note.created_at,
            updated_at: note.updated_at,
            reminder_date: note.reminder_date,
            content_length: note.content.len(),
            linked_plans_count: note.linked_plans.len(),
            linked_commands_count: note.linked_commands.len(),
            cross_references_count: note.cross_references.len(),
        }
    }
}

/// Result of searching notes
#[derive(Debug, Clone)]
pub struct NoteSearchResult {
    pub note_id: Uuid,
    pub session_id: Option<Uuid>,
    pub title: String,
    pub category: NoteCategory,
    pub priority: NotePriority,
    pub is_pinned: bool,
    pub score: i64,
    pub match_location: NoteMatchLocation,
    pub content_preview: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub tags: Vec<String>,
}

/// Where a search match was found in a note
#[derive(Debug, Clone)]
pub enum NoteMatchLocation {
    Title,
    Content,
    Tags,
    None,
}

/// Filters for note search
#[derive(Debug, Clone, Default)]
pub struct NoteSearchFilters {
    pub category: Option<NoteCategory>,
    pub session_id: Option<Uuid>,
    pub tags: Option<Vec<String>>,
    pub pinned_only: bool,
    pub limit: Option<usize>,
}

impl Default for NotesStore {
    fn default() -> Self {
        Self::new().expect("Failed to create default NotesStore")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_and_load_note() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_owned();

        let mut store = NotesStore {
            storage_dir,
            cache: HashMap::new(),
            max_cache_size: 200,
        };

        let session_id = Uuid::new_v4();
        let note_id = store
            .create_note(
                Some(session_id),
                "Test Note".to_string(),
                "This is a test note".to_string(),
                NoteCategory::General,
            )
            .await
            .unwrap();

        let loaded = store.load_note(note_id).await.unwrap().unwrap();
        assert_eq!(loaded.title, "Test Note");
        assert_eq!(loaded.session_id, Some(session_id));
        assert_eq!(loaded.category, NoteCategory::General);
    }

    #[tokio::test]
    async fn test_add_tags_to_note() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_owned();

        let mut store = NotesStore {
            storage_dir,
            cache: HashMap::new(),
            max_cache_size: 200,
        };

        let note_id = store
            .create_note(
                None,
                "Test Note".to_string(),
                "Content".to_string(),
                NoteCategory::Learning,
            )
            .await
            .unwrap();

        store
            .add_tags(note_id, vec!["rust".to_string(), "programming".to_string()])
            .await
            .unwrap();

        let note = store.load_note(note_id).await.unwrap().unwrap();
        assert_eq!(note.tags.len(), 2);
        assert!(note.tags.contains(&"rust".to_string()));
        assert!(note.tags.contains(&"programming".to_string()));
    }

    #[tokio::test]
    async fn test_cross_references() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_owned();

        let mut store = NotesStore {
            storage_dir,
            cache: HashMap::new(),
            max_cache_size: 200,
        };

        let note1_id = store
            .create_note(
                None,
                "Note 1".to_string(),
                "First note".to_string(),
                NoteCategory::General,
            )
            .await
            .unwrap();

        let note2_id = store
            .create_note(
                None,
                "Note 2".to_string(),
                "Second note".to_string(),
                NoteCategory::General,
            )
            .await
            .unwrap();

        store.add_cross_reference(note1_id, note2_id).await.unwrap();

        let note1 = store.load_note(note1_id).await.unwrap().unwrap();
        let note2 = store.load_note(note2_id).await.unwrap().unwrap();

        assert!(note1.cross_references.contains(&note2_id));
        assert!(note2.cross_references.contains(&note1_id));
    }

    #[tokio::test]
    async fn test_search_notes() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_owned();

        let mut store = NotesStore {
            storage_dir,
            cache: HashMap::new(),
            max_cache_size: 200,
        };

        store
            .create_note(
                None,
                "Rust Learning".to_string(),
                "Notes about Rust programming language".to_string(),
                NoteCategory::Learning,
            )
            .await
            .unwrap();

        let filters = NoteSearchFilters {
            category: Some(NoteCategory::Learning),
            ..Default::default()
        };

        let results = store.search_notes("rust", filters).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Rust Learning");
    }
}
