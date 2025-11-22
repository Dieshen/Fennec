use anyhow::Result;
use crate::action_log::Action;
use crate::registry::{CommandContext, CommandDescriptor, CommandExecutor};
use fennec_core::{command::{Capability, CommandPreview, CommandResult}, error::FennecError};
use fennec_security::SandboxLevel;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteArgs {
    pub path: PathBuf,
    #[serde(default)]
    pub recursive: bool,
    #[serde(default = "default_confirm")]
    pub confirm: bool,
}

fn default_confirm() -> bool {
    true
}

pub struct DeleteCommand {
    descriptor: CommandDescriptor,
}

impl DeleteCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor {
                name: "delete".to_string(),
                description: "Delete files or directories with safety checks".to_string(),
                version: "1.0.0".to_string(),
                author: Some("Fennec Contributors".to_string()),
                capabilities_required: vec![Capability::WriteFile],
                sandbox_level_required: SandboxLevel::WorkspaceWrite,
                supports_preview: true,
                supports_dry_run: true,
            },
        }
    }

    fn is_protected_path(path: &Path) -> bool {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            // Protect git, cargo, and other critical directories/files
            matches!(name, ".git" | ".gitignore" | "Cargo.toml" | "Cargo.lock" | "package.json" | "package-lock.json")
        } else {
            false
        }
    }

    async fn perform_delete(&self, args: &DeleteArgs, context: &CommandContext) -> Result<String> {
        let workspace_path_str = context.workspace_path.as_ref().ok_or_else(|| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No workspace path set"
            )))
        })?;
        let workspace_path = Path::new(workspace_path_str);

        // Resolve the target path relative to workspace
        let target_path = if args.path.is_absolute() {
            args.path.clone()
        } else {
            workspace_path.join(&args.path)
        };

        // Validate path is within workspace for safety
        if !target_path.starts_with(workspace_path) {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Cannot delete files outside workspace"
            ))).into());
        }

        // Check if path exists
        if !target_path.exists() {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Path does not exist: {}", target_path.display())
            ))).into());
        }

        // Safety check: don't delete critical paths
        if Self::is_protected_path(&target_path) {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!("Cannot delete protected path: {}", target_path.display())
            ))).into());
        }

        let is_dir = target_path.is_dir();

        // Require confirmation for directory deletion
        if is_dir && !args.confirm {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Directory deletion requires confirmation"
            ))).into());
        }

        // Check if recursive flag is needed for non-empty directories
        if is_dir && !args.recursive {
            let mut entries = fs::read_dir(&target_path).await.map_err(|e| {
                FennecError::Command(Box::new(std::io::Error::new(
                    e.kind(),
                    format!("Failed to read directory: {}", e)
                )))
            })?;

            if entries.next_entry().await.map_err(|e| {
                FennecError::Command(Box::new(std::io::Error::new(
                    e.kind(),
                    format!("Failed to check directory contents: {}", e)
                )))
            })?.is_some() {
                return Err(FennecError::Command(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Directory is not empty; use recursive flag to delete"
                ))).into());
            }
        }

        if context.dry_run {
            return Ok(format!(
                "Would delete {}: {}",
                if is_dir { "directory" } else { "file" },
                target_path.display()
            ));
        }

        // Perform the deletion
        let result = if is_dir {
            if args.recursive {
                fs::remove_dir_all(&target_path).await.map_err(|e| {
                    FennecError::Command(Box::new(std::io::Error::new(
                        e.kind(),
                        format!("Failed to delete directory: {}", e)
                    )))
                })?;
            } else {
                fs::remove_dir(&target_path).await.map_err(|e| {
                    FennecError::Command(Box::new(std::io::Error::new(
                        e.kind(),
                        format!("Failed to delete directory: {}", e)
                    )))
                })?;
            }

            // Record action to log (simplified - not storing directory contents for now)
            if let Some(action_log) = &context.action_log {
                let action = Action::file_deleted(
                    "delete".to_string(),
                    target_path.clone(),
                    Vec::new(), // Empty content for directories
                    format!("Deleted directory: {}", target_path.display()),
                );
                action_log.record(action).await;
            }

            format!("Deleted directory: {}", target_path.display())
        } else {
            // Read file content before deleting for undo capability
            let content = fs::read(&target_path).await.map_err(|e| {
                FennecError::Command(Box::new(std::io::Error::new(
                    e.kind(),
                    format!("Failed to read file before deletion: {}", e)
                )))
            })?;

            fs::remove_file(&target_path).await.map_err(|e| {
                FennecError::Command(Box::new(std::io::Error::new(
                    e.kind(),
                    format!("Failed to delete file: {}", e)
                )))
            })?;

            // Record action to log with content for restore capability
            if let Some(action_log) = &context.action_log {
                let action = Action::file_deleted(
                    "delete".to_string(),
                    target_path.clone(),
                    content,
                    format!("Deleted file: {}", target_path.display()),
                );
                action_log.record(action).await;
            }

            format!("Deleted file: {}", target_path.display())
        };

        Ok(result)
    }
}

