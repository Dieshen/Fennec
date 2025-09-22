use anyhow::Result;
use async_trait::async_trait;
use fennec_core::{
    command::{Capability, CommandPreview, CommandResult, PreviewAction},
    error::FennecError,
};
use fennec_memory::{MemoryFileService, MemoryService};
use fennec_security::SandboxLevel;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;
use uuid::Uuid;
use walkdir::WalkDir;

use crate::registry::{CommandContext, CommandDescriptor, CommandExecutor};

/// Arguments for the summarize command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummarizeArgs {
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Enhanced summarize command for creating summaries of files, directories, sessions, or text
pub struct SummarizeCommand {
    descriptor: CommandDescriptor,
    #[allow(dead_code)]
    memory_service: Option<MemoryService>,
    #[allow(dead_code)]
    memory_file_service: Option<MemoryFileService>,
}

impl SummarizeCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor {
                name: "summarize".to_string(),
                description: "Generate enhanced summaries of files, directories, sessions, or text content with memory integration"
                    .to_string(),
                version: "2.0.0".to_string(),
                author: Some("Fennec Core".to_string()),
                capabilities_required: vec![Capability::ReadFile],
                sandbox_level_required: SandboxLevel::ReadOnly,
                supports_preview: true,
                supports_dry_run: false,
            },
            memory_service: None,
            memory_file_service: None,
        }
    }

    /// Create summarize command with memory services
    #[allow(dead_code)]
    pub async fn with_memory_services() -> Result<Self> {
        let memory_service = MemoryService::new().await?;
        let memory_file_service = MemoryFileService::new()?;

        Ok(Self {
            descriptor: CommandDescriptor {
                name: "summarize".to_string(),
                description: "Generate enhanced summaries of files, directories, sessions, or text content with memory integration"
                    .to_string(),
                version: "2.0.0".to_string(),
                author: Some("Fennec Core".to_string()),
                capabilities_required: vec![Capability::ReadFile],
                sandbox_level_required: SandboxLevel::ReadOnly,
                supports_preview: true,
                supports_dry_run: false,
            },
            memory_service: Some(memory_service),
            memory_file_service: Some(memory_file_service),
        })
    }

    /// Generate summary based on the target type
    async fn generate_summary(
        &self,
        args: &SummarizeArgs,
        _context: &CommandContext,
    ) -> Result<String> {
        if args.is_path.unwrap_or(true) {
            let path = Path::new(&args.target);

            if !path.exists() {
                return Err(FennecError::Command(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Path does not exist: {}", args.target),
                )))
                .into());
            }

            if path.is_file() {
                self.summarize_file(path, args).await
            } else if path.is_dir() {
                self.summarize_directory(path, args).await
            } else {
                Err(FennecError::Command(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Unsupported path type: {}", args.target),
                )))
                .into())
            }
        } else {
            self.summarize_text(&args.target, args)
        }
    }

    /// Summarize a single file
    async fn summarize_file(&self, path: &Path, args: &SummarizeArgs) -> Result<String> {
        let content = fs::read_to_string(path).await.map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to read file {}: {}", path.display(), e),
            )))
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
    async fn summarize_directory(&self, path: &Path, args: &SummarizeArgs) -> Result<String> {
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
    fn summarize_text(&self, text: &str, args: &SummarizeArgs) -> Result<String> {
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
}

/// Helper function to convert SummaryType to string
#[allow(dead_code)]
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
impl CommandExecutor for SummarizeCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn preview(
        &self,
        args: &serde_json::Value,
        _context: &CommandContext,
    ) -> Result<CommandPreview> {
        let args: SummarizeArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Invalid summarize arguments: {}", e),
            )))
        })?;

        let mut actions = Vec::new();

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

        Ok(CommandPreview {
            command_id: Uuid::new_v4(),
            description: format!("Generate summary of: {}", args.target),
            actions,
            requires_approval: false,
        })
    }

    async fn execute(
        &self,
        args: &serde_json::Value,
        context: &CommandContext,
    ) -> Result<CommandResult> {
        let args: SummarizeArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Invalid summarize arguments: {}", e),
            )))
        })?;

        match self.generate_summary(&args, context).await {
            Ok(output) => Ok(CommandResult {
                command_id: Uuid::new_v4(),
                success: true,
                output,
                error: None,
            }),
            Err(e) => Ok(CommandResult {
                command_id: Uuid::new_v4(),
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
            }),
        }
    }

    fn validate_args(&self, args: &serde_json::Value) -> Result<()> {
        let args: SummarizeArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Invalid summarize arguments: {}", e),
            )))
        })?;

        if args.target.trim().is_empty() {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Target cannot be empty",
            )))
            .into());
        }

        if let Some(max_lines) = args.max_lines {
            if max_lines == 0 {
                return Err(FennecError::Command(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "max_lines must be greater than 0",
                )))
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
    async fn test_summarize_command_validation() {
        let command = SummarizeCommand::new();

        // Valid args
        let valid_args = serde_json::json!({
            "target": "test.txt"
        });
        assert!(command.validate_args(&valid_args).is_ok());

        // Empty target
        let empty_target = serde_json::json!({
            "target": ""
        });
        assert!(command.validate_args(&empty_target).is_err());
    }

    #[tokio::test]
    async fn test_summarize_text_content() {
        let command = SummarizeCommand::new();

        let args = serde_json::json!({
            "target": "Hello world\nThis is a test\nWith multiple lines",
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
}
