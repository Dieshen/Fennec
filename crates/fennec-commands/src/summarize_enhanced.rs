use anyhow::Result;
use async_trait::async_trait;
use fennec_core::{
    command::{Capability, CommandPreview, CommandResult, PreviewAction},
    error::FennecError,
    transcript::MessageRole,
};
use fennec_memory::{MemoryFileService, MemoryFileType, MemoryService, SessionMemory};
use fennec_security::SandboxLevel;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;
use uuid::Uuid;
use walkdir::WalkDir;

use crate::registry::{CommandContext, CommandDescriptor, CommandExecutor};

/// Arguments for the enhanced summarize command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedSummarizeArgs {
    /// Target to summarize (file path, directory, text content, or session)
    pub target: String,
    /// Type of summary to generate
    pub summary_type: Option<SummaryType>,
    /// Whether target is a file/directory path (true) or text content (false)
    pub is_path: Option<bool>,
    /// Maximum lines to include in summary
    pub max_lines: Option<usize>,
    /// File extensions to include when summarizing directories
    pub include_extensions: Option<Vec<String>>,
    /// Whether to include file structure in directory summaries
    pub include_structure: Option<bool>,
    /// Output destination for the summary
    pub output_destination: Option<OutputDestination>,
    /// Summary depth level
    pub depth_level: Option<SummaryDepth>,
    /// Time range for session summaries (in hours)
    pub time_range_hours: Option<u32>,
    /// Whether to save summary to memory files
    pub save_to_memory: Option<bool>,
    /// Tags to associate with memory file (if saved)
    pub memory_tags: Option<Vec<String>>,
}

/// Types of summaries that can be generated
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SummaryType {
    /// Summary of files or directories
    File,
    /// Summary of a session's conversation and commands
    Session,
    /// Summary of project state and progress
    Project,
    /// Summary of recent commands and their outcomes
    Commands,
    /// Summary of text content
    Text,
}

/// Output destinations for summaries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputDestination {
    /// Output to console only
    Console,
    /// Save to memory file
    MemoryFile(String), // filename
    /// Update existing progress.md file
    ProgressFile,
    /// Save to custom file path
    CustomFile(String),
    /// Both console and memory file
    Both(String), // memory filename
}

/// Depth levels for summaries
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SummaryDepth {
    /// Brief, high-level summary
    Brief,
    /// Standard detail level
    Standard,
    /// Detailed analysis with insights
    Detailed,
    /// Comprehensive analysis with recommendations
    Comprehensive,
}

/// Enhanced summarize command for creating summaries with memory integration
pub struct EnhancedSummarizeCommand {
    descriptor: CommandDescriptor,
    memory_service: Arc<RwLock<Option<MemoryService>>>,
    memory_file_service: Arc<RwLock<Option<MemoryFileService>>>,
}

