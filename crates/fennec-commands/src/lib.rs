pub mod action_log;
pub mod commit_template;
pub mod common;
pub mod compiler_errors;
pub mod create;
pub mod delete;
pub mod diff;
pub mod edit;
pub mod error;
pub mod file_ops;
pub mod find_symbol;
pub mod fix_errors;
pub mod git_integration;
pub mod history;
pub mod hunks;
pub mod plan;
pub mod pr_summary;
pub mod redo;
pub mod registry;
pub mod rename;
pub mod run;
pub mod search;
pub mod symbols;
pub mod summarize;
pub mod summarize_enhanced;
pub mod test_watch;
pub mod undo;

#[cfg(test)]
mod tests;

// Re-export key types and functions for easy use
pub use action_log::{Action, ActionLog, ActionState};
pub use common::{format_file_size, initialize_builtin_commands, is_text_file, truncate_text};
pub use error::{CommandError, Result as CommandResult};
pub use hunks::{apply_hunks, split_diff_into_hunks, Hunk, HunkStatus};
pub use registry::{
    CommandContext, CommandDescriptor, CommandExecutionResult, CommandExecutor, CommandRegistry,
};

// Re-export individual commands
pub use commit_template::{CommitTemplateArgs, CommitTemplateCommand};
pub use compiler_errors::{CompilerMessage, FixConfidence, MessageLevel, SuggestedFix};
pub use create::{CreateArgs, CreateCommand};
pub use delete::{DeleteArgs, DeleteCommand};
pub use diff::{DiffArgs, DiffCommand};
pub use edit::{EditArgs, EditCommand};
pub use file_ops::{
    EditStrategy, FileEditRequest, FileEditResult, FileOperations, FileOperationsConfig,
};
pub use find_symbol::{FindSymbolArgs, FindSymbolCommand};
pub use fix_errors::{FixErrorsArgs, FixErrorsCommand};
pub use git_integration::{ChangeType, FileChange, GitCommit};
pub use history::{HistoryArgs, HistoryCommand};
pub use plan::{PlanArgs, PlanCommand};
pub use pr_summary::{PrSummaryArgs, PrSummaryCommand};
pub use redo::{RedoArgs, RedoCommand};
pub use rename::{RenameArgs, RenameCommand};
pub use run::{RunArgs, RunCommand};
pub use search::{SearchArgs, SearchCommand, SearchResult};
pub use summarize::{SummarizeArgs, SummarizeCommand};
pub use summarize_enhanced::{
    EnhancedSummarizeArgs, EnhancedSummarizeCommand, OutputDestination, SummaryDepth, SummaryType,
};
pub use symbols::{Symbol, SymbolIndex, SymbolType, Visibility as SymbolVisibility};
pub use test_watch::{TestWatchArgs, TestWatchCommand};
pub use undo::{UndoArgs, UndoCommand};

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
///         action_log: None,
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
