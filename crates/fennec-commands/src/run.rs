use anyhow::Result;
use async_trait::async_trait;
use fennec_core::{
    command::{Capability, CommandPreview, CommandResult, PreviewAction},
    error::FennecError,
};
use fennec_security::SandboxLevel;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::process::Command;
use uuid::Uuid;

use crate::registry::{CommandContext, CommandDescriptor, CommandExecutor};

/// Arguments for the run command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunArgs {
    /// Command to execute
    pub command: String,
    /// Working directory
    pub working_dir: Option<String>,
    /// Environment variables
    pub env: Option<std::collections::HashMap<String, String>>,
    /// Timeout in seconds
    pub timeout_seconds: Option<u64>,
    /// Whether to capture output
    pub capture_output: Option<bool>,
}

/// Run command for executing shell commands
pub struct RunCommand {
    descriptor: CommandDescriptor,
}

impl RunCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor {
                name: "run".to_string(),
                description: "Execute shell commands with security controls".to_string(),
                version: "1.0.0".to_string(),
                author: Some("Fennec Core".to_string()),
                capabilities_required: vec![Capability::ExecuteShell],
                sandbox_level_required: SandboxLevel::WorkspaceWrite,
                supports_preview: true,
                supports_dry_run: true,
            },
        }
    }

    /// Validate if command is safe to execute
    fn validate_command(&self, command: &str, context: &CommandContext) -> Result<()> {
        // Basic security checks
        let dangerous_commands = [
            "rm -rf",
            "del /s",
            "format",
            "fdisk",
            "dd if=",
            "mkfs",
            "shutdown",
            "reboot",
            "halt",
            "poweroff",
            "chmod 777",
            "chown root",
            "sudo su",
            "su -",
        ];

        for dangerous in &dangerous_commands {
            if command.contains(dangerous) {
                return Err(FennecError::Security {
                    message: format!("Dangerous command detected: {}", dangerous),
                }
                .into());
            }
        }

        // Additional restrictions based on sandbox level
        match context.sandbox_level {
            SandboxLevel::ReadOnly => {
                return Err(FennecError::Security {
                    message: "Cannot execute commands in read-only mode".to_string(),
                }
                .into());
            }
            SandboxLevel::WorkspaceWrite => {
                // Only allow commands that work within workspace
                let restricted_commands = ["cd /", "cd ~", "cd .."];
                for restricted in &restricted_commands {
                    if command.starts_with(restricted) {
                        return Err(FennecError::Security {
                            message: "Command restricted to workspace only".to_string(),
                        }
                        .into());
                    }
                }
            }
            SandboxLevel::FullAccess => {
                // Full access - fewer restrictions
            }
        }

        Ok(())
    }

    /// Execute the command
    async fn execute_command(&self, args: &RunArgs, context: &CommandContext) -> Result<String> {
        self.validate_command(&args.command, context)?;

        if context.dry_run {
            return Ok(format!("Would execute: {}", args.command));
        }

        // Parse command and arguments
        let parts: Vec<&str> = args.command.split_whitespace().collect();
        if parts.is_empty() {
            return Err(FennecError::Command {
                message: "Empty command".to_string(),
            }
            .into());
        }

        let mut cmd = Command::new(parts[0]);
        if parts.len() > 1 {
            cmd.args(&parts[1..]);
        }

        // Set working directory
        if let Some(ref work_dir) = args.working_dir {
            cmd.current_dir(work_dir);
        } else if let Some(ref workspace) = context.workspace_path {
            cmd.current_dir(workspace);
        }

        // Set environment variables
        if let Some(ref env_vars) = args.env {
            for (key, value) in env_vars {
                cmd.env(key, value);
            }
        }

        // Configure output capture
        if args.capture_output.unwrap_or(true) {
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());
        }

        // Execute with timeout
        let timeout = std::time::Duration::from_secs(args.timeout_seconds.unwrap_or(30));

        let output = tokio::time::timeout(timeout, cmd.output())
            .await
            .map_err(|_| FennecError::Command {
                message: format!("Command timed out after {} seconds", timeout.as_secs()),
            })?
            .map_err(|e| FennecError::Command {
                message: format!("Failed to execute command: {}", e),
            })?;

        // Check for cancellation
        if context.cancellation_token.is_cancelled() {
            return Err(FennecError::Command {
                message: "Command execution was cancelled".to_string(),
            }
            .into());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let mut result = Vec::new();
        result.push(format!("Exit code: {}", output.status.code().unwrap_or(-1)));

        if !stdout.is_empty() {
            result.push("--- STDOUT ---".to_string());
            result.push(stdout.to_string());
        }

        if !stderr.is_empty() {
            result.push("--- STDERR ---".to_string());
            result.push(stderr.to_string());
        }

        if !output.status.success() {
            return Err(FennecError::Command {
                message: format!(
                    "Command failed with exit code: {}",
                    output.status.code().unwrap_or(-1)
                ),
            }
            .into());
        }

        Ok(result.join("\n"))
    }
}