impl EnhancedSummarizeCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor {
                name: "summarize_enhanced".to_string(),
                description: "Generate enhanced summaries of files, directories, sessions, or text content with memory integration"
                    .to_string(),
                version: "2.0.0".to_string(),
                author: Some("Fennec Core".to_string()),
                capabilities_required: vec![Capability::ReadFile],
                sandbox_level_required: SandboxLevel::ReadOnly,
                supports_preview: true,
                supports_dry_run: false,
            },
            memory_service: Arc::new(RwLock::new(None)),
            memory_file_service: Arc::new(RwLock::new(None)),
        }
    }

    /// Create enhanced summarize command with memory services
    pub async fn with_memory_services() -> Result<Self> {
        let memory_service = MemoryService::new().await?;
        let memory_file_service = MemoryFileService::new()?;

        Ok(Self {
            descriptor: CommandDescriptor {
                name: "summarize_enhanced".to_string(),
                description: "Generate enhanced summaries of files, directories, sessions, or text content with memory integration"
                    .to_string(),
                version: "2.0.0".to_string(),
                author: Some("Fennec Core".to_string()),
                capabilities_required: vec![Capability::ReadFile],
                sandbox_level_required: SandboxLevel::ReadOnly,
                supports_preview: true,
                supports_dry_run: false,
            },
            memory_service: Arc::new(RwLock::new(Some(memory_service))),
            memory_file_service: Arc::new(RwLock::new(Some(memory_file_service))),
        })
    }

    /// Generate summary based on the target type
    async fn generate_summary(
        &self,
        args: &EnhancedSummarizeArgs,
        context: &CommandContext,
    ) -> Result<String> {
        let summary_type = args.summary_type.as_ref().unwrap_or(&SummaryType::File);

        match summary_type {
            SummaryType::Session => self.summarize_session(args, context).await,
            SummaryType::Project => self.summarize_project(args, context).await,
            SummaryType::Commands => self.summarize_commands(args, context).await,
            SummaryType::Text => self.summarize_text(&args.target, args),
            SummaryType::File => {
                if args.is_path.unwrap_or(true) {
                    let path = Path::new(&args.target);

                    if !path.exists() {
                        return Err(FennecError::Command {
                            message: format!("Path does not exist: {}", args.target),
                        }
                        .into());
                    }

                    if path.is_file() {
                        self.summarize_file(path, args).await
                    } else if path.is_dir() {
                        self.summarize_directory(path, args).await
                    } else {
                        Err(FennecError::Command {
                            message: format!("Unsupported path type: {}", args.target),
                        }
                        .into())
                    }
                } else {
                    self.summarize_text(&args.target, args)
                }
            }
        }
    }

    /// Summarize a session's conversation and command history
    async fn summarize_session(
        &self,
        args: &EnhancedSummarizeArgs,
        context: &CommandContext,
    ) -> Result<String> {
        let session_id = if args.target == "current" {
            context.session_id
        } else {
            Uuid::parse_str(&args.target).map_err(|e| FennecError::Command {
                message: format!("Invalid session ID: {}", e),
            })?
        };

        let depth = args.depth_level.as_ref().unwrap_or(&SummaryDepth::Standard);
        let time_range = args.time_range_hours.unwrap_or(24);

        let mut summary = Vec::new();
        summary.push(format!("# Session Summary: {}", session_id));
        summary.push(String::new());
        summary.push(format!(
            "**Generated:** {}",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));
        summary.push(format!("**Time Range:** Last {} hours", time_range));
        summary.push(format!("**Summary Level:** {:?}", depth));
        summary.push(String::new());

        // Get session data from memory service if available
        let memory_service_guard = self.memory_service.read().await;
        if let Some(memory_service) = memory_service_guard.as_ref() {
            if let Some(session_memory) = memory_service.get_session_memory(session_id).await {
                self.add_session_conversation_summary(&mut summary, &session_memory, depth)
                    .await?;
                self.add_session_context_summary(&mut summary, &session_memory.context, depth);
                self.add_session_insights(&mut summary, &session_memory, depth)
                    .await?;
            } else {
                summary.push("## Status".to_string());
                summary.push("⚠️  Session not found in active memory. This may be an inactive or completed session.".to_string());
                summary.push(String::new());
            }
        } else {
            summary.push("## Error".to_string());
            summary.push(
                "❌ Memory service not available. Cannot generate session summary.".to_string(),
            );
            summary.push(String::new());
        }

        Ok(summary.join("\n"))
    }

    /// Summarize project state and progress
    async fn summarize_project(
        &self,
        args: &EnhancedSummarizeArgs,
        context: &CommandContext,
    ) -> Result<String> {
        let workspace_path =
            context
                .workspace_path
                .as_ref()
                .ok_or_else(|| FennecError::Command {
                    message: "No workspace path available for project summary".to_string(),
                })?;

        let depth = args.depth_level.as_ref().unwrap_or(&SummaryDepth::Standard);

        let mut summary = Vec::new();
        summary.push(format!("# Project Summary: {}", workspace_path));
        summary.push(String::new());
        summary.push(format!(
            "**Generated:** {}",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));
        summary.push(format!("**Summary Level:** {:?}", depth));
        summary.push(String::new());

        // Add basic project structure
        let workspace_path_buf = std::path::PathBuf::from(workspace_path);
        self.add_project_structure_summary(&mut summary, &workspace_path_buf, depth)
            .await?;

        // Add recent activity if memory service is available
        let memory_service_guard = self.memory_service.read().await;
        if let Some(memory_service) = memory_service_guard.as_ref() {
            self.add_recent_project_activity(&mut summary, memory_service, depth)
                .await?;
        }

        // Add memory files related to project
        let memory_file_service_guard = self.memory_file_service.read().await;
        if let Some(_memory_file_service) = memory_file_service_guard.as_ref() {
            self.add_project_memory_files(&mut summary, depth).await?;
        }

        Ok(summary.join("\n"))
    }

    /// Summarize recent commands and their outcomes
    async fn summarize_commands(
        &self,
        args: &EnhancedSummarizeArgs,
        context: &CommandContext,
    ) -> Result<String> {
        let time_range = args.time_range_hours.unwrap_or(6);
        let depth = args.depth_level.as_ref().unwrap_or(&SummaryDepth::Standard);

        let mut summary = Vec::new();
        summary.push("# Commands Summary".to_string());
        summary.push(String::new());
        summary.push(format!(
            "**Generated:** {}",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));
        summary.push(format!("**Time Range:** Last {} hours", time_range));
        summary.push(format!("**Summary Level:** {:?}", depth));
        summary.push(String::new());

        // This would need integration with command history tracking
        summary.push("## Recent Command Activity".to_string());
        summary.push("📝 Command history tracking would be implemented here.".to_string());
        summary.push("This would include:".to_string());
        summary.push("- Commands executed in the time range".to_string());
        summary.push("- Success/failure rates".to_string());
        summary.push("- Common error patterns".to_string());
        summary.push("- Performance metrics".to_string());
        summary.push(String::new());

        let memory_service_guard = self.memory_service.read().await;
        if let Some(memory_service) = memory_service_guard.as_ref() {
            summary.push("## Session Context".to_string());
            if let Some(session_memory) =
                memory_service.get_session_memory(context.session_id).await
            {
                for tech in &session_memory.context.technologies {
                    summary.push(format!("- 🔧 **Technology:** {}", tech));
                }
                if let Some(task) = &session_memory.context.current_task {
                    summary.push(format!("- 🎯 **Current Task:** {}", task));
                }
            }
            summary.push(String::new());
        }

        Ok(summary.join("\n"))
    }

    /// Summarize a single file
    async fn summarize_file(&self, path: &Path, args: &EnhancedSummarizeArgs) -> Result<String> {
        let content = fs::read_to_string(path)
            .await
            .map_err(|e| FennecError::Command {
                message: format!("Failed to read file {}: {}", path.display(), e),
            })?;

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();
        let max_lines = args.max_lines.unwrap_or(100);

        let mut summary = Vec::new();
        summary.push(format!("# File Summary: {}", path.display()));
        summary.push(String::new());
        summary.push(format!("- **Total lines:** {}", total_lines));
        summary.push(format!("- **File size:** {} bytes", content.len()));

        // Detect file type based on extension
        if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
            summary.push(format!("- **File type:** {}", extension));
        }

        summary.push(String::new());
        summary.push("## Content Preview".to_string());
        summary.push(String::new());

        if total_lines <= max_lines {
            summary.push("```".to_string());
            summary.extend(lines.iter().map(|s| s.to_string()));
            summary.push("```".to_string());
        } else {
            let preview_lines = max_lines / 2;
            summary.push("```".to_string());
            summary.extend(lines[..preview_lines].iter().map(|s| s.to_string()));
            summary.push(format!(
                "... ({} lines omitted) ...",
                total_lines - max_lines
            ));
            summary.extend(
                lines[total_lines - preview_lines..]
                    .iter()
                    .map(|s| s.to_string()),
            );
            summary.push("```".to_string());
        }

        Ok(summary.join("\n"))
    }

    /// Summarize a directory
    async fn summarize_directory(
        &self,
        path: &Path,
        args: &EnhancedSummarizeArgs,
    ) -> Result<String> {
        let mut summary = Vec::new();
        summary.push(format!("# Directory Summary: {}", path.display()));
        summary.push(String::new());

        let mut file_count = 0;
        let mut dir_count = 0;
        let mut total_size = 0u64;
        let mut file_types: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        // Collect directory statistics
        for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                file_count += 1;
                if let Ok(metadata) = entry.metadata() {
                    total_size += metadata.len();
                }

                if let Some(extension) = entry.path().extension().and_then(|ext| ext.to_str()) {
                    *file_types.entry(extension.to_lowercase()).or_insert(0) += 1;
                }
            } else if entry.file_type().is_dir() {
                dir_count += 1;
            }
        }

        summary.push("## Statistics".to_string());
        summary.push(format!("- **Files:** {}", file_count));
        summary.push(format!("- **Directories:** {}", dir_count));
        summary.push(format!("- **Total size:** {} bytes", total_size));

        if !file_types.is_empty() {
            summary.push(String::new());
            summary.push("## File Types".to_string());
            let mut types: Vec<_> = file_types.iter().collect();
            types.sort_by(|a, b| b.1.cmp(a.1)); // Sort by count descending

            for (ext, count) in types.iter().take(10) {
                summary.push(format!("- **.{}**: {} files", ext, count));
            }
        }

        // Include directory structure if requested
        if args.include_structure.unwrap_or(false) {
            summary.push(String::new());
            summary.push("## Directory Structure".to_string());
            summary.push("```".to_string());

            for entry in WalkDir::new(path).max_depth(3) {
                if let Ok(entry) = entry {
                    let depth = entry.depth();
                    let indent = "  ".repeat(depth);
                    let name = entry.file_name().to_string_lossy();

                    if entry.file_type().is_dir() {
                        summary.push(format!("{}{}/", indent, name));
                    } else {
                        // Filter by extensions if specified
                        if let Some(ref extensions) = args.include_extensions {
                            if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
                                if !extensions.contains(&ext.to_lowercase()) {
                                    continue;
                                }
                            } else {
                                continue;
                            }
                        }
                        summary.push(format!("{}{}", indent, name));
                    }
                }
            }

            summary.push("```".to_string());
        }

        Ok(summary.join("\n"))
    }

    /// Summarize text content
    fn summarize_text(&self, text: &str, args: &EnhancedSummarizeArgs) -> Result<String> {
        let lines: Vec<&str> = text.lines().collect();
        let total_lines = lines.len();
        let total_chars = text.len();
        let total_words = text.split_whitespace().count();
        let max_lines = args.max_lines.unwrap_or(50);

        let mut summary = Vec::new();
        summary.push("# Text Summary".to_string());
        summary.push(String::new());
        summary.push(format!("- **Lines:** {}", total_lines));
        summary.push(format!("- **Words:** {}", total_words));
        summary.push(format!("- **Characters:** {}", total_chars));

        // Detect if text looks like code
        let code_indicators = [
            "function", "class", "import", "export", "const", "let", "var", "def", "if", "else",
            "for", "while", "return", "{", "}", ";",
        ];

        let code_score = code_indicators
            .iter()
            .map(|indicator| text.matches(indicator).count())
            .sum::<usize>();

        if code_score > 5 {
            summary.push("- **Type:** Appears to be code".to_string());
        }

        summary.push(String::new());
        summary.push("## Content Preview".to_string());
        summary.push(String::new());

        if total_lines <= max_lines {
            summary.push("```".to_string());
            summary.extend(lines.iter().map(|s| s.to_string()));
            summary.push("```".to_string());
        } else {
            let preview_lines = max_lines / 2;
            summary.push("```".to_string());
            summary.extend(lines[..preview_lines].iter().map(|s| s.to_string()));
            summary.push(format!(
                "... ({} lines omitted) ...",
                total_lines - max_lines
            ));
            summary.extend(
                lines[total_lines - preview_lines..]
                    .iter()
                    .map(|s| s.to_string()),
            );
            summary.push("```".to_string());
        }

        Ok(summary.join("\n"))
    }

    // Helper methods for session summary generation
    async fn add_session_conversation_summary(
        &self,
        summary: &mut Vec<String>,
        session_memory: &SessionMemory,
        depth: &SummaryDepth,
    ) -> Result<()> {
        summary.push("## Conversation Overview".to_string());

        let message_count = session_memory.transcript.messages.len();
        summary.push(format!("- **Total Messages:** {}", message_count));

        if message_count > 0 {
            let first_message = &session_memory.transcript.messages[0];
            let last_message = &session_memory.transcript.messages[message_count - 1];

            summary.push(format!(
                "- **Started:** {}",
                first_message.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
            ));
            summary.push(format!(
                "- **Last Activity:** {}",
                last_message.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
            ));

            // Count by role
            let user_messages = session_memory
                .transcript
                .messages
                .iter()
                .filter(|m| matches!(m.role, MessageRole::User))
                .count();
            let assistant_messages = session_memory
                .transcript
                .messages
                .iter()
                .filter(|m| matches!(m.role, MessageRole::Assistant))
                .count();

            summary.push(format!("- **User Messages:** {}", user_messages));
            summary.push(format!("- **Assistant Messages:** {}", assistant_messages));
        }

        summary.push(String::new());

        match depth {
            SummaryDepth::Brief => {
                // Just the overview above
            }
            SummaryDepth::Standard | SummaryDepth::Detailed | SummaryDepth::Comprehensive => {
                summary.push("### Recent Messages".to_string());
                let recent_count = match depth {
                    SummaryDepth::Standard => 3,
                    SummaryDepth::Detailed => 5,
                    SummaryDepth::Comprehensive => 10,
                    _ => 3,
                };

                let recent_messages: Vec<_> = session_memory
                    .transcript
                    .messages
                    .iter()
                    .rev()
                    .take(recent_count)
                    .collect();

                for message in recent_messages.iter().rev() {
                    let role_emoji = match message.role {
                        MessageRole::User => "👤",
                        MessageRole::Assistant => "🤖",
                        MessageRole::System => "⚙️",
                    };

                    let preview = if message.content.len() > 100 {
                        format!("{}...", &message.content[..100])
                    } else {
                        message.content.clone()
                    };

                    summary.push(format!(
                        "- {} **{:?}** ({}): {}",
                        role_emoji,
                        message.role,
                        message.timestamp.format("%H:%M"),
                        preview.replace('\n', " ")
                    ));
                }
                summary.push(String::new());
            }
        }

        Ok(())
    }

    fn add_session_context_summary(
        &self,
        summary: &mut Vec<String>,
        context: &fennec_memory::ConversationContext,
        depth: &SummaryDepth,
    ) {
        summary.push("## Session Context".to_string());

        if !context.technologies.is_empty() {
            summary.push("### Technologies Discussed".to_string());
            for tech in &context.technologies {
                summary.push(format!("- 🔧 {}", tech));
            }
            summary.push(String::new());
        }

        if let Some(task) = &context.current_task {
            summary.push("### Current Task".to_string());
            summary.push(format!("🎯 {}", task));
            summary.push(String::new());
        }

        if !context.recent_topics.is_empty()
            && matches!(depth, SummaryDepth::Detailed | SummaryDepth::Comprehensive)
        {
            summary.push("### Recent Topics".to_string());
            let topic_count = match depth {
                SummaryDepth::Detailed => 5,
                SummaryDepth::Comprehensive => 10,
                _ => 5,
            };

            for topic in context.recent_topics.iter().take(topic_count) {
                summary.push(format!("- 💬 {}", topic));
            }
            summary.push(String::new());
        }

        if !context.error_patterns.is_empty() && matches!(depth, SummaryDepth::Comprehensive) {
            summary.push("### Error Patterns".to_string());
            for error in context.error_patterns.iter().take(3) {
                let preview = if error.len() > 80 {
                    format!("{}...", &error[..80])
                } else {
                    error.clone()
                };
                summary.push(format!("- ❌ {}", preview.replace('\n', " ")));
            }
            summary.push(String::new());
        }
    }

    async fn add_session_insights(
        &self,
        summary: &mut Vec<String>,
        session_memory: &SessionMemory,
        depth: &SummaryDepth,
    ) -> Result<()> {
        if !matches!(depth, SummaryDepth::Detailed | SummaryDepth::Comprehensive) {
            return Ok(());
        }

        summary.push("## Insights & Recommendations".to_string());

        // Analyze conversation patterns
        let message_count = session_memory.transcript.messages.len();
        if message_count > 10 {
            summary.push("### Session Analysis".to_string());
            summary.push(format!(
                "- 📊 **Activity Level:** {} messages indicate active session",
                message_count
            ));

            // Check for recent activity
            if let Some(last_message) = session_memory.transcript.messages.last() {
                let time_since_last =
                    chrono::Utc::now().signed_duration_since(last_message.timestamp);
                if time_since_last.num_minutes() < 30 {
                    summary.push("- ⚡ **Status:** Recently active".to_string());
                } else if time_since_last.num_hours() < 24 {
                    summary.push("- 🕐 **Status:** Active today".to_string());
                } else {
                    summary.push("- 💤 **Status:** Inactive (may need follow-up)".to_string());
                }
            }

            summary.push(String::new());
        }

        if matches!(depth, SummaryDepth::Comprehensive) {
            summary.push("### Next Steps".to_string());

            if let Some(_task) = &session_memory.context.current_task {
                summary.push("- 🎯 Continue working on current task".to_string());
                summary.push("- 📝 Consider documenting progress".to_string());
            }

            if !session_memory.context.error_patterns.is_empty() {
                summary.push("- 🔍 Review and resolve error patterns".to_string());
            }

            if !session_memory.context.technologies.is_empty() {
                summary
                    .push("- 📚 Consider updating knowledge base with new learnings".to_string());
            }

            summary.push(String::new());
        }

        Ok(())
    }

    async fn add_project_structure_summary(
        &self,
        summary: &mut Vec<String>,
        workspace_path: &Path,
        depth: &SummaryDepth,
    ) -> Result<()> {
        summary.push("## Project Structure".to_string());

        if workspace_path.exists() {
            let mut file_count = 0;
            let mut dir_count = 0;
            let mut file_types: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();

            for entry in WalkDir::new(workspace_path)
                .max_depth(if matches!(depth, SummaryDepth::Brief) {
                    2
                } else {
                    3
                })
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() {
                    file_count += 1;
                    if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
                        *file_types.entry(ext.to_lowercase()).or_insert(0) += 1;
                    }
                } else if entry.file_type().is_dir() {
                    dir_count += 1;
                }
            }

            summary.push(format!("- **Files:** {}", file_count));
            summary.push(format!("- **Directories:** {}", dir_count));

            if !file_types.is_empty() {
                summary.push("- **File Types:**".to_string());
                let mut types: Vec<_> = file_types.iter().collect();
                types.sort_by(|a, b| b.1.cmp(a.1));

                for (ext, count) in types.iter().take(5) {
                    summary.push(format!("  - **.{}**: {} files", ext, count));
                }
            }
        } else {
            summary.push("❌ Workspace path not accessible".to_string());
        }

        summary.push(String::new());
        Ok(())
    }

    async fn add_recent_project_activity(
        &self,
        summary: &mut Vec<String>,
        memory_service: &MemoryService,
        _depth: &SummaryDepth,
    ) -> Result<()> {
        summary.push("## Recent Activity".to_string());

        let sessions = memory_service.list_sessions().await?;
        let recent_sessions: Vec<_> = sessions
            .into_iter()
            .filter(|s| {
                let hours_since = chrono::Utc::now()
                    .signed_duration_since(s.updated_at)
                    .num_hours();
                hours_since < 24
            })
            .take(5)
            .collect();

        if recent_sessions.is_empty() {
            summary.push("- No recent session activity".to_string());
        } else {
            summary.push(format!(
                "- **Recent Sessions:** {} in last 24h",
                recent_sessions.len()
            ));

            for session_meta in recent_sessions {
                summary.push(format!(
                    "  - Session {} ({})",
                    session_meta.session_id,
                    session_meta.updated_at.format("%m-%d %H:%M")
                ));
            }
        }

        summary.push(String::new());
        Ok(())
    }

    async fn add_project_memory_files(
        &self,
        summary: &mut Vec<String>,
        _depth: &SummaryDepth,
    ) -> Result<()> {
        summary.push("## Knowledge Base".to_string());

        summary.push("📚 Memory files related to this project:".to_string());
        summary.push("  - Project context files would be listed here".to_string());
        summary.push("  - Architecture decision records".to_string());
        summary.push("  - Learning notes and patterns".to_string());

        summary.push(String::new());
        Ok(())
    }

    /// Save summary to memory file if requested
    async fn save_summary_to_memory(
        &self,
        summary: &str,
        args: &EnhancedSummarizeArgs,
        context: &CommandContext,
    ) -> Result<Option<Uuid>> {
        if !args.save_to_memory.unwrap_or(false) {
            return Ok(None);
        }

        let mut memory_file_service_guard = self.memory_file_service.write().await;
        let memory_file_service =
            memory_file_service_guard
                .as_mut()
                .ok_or_else(|| FennecError::Command {
                    message: "Memory file service not available".to_string(),
                })?;

        let summary_type = args.summary_type.as_ref().unwrap_or(&SummaryType::File);
        let file_type = match summary_type {
            SummaryType::Session => MemoryFileType::Learning,
            SummaryType::Project => MemoryFileType::ProjectContext,
            SummaryType::Commands => MemoryFileType::Templates,
            SummaryType::File | SummaryType::Text => MemoryFileType::Knowledge,
        };

        let name = match &args.output_destination {
            Some(OutputDestination::MemoryFile(name)) | Some(OutputDestination::Both(name)) => {
                name.clone()
            }
            _ => {
                format!(
                    "Summary - {} - {}",
                    summary_type_to_string(summary_type),
                    chrono::Utc::now().format("%Y-%m-%d %H:%M")
                )
            }
        };

        let tags = args.memory_tags.clone().unwrap_or_else(|| {
            vec![
                "summary".to_string(),
                summary_type_to_string(summary_type).to_lowercase(),
            ]
        });

        let file_id = memory_file_service
            .create_memory_file(name, summary.to_string(), file_type, tags)
            .await?;

        // Associate with current session if available
        let memory_service_guard = self.memory_service.read().await;
        if memory_service_guard.is_some() {
            memory_file_service
                .associate_with_session(file_id, context.session_id)
                .await?;
        }

        Ok(Some(file_id))
    }

    /// Write summary to progress file
    async fn write_to_progress_file(&self, summary: &str, context: &CommandContext) -> Result<()> {
        let workspace_path =
            context
                .workspace_path
                .as_ref()
                .ok_or_else(|| FennecError::Command {
                    message: "No workspace path available".to_string(),
                })?;

        let workspace_path_buf = std::path::PathBuf::from(workspace_path);
        let progress_file = workspace_path_buf.join("progress.md");

        // Read existing content if file exists
        let existing_content = if progress_file.exists() {
            fs::read_to_string(&progress_file).await.unwrap_or_default()
        } else {
            String::new()
        };

        // Append new summary with separator
        let updated_content = if existing_content.is_empty() {
            summary.to_string()
        } else {
            format!("{}\n\n---\n\n{}", existing_content, summary)
        };

        fs::write(&progress_file, updated_content).await?;

        Ok(())
    }
}

