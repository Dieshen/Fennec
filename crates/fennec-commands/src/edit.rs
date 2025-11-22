use anyhow::Result;
use async_trait::async_trait;
use fennec_core::{
    command::{Capability, CommandPreview, CommandResult, PreviewAction},
    error::FennecError,
};
use fennec_security::SandboxLevel;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

use crate::file_ops::{EditStrategy, FileEditRequest, FileOperations, FileOperationsConfig};
use crate::registry::{CommandContext, CommandDescriptor, CommandExecutor};

/// Arguments for the edit command - enhanced with new edit strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditArgs {
    /// Path to the file to edit
    pub file_path: String,
    /// Edit strategy to apply
    pub strategy: EditStrategyArgs,
    /// Whether to create the file if it doesn't exist
    pub create_if_missing: Option<bool>,
    /// Whether to make a backup before editing
    pub backup: Option<bool>,
}

/// Edit strategy arguments that map to the file_ops EditStrategy enum
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum EditStrategyArgs {
    /// Replace entire file content
    Replace { content: String },
    /// Append content to the end of the file
    Append { content: String },
    /// Prepend content to the beginning of the file
    Prepend { content: String },
    /// Insert content at a specific line number (1-based)
    InsertAtLine { line_number: usize, content: String },
    /// Search and replace text within the file
    SearchReplace { search: String, replace: String },
    /// Replace content in a specific line range (1-based, inclusive)
    LineRange {
        start: usize,
        end: Option<usize>,
        content: String,
    },
}

impl From<EditStrategyArgs> for EditStrategy {
    fn from(args: EditStrategyArgs) -> Self {
        match args {
            EditStrategyArgs::Replace { content } => EditStrategy::Replace { content },
            EditStrategyArgs::Append { content } => EditStrategy::Append { content },
            EditStrategyArgs::Prepend { content } => EditStrategy::Prepend { content },
            EditStrategyArgs::InsertAtLine {
                line_number,
                content,
            } => EditStrategy::InsertAtLine {
                line_number,
                content,
            },
            EditStrategyArgs::SearchReplace { search, replace } => {
                EditStrategy::SearchReplace { search, replace }
            }
            EditStrategyArgs::LineRange {
                start,
                end,
                content,
            } => EditStrategy::LineRange {
                start,
                end,
                content,
            },
        }
    }
}

/// Enhanced edit command using the new file operations module
pub struct EditCommand {
    descriptor: CommandDescriptor,
    file_ops: FileOperations,
}

