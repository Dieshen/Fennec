use anyhow::Result;
use async_trait::async_trait;
use chrono;
use fennec_core::{
    command::{Capability, CommandPreview, CommandResult, PreviewAction},
    error::FennecError,
};
use fennec_security::SandboxLevel;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;
use uuid::Uuid;

use crate::registry::{CommandContext, CommandDescriptor, CommandExecutor};

/// Arguments for the edit command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditArgs {
    /// Path to the file to edit
    pub file_path: String,
    /// Content to write to the file
    pub content: Option<String>,
    /// Line number to start editing from (1-based)
    pub line_start: Option<usize>,
    /// Line number to end editing at (1-based, inclusive)
    pub line_end: Option<usize>,
    /// Text to search for and replace
    pub search: Option<String>,
    /// Replacement text
    pub replace: Option<String>,
    /// Whether to create the file if it doesn't exist
    pub create_if_missing: Option<bool>,
    /// Whether to make a backup before editing
    pub backup: Option<bool>,
}

/// Edit command for modifying files
pub struct EditCommand {
    descriptor: CommandDescriptor,
}

impl EditCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor {
                name: "edit".to_string(),
                description: "Edit files with various modification strategies".to_string(),
                version: "1.0.0".to_string(),
                author: Some("Fennec Core".to_string()),
                capabilities_required: vec![Capability::ReadFile, Capability::WriteFile],
                sandbox_level_required: SandboxLevel::WorkspaceWrite,
                supports_preview: true,
                supports_dry_run: true,
            },
        }
    }

    /// Check if the file path is safe to edit
    fn validate_file_path(&self, file_path: &str, context: &CommandContext) -> Result<()> {
        let path = Path::new(file_path);

        // Convert to absolute path
        let abs_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()?.join(path)
        };

        // Security checks based on sandbox level
        match context.sandbox_level {
            SandboxLevel::ReadOnly => {
                return Err(FennecError::Security {
                    message: "Cannot edit files in read-only mode".to_string(),
                }
                .into());
            }
            SandboxLevel::WorkspaceWrite => {
                // Only allow editing within the current workspace
                let current_dir = std::env::current_dir()?;
                if !abs_path.starts_with(&current_dir) {
                    return Err(FennecError::Security {
                        message: "Cannot edit files outside the current workspace".to_string(),
                    }
                    .into());
                }
            }
            SandboxLevel::FullAccess => {
                // Full access - no restrictions
            }
        }

        // Prevent editing sensitive system files
        let dangerous_paths = [
            "/etc",
            "/usr",
            "/sys",
            "/proc",
            "/dev",
            "C:\\Windows",
            "C:\\Program Files",
            "C:\\System32",
        ];

        for dangerous in &dangerous_paths {
            if abs_path.starts_with(dangerous) {
                return Err(FennecError::Security {
                    message: format!("Cannot edit files in system directory: {}", dangerous),
                }
                .into());
            }
        }

        Ok(())
    }

    /// Perform the file edit operation
    async fn perform_edit(&self, args: &EditArgs, context: &CommandContext) -> Result<String> {
        self.validate_file_path(&args.file_path, context)?;

        let path = Path::new(&args.file_path);
        let mut result_messages = Vec::new();

        // Check for cancellation
        if context.cancellation_token.is_cancelled() {
            return Err(FennecError::Command {
                message: "Edit operation was cancelled".to_string(),
            }
            .into());
        }

        // Read existing content if file exists
        let existing_content = if path.exists() {
            fs::read_to_string(path)
                .await
                .map_err(|e| FennecError::Command {
                    message: format!("Failed to read file {}: {}", args.file_path, e),
                })?
        } else if args.create_if_missing.unwrap_or(false) {
            // Create parent directories if needed
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .await
                    .map_err(|e| FennecError::Command {
                        message: format!("Failed to create directory {}: {}", parent.display(), e),
                    })?;
            }
            String::new()
        } else {
            return Err(FennecError::Command {
                message: format!(
                    "File {} does not exist and create_if_missing is false",
                    args.file_path
                ),
            }
            .into());
        };

        // Create backup if requested
        if args.backup.unwrap_or(false) && path.exists() {
            let backup_path = format!(
                "{}.backup.{}",
                args.file_path,
                chrono::Utc::now().timestamp()
            );
            if !context.dry_run {
                fs::copy(path, &backup_path)
                    .await
                    .map_err(|e| FennecError::Command {
                        message: format!("Failed to create backup: {}", e),
                    })?;
            }
            result_messages.push(format!("Created backup: {}", backup_path));
        }

        let new_content = if let Some(content) = &args.content {
            // Replace entire file content
            result_messages.push("Replaced entire file content".to_string());
            content.clone()
        } else if let (Some(search), Some(replace)) = (&args.search, &args.replace) {
            // Search and replace
            let new_content = existing_content.replace(search, replace);
            let count = existing_content.matches(search).count();
            result_messages.push(format!("Replaced {} occurrences of '{}'", count, search));
            new_content
        } else if let (Some(line_start), line_end) = (args.line_start, args.line_end) {
            // Line-based editing
            let lines: Vec<&str> = existing_content.lines().collect();
            let start_idx = line_start.saturating_sub(1); // Convert to 0-based
            let end_idx = line_end.unwrap_or(line_start).saturating_sub(1);

            if start_idx >= lines.len() {
                return Err(FennecError::Command {
                    message: format!(
                        "Line {} is beyond file length ({})",
                        line_start,
                        lines.len()
                    ),
                }
                .into());
            }

            let replacement = args.content.as_deref().unwrap_or("");
            let mut new_lines = lines[..start_idx].to_vec();
            new_lines.push(replacement);
            new_lines.extend_from_slice(&lines[(end_idx + 1)..]);

            result_messages.push(format!("Edited lines {} to {}", line_start, end_idx + 1));
            new_lines.join("\n")
        } else {
            return Err(FennecError::Command {
                message: "Must specify either content, search/replace, or line range".to_string(),
            }
            .into());
        };

        // Write the new content
        if !context.dry_run {
            fs::write(path, &new_content)
                .await
                .map_err(|e| FennecError::Command {
                    message: format!("Failed to write file {}: {}", args.file_path, e),
                })?;
        }

        result_messages.push(format!(
            "Successfully {} file: {}",
            if context.dry_run {
                "would edit"
            } else {
                "edited"
            },
            args.file_path
        ));

        Ok(result_messages.join("\n"))
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
        let args: EditArgs =
            serde_json::from_value(args.clone()).map_err(|e| FennecError::Command {
                message: format!("Invalid edit arguments: {}", e),
            })?;

        self.validate_file_path(&args.file_path, context)?;

        let mut actions = vec![PreviewAction::ReadFile {
            path: args.file_path.clone(),
        }];

        if let Some(content) = &args.content {
            actions.push(PreviewAction::WriteFile {
                path: args.file_path.clone(),
                content: if content.len() > 200 {
                    format!(
                        "{}... ({} characters total)",
                        &content[..200],
                        content.len()
                    )
                } else {
                    content.clone()
                },
            });
        }

        Ok(CommandPreview {
            command_id: Uuid::new_v4(),
            description: format!("Edit file: {}", args.file_path),
            actions,
            requires_approval: true, // File edits should require approval
        })
    }

    async fn execute(
        &self,
        args: &serde_json::Value,
        context: &CommandContext,
    ) -> Result<CommandResult> {
        let args: EditArgs =
            serde_json::from_value(args.clone()).map_err(|e| FennecError::Command {
                message: format!("Invalid edit arguments: {}", e),
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
        let args: EditArgs =
            serde_json::from_value(args.clone()).map_err(|e| FennecError::Command {
                message: format!("Invalid edit arguments: {}", e),
            })?;

        if args.file_path.trim().is_empty() {
            return Err(FennecError::Command {
                message: "File path cannot be empty".to_string(),
            }
            .into());
        }

        // Validate that we have a valid edit operation
        let has_content = args.content.is_some();
        let has_search_replace = args.search.is_some() && args.replace.is_some();
        let has_line_range = args.line_start.is_some();

        if !(has_content || has_search_replace || has_line_range) {
            return Err(FennecError::Command {
                message: "Must specify content, search/replace pair, or line range".to_string(),
            }
            .into());
        }

        // Validate line numbers
        if let Some(line_start) = args.line_start {
            if line_start == 0 {
                return Err(FennecError::Command {
                    message: "Line numbers must be 1-based (start from 1)".to_string(),
                }
                .into());
            }

            if let Some(line_end) = args.line_end {
                if line_end < line_start {
                    return Err(FennecError::Command {
                        message: "Line end must be greater than or equal to line start".to_string(),
                    }
                    .into());
                }
            }
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

        // Valid args - content replacement
        let valid_args = serde_json::json!({
            "file_path": "test.txt",
            "content": "Hello, world!"
        });
        assert!(command.validate_args(&valid_args).is_ok());

        // Valid args - search and replace
        let search_replace_args = serde_json::json!({
            "file_path": "test.txt",
            "search": "old",
            "replace": "new"
        });
        assert!(command.validate_args(&search_replace_args).is_ok());

        // Invalid args - empty file path
        let empty_path = serde_json::json!({
            "file_path": "",
            "content": "test"
        });
        assert!(command.validate_args(&empty_path).is_err());

        // Invalid args - no edit operation
        let no_operation = serde_json::json!({
            "file_path": "test.txt"
        });
        assert!(command.validate_args(&no_operation).is_err());

        // Invalid args - zero-based line number
        let zero_line = serde_json::json!({
            "file_path": "test.txt",
            "line_start": 0,
            "content": "test"
        });
        assert!(command.validate_args(&zero_line).is_err());
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
            "content": "New content"
        });

        let context = CommandContext {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: Some(temp_dir.path().to_string_lossy().to_string()),
            sandbox_level: SandboxLevel::FullAccess, // Use full access for test
            dry_run: false,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
        };

        let result = command.execute(&args, &context).await.unwrap();
        assert!(result.success);

        let new_content = fs::read_to_string(&test_file).await.unwrap();
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
            "search": "world",
            "replace": "universe"
        });

        let context = CommandContext {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: Some(temp_dir.path().to_string_lossy().to_string()),
            sandbox_level: SandboxLevel::FullAccess,
            dry_run: false,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
        };

        let result = command.execute(&args, &context).await.unwrap();
        assert!(result.success);

        let new_content = fs::read_to_string(&test_file).await.unwrap();
        assert_eq!(new_content, "Hello universe\nGoodbye universe");
    }
}