/// Helper function to convert SummaryType to string
fn summary_type_to_string(summary_type: &SummaryType) -> &str {
    match summary_type {
        SummaryType::File => "File",
        SummaryType::Session => "Session",
        SummaryType::Project => "Project",
        SummaryType::Commands => "Commands",
        SummaryType::Text => "Text",
    }
}

#[async_trait]
impl CommandExecutor for EnhancedSummarizeCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn preview(
        &self,
        args: &serde_json::Value,
        _context: &CommandContext,
    ) -> Result<CommandPreview> {
        let args: EnhancedSummarizeArgs =
            serde_json::from_value(args.clone()).map_err(|e| FennecError::Command {
                message: format!("Invalid enhanced summarize arguments: {}", e),
            })?;

        let mut actions = Vec::new();
        let summary_type = args.summary_type.as_ref().unwrap_or(&SummaryType::File);

        match summary_type {
            SummaryType::Session => {
                actions.push(PreviewAction::ReadFile {
                    path: format!("Session data for: {}", args.target),
                });
            }
            SummaryType::Project => {
                actions.push(PreviewAction::ReadFile {
                    path: "Project structure and recent activity".to_string(),
                });
            }
            SummaryType::Commands => {
                actions.push(PreviewAction::ReadFile {
                    path: "Command history and session context".to_string(),
                });
            }
            SummaryType::File => {
                if args.is_path.unwrap_or(true) {
                    let path = Path::new(&args.target);
                    if path.is_file() {
                        actions.push(PreviewAction::ReadFile {
                            path: args.target.clone(),
                        });
                    } else if path.is_dir() {
                        actions.push(PreviewAction::ReadFile {
                            path: format!("{} (directory contents)", args.target),
                        });
                    }
                }
            }
            SummaryType::Text => {
                // No file reading needed
            }
        }

        // Add memory file save action if requested
        if args.save_to_memory.unwrap_or(false) {
            actions.push(PreviewAction::ReadFile {
                path: "Memory file creation".to_string(),
            });
        }

        Ok(CommandPreview {
            command_id: Uuid::new_v4(),
            description: format!(
                "Generate {} summary: {}",
                summary_type_to_string(summary_type).to_lowercase(),
                args.target
            ),
            actions,
            requires_approval: false,
        })
    }

    async fn execute(
        &self,
        args: &serde_json::Value,
        context: &CommandContext,
    ) -> Result<CommandResult> {
        let args: EnhancedSummarizeArgs =
            serde_json::from_value(args.clone()).map_err(|e| FennecError::Command {
                message: format!("Invalid enhanced summarize arguments: {}", e),
            })?;

        match self.generate_summary(&args, context).await {
            Ok(summary) => {
                let mut output_parts = Vec::new();
                let mut success_messages = Vec::new();

                // Always include the summary in output
                output_parts.push(summary.clone());

                // Handle output destinations
                match &args.output_destination {
                    Some(OutputDestination::Console) | None => {
                        // Already included above
                    }
                    Some(OutputDestination::MemoryFile(name)) => {
                        // Save to memory file only
                        if let Ok(Some(file_id)) =
                            self.save_summary_to_memory(&summary, &args, context).await
                        {
                            success_messages.push(format!(
                                "✅ Summary saved to memory file '{}' (ID: {})",
                                name, file_id
                            ));
                        }
                    }
                    Some(OutputDestination::ProgressFile) => {
                        // Save to progress.md
                        if let Ok(()) = self.write_to_progress_file(&summary, context).await {
                            success_messages.push("✅ Summary appended to progress.md".to_string());
                        }
                    }
                    Some(OutputDestination::CustomFile(path)) => {
                        // Save to custom file
                        if let Ok(()) = fs::write(path, &summary).await {
                            success_messages.push(format!("✅ Summary saved to {}", path));
                        }
                    }
                    Some(OutputDestination::Both(name)) => {
                        // Save to memory file and show in console
                        if let Ok(Some(file_id)) =
                            self.save_summary_to_memory(&summary, &args, context).await
                        {
                            success_messages.push(format!(
                                "✅ Summary saved to memory file '{}' (ID: {})",
                                name, file_id
                            ));
                        }
                    }
                }

                // Auto-save to memory if requested
                if args.save_to_memory.unwrap_or(false) && args.output_destination.is_none() {
                    if let Ok(Some(file_id)) =
                        self.save_summary_to_memory(&summary, &args, context).await
                    {
                        success_messages
                            .push(format!("✅ Summary auto-saved to memory (ID: {})", file_id));
                    }
                }

                // Combine output and success messages
                if !success_messages.is_empty() {
                    output_parts.push(String::new()); // Empty line
                    output_parts.push("## Summary Actions".to_string());
                    output_parts.extend(success_messages);
                }

                Ok(CommandResult {
                    command_id: Uuid::new_v4(),
                    success: true,
                    output: output_parts.join("\n"),
                    error: None,
                })
            }
            Err(e) => Ok(CommandResult {
                command_id: Uuid::new_v4(),
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
            }),
        }
    }

    fn validate_args(&self, args: &serde_json::Value) -> Result<()> {
        let args: EnhancedSummarizeArgs =
            serde_json::from_value(args.clone()).map_err(|e| FennecError::Command {
                message: format!("Invalid enhanced summarize arguments: {}", e),
            })?;

        if args.target.trim().is_empty() {
            return Err(FennecError::Command {
                message: "Target cannot be empty".to_string(),
            }
            .into());
        }

        if let Some(max_lines) = args.max_lines {
            if max_lines == 0 {
                return Err(FennecError::Command {
                    message: "max_lines must be greater than 0".to_string(),
                }
                .into());
            }
        }

        if let Some(time_range) = args.time_range_hours {
            if time_range == 0 {
                return Err(FennecError::Command {
                    message: "time_range_hours must be greater than 0".to_string(),
                }
                .into());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_util::sync::CancellationToken;

    #[tokio::test]
    async fn test_enhanced_summarize_command_validation() {
        let command = EnhancedSummarizeCommand::new();

        // Valid args
        let valid_args = serde_json::json!({
            "target": "test.txt",
            "summary_type": "File"
        });
        assert!(command.validate_args(&valid_args).is_ok());

        // Empty target
        let empty_target = serde_json::json!({
            "target": ""
        });
        assert!(command.validate_args(&empty_target).is_err());
    }

    #[tokio::test]
    async fn test_enhanced_summarize_text_content() {
        let command = EnhancedSummarizeCommand::new();

        let args = serde_json::json!({
            "target": "Hello world\nThis is a test\nWith multiple lines",
            "summary_type": "Text",
            "is_path": false,
            "max_lines": 10
        });

        let context = CommandContext {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: None,
            sandbox_level: SandboxLevel::ReadOnly,
            dry_run: false,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
        };

        let result = command.execute(&args, &context).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("Text Summary"));
        assert!(result.output.contains("**Lines:** 3"));
        assert!(result.output.contains("**Words:** 9"));
    }

    #[tokio::test]
    async fn test_enhanced_summarize_session() {
        let command = EnhancedSummarizeCommand::new();

        let args = serde_json::json!({
            "target": "current",
            "summary_type": "Session",
            "depth_level": "Standard"
        });

        let context = CommandContext {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: Some("/tmp/test".into()),
            sandbox_level: SandboxLevel::ReadOnly,
            dry_run: false,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
        };

        let result = command.execute(&args, &context).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("Session Summary"));
        assert!(result.output.contains("Memory service not available"));
    }
}
