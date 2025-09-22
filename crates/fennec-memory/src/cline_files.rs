//! # Cline-style Memory Files
//!
//! This module implements Cline-style memory files for preserving structured project context
//! and knowledge. Unlike the generic memory files, these are project-scoped and follow
//! specific templates for consistent formatting.
//!
//! ## File Types
//!
//! - **projectbrief.md**: Project overview, goals, and current status
//! - **activeContext.md**: Current session context, active tasks, and immediate focus
//! - **progress.md**: Progress tracking, completed tasks, and session summaries
//!
//! ## Features
//!
//! - Template-based markdown generation with consistent formatting
//! - Automatic file updates based on session events
//! - Version tracking and change history for memory files
//! - File-backed persistence with optional in-memory caching
//! - Project lifecycle management (creation, updates, archiving)

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, info};
use uuid::Uuid;

use fennec_core::{session::Session, transcript::MessageRole};

/// Types of Cline-style memory files
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ClineFileType {
    /// Project overview, goals, and current status
    ProjectBrief,
    /// Current session context, active tasks, and immediate focus
    ActiveContext,
    /// Progress tracking, completed tasks, and session summaries
    Progress,
    /// Custom file type for extensibility
    Custom(String),
}

impl ClineFileType {
    /// Get the filename for this file type
    pub fn filename(&self) -> String {
        match self {
            ClineFileType::ProjectBrief => "projectbrief.md".to_string(),
            ClineFileType::ActiveContext => "activeContext.md".to_string(),
            ClineFileType::Progress => "progress.md".to_string(),
            ClineFileType::Custom(name) => format!("{}.md", name),
        }
    }

    /// Get the metadata filename for this file type
    pub fn metadata_filename(&self) -> String {
        match self {
            ClineFileType::ProjectBrief => "projectbrief.meta.json".to_string(),
            ClineFileType::ActiveContext => "activeContext.meta.json".to_string(),
            ClineFileType::Progress => "progress.meta.json".to_string(),
            ClineFileType::Custom(name) => format!("{}.meta.json", name),
        }
    }
}

/// A Cline-style memory file with structured content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClineMemoryFile {
    /// File type
    pub file_type: ClineFileType,
    /// Project this file belongs to
    pub project_id: Uuid,
    /// Structured content
    pub content: ClineFileContent,
    /// File metadata including version tracking
    pub metadata: ClineFileMetadata,
    /// Current version number
    pub version: u32,
    /// Relative path from project directory
    pub file_path: PathBuf,
}

/// Metadata for a Cline memory file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClineFileMetadata {
    /// When this file was created
    pub created_at: DateTime<Utc>,
    /// When this file was last updated
    pub updated_at: DateTime<Utc>,
    /// Last session that modified this file
    pub last_session_id: Option<Uuid>,
    /// Version history for change tracking
    pub version_history: Vec<VersionEntry>,
}

/// Version history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionEntry {
    /// Version number
    pub version: u32,
    /// When this version was created
    pub timestamp: DateTime<Utc>,
    /// Session that created this version
    pub session_id: Option<Uuid>,
    /// Summary of changes made
    pub change_summary: String,
}

/// Content types for different Cline file types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClineFileContent {
    /// Project brief content
    ProjectBrief(ProjectBriefContent),
    /// Active context content
    ActiveContext(ActiveContextContent),
    /// Progress tracking content
    Progress(ProgressContent),
    /// Custom content for extensibility
    Custom { content: String },
}

/// Project status enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectStatus {
    /// Project is in planning phase
    Planning,
    /// Project is actively being worked on
    Active,
    /// Project is temporarily on hold
    OnHold,
    /// Project has been completed
    Completed,
    /// Project has been archived
    Archived,
}

impl Default for ProjectStatus {
    fn default() -> Self {
        ProjectStatus::Planning
    }
}

/// Content for project brief files
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectBriefContent {
    /// Unique project identifier
    pub project_id: Uuid,
    /// Current project status
    pub status: ProjectStatus,
    /// Primary technology/framework being used
    pub primary_technology: Option<String>,
    /// High-level project description
    pub overview: String,
    /// List of project goals
    pub goals: Vec<String>,
    /// Current status description
    pub current_status: String,
    /// Technologies and frameworks mentioned
    pub technologies: Vec<String>,
}

/// Content for active context files
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActiveContextContent {
    /// Current active session ID
    pub current_session_id: Option<Uuid>,
    /// Current focus area or topic
    pub focus_area: Option<String>,
    /// Current task being worked on
    pub current_task: Option<String>,
    /// Session context summary
    pub session_context: String,
    /// Recently discussed topics
    pub recent_topics: Vec<String>,
    /// Items requiring immediate attention
    pub immediate_focus: Vec<String>,
    /// Planned next steps
    pub next_steps: Vec<String>,
}

/// Content for progress tracking files
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProgressContent {
    /// Total number of sessions for this project
    pub total_sessions: usize,
    /// Number of completed tasks
    pub completed_tasks_count: usize,
    /// Recent session summaries
    pub recent_sessions: Vec<SessionSummary>,
    /// List of completed tasks
    pub completed_tasks: Vec<CompletedTask>,
    /// Project achievements and milestones
    pub achievements: Vec<Achievement>,
}

