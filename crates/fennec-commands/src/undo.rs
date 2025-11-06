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
pub struct UndoArgs {
    #[serde(default = "default_count")]
    pub count: usize,
}

fn default_count() -> usize {
    1
}

pub struct UndoCommand {
    descriptor: CommandDescriptor,
    action_log: Arc<ActionLog>,
}

impl UndoCommand {
    pub fn new(action_log: Arc<ActionLog>) -> Self {
        Self {
            descriptor: CommandDescriptor {
                name: "undo".to_string(),
                description: "Undo the last file operation".to_string(),
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

    async fn perform_undo(&self, args: &UndoArgs, context: &CommandContext) -> Result<String> {
        let workspace_path_str = context.workspace_path.as_ref().ok_or_else(|| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No workspace path set"
            )))
        })?;
        let workspace_path = std::path::Path::new(workspace_path_str);

        let mut undone_actions = Vec::new();

        for _ in 0..args.count {
            if !self.action_log.can_undo().await {
                break;
            }

            let action = self.action_log.undo().await?.ok_or_else(|| {
                FennecError::Command(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "No actions to undo"
                )))
            })?;

            if context.dry_run {
                undone_actions.push(format!("Would undo: {}", action.description));
                continue;
            }

            // Apply the reverse action (state_before)
            match &action.state_before {
                ActionState::FileCreated { path } => {
                    // Reverse of deletion: restore the file
                    if let ActionState::FileDeleted { content, .. } = &action.state_after {
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
                                format!("Failed to restore file: {}", e)
                            )))
                        })?;
                    }
                }
                ActionState::FileModified { path, content, .. } => {
                    // Restore previous content
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
                    // Reverse of creation: delete the file
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
                    // Reverse the move
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

                    if to_full.exists() {
                        fs::rename(&to_full, &from_full).await.map_err(|e| {
                            FennecError::Command(Box::new(std::io::Error::new(
                                e.kind(),
                                format!("Failed to reverse rename: {}", e)
                            )))
                        })?;
                    }
                }
                ActionState::DirectoryCreated { path } => {
                    // Reverse of directory deletion: restore directory
                    if let ActionState::DirectoryDeleted { contents, .. } = &action.state_after {
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

                        // Restore all files in the directory
                        for (file_path, content) in contents {
                            let file_full_path = if file_path.is_absolute() {
                                file_path.clone()
                            } else {
                                workspace_path.join(file_path)
                            };

                            if let Some(parent) = file_full_path.parent() {
                                fs::create_dir_all(parent).await.map_err(|e| {
                                    FennecError::Command(Box::new(std::io::Error::new(
                                        e.kind(),
                                        format!("Failed to create parent directories: {}", e)
                                    )))
                                })?;
                            }

                            fs::write(&file_full_path, content).await.map_err(|e| {
                                FennecError::Command(Box::new(std::io::Error::new(
                                    e.kind(),
                                    format!("Failed to restore file: {}", e)
                                )))
                            })?;
                        }
                    }
                }
                ActionState::DirectoryDeleted { path, .. } => {
                    // Reverse of directory creation: delete directory
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

            undone_actions.push(format!("Undid: {}", action.description));
        }

        if undone_actions.is_empty() {
            Ok("No actions to undo".to_string())
        } else {
            Ok(undone_actions.join("\n"))
        }
    }
}

impl Default for UndoCommand {
    fn default() -> Self {
        Self::new(Arc::new(ActionLog::new()))
    }
}

#[async_trait::async_trait]
impl CommandExecutor for UndoCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn preview(&self, args: &serde_json::Value, _context: &CommandContext) -> Result<CommandPreview> {
        let args: UndoArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid undo arguments: {}", e)
            )))
        })?;

        let can_undo = self.action_log.can_undo_count().await;
        let count = args.count.min(can_undo);

        let description = if count == 0 {
            "No actions to undo".to_string()
        } else if count == 1 {
            "Undo last action".to_string()
        } else {
            format!("Undo last {} actions", count)
        };

        Ok(CommandPreview {
            command_id: Uuid::new_v4(),
            description,
            actions: vec![],
            requires_approval: true,
        })
    }

    async fn execute(&self, args: &serde_json::Value, context: &CommandContext) -> Result<CommandResult> {
        let args: UndoArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid undo arguments: {}", e)
            )))
        })?;

        match self.perform_undo(&args, context).await {
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
        let args: UndoArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid undo arguments: {}", e)
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
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_undo_file_creation() {
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

        let command = UndoCommand::new(action_log);
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
        assert!(!test_file.exists());
    }

    #[tokio::test]
    async fn test_undo_no_actions() {
        let temp_dir = TempDir::new().unwrap();
        let action_log = Arc::new(ActionLog::new());
        let command = UndoCommand::new(action_log);

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
        assert!(result.output.contains("No actions to undo"));
    }

    #[tokio::test]
    async fn test_undo_dry_run() {
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

        let command = UndoCommand::new(action_log);
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
        assert!(result.output.contains("Would undo"));
        assert!(test_file.exists()); // File should still exist in dry-run
    }
}
