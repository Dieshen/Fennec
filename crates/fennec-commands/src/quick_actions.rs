use anyhow::Result;
use crate::registry::{CommandContext, CommandDescriptor, CommandExecutor};
use fennec_core::command::{Capability, CommandPreview, CommandResult};
use fennec_core::error::FennecError;
use fennec_security::SandboxLevel;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A quick action template for common workflows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickAction {
    pub id: String,
    pub name: String,
    pub description: String,
    pub prompt_template: String,
    pub required_context: Vec<String>,
    pub tags: Vec<String>,
}

impl QuickAction {
    /// Apply context variables to the template
    pub fn apply_context(&self, context_vars: &HashMap<String, String>) -> Result<String> {
        let mut result = self.prompt_template.clone();

        for (key, value) in context_vars {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }

        Ok(result)
    }
}

/// Built-in quick actions
pub fn get_builtin_actions() -> Vec<QuickAction> {
    vec![
        QuickAction {
            id: "fix-error".to_string(),
            name: "Fix This Error".to_string(),
            description: "Suggest fixes for the error at current location".to_string(),
            prompt_template: "The following error occurred in {{file}} at line {{line}}:\n\n{{error}}\n\nPlease suggest fixes for this error.".to_string(),
            required_context: vec!["file".to_string(), "line".to_string(), "error".to_string()],
            tags: vec!["error".to_string(), "fix".to_string()],
        },
        QuickAction {
            id: "add-tests".to_string(),
            name: "Add Tests for Function".to_string(),
            description: "Generate unit tests for the selected function".to_string(),
            prompt_template: "Generate comprehensive unit tests for the following function in {{file}}:\n\n```rust\n{{code}}\n```\n\nInclude edge cases and error handling tests.".to_string(),
            required_context: vec!["file".to_string(), "code".to_string()],
            tags: vec!["testing".to_string(), "quality".to_string()],
        },
        QuickAction {
            id: "document-function".to_string(),
            name: "Document This Function".to_string(),
            description: "Add documentation comments to a function".to_string(),
            prompt_template: "Add comprehensive Rust doc comments to this function:\n\n```rust\n{{code}}\n```\n\nInclude:\n- Brief description\n- Parameters\n- Return value\n- Examples\n- Errors/Panics if applicable".to_string(),
            required_context: vec!["code".to_string()],
            tags: vec!["docs".to_string(), "quality".to_string()],
        },
        QuickAction {
            id: "add-error-handling".to_string(),
            name: "Add Error Handling".to_string(),
            description: "Add proper error handling to code".to_string(),
            prompt_template: "Refactor this code to add proper error handling:\n\n```rust\n{{code}}\n```\n\nUse Result<T, E> pattern and handle all error cases appropriately.".to_string(),
            required_context: vec!["code".to_string()],
            tags: vec!["error-handling".to_string(), "quality".to_string()],
        },
        QuickAction {
            id: "optimize-code".to_string(),
            name: "Optimize This Code".to_string(),
            description: "Suggest performance optimizations".to_string(),
            prompt_template: "Analyze and suggest performance optimizations for:\n\n```rust\n{{code}}\n```\n\nFocus on:\n- Algorithmic complexity\n- Memory allocations\n- Unnecessary cloning\n- Iterator efficiency".to_string(),
            required_context: vec!["code".to_string()],
            tags: vec!["performance".to_string(), "optimization".to_string()],
        },
        QuickAction {
            id: "refactor-pattern".to_string(),
            name: "Refactor to Pattern".to_string(),
            description: "Refactor code to use a specific design pattern".to_string(),
            prompt_template: "Refactor this code to use the {{pattern}} pattern:\n\n```rust\n{{code}}\n```\n\nExplain the benefits of this refactoring.".to_string(),
            required_context: vec!["code".to_string(), "pattern".to_string()],
            tags: vec!["refactoring".to_string(), "patterns".to_string()],
        },
        QuickAction {
            id: "explain-code".to_string(),
            name: "Explain This Code".to_string(),
            description: "Provide detailed explanation of code".to_string(),
            prompt_template: "Explain what this code does in detail:\n\n```rust\n{{code}}\n```\n\nInclude:\n- High-level overview\n- Step-by-step breakdown\n- Key concepts used\n- Potential improvements".to_string(),
            required_context: vec!["code".to_string()],
            tags: vec!["learning".to_string(), "documentation".to_string()],
        },
        QuickAction {
            id: "security-review".to_string(),
            name: "Security Review".to_string(),
            description: "Review code for security issues".to_string(),
            prompt_template: "Perform a security review of this code:\n\n```rust\n{{code}}\n```\n\nCheck for:\n- Input validation\n- Injection vulnerabilities\n- Authentication/authorization issues\n- Resource exhaustion risks\n- Unsafe operations".to_string(),
            required_context: vec!["code".to_string()],
            tags: vec!["security".to_string(), "review".to_string()],
        },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickActionArgs {
    /// ID of the quick action to execute
    pub action_id: String,

    /// Context variables to apply to the template
    #[serde(default)]
    pub context: HashMap<String, String>,

    /// List all available actions instead of executing
    #[serde(default)]
    pub list: bool,
}

pub struct QuickActionCommand {
    descriptor: CommandDescriptor,
    actions: Vec<QuickAction>,
}

impl QuickActionCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor {
                name: "quick-action".to_string(),
                description: "Execute pre-defined workflow templates for common tasks".to_string(),
                version: "1.0.0".to_string(),
                author: Some("Fennec Contributors".to_string()),
                capabilities_required: vec![Capability::ReadFile],
                sandbox_level_required: SandboxLevel::ReadOnly,
                supports_preview: true,
                supports_dry_run: false,
            },
            actions: get_builtin_actions(),
        }
    }

    fn find_action(&self, id: &str) -> Option<&QuickAction> {
        self.actions.iter().find(|a| a.id == id)
    }

    fn list_actions(&self) -> String {
        let mut output = String::new();
        output.push_str("⚡ Available Quick Actions\n\n");

        for action in &self.actions {
            output.push_str(&format!("**{}** ({})\n", action.name, action.id));
            output.push_str(&format!("  {}\n", action.description));
            output.push_str("  Required context: ");
            output.push_str(&action.required_context.join(", "));
            output.push_str("\n");
            output.push_str(&format!("  Tags: {}\n\n", action.tags.join(", ")));
        }

        output
    }

    async fn execute_action(
        &self,
        args: &QuickActionArgs,
        _context: &CommandContext,
    ) -> Result<String> {
        if args.list {
            return Ok(self.list_actions());
        }

        let action = self.find_action(&args.action_id).ok_or_else(|| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Quick action '{}' not found", args.action_id),
            )))
        })?;

        // Validate required context
        for required in &action.required_context {
            if !args.context.contains_key(required) {
                return Err(FennecError::Command(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Missing required context variable: '{}'", required),
                )))
                .into());
            }
        }

        // Apply context to template
        let prompt = action.apply_context(&args.context)?;

        let mut output = String::new();
        output.push_str(&format!("⚡ Executing Quick Action: {}\n\n", action.name));
        output.push_str("Generated Prompt:\n");
        output.push_str("─────────────────────────────────────\n");
        output.push_str(&prompt);
        output.push_str("\n─────────────────────────────────────\n\n");
        output.push_str("💡 Tip: Use this prompt with your AI assistant to get suggestions.\n");

        Ok(output)
    }
}

