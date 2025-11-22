use crate::compiler_errors::{extract_fixes, parse_cargo_json, FixConfidence, SuggestedFix};
use crate::registry::{CommandContext, CommandDescriptor, CommandExecutor};
use anyhow::Result;
use fennec_core::command::{Capability, CommandPreview, CommandResult};
use fennec_core::error::FennecError;
use fennec_security::SandboxLevel;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixErrorsArgs {
    /// Type of check to run: "check", "build", "clippy", or "test"
    #[serde(default = "default_check_type")]
    pub check_type: String,

    /// Minimum confidence level for fixes: "high", "medium", "low"
    #[serde(default = "default_min_confidence")]
    pub min_confidence: String,

    /// Maximum number of fixes to display
    #[serde(default = "default_max_fixes")]
    pub max_fixes: usize,

    /// Additional cargo arguments
    #[serde(default)]
    pub cargo_args: Vec<String>,
}

fn default_check_type() -> String {
    "check".to_string()
}

fn default_min_confidence() -> String {
    "medium".to_string()
}

fn default_max_fixes() -> usize {
    20
}

pub struct FixErrorsCommand {
    descriptor: CommandDescriptor,
}

impl FixErrorsCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor {
                name: "fix-errors".to_string(),
                description: "Analyze Rust compiler errors and suggest fixes".to_string(),
                version: "1.0.0".to_string(),
                author: Some("Fennec Contributors".to_string()),
                capabilities_required: vec![Capability::ExecuteShell],
                sandbox_level_required: SandboxLevel::WorkspaceWrite,
                supports_preview: false,
                supports_dry_run: false,
            },
        }
    }

    fn parse_confidence(s: &str) -> FixConfidence {
        match s.to_lowercase().as_str() {
            "high" => FixConfidence::High,
            "low" => FixConfidence::Low,
            _ => FixConfidence::Medium,
        }
    }

    async fn run_cargo_check(
        &self,
        check_type: &str,
        cargo_args: &[String],
        context: &CommandContext,
    ) -> Result<Vec<SuggestedFix>> {
        let workspace_path = context.workspace_path.as_ref().ok_or_else(|| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No workspace path set",
            )))
        })?;

        // Build cargo command
        let mut cmd = Command::new("cargo");
        cmd.current_dir(workspace_path);

        // Add the check type
        match check_type {
            "check" => cmd.arg("check"),
            "build" => cmd.arg("build"),
            "clippy" => cmd.arg("clippy"),
            "test" => cmd.arg("test").arg("--no-run"),
            _ => cmd.arg("check"),
        };

        // Add JSON message format
        cmd.arg("--message-format=json");

        // Add additional args
        for arg in cargo_args {
            cmd.arg(arg);
        }

        // Configure stdio
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Spawn the process
        let mut child = cmd.spawn().map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                e.kind(),
                format!("Failed to spawn cargo: {}", e),
            )))
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to capture stdout",
            )))
        })?;

        let mut reader = BufReader::new(stdout).lines();
        let mut all_fixes = Vec::new();

        // Parse output line by line
        while let Some(line) = reader
            .next_line()
            .await
            .map_err(|e| FennecError::Command(Box::new(e)))?
        {
            // Check for cancellation
            if context.cancellation_token.is_cancelled() {
                let _ = child.kill().await;
                return Err(FennecError::Command(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "Operation cancelled",
                )))
                .into());
            }

            // Try to parse as JSON compiler message
            if let Some(message) = parse_cargo_json(&line) {
                let fixes = extract_fixes(&message);
                all_fixes.extend(fixes);
            }
        }

        // Wait for the command to complete
        let _ = child.wait().await;

        Ok(all_fixes)
    }

    async fn analyze_and_suggest(
        &self,
        args: &FixErrorsArgs,
        context: &CommandContext,
    ) -> Result<String> {
        // Run cargo check/build/clippy
        let all_fixes = self
            .run_cargo_check(&args.check_type, &args.cargo_args, context)
            .await?;

        if all_fixes.is_empty() {
            return Ok(format!(
                "No compiler errors or warnings found. Great job! 🎉"
            ));
        }

        // Filter by confidence
        let min_confidence = Self::parse_confidence(&args.min_confidence);
        let filtered_fixes: Vec<_> = all_fixes
            .into_iter()
            .filter(|f| match (&f.confidence, &min_confidence) {
                (FixConfidence::High, _) => true,
                (FixConfidence::Medium, FixConfidence::Low) => true,
                (FixConfidence::Medium, FixConfidence::Medium) => true,
                (FixConfidence::Low, FixConfidence::Low) => true,
                _ => false,
            })
            .take(args.max_fixes)
            .collect();

        if filtered_fixes.is_empty() {
            return Ok(format!(
                "Found errors but no automatic fixes available at '{}' confidence level",
                args.min_confidence
            ));
        }

        // Format output
        let mut output = format!(
            "Found {} suggested fixes from cargo {}:\n\n",
            filtered_fixes.len(),
            args.check_type
        );

        for (idx, fix) in filtered_fixes.iter().enumerate() {
            output.push_str(&format!("{}. {}\n\n", idx + 1, fix.format()));
        }

        output.push_str(&format!(
            "\nTip: You can apply these fixes manually or use the 'edit' command to make changes.\n"
        ));

        Ok(output)
    }
}

