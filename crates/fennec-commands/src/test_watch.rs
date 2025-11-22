use crate::registry::{CommandContext, CommandDescriptor, CommandExecutor};
use anyhow::Result;
use fennec_core::command::{Capability, CommandPreview, CommandResult};
use fennec_core::error::FennecError;
use fennec_security::SandboxLevel;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestWatchArgs {
    /// Test command to run (defaults to "cargo test")
    #[serde(default = "default_test_command")]
    pub test_command: String,

    /// Additional arguments for the test command
    #[serde(default)]
    pub test_args: Vec<String>,

    /// File patterns to watch (glob patterns)
    #[serde(default = "default_watch_patterns")]
    pub watch_patterns: Vec<String>,

    /// Debounce delay in milliseconds (wait time after file change before running tests)
    #[serde(default = "default_debounce_ms")]
    pub debounce_ms: u64,

    /// Maximum duration to watch in seconds (0 = unlimited)
    #[serde(default)]
    pub max_duration_seconds: u64,
}

fn default_test_command() -> String {
    "cargo test".to_string()
}

fn default_watch_patterns() -> Vec<String> {
    vec!["**/*.rs".to_string(), "**/Cargo.toml".to_string()]
}

fn default_debounce_ms() -> u64 {
    500
}

pub struct TestWatchCommand {
    descriptor: CommandDescriptor,
}

impl TestWatchCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor {
                name: "test-watch".to_string(),
                description: "Watch files and automatically rerun tests on changes".to_string(),
                version: "1.0.0".to_string(),
                author: Some("Fennec Contributors".to_string()),
                capabilities_required: vec![Capability::ExecuteShell, Capability::ReadFile],
                sandbox_level_required: SandboxLevel::WorkspaceWrite,
                supports_preview: false,
                supports_dry_run: true,
            },
        }
    }

    async fn run_tests(
        &self,
        test_command: &str,
        test_args: &[String],
        workspace_path: &str,
    ) -> Result<(bool, String)> {
        let parts: Vec<&str> = test_command.split_whitespace().collect();
        if parts.is_empty() {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Empty test command",
            )))
            .into());
        }

        let mut cmd = Command::new(parts[0]);
        cmd.current_dir(workspace_path);

        // Add command parts
        for part in &parts[1..] {
            cmd.arg(part);
        }

        // Add test args
        for arg in test_args {
            cmd.arg(arg);
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                e.kind(),
                format!("Failed to spawn test command: {}", e),
            )))
        })?;

        // Capture output
        let stdout = child.stdout.take().ok_or_else(|| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to capture stdout",
            )))
        })?;

        let stderr = child.stderr.take().ok_or_else(|| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to capture stderr",
            )))
        })?;

        let mut stdout_lines = BufReader::new(stdout).lines();
        let mut stderr_lines = BufReader::new(stderr).lines();

        let mut output = String::new();

        // Read stdout
        while let Some(line) = stdout_lines
            .next_line()
            .await
            .map_err(|e| FennecError::Command(Box::new(e)))?
        {
            output.push_str(&line);
            output.push('\n');
        }

        // Read stderr
        while let Some(line) = stderr_lines
            .next_line()
            .await
            .map_err(|e| FennecError::Command(Box::new(e)))?
        {
            output.push_str(&line);
            output.push('\n');
        }

        // Wait for completion
        let status = child
            .wait()
            .await
            .map_err(|e| FennecError::Command(Box::new(e)))?;

        Ok((status.success(), output))
    }

    #[allow(dead_code)]
    fn should_watch_path(path: &Path, patterns: &[String]) -> bool {
        let path_str = path.to_string_lossy();

        // Skip hidden files and directories
        if path_str.contains("/.") {
            return false;
        }

        // Skip target directory
        if path_str.contains("/target/") {
            return false;
        }

        // Check patterns
        for pattern in patterns {
            if pattern == "**/*.rs" && path_str.ends_with(".rs") {
                return true;
            }
            if pattern == "**/Cargo.toml" && path_str.ends_with("Cargo.toml") {
                return true;
            }
            // Add more pattern matching as needed
        }

        false
    }

    async fn watch_and_test(
        &self,
        args: &TestWatchArgs,
        context: &CommandContext,
    ) -> Result<String> {
        let workspace_path = context.workspace_path.as_ref().ok_or_else(|| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No workspace path set",
            )))
        })?;

        if context.dry_run {
            return Ok(format!(
                "DRY RUN: Would watch files matching {:?} and run '{}' on changes",
                args.watch_patterns, args.test_command
            ));
        }

        let mut output = String::new();
        output.push_str(&format!(
            "🔍 Watching files in {} for changes...\n",
            workspace_path
        ));
        output.push_str(&format!("📋 Test command: {}\n", args.test_command));
        output.push_str(&format!("⏱️  Debounce: {}ms\n\n", args.debounce_ms));

        // Run tests initially
        output.push_str("▶️  Running initial tests...\n\n");
        let (success, test_output) = self
            .run_tests(&args.test_command, &args.test_args, workspace_path)
            .await?;

        let status_icon = if success { "✅" } else { "❌" };
        output.push_str(&format!(
            "{} Tests {}\n\n",
            status_icon,
            if success { "passed" } else { "failed" }
        ));

        // Limit output length
        let truncated_output = if test_output.len() > 1000 {
            format!("{}...\n(output truncated)", &test_output[..1000])
        } else {
            test_output
        };
        output.push_str(&truncated_output);
        output.push_str("\n\n");

        output.push_str("Note: File watching in background mode is not yet implemented.\n");
        output.push_str("This is a simplified version that runs tests once.\n");
        output.push_str("For continuous watching, use 'cargo watch' or similar tools.\n");

        Ok(output)
    }
}