impl Default for QuickActionCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl CommandExecutor for QuickActionCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn preview(
        &self,
        args: &serde_json::Value,
        _context: &CommandContext,
    ) -> Result<CommandPreview> {
        let args: QuickActionArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid quick-action arguments: {}", e),
            )))
        })?;

        let description = if args.list {
            "List all available quick actions".to_string()
        } else if let Some(action) = self.find_action(&args.action_id) {
            format!("Execute: {}", action.name)
        } else {
            format!("Unknown action: {}", args.action_id)
        };

        Ok(CommandPreview {
            command_id: Uuid::new_v4(),
            description,
            actions: vec![],
            requires_approval: false,
        })
    }

    async fn execute(
        &self,
        args: &serde_json::Value,
        context: &CommandContext,
    ) -> Result<CommandResult> {
        let args: QuickActionArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid quick-action arguments: {}", e),
            )))
        })?;

        match self.execute_action(&args, context).await {
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
        let args: QuickActionArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid quick-action arguments: {}", e),
            )))
        })?;

        if !args.list && args.action_id.is_empty() {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "action_id is required (or use --list to see available actions)",
            )))
            .into());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_actions_exist() {
        let actions = get_builtin_actions();
        assert!(!actions.is_empty());
        assert!(actions.iter().any(|a| a.id == "fix-error"));
        assert!(actions.iter().any(|a| a.id == "add-tests"));
    }

    #[test]
    fn test_apply_context() {
        let action = QuickAction {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test".to_string(),
            prompt_template: "File: {{file}}, Line: {{line}}".to_string(),
            required_context: vec!["file".to_string(), "line".to_string()],
            tags: vec![],
        };

        let mut context = HashMap::new();
        context.insert("file".to_string(), "main.rs".to_string());
        context.insert("line".to_string(), "42".to_string());

        let result = action.apply_context(&context).unwrap();
        assert_eq!(result, "File: main.rs, Line: 42");
    }

    #[test]
    fn test_find_action() {
        let command = QuickActionCommand::new();
        assert!(command.find_action("fix-error").is_some());
        assert!(command.find_action("nonexistent").is_none());
    }

    #[test]
    fn test_validate_args() {
        let command = QuickActionCommand::new();

        // Valid - list mode
        let list_args = serde_json::json!({
            "action_id": "",
            "list": true
        });
        assert!(command.validate_args(&list_args).is_ok());

        // Valid - with action
        let action_args = serde_json::json!({
            "action_id": "fix-error"
        });
        assert!(command.validate_args(&action_args).is_ok());

        // Invalid - no action and no list
        let invalid_args = serde_json::json!({
            "action_id": ""
        });
        assert!(command.validate_args(&invalid_args).is_err());
    }
}