impl Default for DeleteCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl CommandExecutor for DeleteCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn preview(&self, args: &serde_json::Value, context: &CommandContext) -> Result<CommandPreview> {
        let args: DeleteArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid delete arguments: {}", e)
            )))
        })?;

        let workspace_path_str = context.workspace_path.as_ref().ok_or_else(|| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No workspace path set"
            )))
        })?;
        let workspace_path = Path::new(workspace_path_str);

        let target_path = if args.path.is_absolute() {
            args.path.clone()
        } else {
            workspace_path.join(&args.path)
        };

        let is_dir = target_path.is_dir();
        let description = format!(
            "Delete {}{}: {}",
            if is_dir { "directory" } else { "file" },
            if args.recursive { " (recursive)" } else { "" },
            target_path.display()
        );

        Ok(CommandPreview {
            command_id: Uuid::new_v4(),
            description,
            actions: vec![],
            requires_approval: true,
        })
    }

    async fn execute(&self, args: &serde_json::Value, context: &CommandContext) -> Result<CommandResult> {
        let args: DeleteArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid delete arguments: {}", e)
            )))
        })?;

        match self.perform_delete(&args, context).await {
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
        let args: DeleteArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid delete arguments: {}", e)
            )))
        })?;

        if args.path.as_os_str().is_empty() {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Path cannot be empty"
            ))).into());
        }

        // Validate no parent directory traversal attempts
        let path_str = args.path.to_string_lossy();
        if path_str.contains("..") {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Path traversal not allowed"
            ))).into());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio_util::sync::CancellationToken;

    #[tokio::test]
    async fn test_delete_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, "content").unwrap();

        let command = DeleteCommand::new();
        let args = serde_json::json!({
            "path": "test.txt",
            "recursive": false,
            "confirm": true
        });

        let context = CommandContext {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: Some(temp_dir.path().to_string_lossy().to_string()),
            sandbox_level: SandboxLevel::WorkspaceWrite,
            dry_run: false,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
            action_log: None,
        };

        let result = command.execute(&args, &context).await.unwrap();
        assert!(result.success);
        assert!(!test_file.exists());
    }

    #[tokio::test]
    async fn test_delete_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let test_dir = temp_dir.path().join("empty_dir");
        std::fs::create_dir(&test_dir).unwrap();

        let command = DeleteCommand::new();
        let args = serde_json::json!({
            "path": "empty_dir",
            "recursive": false,
            "confirm": true
        });

        let context = CommandContext {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: Some(temp_dir.path().to_string_lossy().to_string()),
            sandbox_level: SandboxLevel::WorkspaceWrite,
            dry_run: false,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
            action_log: None,
        };

        let result = command.execute(&args, &context).await.unwrap();
        assert!(result.success);
        assert!(!test_dir.exists());
    }

    #[tokio::test]
    async fn test_delete_directory_recursive() {
        let temp_dir = TempDir::new().unwrap();
        let test_dir = temp_dir.path().join("test_dir");
        std::fs::create_dir(&test_dir).unwrap();
        std::fs::write(test_dir.join("file.txt"), "content").unwrap();
        std::fs::create_dir(test_dir.join("subdir")).unwrap();

        let command = DeleteCommand::new();
        let args = serde_json::json!({
            "path": "test_dir",
            "recursive": true,
            "confirm": true
        });

        let context = CommandContext {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: Some(temp_dir.path().to_string_lossy().to_string()),
            sandbox_level: SandboxLevel::WorkspaceWrite,
            dry_run: false,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
            action_log: None,
        };

        let result = command.execute(&args, &context).await.unwrap();
        assert!(result.success);
        assert!(!test_dir.exists());
    }

    #[tokio::test]
    async fn test_delete_nonempty_dir_without_recursive_fails() {
        let temp_dir = TempDir::new().unwrap();
        let test_dir = temp_dir.path().join("test_dir");
        std::fs::create_dir(&test_dir).unwrap();
        std::fs::write(test_dir.join("file.txt"), "content").unwrap();

        let command = DeleteCommand::new();
        let args = serde_json::json!({
            "path": "test_dir",
            "recursive": false,
            "confirm": true
        });

        let context = CommandContext {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: Some(temp_dir.path().to_string_lossy().to_string()),
            sandbox_level: SandboxLevel::WorkspaceWrite,
            dry_run: false,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
            action_log: None,
        };

        let result = command.execute(&args, &context).await.unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap().contains("not empty"));
        assert!(test_dir.exists());
    }

    #[tokio::test]
    async fn test_delete_directory_without_confirm_fails() {
        let temp_dir = TempDir::new().unwrap();
        let test_dir = temp_dir.path().join("test_dir");
        std::fs::create_dir(&test_dir).unwrap();

        let command = DeleteCommand::new();
        let args = serde_json::json!({
            "path": "test_dir",
            "recursive": false,
            "confirm": false
        });

        let context = CommandContext {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: Some(temp_dir.path().to_string_lossy().to_string()),
            sandbox_level: SandboxLevel::WorkspaceWrite,
            dry_run: false,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
            action_log: None,
        };

        let result = command.execute(&args, &context).await.unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap().contains("requires confirmation"));
        assert!(test_dir.exists());
    }

    #[tokio::test]
    async fn test_delete_protected_path_fails() {
        let temp_dir = TempDir::new().unwrap();
        let git_dir = temp_dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();

        let command = DeleteCommand::new();
        let args = serde_json::json!({
            "path": ".git",
            "recursive": true,
            "confirm": true
        });

        let context = CommandContext {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: Some(temp_dir.path().to_string_lossy().to_string()),
            sandbox_level: SandboxLevel::WorkspaceWrite,
            dry_run: false,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
            action_log: None,
        };

        let result = command.execute(&args, &context).await.unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap().contains("protected"));
        assert!(git_dir.exists());
    }

    #[tokio::test]
    async fn test_delete_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, "content").unwrap();

        let command = DeleteCommand::new();
        let args = serde_json::json!({
            "path": "test.txt",
            "recursive": false,
            "confirm": true
        });

        let context = CommandContext {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: Some(temp_dir.path().to_string_lossy().to_string()),
            sandbox_level: SandboxLevel::WorkspaceWrite,
            dry_run: true,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
            action_log: None,
        };

        let result = command.execute(&args, &context).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("Would delete"));
        assert!(test_file.exists());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_fails() {
        let temp_dir = TempDir::new().unwrap();
        let command = DeleteCommand::new();

        let args = serde_json::json!({
            "path": "nonexistent.txt",
            "recursive": false,
            "confirm": true
        });

        let context = CommandContext {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: Some(temp_dir.path().to_string_lossy().to_string()),
            sandbox_level: SandboxLevel::WorkspaceWrite,
            dry_run: false,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
            action_log: None,
        };

        let result = command.execute(&args, &context).await.unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap().contains("does not exist"));
    }
}