/// Summary of a completed session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    /// Session identifier
    pub session_id: Uuid,
    /// Session timestamp
    pub timestamp: DateTime<Utc>,
    /// Summary of what was accomplished
    pub summary: String,
    /// Topics discussed in the session
    pub topics: Vec<String>,
    /// Duration in minutes (if available)
    pub duration_minutes: Option<u32>,
}

/// A completed task record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedTask {
    /// Task description
    pub task: String,
    /// When the task was completed
    pub completed_at: DateTime<Utc>,
    /// Session where task was completed
    pub session_id: Option<Uuid>,
    /// Outcome or result of the task
    pub outcome: String,
}

/// Project achievement or milestone
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achievement {
    /// Achievement title
    pub title: String,
    /// Detailed description
    pub description: String,
    /// When achievement was reached
    pub achieved_at: DateTime<Utc>,
    /// Session where achievement occurred
    pub session_id: Option<Uuid>,
}

/// Events that trigger memory file updates
#[derive(Debug, Clone)]
pub enum MemoryEvent {
    /// Session started
    SessionStarted { project_id: Uuid, session: Session },
    /// Session ended
    SessionEnded {
        project_id: Uuid,
        session: Session,
        summary: Option<String>,
    },
    /// Message added to session
    MessageAdded {
        project_id: Uuid,
        session_id: Uuid,
        role: MessageRole,
        content: String,
    },
    /// Task was completed
    TaskCompleted {
        project_id: Uuid,
        session_id: Option<Uuid>,
        task: String,
        outcome: String,
    },
    /// Achievement reached
    AchievementReached {
        project_id: Uuid,
        session_id: Option<Uuid>,
        achievement: Achievement,
    },
    /// Project goals updated
    ProjectGoalUpdated {
        project_id: Uuid,
        goals: Vec<String>,
    },
    /// Project status changed
    ProjectStatusChanged {
        project_id: Uuid,
        status: ProjectStatus,
    },
}

/// Template engine for generating markdown from structured content
#[derive(Debug)]
pub struct TemplateEngine;

impl TemplateEngine {
    /// Create a new template engine
    pub fn new() -> Self {
        Self
    }

    /// Render a Cline memory file to markdown
    pub fn render_to_markdown(&self, file: &ClineMemoryFile) -> Result<String> {
        match &file.content {
            ClineFileContent::ProjectBrief(content) => {
                self.render_project_brief(content, &file.metadata)
            }
            ClineFileContent::ActiveContext(content) => {
                self.render_active_context(content, &file.metadata)
            }
            ClineFileContent::Progress(content) => self.render_progress(content, &file.metadata),
            ClineFileContent::Custom { content } => Ok(content.clone()),
        }
    }

