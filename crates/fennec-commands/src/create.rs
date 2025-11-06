use anyhow::Result;
use crate::registry::{CommandContext, CommandDescriptor, CommandExecutor};
use fennec_core::{command::{Capability, CommandPreview, CommandResult}, error::FennecError};
use fennec_security::SandboxLevel;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateArgs {
    pub path: PathBuf,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub is_directory: bool,
}

pub struct CreateCommand {
    descriptor: CommandDescriptor,
}

impl CreateCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor {
                name: "create".to_string(),
                description: "Create new files or directories".to_string(),
                version: "1.0.0".to_string(),
                author: Some("Fennec Contributors".to_string()),
                capabilities_required: vec![Capability::WriteFile],
                sandbox_level_required: SandboxLevel::WorkspaceWrite,
                supports_preview: true,
                supports_dry_run: true,
            },
        }
    }

    async fn perform_create(&self, args: &CreateArgs, context: &CommandContext) -> Result<String> {
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
                "Cannot create files outside workspace"
            ))).into());
        }

        // Check if path already exists
        if target_path.exists() {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("Path already exists: {}", target_path.display())
            ))).into());
        }

        // Create parent directories if they don't exist
        if let Some(parent) = target_path.parent() {
            if !parent.exists() {
                if context.dry_run {
                    return Ok(format!(
                        "Would create parent directories: {}\n{}",
                        parent.display(),
                        if args.is_directory {
                            format!("Would create directory: {}", target_path.display())
                        } else {
                            format!("Would create file: {}", target_path.display())
                        }
                    ));
                }
                fs::create_dir_all(parent).await.map_err(|e| {
                    FennecError::Command(Box::new(std::io::Error::new(
                        e.kind(),
                        format!("Failed to create parent directories: {}", e)
                    )))
                })?;
            }
        }

        if context.dry_run {
            return Ok(format!(
                "{}",
                if args.is_directory {
                    format!("Would create directory: {}", target_path.display())
                } else {
                    format!("Would create file: {} with {} bytes",
                        target_path.display(),
                        args.content.as_ref().map(|c| c.len()).unwrap_or(0))
                }
            ));
        }

        // Create the file or directory
        if args.is_directory {
            fs::create_dir(&target_path).await.map_err(|e| {
                FennecError::Command(Box::new(std::io::Error::new(
                    e.kind(),
                    format!("Failed to create directory: {}", e)
                )))
            })?;
            Ok(format!("Created directory: {}", target_path.display()))
        } else {
            let content = args.content.as_deref().unwrap_or("");
            fs::write(&target_path, content).await.map_err(|e| {
                FennecError::Command(Box::new(std::io::Error::new(
                    e.kind(),
                    format!("Failed to create file: {}", e)
                )))
            })?;
            Ok(format!("Created file: {} ({} bytes)", target_path.display(), content.len()))
        }
    }
}

impl Default for CreateCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl CommandExecutor for CreateCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn preview(&self, args: &serde_json::Value, context: &CommandContext) -> Result<CommandPreview> {
        let args: CreateArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid create arguments: {}", e)
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

        let description = if args.is_directory {
            format!("Create directory: {}", target_path.display())
        } else {
            format!("Create file: {} ({} bytes)",
                target_path.display(),
                args.content.as_ref().map(|c| c.len()).unwrap_or(0))
        };

        Ok(CommandPreview {
            command_id: Uuid::new_v4(),
            description,
            actions: vec![],
            requires_approval: true,
        })
    }

    async fn execute(&self, args: &serde_json::Value, context: &CommandContext) -> Result<CommandResult> {
        let args: CreateArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid create arguments: {}", e)
            )))
        })?;

        match self.perform_create(&args, context).await {
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
        let args: CreateArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid create arguments: {}", e)
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
    async fn test_create_file() {
        let temp_dir = TempDir::new().unwrap();
        let command = CreateCommand::new();

        let args = serde_json::json!({
            "path": "test.txt",
            "content": "Hello, World!",
            "is_directory": false
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

        let created_file = temp_dir.path().join("test.txt");
        assert!(created_file.exists());

        let content = fs::read_to_string(created_file).await.unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[tokio::test]
    async fn test_create_directory() {
        let temp_dir = TempDir::new().unwrap();
        let command = CreateCommand::new();

        let args = serde_json::json!({
            "path": "test_dir",
            "is_directory": true
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

        let created_dir = temp_dir.path().join("test_dir");
        assert!(created_dir.exists());
        assert!(created_dir.is_dir());
    }

    #[tokio::test]
    async fn test_create_with_parent_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let command = CreateCommand::new();

        let args = serde_json::json!({
            "path": "nested/dir/test.txt",
            "content": "Nested file",
            "is_directory": false
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

        let created_file = temp_dir.path().join("nested/dir/test.txt");
        assert!(created_file.exists());
    }

    #[tokio::test]
    async fn test_create_existing_file_fails() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("existing.txt");
        std::fs::write(&test_file, "existing").unwrap();

        let command = CreateCommand::new();
        let args = serde_json::json!({
            "path": "existing.txt",
            "content": "new",
            "is_directory": false
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
        assert!(result.error.unwrap().contains("already exists"));
    }

    #[tokio::test]
    async fn test_create_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let command = CreateCommand::new();

        let args = serde_json::json!({
            "path": "dry_run_test.txt",
            "content": "Should not be created",
            "is_directory": false
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
        assert!(result.output.contains("Would create file"));

        let test_file = temp_dir.path().join("dry_run_test.txt");
        assert!(!test_file.exists());
    }
}