impl EditCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor {
                name: "edit".to_string(),
                description: "Edit files with various modification strategies and safety features"
                    .to_string(),
                version: "2.0.0".to_string(),
                author: Some("Fennec Core".to_string()),
                capabilities_required: vec![Capability::ReadFile, Capability::WriteFile],
                sandbox_level_required: SandboxLevel::WorkspaceWrite,
                supports_preview: true,
                supports_dry_run: true,
            },
            file_ops: FileOperations::new(FileOperationsConfig {
                backup_directory: None,
                max_file_size: 100 * 1024 * 1024, // 100MB
                detect_encoding: true,
                atomic_writes: true,
            }),
        }
    }

    pub fn with_config(config: FileOperationsConfig) -> Self {
        Self {
            descriptor: CommandDescriptor {
                name: "edit".to_string(),
                description: "Edit files with various modification strategies and safety features"
                    .to_string(),
                version: "2.0.0".to_string(),
                author: Some("Fennec Core".to_string()),
                capabilities_required: vec![Capability::ReadFile, Capability::WriteFile],
                sandbox_level_required: SandboxLevel::WorkspaceWrite,
                supports_preview: true,
                supports_dry_run: true,
            },
            file_ops: FileOperations::new(config),
        }
    }

    /// Generate a preview with actual diff content
    async fn generate_preview(
        &self,
        args: &EditArgs,
        context: &CommandContext,
    ) -> Result<CommandPreview> {
        let file_path = PathBuf::from(&args.file_path);

        // Validate file path first
        let validated_path = self
            .file_ops
            .validate_file_path(
                &file_path,
                &context.sandbox_level,
                context.workspace_path.as_deref(),
            )
            .await?;

        let mut actions = vec![PreviewAction::ReadFile {
            path: validated_path.to_string_lossy().to_string(),
        }];

        // Try to generate preview content by reading current file and applying strategy
        let preview_description = if validated_path.exists() {
            match self.file_ops.safe_read_file(&validated_path).await {
                Ok(original_content) => {
                    let strategy: EditStrategy = args.strategy.clone().into();
                    match self
                        .file_ops
                        .apply_edit_strategy(&original_content, &strategy)
                    {
                        Ok(new_content) => {
                            // Generate diff for the preview
                            match self.file_ops.generate_diff(&original_content, &new_content) {
                                Ok(diff) => {
                                    actions.push(PreviewAction::WriteFile {
                                        path: validated_path.to_string_lossy().to_string(),
                                        content: if diff.len() > 1000 {
                                            format!(
                                                "{}... (diff truncated, {} total characters)",
                                                &diff[..1000],
                                                diff.len()
                                            )
                                        } else {
                                            diff
                                        },
                                    });
                                    format!(
                                        "Edit file: {} with strategy: {:?}",
                                        args.file_path, args.strategy
                                    )
                                }
                                Err(_) => format!(
                                    "Edit file: {} (diff generation failed)",
                                    args.file_path
                                ),
                            }
                        }
                        Err(e) => format!("Edit file: {} (preview failed: {})", args.file_path, e),
                    }
                }
                Err(_) => format!(
                    "Edit file: {} (cannot read current content)",
                    args.file_path
                ),
            }
        } else if args.create_if_missing.unwrap_or(false) {
            actions.push(PreviewAction::WriteFile {
                path: validated_path.to_string_lossy().to_string(),
                content: "New file will be created".to_string(),
            });
            format!("Create new file: {}", args.file_path)
        } else {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "File {} does not exist and create_if_missing is false",
                    args.file_path
                ),
            )))
            .into());
        };

        Ok(CommandPreview {
            command_id: Uuid::new_v4(),
            description: preview_description,
            actions,
            requires_approval: true, // Always require approval for file edits
        })
    }

    /// Perform the file edit operation using the new file operations module
    async fn perform_edit(&self, args: &EditArgs, context: &CommandContext) -> Result<String> {
        // Check for cancellation
        if context.cancellation_token.is_cancelled() {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Edit operation was cancelled",
            )))
            .into());
        }

        let file_path = PathBuf::from(&args.file_path);
        let strategy: EditStrategy = args.strategy.clone().into();

        let request = FileEditRequest {
            path: file_path,
            strategy,
            create_backup: args.backup.unwrap_or(false),
            create_if_missing: args.create_if_missing.unwrap_or(false),
        };

        if context.dry_run {
            // For dry run, just validate and show what would happen
            let validated_path = self
                .file_ops
                .validate_file_path(
                    &request.path,
                    &context.sandbox_level,
                    context.workspace_path.as_deref(),
                )
                .await?;

            let original_content = if validated_path.exists() {
                self.file_ops.safe_read_file(&validated_path).await?
            } else if request.create_if_missing {
                String::new()
            } else {
                return Err(FennecError::Command(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "File {} does not exist and create_if_missing is false",
                        validated_path.display()
                    ),
                )))
                .into());
            };

            let new_content = self
                .file_ops
                .apply_edit_strategy(&original_content, &request.strategy)?;
            let diff = self
                .file_ops
                .generate_diff(&original_content, &new_content)?;

            return Ok(format!(
                "DRY RUN: Would edit file: {}\n\nDiff preview:\n{}",
                validated_path.display(),
                diff
            ));
        }

        // Perform the actual edit
        let result = self
            .file_ops
            .edit_file(
                request,
                &context.sandbox_level,
                context.workspace_path.as_deref(),
            )
            .await?;

        let mut messages = vec![
            format!("Successfully edited file: {}", args.file_path),
            format!("Bytes written: {}", result.bytes_written),
        ];

        if let Some(backup_path) = result.backup_path {
            messages.push(format!("Created backup: {}", backup_path.display()));
        }

        if !result.diff.is_empty() {
            messages.push(format!("Changes made:\n{}", result.diff));
        }

        Ok(messages.join("\n"))
    }
}