    /// Render project brief content to markdown
    fn render_project_brief(
        &self,
        content: &ProjectBriefContent,
        metadata: &ClineFileMetadata,
    ) -> Result<String> {
        let status_str = match content.status {
            ProjectStatus::Planning => "Planning",
            ProjectStatus::Active => "Active",
            ProjectStatus::OnHold => "On Hold",
            ProjectStatus::Completed => "Completed",
            ProjectStatus::Archived => "Archived",
        };

        let primary_tech = content
            .primary_technology
            .as_deref()
            .unwrap_or("Not specified");

        let technologies_list = if content.technologies.is_empty() {
            "- None specified".to_string()
        } else {
            content
                .technologies
                .iter()
                .map(|tech| format!("- {}", tech))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let goals_list = if content.goals.is_empty() {
            "- No goals specified".to_string()
        } else {
            content
                .goals
                .iter()
                .map(|goal| format!("- {}", goal))
                .collect::<Vec<_>>()
                .join("\n")
        };

        Ok(format!(
            r#"# Project Brief

## Metadata
- **Project ID**: {}
- **Created**: {}
- **Last Updated**: {}
- **Status**: {}
- **Primary Technology**: {}

## Overview
{}

## Goals
{}

## Current Status
{}

## Key Technologies
{}

## Related Files
- [Active Context](./activeContext.md)
- [Progress Tracking](./progress.md)
"#,
            content.project_id,
            metadata.created_at.format("%Y-%m-%d %H:%M:%S UTC"),
            metadata.updated_at.format("%Y-%m-%d %H:%M:%S UTC"),
            status_str,
            primary_tech,
            if content.overview.is_empty() {
                "No overview provided."
            } else {
                &content.overview
            },
            goals_list,
            if content.current_status.is_empty() {
                "No current status provided."
            } else {
                &content.current_status
            },
            technologies_list
        ))
    }

    /// Render active context content to markdown
    fn render_active_context(
        &self,
        content: &ActiveContextContent,
        metadata: &ClineFileMetadata,
    ) -> Result<String> {
        let session_id = content
            .current_session_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "None".to_string());

        let focus_area = content.focus_area.as_deref().unwrap_or("Not specified");

        let current_task = content.current_task.as_deref().unwrap_or("No current task");

        let recent_topics = if content.recent_topics.is_empty() {
            "- No recent topics".to_string()
        } else {
            content
                .recent_topics
                .iter()
                .map(|topic| format!("- {}", topic))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let immediate_focus = if content.immediate_focus.is_empty() {
            "- No immediate focus items".to_string()
        } else {
            content
                .immediate_focus
                .iter()
                .map(|item| format!("- {}", item))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let next_steps = if content.next_steps.is_empty() {
            "- No next steps defined".to_string()
        } else {
            content
                .next_steps
                .iter()
                .map(|step| format!("- {}", step))
                .collect::<Vec<_>>()
                .join("\n")
        };

        Ok(format!(
            r#"# Active Context

## Metadata
- **Session ID**: {}
- **Last Updated**: {}
- **Focus Area**: {}

## Current Task
{}

## Active Session Context
{}

## Recent Topics
{}

## Immediate Focus
{}

## Next Steps
{}

## Related Files
- [Project Brief](./projectbrief.md)
- [Progress Tracking](./progress.md)
"#,
            session_id,
            metadata.updated_at.format("%Y-%m-%d %H:%M:%S UTC"),
            focus_area,
            current_task,
            if content.session_context.is_empty() {
                "No session context available."
            } else {
                &content.session_context
            },
            recent_topics,
            immediate_focus,
            next_steps
        ))
    }

    /// Render progress content to markdown
    fn render_progress(
        &self,
        content: &ProgressContent,
        metadata: &ClineFileMetadata,
    ) -> Result<String> {
        let recent_sessions = if content.recent_sessions.is_empty() {
            "No recent sessions recorded.".to_string()
        } else {
            content
                .recent_sessions
                .iter()
                .map(|session| {
                    let duration = session
                        .duration_minutes
                        .map(|d| format!(" ({}m)", d))
                        .unwrap_or_default();
                    format!(
                        "### {} - {}{}\n{}\n\n**Topics**: {}\n",
                        session.timestamp.format("%Y-%m-%d %H:%M"),
                        session.session_id,
                        duration,
                        session.summary,
                        session.topics.join(", ")
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        let completed_tasks = if content.completed_tasks.is_empty() {
            "No completed tasks recorded.".to_string()
        } else {
            content
                .completed_tasks
                .iter()
                .map(|task| {
                    format!(
                        "### {} - {}\n**Task**: {}\n**Outcome**: {}\n",
                        task.completed_at.format("%Y-%m-%d %H:%M"),
                        task.session_id
                            .map(|id| id.to_string())
                            .unwrap_or_else(|| "Manual".to_string()),
                        task.task,
                        task.outcome
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        let achievements = if content.achievements.is_empty() {
            "No achievements recorded.".to_string()
        } else {
            content
                .achievements
                .iter()
                .map(|achievement| {
                    format!(
                        "### {} - {}\n**{}**\n{}\n",
                        achievement.achieved_at.format("%Y-%m-%d %H:%M"),
                        achievement
                            .session_id
                            .map(|id| id.to_string())
                            .unwrap_or_else(|| "Manual".to_string()),
                        achievement.title,
                        achievement.description
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        Ok(format!(
            r#"# Progress Tracking

## Metadata
- **Last Updated**: {}
- **Total Sessions**: {}
- **Completed Tasks**: {}

## Recent Sessions
{}

## Completed Tasks
{}

## Achievements
{}

## Related Files
- [Project Brief](./projectbrief.md)
- [Active Context](./activeContext.md)
"#,
            metadata.updated_at.format("%Y-%m-%d %H:%M:%S UTC"),
            content.total_sessions,
            content.completed_tasks_count,
            recent_sessions,
            completed_tasks,
            achievements
        ))
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Service for managing Cline-style memory files
#[derive(Debug)]
pub struct ClineMemoryFileService {
    /// Base directory for storing all project memory files
    storage_dir: PathBuf,
    /// Template engine for rendering markdown
    template_engine: TemplateEngine,
    /// In-memory cache of recently accessed files
    cache: HashMap<(Uuid, ClineFileType), ClineMemoryFile>,
    /// Maximum cache size
    max_cache_size: usize,
}

impl ClineMemoryFileService {
    /// Create a new Cline memory file service
    pub fn new() -> Result<Self> {
        let storage_dir = Self::get_storage_dir()?;

        // Ensure storage directory exists
        std::fs::create_dir_all(&storage_dir).with_context(|| {
            format!(
                "Failed to create Cline memory files directory: {}",
                storage_dir.display()
            )
        })?;

        Ok(Self {
            storage_dir,
            template_engine: TemplateEngine::new(),
            cache: HashMap::new(),
            max_cache_size: 100, // Cache more files since they're structured
        })
    }

    /// Get the storage directory for Cline memory files
    fn get_storage_dir() -> Result<PathBuf> {
        let proj_dirs =
            ProjectDirs::from("", "", "fennec").context("Failed to get project directories")?;

        Ok(proj_dirs.data_dir().join("projects"))
    }

    /// Initialize memory files for a new project
    pub async fn initialize_project(&mut self, project_id: Uuid) -> Result<()> {
        let project_dir = self.get_project_directory(project_id);
        let meta_dir = project_dir.join(".meta");

        // Create project directories
        fs::create_dir_all(&project_dir).await.with_context(|| {
            format!(
                "Failed to create project directory: {}",
                project_dir.display()
            )
        })?;

        fs::create_dir_all(&meta_dir).await.with_context(|| {
            format!(
                "Failed to create project metadata directory: {}",
                meta_dir.display()
            )
        })?;

        // Initialize all three core file types
        self.create_initial_file(project_id, ClineFileType::ProjectBrief)
            .await?;
        self.create_initial_file(project_id, ClineFileType::ActiveContext)
            .await?;
        self.create_initial_file(project_id, ClineFileType::Progress)
            .await?;

        info!("Initialized Cline memory files for project: {}", project_id);
        Ok(())
    }

    /// Get the directory path for a project
    pub fn get_project_directory(&self, project_id: Uuid) -> PathBuf {
        self.storage_dir.join(project_id.to_string())
    }

    /// Get a memory file, loading from disk if not in cache
    pub async fn get_file(
        &mut self,
        project_id: Uuid,
        file_type: ClineFileType,
    ) -> Result<Option<ClineMemoryFile>> {
        let cache_key = (project_id, file_type.clone());

        // Check cache first
        if let Some(file) = self.cache.get(&cache_key) {
            debug!(
                "Found Cline memory file in cache: {} - {}",
                project_id,
                file_type.filename()
            );
            return Ok(Some(file.clone()));
        }

        // Load from disk
        let project_dir = self.get_project_directory(project_id);
        let file_path = project_dir.join(file_type.filename());
        let meta_path = project_dir
            .join(".meta")
            .join(file_type.metadata_filename());

        if !file_path.exists() || !meta_path.exists() {
            return Ok(None);
        }

        // Load metadata
        let meta_json = fs::read_to_string(&meta_path)
            .await
            .with_context(|| format!("Failed to read metadata file: {}", meta_path.display()))?;

        let metadata: ClineFileMetadata = serde_json::from_str(&meta_json)
            .with_context(|| format!("Failed to deserialize metadata: {}", meta_path.display()))?;

        // Load content based on file type
        let content = self.load_file_content(project_id, &file_type).await?;

        let memory_file = ClineMemoryFile {
            file_type: file_type.clone(),
            project_id,
            content,
            metadata,
            version: 1, // Will be updated from metadata
            file_path: file_type.filename().into(),
        };

        // Add to cache
        self.cache.insert(cache_key, memory_file.clone());
        self.manage_cache_size();

        debug!(
            "Loaded Cline memory file from disk: {} - {}",
            project_id,
            file_type.filename()
        );
        Ok(Some(memory_file))
    }

    /// Update a memory file with new content
    pub async fn update_file(
        &mut self,
        project_id: Uuid,
        file_type: ClineFileType,
        content: ClineFileContent,
        session_id: Option<Uuid>,
        change_summary: String,
    ) -> Result<()> {
        let mut file = self
            .get_file(project_id, file_type.clone())
            .await?
            .unwrap_or_else(|| {
                // Create new file if it doesn't exist
                let now = Utc::now();
                ClineMemoryFile {
                    file_type: file_type.clone(),
                    project_id,
                    content: content.clone(),
                    metadata: ClineFileMetadata {
                        created_at: now,
                        updated_at: now,
                        last_session_id: session_id,
                        version_history: Vec::new(),
                    },
                    version: 1,
                    file_path: file_type.filename().into(),
                }
            });

        // Update content and metadata
        file.content = content;
        file.metadata.updated_at = Utc::now();
        file.metadata.last_session_id = session_id;
        file.version += 1;

        // Add version history entry
        file.metadata.version_history.push(VersionEntry {
            version: file.version,
            timestamp: file.metadata.updated_at,
            session_id,
            change_summary,
        });

        // Keep only last 10 version entries to prevent unbounded growth
        if file.metadata.version_history.len() > 10 {
            file.metadata.version_history.remove(0);
        }

        // Save to disk
        self.save_file(&file).await?;

        // Update cache
        let cache_key = (project_id, file_type.clone());
        self.cache.insert(cache_key, file);

        debug!(
            "Updated Cline memory file: {} - {}",
            project_id,
            file_type.filename()
        );
        Ok(())
    }

    /// Render a memory file to markdown
    pub async fn render_to_markdown(
        &mut self,
        project_id: Uuid,
        file_type: ClineFileType,
    ) -> Result<Option<String>> {
        if let Some(file) = self.get_file(project_id, file_type).await? {
            let markdown = self.template_engine.render_to_markdown(&file)?;
            Ok(Some(markdown))
        } else {
            Ok(None)
        }
    }

    /// Handle memory events to automatically update files
    pub async fn handle_event(&mut self, event: MemoryEvent) -> Result<()> {
        match event {
            MemoryEvent::SessionStarted {
                project_id,
                session,
            } => {
                self.on_session_start(project_id, &session).await?;
            }
            MemoryEvent::SessionEnded {
                project_id,
                session,
                summary,
            } => {
                self.on_session_end(project_id, &session, summary).await?;
            }
            MemoryEvent::MessageAdded {
                project_id,
                session_id,
                role: _,
                content,
            } => {
                self.on_message_added(project_id, session_id, &content)
                    .await?;
            }
            MemoryEvent::TaskCompleted {
                project_id,
                session_id,
                task,
                outcome,
            } => {
                self.on_task_completed(project_id, session_id, task, outcome)
                    .await?;
            }
            MemoryEvent::AchievementReached {
                project_id,
                session_id,
                achievement,
            } => {
                self.on_achievement_reached(project_id, session_id, achievement)
                    .await?;
            }
            MemoryEvent::ProjectGoalUpdated { project_id, goals } => {
                self.on_project_goals_updated(project_id, goals).await?;
            }
            MemoryEvent::ProjectStatusChanged { project_id, status } => {
                self.on_project_status_changed(project_id, status).await?;
            }
        }
        Ok(())
    }

    /// Archive a project's memory files
    pub async fn archive_project(&mut self, project_id: Uuid) -> Result<PathBuf> {
        let project_dir = self.get_project_directory(project_id);
        let archive_dir = self
            .storage_dir
            .join("archived")
            .join(project_id.to_string());

        fs::create_dir_all(&archive_dir).await.with_context(|| {
            format!(
                "Failed to create archive directory: {}",
                archive_dir.display()
            )
        })?;

        // Copy all files to archive
        let mut dir = fs::read_dir(&project_dir).await.with_context(|| {
            format!(
                "Failed to read project directory: {}",
                project_dir.display()
            )
        })?;

        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                let filename = path.file_name().unwrap();
                let dest = archive_dir.join(filename);
                fs::copy(&path, &dest)
                    .await
                    .with_context(|| format!("Failed to copy {} to archive", path.display()))?;
            }
        }

        // Also copy metadata directory
        let meta_dir = project_dir.join(".meta");
        if meta_dir.exists() {
            let archive_meta_dir = archive_dir.join(".meta");
            fs::create_dir_all(&archive_meta_dir).await?;

            let mut meta_entries = fs::read_dir(&meta_dir).await?;
            while let Some(entry) = meta_entries.next_entry().await? {
                let path = entry.path();
                if path.is_file() {
                    let filename = path.file_name().unwrap();
                    let dest = archive_meta_dir.join(filename);
                    fs::copy(&path, &dest).await?;
                }
            }
        }

        // Remove from cache
        let file_types = [
            ClineFileType::ProjectBrief,
            ClineFileType::ActiveContext,
            ClineFileType::Progress,
        ];
        for file_type in file_types.iter() {
            self.cache.remove(&(project_id, file_type.clone()));
        }

        // Remove original directory
        fs::remove_dir_all(&project_dir).await.with_context(|| {
            format!(
                "Failed to remove original project directory: {}",
                project_dir.display()
            )
        })?;

        info!(
            "Archived project: {} to {}",
            project_id,
            archive_dir.display()
        );
        Ok(archive_dir)
    }

    /// Create backup of project files
    pub async fn backup_files(&self, project_id: Uuid) -> Result<PathBuf> {
        let project_dir = self.get_project_directory(project_id);
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let backup_dir = self
            .storage_dir
            .join("backups")
            .join(format!("{}_{}", project_id, timestamp));

        fs::create_dir_all(&backup_dir).await.with_context(|| {
            format!(
                "Failed to create backup directory: {}",
                backup_dir.display()
            )
        })?;

        // Copy all files
        let mut dir = fs::read_dir(&project_dir).await.with_context(|| {
            format!(
                "Failed to read project directory: {}",
                project_dir.display()
            )
        })?;

        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                let filename = path.file_name().unwrap();
                let dest = backup_dir.join(filename);
                fs::copy(&path, &dest)
                    .await
                    .with_context(|| format!("Failed to backup file: {}", path.display()))?;
            } else if path.is_dir() && path.file_name().unwrap() == ".meta" {
                // Copy metadata directory
                let backup_meta_dir = backup_dir.join(".meta");
                fs::create_dir_all(&backup_meta_dir).await?;

                let mut meta_entries = fs::read_dir(&path).await?;
                while let Some(meta_entry) = meta_entries.next_entry().await? {
                    let meta_path = meta_entry.path();
                    if meta_path.is_file() {
                        let filename = meta_path.file_name().unwrap();
                        let dest = backup_meta_dir.join(filename);
                        fs::copy(&meta_path, &dest).await?;
                    }
                }
            }
        }

        info!("Created backup: {} to {}", project_id, backup_dir.display());
        Ok(backup_dir)
    }

    /// List all projects with memory files
    pub async fn list_projects(&self) -> Result<Vec<Uuid>> {
        let mut projects = Vec::new();

        if !self.storage_dir.exists() {
            return Ok(projects);
        }

        let mut dir = fs::read_dir(&self.storage_dir).await.with_context(|| {
            format!(
                "Failed to read projects directory: {}",
                self.storage_dir.display()
            )
        })?;

        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                    // Skip special directories
                    if dir_name == "archived" || dir_name == "backups" {
                        continue;
                    }

                    if let Ok(project_id) = Uuid::parse_str(dir_name) {
                        projects.push(project_id);
                    }
                }
            }
        }

        Ok(projects)
    }

    /// Create initial file for a project
    async fn create_initial_file(
        &mut self,
        project_id: Uuid,
        file_type: ClineFileType,
    ) -> Result<()> {
        let content = match file_type {
            ClineFileType::ProjectBrief => ClineFileContent::ProjectBrief(ProjectBriefContent {
                project_id,
                ..Default::default()
            }),
            ClineFileType::ActiveContext => {
                ClineFileContent::ActiveContext(ActiveContextContent::default())
            }
            ClineFileType::Progress => ClineFileContent::Progress(ProgressContent::default()),
            ClineFileType::Custom(_) => ClineFileContent::Custom {
                content: String::new(),
            },
        };

        self.update_file(
            project_id,
            file_type,
            content,
            None,
            "Initial file creation".to_string(),
        )
        .await?;

        Ok(())
    }

    /// Load file content based on type
    async fn load_file_content(
        &self,
        project_id: Uuid,
        file_type: &ClineFileType,
    ) -> Result<ClineFileContent> {
        let project_dir = self.get_project_directory(project_id);
        let content_path = project_dir.join(format!("{}.json", file_type.filename()));

        if content_path.exists() {
            // Load from JSON file (structured data)
            let json = fs::read_to_string(&content_path).await.with_context(|| {
                format!("Failed to read content file: {}", content_path.display())
            })?;

            let content: ClineFileContent = serde_json::from_str(&json).with_context(|| {
                format!("Failed to deserialize content: {}", content_path.display())
            })?;

            Ok(content)
        } else {
            // Create default content
            let content = match file_type {
                ClineFileType::ProjectBrief => {
                    ClineFileContent::ProjectBrief(ProjectBriefContent {
                        project_id,
                        ..Default::default()
                    })
                }
                ClineFileType::ActiveContext => {
                    ClineFileContent::ActiveContext(ActiveContextContent::default())
                }
                ClineFileType::Progress => ClineFileContent::Progress(ProgressContent::default()),
                ClineFileType::Custom(_) => ClineFileContent::Custom {
                    content: String::new(),
                },
            };

            Ok(content)
        }
    }

    /// Save a memory file to disk
    async fn save_file(&self, file: &ClineMemoryFile) -> Result<()> {
        let project_dir = self.get_project_directory(file.project_id);
        let meta_dir = project_dir.join(".meta");

        // Ensure directories exist
        fs::create_dir_all(&meta_dir).await.with_context(|| {
            format!(
                "Failed to create metadata directory: {}",
                meta_dir.display()
            )
        })?;

        // Save structured content as JSON
        let content_path = project_dir.join(format!("{}.json", file.file_type.filename()));
        let content_json = serde_json::to_string_pretty(&file.content)
            .context("Failed to serialize file content")?;
        fs::write(&content_path, content_json)
            .await
            .with_context(|| format!("Failed to write content: {}", content_path.display()))?;

        // Save rendered markdown
        let markdown_path = project_dir.join(&file.file_path);
        let markdown = self.template_engine.render_to_markdown(file)?;
        fs::write(&markdown_path, markdown)
            .await
            .with_context(|| format!("Failed to write markdown: {}", markdown_path.display()))?;

        // Save metadata
        let meta_path = meta_dir.join(file.file_type.metadata_filename());
        let meta_json =
            serde_json::to_string_pretty(&file.metadata).context("Failed to serialize metadata")?;
        fs::write(&meta_path, meta_json)
            .await
            .with_context(|| format!("Failed to write metadata: {}", meta_path.display()))?;

        Ok(())
    }

    /// Manage cache size by removing oldest entries
    fn manage_cache_size(&mut self) {
        while self.cache.len() > self.max_cache_size {
            if let Some((oldest_key, _)) = self
                .cache
                .iter()
                .min_by_key(|(_, file)| file.metadata.updated_at)
                .map(|(key, file)| (key.clone(), file.clone()))
            {
                self.cache.remove(&oldest_key);
                debug!("Evicted Cline memory file from cache: {:?}", oldest_key);
            } else {
                break;
            }
        }
    }

    // Event handler methods
    async fn on_session_start(&mut self, project_id: Uuid, session: &Session) -> Result<()> {
        // Update active context with new session
        if let Some(mut file) = self
            .get_file(project_id, ClineFileType::ActiveContext)
            .await?
        {
            if let ClineFileContent::ActiveContext(ref mut content) = file.content {
                content.current_session_id = Some(session.id);
                // Clear previous session context
                content.session_context = "New session started".to_string();
                content.immediate_focus.clear();
            }

            self.update_file(
                project_id,
                ClineFileType::ActiveContext,
                file.content,
                Some(session.id),
                "Session started".to_string(),
            )
            .await?;
        }

        Ok(())
    }

    async fn on_session_end(
        &mut self,
        project_id: Uuid,
        session: &Session,
        summary: Option<String>,
    ) -> Result<()> {
        // Update progress with session summary
        if let Some(mut file) = self.get_file(project_id, ClineFileType::Progress).await? {
            if let ClineFileContent::Progress(ref mut content) = file.content {
                content.total_sessions += 1;

                let session_summary = SessionSummary {
                    session_id: session.id,
                    timestamp: Utc::now(),
                    summary: summary.unwrap_or_else(|| "Session completed".to_string()),
                    topics: Vec::new(),     // Would be extracted from session data
                    duration_minutes: None, // Would be calculated from session duration
                };

                content.recent_sessions.push(session_summary);

                // Keep only last 20 sessions
                if content.recent_sessions.len() > 20 {
                    content.recent_sessions.remove(0);
                }
            }

            self.update_file(
                project_id,
                ClineFileType::Progress,
                file.content,
                Some(session.id),
                "Session ended".to_string(),
            )
            .await?;
        }

        // Clear active context
        if let Some(mut file) = self
            .get_file(project_id, ClineFileType::ActiveContext)
            .await?
        {
            if let ClineFileContent::ActiveContext(ref mut content) = file.content {
                content.current_session_id = None;
                content.session_context = "No active session".to_string();
            }

            self.update_file(
                project_id,
                ClineFileType::ActiveContext,
                file.content,
                Some(session.id),
                "Session ended, cleared active context".to_string(),
            )
            .await?;
        }

        Ok(())
    }

    async fn on_message_added(
        &mut self,
        project_id: Uuid,
        session_id: Uuid,
        content: &str,
    ) -> Result<()> {
        // Extract topics and technologies from message content
        let topics = self.extract_topics(content);
        let technologies = self.extract_technologies(content);

        // Update active context with recent topics
        if let Some(mut file) = self
            .get_file(project_id, ClineFileType::ActiveContext)
            .await?
        {
            if let ClineFileContent::ActiveContext(ref mut active_content) = file.content {
                // Add new topics
                for topic in topics {
                    if !active_content.recent_topics.contains(&topic) {
                        active_content.recent_topics.push(topic);
                    }
                }

                // Keep only last 10 topics
                if active_content.recent_topics.len() > 10 {
                    active_content.recent_topics.remove(0);
                }

                // Update session context with recent message content
                active_content.session_context = if content.len() > 200 {
                    format!("{}...", &content[..200])
                } else {
                    content.to_string()
                };
            }

            self.update_file(
                project_id,
                ClineFileType::ActiveContext,
                file.content,
                Some(session_id),
                "Updated with message content".to_string(),
            )
            .await?;
        }

        // Update project brief with new technologies
        if !technologies.is_empty() {
            if let Some(mut file) = self
                .get_file(project_id, ClineFileType::ProjectBrief)
                .await?
            {
                if let ClineFileContent::ProjectBrief(ref mut brief_content) = file.content {
                    let mut updated = false;
                    for tech in technologies {
                        if !brief_content.technologies.contains(&tech) {
                            brief_content.technologies.push(tech);
                            updated = true;
                        }
                    }

                    if updated {
                        self.update_file(
                            project_id,
                            ClineFileType::ProjectBrief,
                            file.content,
                            Some(session_id),
                            "Added new technologies from conversation".to_string(),
                        )
                        .await?;
                    }
                }
            }
        }

        Ok(())
    }

    async fn on_task_completed(
        &mut self,
        project_id: Uuid,
        session_id: Option<Uuid>,
        task: String,
        outcome: String,
    ) -> Result<()> {
        if let Some(mut file) = self.get_file(project_id, ClineFileType::Progress).await? {
            if let ClineFileContent::Progress(ref mut content) = file.content {
                content.completed_tasks_count += 1;

                let completed_task = CompletedTask {
                    task,
                    completed_at: Utc::now(),
                    session_id,
                    outcome,
                };

                content.completed_tasks.push(completed_task);

                // Keep only last 50 completed tasks
                if content.completed_tasks.len() > 50 {
                    content.completed_tasks.remove(0);
                }
            }

            self.update_file(
                project_id,
                ClineFileType::Progress,
                file.content,
                session_id,
                "Task completed".to_string(),
            )
            .await?;
        }

        Ok(())
    }

    async fn on_achievement_reached(
        &mut self,
        project_id: Uuid,
        session_id: Option<Uuid>,
        achievement: Achievement,
    ) -> Result<()> {
        if let Some(mut file) = self.get_file(project_id, ClineFileType::Progress).await? {
            if let ClineFileContent::Progress(ref mut content) = file.content {
                content.achievements.push(achievement);
            }

            self.update_file(
                project_id,
                ClineFileType::Progress,
                file.content,
                session_id,
                "Achievement reached".to_string(),
            )
            .await?;
        }

        Ok(())
    }

    async fn on_project_goals_updated(
        &mut self,
        project_id: Uuid,
        goals: Vec<String>,
    ) -> Result<()> {
        if let Some(mut file) = self
            .get_file(project_id, ClineFileType::ProjectBrief)
            .await?
        {
            if let ClineFileContent::ProjectBrief(ref mut content) = file.content {
                content.goals = goals;
            }

            self.update_file(
                project_id,
                ClineFileType::ProjectBrief,
                file.content,
                None,
                "Project goals updated".to_string(),
            )
            .await?;
        }

        Ok(())
    }

    async fn on_project_status_changed(
        &mut self,
        project_id: Uuid,
        status: ProjectStatus,
    ) -> Result<()> {
        if let Some(mut file) = self
            .get_file(project_id, ClineFileType::ProjectBrief)
            .await?
        {
            if let ClineFileContent::ProjectBrief(ref mut content) = file.content {
                content.status = status;
            }

            self.update_file(
                project_id,
                ClineFileType::ProjectBrief,
                file.content,
                None,
                "Project status changed".to_string(),
            )
            .await?;
        }

        Ok(())
    }

    /// Extract topics from message content (simplified implementation)
    fn extract_topics(&self, content: &str) -> Vec<String> {
        let mut topics = Vec::new();
        let words: Vec<&str> = content.split_whitespace().collect();

        // Look for task-related keywords
        let task_keywords = [
            "implement",
            "fix",
            "debug",
            "create",
            "build",
            "test",
            "deploy",
        ];
        for keyword in task_keywords {
            if content.to_lowercase().contains(keyword) {
                // Try to extract context around the keyword
                if let Some(pos) = content.to_lowercase().find(keyword) {
                    let start = pos.saturating_sub(20);
                    let end = std::cmp::min(pos + keyword.len() + 20, content.len());
                    let context = &content[start..end];
                    topics.push(format!("{}: {}", keyword, context.trim()));
                }
            }
        }

        // Extract phrases that might be topics (simplified)
        if words.len() > 2 {
            let topic = words.iter().take(3).cloned().collect::<Vec<_>>().join(" ");
            topics.push(topic);
        }

        topics
    }

    /// Extract technologies from message content (simplified implementation)
    fn extract_technologies(&self, content: &str) -> Vec<String> {
        let mut technologies = Vec::new();
        let content_lower = content.to_lowercase();

        let tech_keywords = [
            "rust",
            "python",
            "javascript",
            "typescript",
            "react",
            "vue",
            "angular",
            "node",
            "go",
            "java",
            "c++",
            "docker",
            "kubernetes",
            "aws",
            "gcp",
            "postgres",
            "mysql",
            "redis",
            "mongodb",
            "git",
            "github",
            "tokio",
            "axum",
            "warp",
            "actix",
            "serde",
            "clap",
            "tracing",
            "anyhow",
        ];

        for tech in tech_keywords {
            if content_lower.contains(tech) {
                technologies.push(tech.to_string());
            }
        }

        technologies
    }
}

impl Default for ClineMemoryFileService {
    fn default() -> Self {
        Self::new().expect("Failed to create default ClineMemoryFileService")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cline_file_type_filename() {
        assert_eq!(ClineFileType::ProjectBrief.filename(), "projectbrief.md");
        assert_eq!(ClineFileType::ActiveContext.filename(), "activeContext.md");
        assert_eq!(ClineFileType::Progress.filename(), "progress.md");
        assert_eq!(
            ClineFileType::Custom("custom".to_string()).filename(),
            "custom.md"
        );
    }

    #[test]
    fn test_template_engine_project_brief() {
        let engine = TemplateEngine::new();
        let content = ProjectBriefContent {
            project_id: Uuid::new_v4(),
            status: ProjectStatus::Active,
            primary_technology: Some("Rust".to_string()),
            overview: "A test project".to_string(),
            goals: vec!["Goal 1".to_string(), "Goal 2".to_string()],
            current_status: "In progress".to_string(),
            technologies: vec!["Rust".to_string(), "tokio".to_string()],
        };

        let metadata = ClineFileMetadata {
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_session_id: None,
            version_history: Vec::new(),
        };

        let result = engine.render_project_brief(&content, &metadata);
        assert!(result.is_ok());

        let markdown = result.unwrap();
        assert!(markdown.contains("# Project Brief"));
        assert!(markdown.contains("A test project"));
        assert!(markdown.contains("- Goal 1"));
        assert!(markdown.contains("- Goal 2"));
        assert!(markdown.contains("Active"));
    }

    #[test]
    fn test_template_engine_active_context() {
        let engine = TemplateEngine::new();
        let content = ActiveContextContent {
            current_session_id: Some(Uuid::new_v4()),
            focus_area: Some("Testing".to_string()),
            current_task: Some("Write tests".to_string()),
            session_context: "Working on unit tests".to_string(),
            recent_topics: vec!["Testing".to_string(), "Rust".to_string()],
            immediate_focus: vec!["Fix failing test".to_string()],
            next_steps: vec!["Add integration tests".to_string()],
        };

        let metadata = ClineFileMetadata {
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_session_id: None,
            version_history: Vec::new(),
        };

        let result = engine.render_active_context(&content, &metadata);
        assert!(result.is_ok());

        let markdown = result.unwrap();
        assert!(markdown.contains("# Active Context"));
        assert!(markdown.contains("Write tests"));
        assert!(markdown.contains("- Testing"));
        assert!(markdown.contains("- Fix failing test"));
    }
}