impl Default for FixErrorsCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl CommandExecutor for FixErrorsCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn preview(
        &self,
        args: &serde_json::Value,
        _context: &CommandContext,
    ) -> Result<CommandPreview> {
        let args: FixErrorsArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid fix-errors arguments: {}", e),
            )))
        })?;

        Ok(CommandPreview {
            command_id: Uuid::new_v4(),
            description: format!("Run cargo {} and suggest fixes", args.check_type),
            actions: vec![],
            requires_approval: false,
        })
    }

    async fn execute(
        &self,
        args: &serde_json::Value,
        context: &CommandContext,
    ) -> Result<CommandResult> {
        let args: FixErrorsArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid fix-errors arguments: {}", e),
            )))
        })?;

        match self.analyze_and_suggest(&args, context).await {
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
        let args: FixErrorsArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid fix-errors arguments: {}", e),
            )))
        })?;

        // Validate check type
        if !["check", "build", "clippy", "test"].contains(&args.check_type.as_str()) {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "Invalid check_type: '{}'. Must be one of: check, build, clippy, test",
                    args.check_type
                ),
            )))
            .into());
        }

        // Validate confidence
        if !["high", "medium", "low"].contains(&args.min_confidence.to_lowercase().as_str()) {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "Invalid min_confidence: '{}'. Must be one of: high, medium, low",
                    args.min_confidence
                ),
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
    fn test_parse_confidence() {
        assert_eq!(
            FixErrorsCommand::parse_confidence("high"),
            FixConfidence::High
        );
        assert_eq!(
            FixErrorsCommand::parse_confidence("medium"),
            FixConfidence::Medium
        );
        assert_eq!(
            FixErrorsCommand::parse_confidence("low"),
            FixConfidence::Low
        );
        assert_eq!(
            FixErrorsCommand::parse_confidence("invalid"),
            FixConfidence::Medium
        );
    }

    #[test]
    fn test_validate_args() {
        let command = FixErrorsCommand::new();

        // Valid args
        let valid_args = serde_json::json!({
            "check_type": "check",
            "min_confidence": "medium",
            "max_fixes": 10
        });
        assert!(command.validate_args(&valid_args).is_ok());

        // Invalid check_type
        let invalid_check = serde_json::json!({
            "check_type": "invalid",
        });
        assert!(command.validate_args(&invalid_check).is_err());

        // Invalid confidence
        let invalid_conf = serde_json::json!({
            "check_type": "check",
            "min_confidence": "invalid",
        });
        assert!(command.validate_args(&invalid_conf).is_err());
    }

    #[tokio::test]
    async fn test_fix_errors_no_workspace() {
        let command = FixErrorsCommand::new();
        let args = serde_json::json!({
            "check_type": "check"
        });

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
