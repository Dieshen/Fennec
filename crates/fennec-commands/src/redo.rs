use anyhow::Result;
use crate::action_log::{ActionLog, ActionState};
use crate::registry::{CommandContext, CommandDescriptor, CommandExecutor};
use fennec_core::{command::{Capability, CommandPreview, CommandResult}, error::FennecError};
use fennec_security::SandboxLevel;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::fs;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedoArgs {
    #[serde(default = "default_count")]
    pub count: usize,
}

fn default_count() -> usize {
    1
}

pub struct RedoCommand {
    descriptor: CommandDescriptor,
    action_log: Arc<ActionLog>,
}

impl RedoCommand {
    pub fn new(action_log: Arc<ActionLog>) -> Self {
        Self {
            descriptor: CommandDescriptor {
                name: "redo".to_string(),
                description: "Redo the last undone file operation".to_string(),
                version: "1.0.0".to_string(),
                author: Some("Fennec Contributors".to_string()),
                capabilities_required: vec![Capability::WriteFile, Capability::ReadFile],
                sandbox_level_required: SandboxLevel::WorkspaceWrite,
                supports_preview: true,
                supports_dry_run: true,
            },
            action_log,
        }
    }

    async fn perform_redo(&self, args: &RedoArgs, context: &CommandContext) -> Result<String> {
        let workspace_path_str = context.workspace_path.as_ref().ok_or_else(|| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No workspace path set"
            )))
        })?;
        let workspace_path = std::path::Path::new(workspace_path_str);

        let mut redone_actions = Vec::new();

        for _ in 0..args.count {
            if !self.action_log.can_redo().await {
                break;
            }

            let action = self.action_log.redo().await?.ok_or_else(|| {
                FennecError::Command(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "No actions to redo"
                )))
            })?;

            if context.dry_run {
                redone_actions.push(format!("Would redo: {}", action.description));
                continue;
            }

            // Apply the forward action (state_after)
            match &action.state_after {
                ActionState::FileCreated { path } => {
                    // Remove the file (if redo is creating it, it means undo deleted it)
                    if let ActionState::FileDeleted { content, .. } = &action.state_before {
                        let full_path = if path.is_absolute() {
                            path.clone()
                        } else {
                            workspace_path.join(path)
                        };

                        if let Some(parent) = full_path.parent() {
                            fs::create_dir_all(parent).await.map_err(|e| {
                                FennecError::Command(Box::new(std::io::Error::new(
                                    e.kind(),
                                    format!("Failed to create parent directories: {}", e)
                                )))
                            })?;
                        }

                        fs::write(&full_path, content).await.map_err(|e| {
                            FennecError::Command(Box::new(std::io::Error::new(
                                e.kind(),
                                format!("Failed to create file: {}", e)
                            )))
                        })?;
                    }
                }
                ActionState::FileModified { path, content, .. } => {
                    // Restore the modified content
                    let full_path = if path.is_absolute() {
                        path.clone()
                    } else {
                        workspace_path.join(path)
                    };

                    fs::write(&full_path, content).await.map_err(|e| {
                        FennecError::Command(Box::new(std::io::Error::new(
                            e.kind(),
                            format!("Failed to restore file content: {}", e)
                        )))
                    })?;
                }
                ActionState::FileDeleted { path, .. } => {
                    // Delete the file
                    let full_path = if path.is_absolute() {
                        path.clone()
                    } else {
                        workspace_path.join(path)
                    };

                    if full_path.exists() {
                        fs::remove_file(&full_path).await.map_err(|e| {
                            FennecError::Command(Box::new(std::io::Error::new(
                                e.kind(),
                                format!("Failed to remove file: {}", e)
                            )))
                        })?;
                    }
                }
                ActionState::FileMoved { from, to } => {
                    // Apply the move
                    let from_full = if from.is_absolute() {
                        from.clone()
                    } else {
                        workspace_path.join(from)
                    };
                    let to_full = if to.is_absolute() {
                        to.clone()
                    } else {
                        workspace_path.join(to)
                    };

                    if from_full.exists() {
                        if let Some(parent) = to_full.parent() {
                            fs::create_dir_all(parent).await.map_err(|e| {
                                FennecError::Command(Box::new(std::io::Error::new(
                                    e.kind(),
                                    format!("Failed to create parent directories: {}", e)
                                )))
                            })?;
                        }

                        fs::rename(&from_full, &to_full).await.map_err(|e| {
                            FennecError::Command(Box::new(std::io::Error::new(
                                e.kind(),
                                format!("Failed to rename: {}", e)
                            )))
                        })?;
                    }
                }
                ActionState::DirectoryCreated { path } => {
                    // Create the directory
                    let full_path = if path.is_absolute() {
                        path.clone()
                    } else {
                        workspace_path.join(path)
                    };

                    fs::create_dir_all(&full_path).await.map_err(|e| {
                        FennecError::Command(Box::new(std::io::Error::new(
                            e.kind(),
                            format!("Failed to create directory: {}", e)
                        )))
                    })?;
                }
                ActionState::DirectoryDeleted { path, .. } => {
                    // Delete the directory
                    let full_path = if path.is_absolute() {
                        path.clone()
                    } else {
                        workspace_path.join(path)
                    };

                    if full_path.exists() {
                        fs::remove_dir_all(&full_path).await.map_err(|e| {
                            FennecError::Command(Box::new(std::io::Error::new(
                                e.kind(),
                                format!("Failed to remove directory: {}", e)
                            )))
                        })?;
                    }
                }
            }

            redone_actions.push(format!("Redid: {}", action.description));
        }

        if redone_actions.is_empty() {
            Ok("No actions to redo".to_string())
        } else {
            Ok(redone_actions.join("\n"))
        }
    }
}