impl Default for TestWatchCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl CommandExecutor for TestWatchCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn preview(
        &self,
        args: &serde_json::Value,
        _context: &CommandContext,
    ) -> Result<CommandPreview> {
        let args: TestWatchArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid test-watch arguments: {}", e),
            )))
        })?;

        Ok(CommandPreview {
            command_id: Uuid::new_v4(),
            description: format!("Watch files and run '{}'", args.test_command),
            actions: vec![],
            requires_approval: false,
        })
    }

    async fn execute(
        &self,
        args: &serde_json::Value,
        context: &CommandContext,
    ) -> Result<CommandResult> {
        let args: TestWatchArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid test-watch arguments: {}", e),
            )))
        })?;

        match self.watch_and_test(&args, context).await {
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
        let args: TestWatchArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid test-watch arguments: {}", e),
            )))
        })?;

        if args.test_command.trim().is_empty() {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Test command cannot be empty",
            )))
            .into());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_util::sync::CancellationToken;

    #[test]
    fn test_default_values() {
        assert_eq!(default_test_command(), "cargo test");
        assert_eq!(default_watch_patterns(), vec!["**/*.rs", "**/Cargo.toml"]);
        assert_eq!(default_debounce_ms(), 500);
    }

    #[test]
    fn test_should_watch_path() {
        let patterns = default_watch_patterns();

        assert!(TestWatchCommand::should_watch_path(
            Path::new("src/main.rs"),
            &patterns
        ));
        assert!(TestWatchCommand::should_watch_path(
            Path::new("Cargo.toml"),
            &patterns
        ));
        assert!(!TestWatchCommand::should_watch_path(
            Path::new("target/debug/main"),
            &patterns
        ));
        assert!(!TestWatchCommand::should_watch_path(
            Path::new(".git/config"),
            &patterns
        ));
    }

    #[tokio::test]
    async fn test_test_watch_no_workspace() {
        let command = TestWatchCommand::new();
        let args = serde_json::json!({});

        let context = CommandContext {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: None,
            sandbox_level: SandboxLevel::WorkspaceWrite,
            dry_run: false,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
            action_log: None,
        };

        let result = command.execute(&args, &context).await.unwrap();
        assert!(!result.success);
        assert!(result.error.is_some());
    }
}
