use anyhow::Result;
use crate::action_log::ActionLog;
use crate::registry::{CommandContext, CommandDescriptor, CommandExecutor};
use fennec_core::command::{Capability, CommandPreview, CommandResult};
use fennec_security::SandboxLevel;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryArgs {
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub show_all: bool,
}

fn default_limit() -> usize {
    20
}

pub struct HistoryCommand {
    descriptor: CommandDescriptor,
    action_log: Arc<ActionLog>,
}

impl HistoryCommand {
    pub fn new(action_log: Arc<ActionLog>) -> Self {
        Self {
            descriptor: CommandDescriptor {
                name: "history".to_string(),
                description: "Show action history log".to_string(),
                version: "1.0.0".to_string(),
                author: Some("Fennec Contributors".to_string()),
                capabilities_required: vec![Capability::ReadFile],
                sandbox_level_required: SandboxLevel::ReadOnly,
                supports_preview: false,
                supports_dry_run: false,
            },
            action_log,
        }
    }

    async fn get_history(&self, args: &HistoryArgs) -> Result<String> {
        let actions = self.action_log.get_history().await;
        let current_index = self.action_log.current_index().await;

        if actions.is_empty() {
            return Ok("No actions in history".to_string());
        }

        let limit = if args.show_all {
            actions.len()
        } else {
            args.limit.min(actions.len())
        };

        let mut output = String::new();
        output.push_str(&format!(
            "Action History ({} total, showing last {}):\n",
            actions.len(),
            limit
        ));
        output.push_str(&format!(
            "Current position: {} (can undo: {}, can redo: {})\n\n",
            current_index,
            self.action_log.can_undo_count().await,
            self.action_log.can_redo_count().await
        ));

        let start = actions.len().saturating_sub(limit);
        for (idx, action) in actions.iter().enumerate().skip(start) {
            let marker = if idx < current_index {
                "✓"
            } else {
                "○"
            };

            let timestamp = action.timestamp.format("%Y-%m-%d %H:%M:%S");

            output.push_str(&format!(
                "{} [{}] {} - {} ({})\n",
                marker, idx + 1, timestamp, action.description, action.command
            ));

            // Show the state change
            output.push_str(&format!("   Path: {}\n", action.state_after.path().display()));
        }

        output.push_str("\nLegend: ✓ = applied, ○ = undone\n");

        Ok(output)
    }
}

impl Default for HistoryCommand {
    fn default() -> Self {
        Self::new(Arc::new(ActionLog::new()))
    }
}

#[async_trait::async_trait]
impl CommandExecutor for HistoryCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn preview(&self, _args: &serde_json::Value, _context: &CommandContext) -> Result<CommandPreview> {
        let count = self.action_log.get_history().await.len();

        Ok(CommandPreview {
            command_id: Uuid::new_v4(),
            description: format!("Show action history ({} actions)", count),
            actions: vec![],
            requires_approval: false,
        })
    }

    async fn execute(&self, args: &serde_json::Value, _context: &CommandContext) -> Result<CommandResult> {
        let args: HistoryArgs = serde_json::from_value(args.clone()).unwrap_or(HistoryArgs {
            limit: default_limit(),
            show_all: false,
        });

        match self.get_history(&args).await {
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

    fn validate_args(&self, _args: &serde_json::Value) -> Result<()> {
        // History command args are all optional with defaults
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action_log::Action;
    use std::path::PathBuf;
    use tokio_util::sync::CancellationToken;

    #[tokio::test]
    async fn test_history_empty() {
        let action_log = Arc::new(ActionLog::new());
        let command = HistoryCommand::new(action_log);

        let args = serde_json::json!({});

        let context = CommandContext {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: None,
            sandbox_level: SandboxLevel::ReadOnly,
            dry_run: false,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
            action_log: None,
        };

        let result = command.execute(&args, &context).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("No actions in history"));
    }

    #[tokio::test]
    async fn test_history_with_actions() {
        let action_log = Arc::new(ActionLog::new());

        // Add some actions
        for i in 0..5 {
            let action = Action::file_created(
                "create".to_string(),
                PathBuf::from(format!("test{}.txt", i)),
                format!("Created test{}.txt", i),
            );
            action_log.record(action).await;
        }

        let command = HistoryCommand::new(action_log);
        let args = serde_json::json!({ "limit": 3 });

        let context = CommandContext {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: None,
            sandbox_level: SandboxLevel::ReadOnly,
            dry_run: false,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
            action_log: None,
        };

        let result = command.execute(&args, &context).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("5 total, showing last 3"));
    }

    #[tokio::test]
    async fn test_history_show_all() {
        let action_log = Arc::new(ActionLog::new());

        for i in 0..10 {
            let action = Action::file_created(
                "create".to_string(),
                PathBuf::from(format!("test{}.txt", i)),
                format!("Created test{}.txt", i),
            );
            action_log.record(action).await;
        }

        let command = HistoryCommand::new(action_log);
        let args = serde_json::json!({ "show_all": true });

        let context = CommandContext {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: None,
            sandbox_level: SandboxLevel::ReadOnly,
            dry_run: false,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
            action_log: None,
        };

        let result = command.execute(&args, &context).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("10 total, showing last 10"));
    }

    #[tokio::test]
    async fn test_history_with_undo() {
        let action_log = Arc::new(ActionLog::new());

        for i in 0..3 {
            let action = Action::file_created(
                "create".to_string(),
                PathBuf::from(format!("test{}.txt", i)),
                format!("Created test{}.txt", i),
            );
            action_log.record(action).await;
        }

        // Undo one action
        action_log.undo().await.unwrap();

        let command = HistoryCommand::new(action_log.clone());
        let args = serde_json::json!({});

        let context = CommandContext {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: None,
            sandbox_level: SandboxLevel::ReadOnly,
            dry_run: false,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
            action_log: None,
        };

        let result = command.execute(&args, &context).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("can undo: 2, can redo: 1"));
        assert!(result.output.contains("✓")); // Applied actions
        assert!(result.output.contains("○")); // Undone action
    }
}
