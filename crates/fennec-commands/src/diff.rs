use anyhow::Result;
use async_trait::async_trait;
use fennec_core::{
    command::{Capability, CommandPreview, CommandResult, PreviewAction},
    error::FennecError,
};
use fennec_security::SandboxLevel;
use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};
use std::path::Path;
use tokio::fs;
use uuid::Uuid;

use crate::registry::{CommandContext, CommandDescriptor, CommandExecutor};

/// Arguments for the diff command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffArgs {
    /// First file or text to compare
    pub left: String,
    /// Second file or text to compare
    pub right: String,
    /// Whether inputs are file paths (true) or text content (false)
    pub is_file_path: Option<bool>,
    /// Number of context lines to show
    pub context_lines: Option<usize>,
    /// Output format (unified, side-by-side, brief)
    pub format: Option<String>,
}

/// Diff command for comparing files or text
pub struct DiffCommand {
    descriptor: CommandDescriptor,
}

impl DiffCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor {
                name: "diff".to_string(),
                description: "Compare files or text content and show differences".to_string(),
                version: "1.0.0".to_string(),
                author: Some("Fennec Core".to_string()),
                capabilities_required: vec![Capability::ReadFile],
                sandbox_level_required: SandboxLevel::ReadOnly,
                supports_preview: true,
                supports_dry_run: false,
            },
        }
    }

    /// Generate diff output
    async fn generate_diff(&self, args: &DiffArgs, _context: &CommandContext) -> Result<String> {
        let (left_content, right_content) = if args.is_file_path.unwrap_or(true) {
            // Read from files
            let left_path = Path::new(&args.left);
            let right_path = Path::new(&args.right);

            let left_content = if left_path.exists() {
                fs::read_to_string(left_path).await.map_err(|e| {
                    FennecError::Command(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to read {}: {}", args.left, e),
                    )))
                })?
            } else {
                return Err(FennecError::Command(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("File not found: {}", args.left),
                )))
                .into());
            };

            let right_content = if right_path.exists() {
                fs::read_to_string(right_path).await.map_err(|e| {
                    FennecError::Command(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to read {}: {}", args.right, e),
                    )))
                })?
            } else {
                return Err(FennecError::Command(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("File not found: {}", args.right),
                )))
                .into());
            };

            (left_content, right_content)
        } else {
            // Use provided text content
            (args.left.clone(), args.right.clone())
        };

        // Generate diff
        let diff = TextDiff::from_lines(&left_content, &right_content);

        let format = args.format.as_deref().unwrap_or("unified");
        match format {
            "unified" => {
                let mut output = Vec::new();

                if args.is_file_path.unwrap_or(true) {
                    output.push(format!("--- {}", args.left));
                    output.push(format!("+++ {}", args.right));
                } else {
                    output.push("--- left".to_string());
                    output.push("+++ right".to_string());
                }

                for group in diff.grouped_ops(args.context_lines.unwrap_or(3)) {
                    let mut hunk_output = Vec::new();

                    for op in &group {
                        for change in diff.iter_changes(op) {
                            let sign = match change.tag() {
                                ChangeTag::Delete => "-",
                                ChangeTag::Insert => "+",
                                ChangeTag::Equal => " ",
                            };
                            hunk_output.push(format!("{}{}", sign, change.value().trim_end()));
                        }
                    }

                    if !hunk_output.is_empty() {
                        output.extend(hunk_output);
                    }
                }

                Ok(output.join("\n"))
            }
            "brief" => {
                if diff.ratio() == 1.0 {
                    Ok("Files are identical".to_string())
                } else {
                    // Calculate basic stats manually
                    let mut insertions = 0;
                    let mut deletions = 0;

                    for op in diff.ops() {
                        for change in diff.iter_changes(op) {
                            match change.tag() {
                                ChangeTag::Insert => insertions += 1,
                                ChangeTag::Delete => deletions += 1,
                                _ => {}
                            }
                        }
                    }

                    Ok(format!(
                        "Files differ: {} insertions(+), {} deletions(-)",
                        insertions, deletions
                    ))
                }
            }
            "side-by-side" => {
                let mut output = Vec::new();

                for group in diff.grouped_ops(args.context_lines.unwrap_or(3)) {
                    for op in &group {
                        for change in diff.iter_changes(op) {
                            let line = change.value().trim_end();
                            match change.tag() {
                                ChangeTag::Delete => output.push(format!("< {}", line)),
                                ChangeTag::Insert => output.push(format!("> {}", line)),
                                ChangeTag::Equal => output.push(format!("  {}", line)),
                            }
                        }
                    }
                }

                Ok(output.join("\n"))
            }
            _ => Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Unknown diff format: {}", format),
            )))
            .into()),
        }
    }
}

#[async_trait]
impl CommandExecutor for DiffCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn preview(
        &self,
        args: &serde_json::Value,
        _context: &CommandContext,
    ) -> Result<CommandPreview> {
        let args: DiffArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Invalid diff arguments: {}", e),
            )))
        })?;

        let mut actions = Vec::new();

        if args.is_file_path.unwrap_or(true) {
            actions.push(PreviewAction::ReadFile {
                path: args.left.clone(),
            });
            actions.push(PreviewAction::ReadFile {
                path: args.right.clone(),
            });
        }

        Ok(CommandPreview {
            command_id: Uuid::new_v4(),
            description: format!("Compare {} and {}", args.left, args.right),
            actions,
            requires_approval: false,
        })
    }

    async fn execute(
        &self,
        args: &serde_json::Value,
        context: &CommandContext,
    ) -> Result<CommandResult> {
        let args: DiffArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Invalid diff arguments: {}", e),
            )))
        })?;

        match self.generate_diff(&args, context).await {
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
        let args: DiffArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Invalid diff arguments: {}", e),
            )))
        })?;

        if args.left.trim().is_empty() || args.right.trim().is_empty() {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Both left and right inputs must be provided",
            )))
            .into());
        }

        if let Some(ref format) = args.format {
            if !matches!(format.as_str(), "unified" | "side-by-side" | "brief") {
                return Err(FennecError::Command(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Format must be one of: unified, side-by-side, brief",
                )))
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
    async fn test_diff_command_validation() {
        let command = DiffCommand::new();

        // Valid args
        let valid_args = serde_json::json!({
            "left": "file1.txt",
            "right": "file2.txt"
        });
        assert!(command.validate_args(&valid_args).is_ok());

        // Empty inputs
        let empty_args = serde_json::json!({
            "left": "",
            "right": "file2.txt"
        });
        assert!(command.validate_args(&empty_args).is_err());
    }

    #[tokio::test]
    async fn test_diff_text_content() {
        let command = DiffCommand::new();

        let args = serde_json::json!({
            "left": "Hello\nWorld",
            "right": "Hello\nUniverse",
            "is_file_path": false,
            "format": "unified"
        });

        let context = CommandContext {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: None,
            sandbox_level: SandboxLevel::ReadOnly,
            dry_run: false,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
        };

        let result = command.execute(&args, &context).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("-World"));
        assert!(result.output.contains("+Universe"));
    }
}
