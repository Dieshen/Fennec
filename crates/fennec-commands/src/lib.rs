pub mod common;
pub mod diff;
pub mod edit;
pub mod file_ops;
pub mod plan;
pub mod registry;
pub mod run;
pub mod summarize;

// Re-export key types and functions for easy use
pub use common::{format_file_size, initialize_builtin_commands, is_text_file, truncate_text};
pub use registry::{
    CommandContext, CommandDescriptor, CommandExecutionResult, CommandExecutor, CommandRegistry,
};

// Re-export individual commands
pub use diff::{DiffArgs, DiffCommand};
pub use edit::{EditArgs, EditCommand};
pub use file_ops::{
    EditStrategy, FileEditRequest, FileEditResult, FileOperations, FileOperationsConfig,
};
pub use plan::{PlanArgs, PlanCommand};
pub use run::{RunArgs, RunCommand};
pub use summarize::{SummarizeArgs, SummarizeCommand};

/// Create a fully initialized command registry with all built-in commands
///
/// This is the main entry point for getting a ready-to-use command system.
///
/// # Example
///
/// ```rust
/// use fennec_commands::create_command_registry;
/// use fennec_core::command::Capability;
/// use fennec_security::SandboxLevel;
/// use tokio_util::sync::CancellationToken;
/// use uuid::Uuid;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let registry = create_command_registry().await?;
///     
///     // List available commands
///     let commands = registry.list_commands().await;
///     println!("Available commands: {:?}",
///         commands.iter().map(|c| &c.name).collect::<Vec<_>>());
///     
///     // Execute a command
///     let context = fennec_commands::CommandContext {
///         session_id: Uuid::new_v4(),
///         user_id: None,
///         workspace_path: None,
///         sandbox_level: SandboxLevel::ReadOnly,
///         dry_run: false,
///         preview_only: false,
///         cancellation_token: CancellationToken::new(),
///     };
///     
///     let args = serde_json::json!({
///         "task": "Implement a simple web server",
///         "complexity": "moderate"
///     });
///     
///     let result = registry.execute_command("plan", &args, &context).await?;
///     println!("Plan result: {}", result.output);
///     
///     Ok(())
/// }
/// ```
pub async fn create_command_registry() -> anyhow::Result<CommandRegistry> {
    initialize_builtin_commands().await
}