#[async_trait]
impl CommandExecutor for RunCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn preview(
        &self,
        args: &serde_json::Value,
        context: &CommandContext,
    ) -> Result<CommandPreview> {
        let args: RunArgs =
            serde_json::from_value(args.clone()).map_err(|e| FennecError::Command {
                message: format!("Invalid run arguments: {}", e),
            })?;

        self.validate_command(&args.command, context)?;

        Ok(CommandPreview {
            command_id: Uuid::new_v4(),
            description: format!("Execute command: {}", args.command),
            actions: vec![PreviewAction::ExecuteShell {
                command: args.command.clone(),
            }],
            requires_approval: true, // Shell execution should require approval
        })
    }

    async fn execute(
        &self,
        args: &serde_json::Value,
        context: &CommandContext,
    ) -> Result<CommandResult> {
        let args: RunArgs =
            serde_json::from_value(args.clone()).map_err(|e| FennecError::Command {
                message: format!("Invalid run arguments: {}", e),
            })?;

        match self.execute_command(&args, context).await {
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
        let args: RunArgs =
            serde_json::from_value(args.clone()).map_err(|e| FennecError::Command {
                message: format!("Invalid run arguments: {}", e),
            })?;

        if args.command.trim().is_empty() {
            return Err(FennecError::Command {
                message: "Command cannot be empty".to_string(),
            }
            .into());
        }

        if let Some(timeout) = args.timeout_seconds {
            if timeout > 300 {
                // 5 minutes max
                return Err(FennecError::Command {
                    message: "Timeout cannot exceed 300 seconds".to_string(),
                }
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
    async fn test_run_command_validation() {
        let command = RunCommand::new();

        // Valid args
        let valid_args = serde_json::json!({
            "command": "echo hello"
        });
        assert!(command.validate_args(&valid_args).is_ok());

        // Empty command
        let empty_command = serde_json::json!({
            "command": ""
        });
        assert!(command.validate_args(&empty_command).is_err());

        // Timeout too long
        let long_timeout = serde_json::json!({
            "command": "echo hello",
            "timeout_seconds": 400
        });
        assert!(command.validate_args(&long_timeout).is_err());
    }

    #[tokio::test]
    async fn test_run_command_security() {
        let command = RunCommand::new();

        let context = CommandContext {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: None,
            sandbox_level: SandboxLevel::WorkspaceWrite,
            dry_run: false,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
        };

        // Dangerous command should be rejected
        assert!(command.validate_command("rm -rf /", &context).is_err());
        assert!(command
            .validate_command("shutdown -h now", &context)
            .is_err());

        // Safe command should be allowed
        assert!(command.validate_command("echo hello", &context).is_ok());
        assert!(command.validate_command("ls -la", &context).is_ok());
    }

    #[tokio::test]
    async fn test_run_command_execution() {
        let command = RunCommand::new();

        let args = serde_json::json!({
            "command": "echo hello world",
            "capture_output": true,
            "timeout_seconds": 10
        });

        let context = CommandContext {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: None,
            sandbox_level: SandboxLevel::FullAccess,
            dry_run: false,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
        };

        let result = command.execute(&args, &context).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("hello world"));
    }
}
