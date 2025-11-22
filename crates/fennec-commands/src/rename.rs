use crate::action_log::Action;
use crate::registry::{CommandContext, CommandDescriptor, CommandExecutor};
use anyhow::Result;
use fennec_core::{
    command::{Capability, CommandPreview, CommandResult},
    error::FennecError,
};
use fennec_security::SandboxLevel;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameArgs {
    pub from: PathBuf,
    pub to: PathBuf,
}

pub struct RenameCommand {
    descriptor: CommandDescriptor,
}

impl RenameCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor {
                name: "rename".to_string(),
                description: "Rename files or directories".to_string(),
                version: "1.0.0".to_string(),
                author: Some("Fennec Contributors".to_string()),
                capabilities_required: vec![Capability::WriteFile],
                sandbox_level_required: SandboxLevel::WorkspaceWrite,
                supports_preview: true,
                supports_dry_run: true,
            },
        }
    }

    async fn perform_rename(&self, args: &RenameArgs, context: &CommandContext) -> Result<String> {
        let workspace_path_str = context.workspace_path.as_ref().ok_or_else(|| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No workspace path set",
            )))
        })?;
        let workspace_path = Path::new(workspace_path_str);

        // Resolve paths relative to workspace
        let from_path = if args.from.is_absolute() {
            args.from.clone()
        } else {
            workspace_path.join(&args.from)
        };

        let to_path = if args.to.is_absolute() {
            args.to.clone()
        } else {
            workspace_path.join(&args.to)
        };

        // Validate both paths are within workspace for safety
        if !from_path.starts_with(workspace_path) || !to_path.starts_with(workspace_path) {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Cannot rename files outside workspace",
            )))
            .into());
        }

        // Check if source exists
        if !from_path.exists() {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Source path does not exist: {}", from_path.display()),
            )))
            .into());
        }

        // Check if destination already exists
        if to_path.exists() {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("Destination path already exists: {}", to_path.display()),
            )))
            .into());
        }

        // Create parent directory of destination if needed
        if let Some(parent) = to_path.parent() {
            if !parent.exists() {
                if context.dry_run {
                    return Ok(format!(
                        "Would create parent directories: {}\nWould rename: {} -> {}",
                        parent.display(),
                        from_path.display(),
                        to_path.display()
                    ));
                }
                fs::create_dir_all(parent).await.map_err(|e| {
                    FennecError::Command(Box::new(std::io::Error::new(
                        e.kind(),
                        format!("Failed to create parent directories: {}", e),
                    )))
                })?;
            }
        }

        if context.dry_run {
            return Ok(format!(
                "Would rename: {} -> {}",
                from_path.display(),
                to_path.display()
            ));
        }

        // Perform the rename
        fs::rename(&from_path, &to_path).await.map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                e.kind(),
                format!("Failed to rename: {}", e),
            )))
        })?;

        // Record action to log
        if let Some(action_log) = &context.action_log {
            let action = Action::file_moved(
                "rename".to_string(),
                from_path.clone(),
                to_path.clone(),
                format!("Renamed: {} -> {}", from_path.display(), to_path.display()),
            );
            action_log.record(action).await;
        }

        Ok(format!(
            "Renamed: {} -> {}",
            from_path.display(),
            to_path.display()
        ))
    }
}

