use anyhow::Result;
use async_trait::async_trait;
use fennec_core::{
    command::{Capability, CommandPreview, CommandResult, PreviewAction},
    error::FennecError,
};
use fennec_memory::agents::AgentsService;
use fennec_security::SandboxLevel;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::registry::{CommandContext, CommandDescriptor, CommandExecutor};

/// Arguments for the plan command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanArgs {
    /// The task or goal to plan for
    pub task: String,
    /// Optional context or requirements
    pub context: Option<String>,
    /// Whether to include implementation steps
    pub include_implementation: Option<bool>,
    /// Target complexity level (simple, moderate, complex)
    pub complexity: Option<String>,
}

/// Plan command for creating structured task plans
pub struct PlanCommand {
    descriptor: CommandDescriptor,
    agents_service: AgentsService,
}

impl PlanCommand {
    pub async fn new() -> Result<Self> {
        let agents_service = AgentsService::new().await?;

        Ok(Self {
            descriptor: CommandDescriptor {
                name: "plan".to_string(),
                description: "Create a structured plan for accomplishing a task or goal"
                    .to_string(),
                version: "1.0.0".to_string(),
                author: Some("Fennec Core".to_string()),
                capabilities_required: vec![Capability::ReadFile],
                sandbox_level_required: SandboxLevel::ReadOnly,
                supports_preview: true,
                supports_dry_run: true,
            },
            agents_service,
        })
    }

    /// Generate a structured plan based on the task and available guidance
    async fn generate_plan(&self, args: &PlanArgs, context: &CommandContext) -> Result<String> {
        let mut plan_parts = Vec::new();

        // Add task description
        plan_parts.push(format!("# Plan: {}", args.task));
        plan_parts.push(String::new());

        // Add context if provided
        if let Some(ref ctx) = args.context {
            plan_parts.push("## Context".to_string());
            plan_parts.push(ctx.clone());
            plan_parts.push(String::new());
        }

        // Search for relevant guidance from AGENTS.md
        let guidance_matches = self.agents_service.search_guidance(&args.task);
        if !guidance_matches.is_empty() {
            plan_parts.push("## Relevant Guidance".to_string());
            for (i, guidance) in guidance_matches.iter().take(3).enumerate() {
                plan_parts.push(format!("### {}", guidance.section_title));
                // Truncate content if too long
                let content = if guidance.content.len() > 300 {
                    format!("{}...", &guidance.content[..300])
                } else {
                    guidance.content.clone()
                };
                plan_parts.push(content);
                if i < 2 {
                    plan_parts.push(String::new());
                }
            }
            plan_parts.push(String::new());
        }

        // Generate plan structure based on complexity
        let complexity = args.complexity.as_deref().unwrap_or("moderate");
        plan_parts.push("## Plan Structure".to_string());

        match complexity {
            "simple" => {
                plan_parts.extend(vec![
                    "### 1. Preparation".to_string(),
                    "- Review requirements and constraints".to_string(),
                    "- Gather necessary resources".to_string(),
                    String::new(),
                    "### 2. Implementation".to_string(),
                    "- Execute the main task".to_string(),
                    "- Monitor progress".to_string(),
                    String::new(),
                    "### 3. Validation".to_string(),
                    "- Test and verify results".to_string(),
                    "- Document outcomes".to_string(),
                ]);
            }
            "complex" => {
                plan_parts.extend(vec![
                    "### 1. Analysis & Planning".to_string(),
                    "- Break down the task into components".to_string(),
                    "- Identify dependencies and risks".to_string(),
                    "- Create detailed timeline".to_string(),
                    String::new(),
                    "### 2. Design & Architecture".to_string(),
                    "- Design overall approach".to_string(),
                    "- Plan integration points".to_string(),
                    "- Consider scalability and maintainability".to_string(),
                    String::new(),
                    "### 3. Implementation".to_string(),
                    "- Implement core functionality".to_string(),
                    "- Add error handling and logging".to_string(),
                    "- Implement tests".to_string(),
                    String::new(),
                    "### 4. Integration & Testing".to_string(),
                    "- Integration testing".to_string(),
                    "- Performance testing".to_string(),
                    "- User acceptance testing".to_string(),
                    String::new(),
                    "### 5. Deployment & Monitoring".to_string(),
                    "- Deploy to staging/production".to_string(),
                    "- Set up monitoring and alerts".to_string(),
                    "- Document deployment process".to_string(),
                ]);
            }
            _ => {
                // moderate
                plan_parts.extend(vec![
                    "### 1. Planning & Research".to_string(),
                    "- Understand requirements thoroughly".to_string(),
                    "- Research best practices and existing solutions".to_string(),
                    "- Identify potential challenges".to_string(),
                    String::new(),
                    "### 2. Design & Setup".to_string(),
                    "- Design the approach and architecture".to_string(),
                    "- Set up development environment".to_string(),
                    "- Create project structure".to_string(),
                    String::new(),
                    "### 3. Implementation".to_string(),
                    "- Implement core functionality".to_string(),
                    "- Add proper error handling".to_string(),
                    "- Write tests as you go".to_string(),
                    String::new(),
                    "### 4. Testing & Refinement".to_string(),
                    "- Comprehensive testing".to_string(),
                    "- Performance optimization".to_string(),
                    "- Code review and refactoring".to_string(),
                    String::new(),
                    "### 5. Documentation & Deployment".to_string(),
                    "- Write documentation".to_string(),
                    "- Prepare for deployment".to_string(),
                    "- Create maintenance plan".to_string(),
                ]);
            }
        }

        // Add implementation steps if requested
        if args.include_implementation.unwrap_or(false) {
            plan_parts.push(String::new());
            plan_parts.push("## Implementation Checklist".to_string());
            plan_parts.extend(vec![
                "- [ ] Set up development environment".to_string(),
                "- [ ] Create initial project structure".to_string(),
                "- [ ] Implement core functionality".to_string(),
                "- [ ] Add comprehensive error handling".to_string(),
                "- [ ] Write unit tests".to_string(),
                "- [ ] Write integration tests".to_string(),
                "- [ ] Performance testing".to_string(),
                "- [ ] Code review".to_string(),
                "- [ ] Documentation".to_string(),
                "- [ ] Deployment preparation".to_string(),
            ]);
        }

        // Add notes section
        plan_parts.push(String::new());
        plan_parts.push("## Notes".to_string());
        plan_parts
            .push("- Review and adjust this plan as needed during implementation".to_string());
        plan_parts.push(
            "- Consider breaking down large steps into smaller, manageable tasks".to_string(),
        );
        plan_parts
            .push("- Don't hesitate to ask for help or clarification when needed".to_string());

        if context.dry_run {
            plan_parts.push(String::new());
            plan_parts.push("*Note: This plan was generated in dry-run mode*".to_string());
        }

        Ok(plan_parts.join("\n"))
    }
}