#[async_trait]
impl CommandExecutor for EditCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn preview(
        &self,
        args: &serde_json::Value,
        context: &CommandContext,
    ) -> Result<CommandPreview> {
        let args: EditArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Invalid edit arguments: {}", e),
            )))
        })?;

        self.generate_preview(&args, context).await
    }

    async fn execute(
        &self,
        args: &serde_json::Value,
        context: &CommandContext,
    ) -> Result<CommandResult> {
        let args: EditArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Invalid edit arguments: {}", e),
            )))
        })?;

        match self.perform_edit(&args, context).await {
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
        let args: EditArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Invalid edit arguments: {}", e),
            )))
        })?;

        if args.file_path.trim().is_empty() {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "File path cannot be empty",
            )))
            .into());
        }

        // Validate strategy-specific constraints
        match &args.strategy {
            EditStrategyArgs::Replace { content } => {
                if content.is_empty() {
                    return Err(FennecError::Command(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Replacement content cannot be empty (use empty string \"\" for clearing file)")))
                    .into());
                }
            }
            EditStrategyArgs::InsertAtLine { line_number, .. } => {
                if *line_number == 0 {
                    return Err(FennecError::Command(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Line numbers must be 1-based (start from 1)",
                    )))
                    .into());
                }
            }
            EditStrategyArgs::SearchReplace { search, .. } => {
                if search.is_empty() {
                    return Err(FennecError::Command(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Search string cannot be empty",
                    )))
                    .into());
                }
            }
            EditStrategyArgs::LineRange { start, end, .. } => {
                if *start == 0 {
                    return Err(FennecError::Command(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Line numbers must be 1-based (start from 1)",
                    )))
                    .into());
                }
                if let Some(end_line) = end {
                    if *end_line < *start {
                        return Err(FennecError::Command(Box::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "End line must be greater than or equal to start line",
                        )))
                        .into());
                    }
                }
            }
            _ => {} // Other strategies are always valid if they deserialize correctly
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::fs::write;
    use tokio_util::sync::CancellationToken;

    #[tokio::test]
    async fn test_edit_command_validation() {
        let command = EditCommand::new();

        // Valid args - replace strategy
        let valid_args = serde_json::json!({
            "file_path": "test.txt",
            "strategy": {
                "type": "Replace",
                "data": { "content": "Hello, world!" }
            }
        });
        assert!(command.validate_args(&valid_args).is_ok());

        // Valid args - search and replace
        let search_replace_args = serde_json::json!({
            "file_path": "test.txt",
            "strategy": {
                "type": "SearchReplace",
                "data": { "search": "old", "replace": "new" }
            }
        });
        assert!(command.validate_args(&search_replace_args).is_ok());

        // Invalid args - empty file path
        let empty_path = serde_json::json!({
            "file_path": "",
            "strategy": {
                "type": "Replace",
                "data": { "content": "test" }
            }
        });
        assert!(command.validate_args(&empty_path).is_err());

        // Invalid args - zero-based line number
        let zero_line = serde_json::json!({
            "file_path": "test.txt",
            "strategy": {
                "type": "InsertAtLine",
                "data": { "line_number": 0, "content": "test" }
            }
        });
        assert!(command.validate_args(&zero_line).is_err());

        // Invalid args - empty search string
        let empty_search = serde_json::json!({
            "file_path": "test.txt",
            "strategy": {
                "type": "SearchReplace",
                "data": { "search": "", "replace": "new" }
            }
        });
        assert!(command.validate_args(&empty_search).is_err());
    }

    #[tokio::test]
    async fn test_edit_command_execution() {
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        let initial_content = "line 1\nline 2\nline 3";

        write(&test_file, initial_content).await.unwrap();

        let command = EditCommand::new();

        // Test content replacement
        let args = serde_json::json!({
            "file_path": test_file.to_string_lossy(),
            "strategy": {
                "type": "Replace",
                "data": { "content": "New content" }
            },
            "backup": true
        });

        let context = CommandContext {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: Some(temp_dir.path().to_string_lossy().to_string()),
            sandbox_level: SandboxLevel::FullAccess,
            dry_run: false,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
            action_log: None,
        };

        let result = command.execute(&args, &context).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("Successfully edited file"));
        assert!(result.output.contains("Created backup"));

        let new_content = tokio::fs::read_to_string(&test_file).await.unwrap();
        assert_eq!(new_content, "New content");
    }

    #[tokio::test]
    async fn test_search_and_replace() {
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        let initial_content = "Hello world\nGoodbye world";

        write(&test_file, initial_content).await.unwrap();

        let command = EditCommand::new();

        let args = serde_json::json!({
            "file_path": test_file.to_string_lossy(),
            "strategy": {
                "type": "SearchReplace",
                "data": { "search": "world", "replace": "universe" }
            }
        });

        let context = CommandContext {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: Some(temp_dir.path().to_string_lossy().to_string()),
            sandbox_level: SandboxLevel::FullAccess,
            dry_run: false,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
            action_log: None,
        };

        let result = command.execute(&args, &context).await.unwrap();
        assert!(result.success);

        let new_content = tokio::fs::read_to_string(&test_file).await.unwrap();
        assert_eq!(new_content, "Hello universe\nGoodbye universe");
    }

    #[tokio::test]
    async fn test_append_strategy() {
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        let initial_content = "line 1\nline 2";

        write(&test_file, initial_content).await.unwrap();

        let command = EditCommand::new();

        let args = serde_json::json!({
            "file_path": test_file.to_string_lossy(),
            "strategy": {
                "type": "Append",
                "data": { "content": "line 3" }
            }
        });

        let context = CommandContext {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: Some(temp_dir.path().to_string_lossy().to_string()),
            sandbox_level: SandboxLevel::FullAccess,
            dry_run: false,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
            action_log: None,
        };

        let result = command.execute(&args, &context).await.unwrap();
        assert!(result.success);

        let new_content = tokio::fs::read_to_string(&test_file).await.unwrap();
        assert_eq!(new_content, "line 1\nline 2\nline 3");
    }

    #[tokio::test]
    async fn test_dry_run() {
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        let initial_content = "line 1\nline 2\nline 3";

        write(&test_file, initial_content).await.unwrap();

        let command = EditCommand::new();

        let args = serde_json::json!({
            "file_path": test_file.to_string_lossy(),
            "strategy": {
                "type": "SearchReplace",
                "data": { "search": "line 2", "replace": "modified line 2" }
            }
        });

        let context = CommandContext {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: Some(temp_dir.path().to_string_lossy().to_string()),
            sandbox_level: SandboxLevel::FullAccess,
            dry_run: true,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
            action_log: None,
        };

        let result = command.execute(&args, &context).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("DRY RUN"));
        assert!(result.output.contains("-line 2"));
        assert!(result.output.contains("+modified line 2"));

        // File should remain unchanged
        let unchanged_content = tokio::fs::read_to_string(&test_file).await.unwrap();
        assert_eq!(unchanged_content, initial_content);
    }
}
