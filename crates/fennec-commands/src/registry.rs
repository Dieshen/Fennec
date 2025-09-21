use anyhow::Result;
use async_trait::async_trait;
use fennec_core::{
    command::{Capability, CommandPreview, CommandResult},
    error::FennecError,
};
use fennec_security::SandboxLevel;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Descriptor for a command containing metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDescriptor {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: Option<String>,
    pub capabilities_required: Vec<Capability>,
    pub sandbox_level_required: SandboxLevel,
    pub supports_preview: bool,
    pub supports_dry_run: bool,
}

/// Context passed to commands during execution
#[derive(Debug, Clone)]
pub struct CommandContext {
    pub session_id: Uuid,
    pub user_id: Option<String>,
    pub workspace_path: Option<String>,
    pub sandbox_level: SandboxLevel,
    pub dry_run: bool,
    pub preview_only: bool,
    pub cancellation_token: tokio_util::sync::CancellationToken,
}

/// Result of command execution including metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecutionResult {
    pub command_id: Uuid,
    pub command_name: String,
    pub execution_id: Uuid,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub preview: Option<CommandPreview>,
    pub execution_time_ms: u64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Trait that all commands must implement
#[async_trait]
pub trait CommandExecutor: Send + Sync {
    /// Get the command descriptor
    fn descriptor(&self) -> &CommandDescriptor;

    /// Generate a preview of what this command will do
    async fn preview(
        &self,
        args: &serde_json::Value,
        context: &CommandContext,
    ) -> Result<CommandPreview>;

    /// Execute the command
    async fn execute(
        &self,
        args: &serde_json::Value,
        context: &CommandContext,
    ) -> Result<CommandResult>;

    /// Validate command arguments
    fn validate_args(&self, args: &serde_json::Value) -> Result<()>;

    /// Check if this command can run with the given sandbox level
    fn can_run_in_sandbox(&self, level: &SandboxLevel) -> bool {
        match (&self.descriptor().sandbox_level_required, level) {
            (SandboxLevel::ReadOnly, _) => true,
            (
                SandboxLevel::WorkspaceWrite,
                SandboxLevel::WorkspaceWrite | SandboxLevel::FullAccess,
            ) => true,
            (SandboxLevel::FullAccess, SandboxLevel::FullAccess) => true,
            _ => false,
        }
    }
}

/// Registry for managing commands
#[derive(Default)]
pub struct CommandRegistry {
    commands: Arc<RwLock<HashMap<String, Arc<dyn CommandExecutor>>>>,
    builtin_commands: Arc<RwLock<HashMap<String, Arc<dyn CommandExecutor>>>>,
    custom_commands: Arc<RwLock<HashMap<String, Arc<dyn CommandExecutor>>>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a built-in command
    pub async fn register_builtin(&self, executor: Arc<dyn CommandExecutor>) -> Result<()> {
        let name = executor.descriptor().name.clone();

        {
            let mut builtin = self.builtin_commands.write().await;
            builtin.insert(name.clone(), executor.clone());
        }

        {
            let mut commands = self.commands.write().await;
            commands.insert(name, executor);
        }

        Ok(())
    }

    /// Register a custom command (can override built-ins)
    pub async fn register_custom(&self, executor: Arc<dyn CommandExecutor>) -> Result<()> {
        let name = executor.descriptor().name.clone();

        {
            let mut custom = self.custom_commands.write().await;
            custom.insert(name.clone(), executor.clone());
        }

        {
            let mut commands = self.commands.write().await;
            commands.insert(name, executor);
        }

        Ok(())
    }

    /// Get a command by name
    pub async fn get_command(&self, name: &str) -> Option<Arc<dyn CommandExecutor>> {
        let commands = self.commands.read().await;
        commands.get(name).cloned()
    }

    /// List all available commands
    pub async fn list_commands(&self) -> Vec<CommandDescriptor> {
        let commands = self.commands.read().await;
        commands
            .values()
            .map(|cmd| cmd.descriptor().clone())
            .collect()
    }

    /// List commands by capability
    pub async fn list_commands_by_capability(
        &self,
        capability: &Capability,
    ) -> Vec<CommandDescriptor> {
        let commands = self.commands.read().await;
        commands
            .values()
            .filter(|cmd| cmd.descriptor().capabilities_required.contains(capability))
            .map(|cmd| cmd.descriptor().clone())
            .collect()
    }

    /// List commands that can run in the given sandbox level
    pub async fn list_commands_for_sandbox(&self, level: &SandboxLevel) -> Vec<CommandDescriptor> {
        let commands = self.commands.read().await;
        commands
            .values()
            .filter(|cmd| cmd.can_run_in_sandbox(level))
            .map(|cmd| cmd.descriptor().clone())
            .collect()
    }