#[async_trait]
impl CommandExecutor for PlanCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn preview(
        &self,
        args: &serde_json::Value,
        _context: &CommandContext,
    ) -> Result<CommandPreview> {
        let args: PlanArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Invalid plan arguments: {}", e),
            )))
        })?;

        Ok(CommandPreview {
            command_id: Uuid::new_v4(),
            description: format!("Generate a structured plan for: {}", args.task),
            actions: vec![PreviewAction::ReadFile {
                path: "AGENTS.md (if available)".to_string(),
            }],
            requires_approval: false,
        })
    }

    async fn execute(
        &self,
        args: &serde_json::Value,
        context: &CommandContext,
    ) -> Result<CommandResult> {
        let args: PlanArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Invalid plan arguments: {}", e),
            )))
        })?;

        // Check for cancellation
        if context.cancellation_token.is_cancelled() {
            return Ok(CommandResult {
                command_id: Uuid::new_v4(),
                success: false,
                output: String::new(),
                error: Some("Command was cancelled".to_string()),
            });
        }

        match self.generate_plan(&args, context).await {
            Ok(plan) => Ok(CommandResult {
                command_id: Uuid::new_v4(),
                success: true,
                output: plan,
                error: None,
            }),
            Err(e) => Ok(CommandResult {
                command_id: Uuid::new_v4(),
                success: false,
                output: String::new(),
                error: Some(format!("Failed to generate plan: {}", e)),
            }),
        }
    }

    fn validate_args(&self, args: &serde_json::Value) -> Result<()> {
        let args: PlanArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Invalid plan arguments: {}", e),
            )))
        })?;

        if args.task.trim().is_empty() {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Task description cannot be empty",
            )))
            .into());
        }

        if let Some(ref complexity) = args.complexity {
            if !matches!(complexity.as_str(), "simple" | "moderate" | "complex") {
                return Err(FennecError::Command(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Complexity must be one of: simple, moderate, complex",
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
    async fn test_plan_command_validation() {
        let command = PlanCommand::new().await.unwrap();

        // Valid args
        let valid_args = serde_json::json!({
            "task": "Implement user authentication",
            "complexity": "moderate"
        });
        assert!(command.validate_args(&valid_args).is_ok());

        // Empty task
        let empty_task = serde_json::json!({
            "task": "",
        });
        assert!(command.validate_args(&empty_task).is_err());

        // Invalid complexity
        let invalid_complexity = serde_json::json!({
            "task": "Test task",
            "complexity": "invalid"
        });
        assert!(command.validate_args(&invalid_complexity).is_err());
    }

    #[tokio::test]
    async fn test_plan_command_execution() {
        let command = PlanCommand::new().await.unwrap();

        let args = serde_json::json!({
            "task": "Implement user authentication",
            "context": "Using Rust and JWT tokens",
            "complexity": "moderate",
            "include_implementation": true
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
        assert!(result
            .output
            .contains("Plan: Implement user authentication"));
        assert!(result.output.contains("Context"));
        assert!(result.output.contains("Implementation Checklist"));
    }
}