impl Default for RenameCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl CommandExecutor for RenameCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn preview(
        &self,
        args: &serde_json::Value,
        context: &CommandContext,
    ) -> Result<CommandPreview> {
        let args: RenameArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid rename arguments: {}", e),
            )))
        })?;

        let workspace_path_str = context.workspace_path.as_ref().ok_or_else(|| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No workspace path set",
            )))
        })?;
        let workspace_path = Path::new(workspace_path_str);

        let from_path = if args.from.is_absolute() {
            args.from.clone()
        } else {
            workspace_path.join(&args.from)
        };

        let to_path = if args.to.is_absolute() {
            args.to.clone()
        } else {
            workspace_path.join(&args.to)
        };

        let description = format!("Rename: {} -> {}", from_path.display(), to_path.display());

        Ok(CommandPreview {
            command_id: Uuid::new_v4(),
            description,
            actions: vec![],
            requires_approval: true,
        })
    }

    async fn execute(
        &self,
        args: &serde_json::Value,
        context: &CommandContext,
    ) -> Result<CommandResult> {
        let args: RenameArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid rename arguments: {}", e),
            )))
        })?;

        match self.perform_rename(&args, context).await {
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
        let args: RenameArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid rename arguments: {}", e),
            )))
        })?;

        if args.from.as_os_str().is_empty() {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Source path cannot be empty",
            )))
            .into());
        }

        if args.to.as_os_str().is_empty() {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Destination path cannot be empty",
            )))
            .into());
        }

        // Validate no parent directory traversal attempts
        let from_str = args.from.to_string_lossy();
        let to_str = args.to.to_string_lossy();
        if from_str.contains("..") || to_str.contains("..") {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Path traversal not allowed",
            )))
            .into());
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
    async fn test_rename_file() {
        let temp_dir = TempDir::new().unwrap();
        let from_file = temp_dir.path().join("old.txt");
        std::fs::write(&from_file, "content").unwrap();

        let command = RenameCommand::new();
        let args = serde_json::json!({
            "from": "old.txt",
            "to": "new.txt"
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

        let new_file = temp_dir.path().join("new.txt");
        assert!(new_file.exists());
        assert!(!from_file.exists());

        let content = fs::read_to_string(new_file).await.unwrap();
        assert_eq!(content, "content");
    }

    #[tokio::test]
    async fn test_rename_directory() {
        let temp_dir = TempDir::new().unwrap();
        let old_dir = temp_dir.path().join("old_dir");
        std::fs::create_dir(&old_dir).unwrap();
        std::fs::write(old_dir.join("file.txt"), "content").unwrap();

        let command = RenameCommand::new();
        let args = serde_json::json!({
            "from": "old_dir",
            "to": "new_dir"
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

        let new_dir = temp_dir.path().join("new_dir");
        assert!(new_dir.exists());
        assert!(!old_dir.exists());
        assert!(new_dir.join("file.txt").exists());
    }

    #[tokio::test]
    async fn test_rename_to_subdirectory() {
        let temp_dir = TempDir::new().unwrap();
        let from_file = temp_dir.path().join("file.txt");
        std::fs::write(&from_file, "content").unwrap();

        let command = RenameCommand::new();
        let args = serde_json::json!({
            "from": "file.txt",
            "to": "subdir/file.txt"
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

        let new_file = temp_dir.path().join("subdir/file.txt");
        assert!(new_file.exists());
        assert!(!from_file.exists());
    }

    #[tokio::test]
    async fn test_rename_nonexistent_fails() {
        let temp_dir = TempDir::new().unwrap();
        let command = RenameCommand::new();

        let args = serde_json::json!({
            "from": "nonexistent.txt",
            "to": "new.txt"
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

    #[tokio::test]
    async fn test_rename_to_existing_fails() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::write(temp_dir.path().join("old.txt"), "content1").unwrap();
        std::fs::write(temp_dir.path().join("existing.txt"), "content2").unwrap();

        let command = RenameCommand::new();
        let args = serde_json::json!({
            "from": "old.txt",
            "to": "existing.txt"
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
    async fn test_rename_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let from_file = temp_dir.path().join("old.txt");
        std::fs::write(&from_file, "content").unwrap();

        let command = RenameCommand::new();
        let args = serde_json::json!({
            "from": "old.txt",
            "to": "new.txt"
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
        assert!(result.output.contains("Would rename"));

        // File should still exist at old location
        assert!(from_file.exists());
        assert!(!temp_dir.path().join("new.txt").exists());
    }
}