    /// Execute a command with preview and validation
    pub async fn execute_command(
        &self,
        name: &str,
        args: &serde_json::Value,
        context: &CommandContext,
    ) -> Result<CommandExecutionResult> {
        let start_time = std::time::Instant::now();
        let execution_id = Uuid::new_v4();

        let command = self
            .get_command(name)
            .await
            .ok_or_else(|| FennecError::Command {
                message: format!("Command '{}' not found", name),
            })?;

        let mut result = CommandExecutionResult {
            command_id: Uuid::new_v4(),
            command_name: name.to_string(),
            execution_id,
            success: false,
            output: String::new(),
            error: None,
            preview: None,
            execution_time_ms: 0,
            created_at: chrono::Utc::now(),
        };

        // Validate sandbox permissions
        if !command.can_run_in_sandbox(&context.sandbox_level) {
            result.error = Some(format!(
                "Command '{}' requires {:?} but only {:?} is available",
                name,
                command.descriptor().sandbox_level_required,
                context.sandbox_level
            ));
            result.execution_time_ms = start_time.elapsed().as_millis() as u64;
            return Ok(result);
        }

        // Validate arguments
        if let Err(e) = command.validate_args(args) {
            result.error = Some(format!("Invalid arguments for '{}': {}", name, e));
            result.execution_time_ms = start_time.elapsed().as_millis() as u64;
            return Ok(result);
        }

        // Generate preview if requested or if dry run
        if context.preview_only || context.dry_run {
            match command.preview(args, context).await {
                Ok(preview) => {
                    result.preview = Some(preview);
                    if context.preview_only {
                        result.success = true;
                        result.output = "Preview generated successfully".to_string();
                        result.execution_time_ms = start_time.elapsed().as_millis() as u64;
                        return Ok(result);
                    }
                }
                Err(e) => {
                    result.error = Some(format!("Preview generation failed: {}", e));
                    result.execution_time_ms = start_time.elapsed().as_millis() as u64;
                    return Ok(result);
                }
            }
        }

        // Execute the command if not preview-only
        if !context.preview_only {
            match command.execute(args, context).await {
                Ok(command_result) => {
                    result.success = command_result.success;
                    result.output = command_result.output;
                    result.error = command_result.error;
                }
                Err(e) => {
                    result.error = Some(e.to_string());
                }
            }
        }

        result.execution_time_ms = start_time.elapsed().as_millis() as u64;
        Ok(result)
    }

    /// Remove a command from the registry
    pub async fn unregister_command(&self, name: &str) -> Result<()> {
        {
            let mut commands = self.commands.write().await;
            commands.remove(name);
        }

        {
            let mut custom = self.custom_commands.write().await;
            custom.remove(name);
        }

        // Don't remove from builtin_commands to allow re-registration
        Ok(())
    }

    /// Reset to only built-in commands
    pub async fn reset_to_builtins(&self) -> Result<()> {
        let builtin = self.builtin_commands.read().await;
        let mut commands = self.commands.write().await;
        let mut custom = self.custom_commands.write().await;

        commands.clear();
        custom.clear();

        for (name, executor) in builtin.iter() {
            commands.insert(name.clone(), executor.clone());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fennec_core::command::PreviewAction;
    use tokio_util::sync::CancellationToken;

    struct TestCommand {
        descriptor: CommandDescriptor,
    }

    #[async_trait]
    impl CommandExecutor for TestCommand {
        fn descriptor(&self) -> &CommandDescriptor {
            &self.descriptor
        }

        async fn preview(
            &self,
            _args: &serde_json::Value,
            _context: &CommandContext,
        ) -> Result<CommandPreview> {
            Ok(CommandPreview {
                command_id: Uuid::new_v4(),
                description: "Test command preview".to_string(),
                actions: vec![PreviewAction::ReadFile {
                    path: "/test/file.txt".to_string(),
                }],
                requires_approval: false,
            })
        }

        async fn execute(
            &self,
            _args: &serde_json::Value,
            _context: &CommandContext,
        ) -> Result<CommandResult> {
            Ok(CommandResult {
                command_id: Uuid::new_v4(),
                success: true,
                output: "Test command executed".to_string(),
                error: None,
            })
        }

        fn validate_args(&self, _args: &serde_json::Value) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_command_registry() {
        let registry = CommandRegistry::new();

        let test_command = Arc::new(TestCommand {
            descriptor: CommandDescriptor {
                name: "test".to_string(),
                description: "Test command".to_string(),
                version: "1.0.0".to_string(),
                author: Some("Test Author".to_string()),
                capabilities_required: vec![Capability::ReadFile],
                sandbox_level_required: SandboxLevel::ReadOnly,
                supports_preview: true,
                supports_dry_run: true,
            },
        });

        registry.register_builtin(test_command).await.unwrap();

        let commands = registry.list_commands().await;
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].name, "test");

        let context = CommandContext {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: None,
            sandbox_level: SandboxLevel::ReadOnly,
            dry_run: false,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
        };

        let result = registry
            .execute_command("test", &serde_json::json!({}), &context)
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.command_name, "test");
    }
}
