use crate::git_integration::generate_commit_template;
use crate::registry::{CommandContext, CommandDescriptor, CommandExecutor};
use anyhow::Result;
use fennec_core::command::{Capability, CommandPreview, CommandResult};
use fennec_core::error::FennecError;
use fennec_security::SandboxLevel;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitTemplateArgs {
    /// Whether to include testing section
    #[serde(default = "default_true")]
    pub include_testing: bool,

    /// Whether to include description section
    #[serde(default = "default_true")]
    pub include_description: bool,
}

fn default_true() -> bool {
    true
}

pub struct CommitTemplateCommand {
    descriptor: CommandDescriptor,
}

impl CommitTemplateCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor {
                name: "commit-template".to_string(),
                description: "Generate a commit message template based on staged changes"
                    .to_string(),
                version: "1.0.0".to_string(),
                author: Some("Fennec Contributors".to_string()),
                capabilities_required: vec![Capability::ExecuteShell],
                sandbox_level_required: SandboxLevel::ReadOnly,
                supports_preview: false,
                supports_dry_run: false,
            },
        }
    }

    async fn generate_template(
        &self,
        args: &CommitTemplateArgs,
        context: &CommandContext,
    ) -> Result<String> {
        let workspace_path = context.workspace_path.as_ref().ok_or_else(|| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No workspace path set",
            )))
        })?;

        let mut template = generate_commit_template(workspace_path)
            .await
            .map_err(|e| {
                FennecError::Command(Box::new(std::io::Error::new(
                    e.kind(),
                    format!("Failed to generate commit template: {}", e),
                )))
            })?;

        // Remove sections if not requested
        if !args.include_testing {
            if let Some(pos) = template.find("## Testing") {
                template = template[..pos].to_string();
            }
        }

        if !args.include_description {
            if let Some(pos) = template.find("## Description") {
                let end_pos = template.find("## Testing").unwrap_or(template.len());
                let before = &template[..pos];
                let after = &template[end_pos..];
                template = format!("{}{}", before, after);
            }
        }

        Ok(template)
    }
}

impl Default for CommitTemplateCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl CommandExecutor for CommitTemplateCommand {
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
            description: "Generate commit message template from staged changes".to_string(),
            actions: vec![],
            requires_approval: false,
        })
    }

    async fn execute(
        &self,
        args: &serde_json::Value,
        context: &CommandContext,
    ) -> Result<CommandResult> {
        let args: CommitTemplateArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid commit-template arguments: {}", e),
            )))
        })?;

        match self.generate_template(&args, context).await {
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
        let _args: CommitTemplateArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid commit-template arguments: {}", e),
            )))
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_util::sync::CancellationToken;

    #[test]
    fn test_default_true() {
        assert!(default_true());
    }

    #[tokio::test]
    async fn test_commit_template_no_workspace() {
        let command = CommitTemplateCommand::new();
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
        assert!(!result.success);
        assert!(result.error.is_some());
    }
}