impl Default for RedoCommand {
    fn default() -> Self {
        Self::new(Arc::new(ActionLog::new()))
    }
}

#[async_trait::async_trait]
impl CommandExecutor for RedoCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn preview(&self, args: &serde_json::Value, _context: &CommandContext) -> Result<CommandPreview> {
        let args: RedoArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid redo arguments: {}", e)
            )))
        })?;

        let can_redo = self.action_log.can_redo_count().await;
        let count = args.count.min(can_redo);

        let description = if count == 0 {
            "No actions to redo".to_string()
        } else if count == 1 {
            "Redo last undone action".to_string()
        } else {
            format!("Redo last {} undone actions", count)
        };

        Ok(CommandPreview {
            command_id: Uuid::new_v4(),
            description,
            actions: vec![],
            requires_approval: true,
        })
    }

    async fn execute(&self, args: &serde_json::Value, context: &CommandContext) -> Result<CommandResult> {
        let args: RedoArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid redo arguments: {}", e)
            )))
        })?;

        match self.perform_redo(&args, context).await {
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
        let args: RedoArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid redo arguments: {}", e)
            )))
        })?;

        if args.count == 0 {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Count must be greater than 0"
            ))).into());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action_log::Action;
    use tempfile::TempDir;
    use tokio_util::sync::CancellationToken;

    #[tokio::test]
    async fn test_redo_file_creation() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, "content").unwrap();

        let action_log = Arc::new(ActionLog::new());
        let action = Action::file_created(
            "create".to_string(),
            test_file.clone(),
            "Created test.txt".to_string(),
        );
        action_log.record(action).await;

        // Undo first
        action_log.undo().await.unwrap();

        let command = RedoCommand::new(action_log);
        let args = serde_json::json!({ "count": 1 });

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
    }

    #[tokio::test]
    async fn test_redo_no_actions() {
        let temp_dir = TempDir::new().unwrap();
        let action_log = Arc::new(ActionLog::new());
        let command = RedoCommand::new(action_log);

        let args = serde_json::json!({ "count": 1 });

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
        assert!(result.output.contains("No actions to redo"));
    }

    #[tokio::test]
    async fn test_redo_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, "content").unwrap();

        let action_log = Arc::new(ActionLog::new());
        let action = Action::file_created(
            "create".to_string(),
            test_file.clone(),
            "Created test.txt".to_string(),
        );
        action_log.record(action).await;

        // Undo first
        action_log.undo().await.unwrap();

        let command = RedoCommand::new(action_log);
        let args = serde_json::json!({ "count": 1 });

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
        assert!(result.output.contains("Would redo"));
    }
}
