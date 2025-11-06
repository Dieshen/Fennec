use anyhow::Result;
use crate::git_integration::{generate_pr_summary, get_commits, get_current_branch};
use crate::registry::{CommandContext, CommandDescriptor, CommandExecutor};
use fennec_core::command::{Capability, CommandPreview, CommandResult};
use fennec_core::error::FennecError;
use fennec_security::SandboxLevel;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrSummaryArgs {
    /// Branch to compare against (defaults to main/master)
    #[serde(default)]
    pub base_branch: Option<String>,

    /// Branch to summarize (defaults to current branch)
    #[serde(default)]
    pub head_branch: Option<String>,

    /// Maximum number of commits to include
    #[serde(default = "default_max_commits")]
    pub max_commits: usize,
}

fn default_max_commits() -> usize {
    50
}

pub struct PrSummaryCommand {
    descriptor: CommandDescriptor,
}

impl PrSummaryCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor {
                name: "pr-summary".to_string(),
                description: "Generate a pull request summary from git commits".to_string(),
                version: "1.0.0".to_string(),
                author: Some("Fennec Contributors".to_string()),
                capabilities_required: vec![Capability::ExecuteShell],
                sandbox_level_required: SandboxLevel::ReadOnly,
                supports_preview: false,
                supports_dry_run: false,
            },
        }
    }

    async fn generate_summary(
        &self,
        args: &PrSummaryArgs,
        context: &CommandContext,
    ) -> Result<String> {
        let workspace_path = context.workspace_path.as_ref().ok_or_else(|| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No workspace path set",
            )))
        })?;

        // Get current branch if not specified
        let current_branch = if args.head_branch.is_none() {
            match get_current_branch(workspace_path).await {
                Ok(branch) => branch,
                Err(_) => "HEAD".to_string(),
            }
        } else {
            args.head_branch.clone().unwrap()
        };

        // Determine base branch
        let base_branch = args.base_branch.clone().unwrap_or_else(|| {
            // Try to detect main branch name
            "main".to_string()
        });

        // Get commits between base and head
        let branch_range = if current_branch == "HEAD" {
            format!("{}..HEAD", base_branch)
        } else {
            format!("{}..{}", base_branch, current_branch)
        };

        let commits = get_commits(workspace_path, Some(&branch_range), Some(args.max_commits))
            .await
            .map_err(|e| {
                FennecError::Command(Box::new(std::io::Error::new(
                    e.kind(),
                    format!("Failed to get git commits: {}", e),
                )))
            })?;

        if commits.is_empty() {
            return Ok(format!(
                "No commits found between '{}' and '{}'.\n\nThis could mean:\n- The branches are up to date\n- The base branch doesn't exist\n- The current branch is not ahead of the base branch",
                base_branch, current_branch
            ));
        }

        // Generate summary
        let mut output = format!(
            "# Pull Request Summary\n\n**From**: `{}` **To**: `{}`\n\n",
            current_branch, base_branch
        );

        output.push_str(&generate_pr_summary(&commits));

        Ok(output)
    }
}

impl Default for PrSummaryCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl CommandExecutor for PrSummaryCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn preview(
        &self,
        args: &serde_json::Value,
        _context: &CommandContext,
    ) -> Result<CommandPreview> {
        let args: PrSummaryArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid pr-summary arguments: {}", e),
            )))
        })?;

        let base = args.base_branch.unwrap_or_else(|| "main".to_string());
        let head = args.head_branch.unwrap_or_else(|| "current".to_string());

        Ok(CommandPreview {
            command_id: Uuid::new_v4(),
            description: format!("Generate PR summary from {} to {}", head, base),
            actions: vec![],
            requires_approval: false,
        })
    }

    async fn execute(
        &self,
        args: &serde_json::Value,
        context: &CommandContext,
    ) -> Result<CommandResult> {
        let args: PrSummaryArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid pr-summary arguments: {}", e),
            )))
        })?;

        match self.generate_summary(&args, context).await {
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
        let _args: PrSummaryArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid pr-summary arguments: {}", e),
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
    fn test_default_max_commits() {
        assert_eq!(default_max_commits(), 50);
    }

    #[tokio::test]
    async fn test_pr_summary_no_workspace() {
        let command = PrSummaryCommand::new();
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
