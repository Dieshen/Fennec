use anyhow::Result;
use fennec_commands::{create_command_registry, CommandContext};
use fennec_security::SandboxLevel;
use tempfile::tempdir;
use tokio::fs;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// Helper function to create a test command context
fn create_test_context(
    sandbox_level: SandboxLevel,
    dry_run: bool,
    workspace_path: Option<String>,
) -> CommandContext {
    CommandContext {
        session_id: Uuid::new_v4(),
        user_id: Some("test_user".to_string()),
        workspace_path,
        sandbox_level,
        dry_run,
        preview_only: false,
        cancellation_token: CancellationToken::new(),
        action_log: None,
    }
}

#[tokio::test]
async fn test_command_registry_initialization() -> Result<()> {
    let registry = create_command_registry().await?;

    // Verify all built-in commands are registered
    let commands = registry.list_commands().await;
    let command_names: Vec<String> = commands.iter().map(|c| c.name.clone()).collect();

    // Expect all 12 built-in commands
    assert!(command_names.contains(&"plan".to_string()));
    assert!(command_names.contains(&"create".to_string()));
    assert!(command_names.contains(&"delete".to_string()));
    assert!(command_names.contains(&"rename".to_string()));
    assert!(command_names.contains(&"edit".to_string()));
    assert!(command_names.contains(&"run".to_string()));
    assert!(command_names.contains(&"diff".to_string()));
    assert!(command_names.contains(&"search".to_string()));
    assert!(command_names.contains(&"find-symbol".to_string()));
    assert!(command_names.contains(&"fix-errors".to_string()));
    assert!(command_names.contains(&"summarize".to_string()));
    assert!(command_names.contains(&"summarize_enhanced".to_string()));

    // Ensure we didn't unintentionally register duplicates
    assert_eq!(command_names.len(), 12);

    Ok(())
}

#[tokio::test]
async fn test_plan_command_integration() -> Result<()> {
    let registry = create_command_registry().await?;
    let context = create_test_context(SandboxLevel::ReadOnly, false, None);

    let args = serde_json::json!({
        "task": "Create a REST API for user management",
        "context": "Using Rust and Axum framework",
        "complexity": "moderate",
        "include_implementation": true
    });

    // Test preview first
    let preview_context = CommandContext {
        preview_only: true,
        ..context.clone()
    };

    let preview_result = registry
        .execute_command("plan", &args, &preview_context)
        .await?;
    assert!(preview_result.success);
    assert!(preview_result.preview.is_some());

    // Test actual execution
    let result = registry.execute_command("plan", &args, &context).await?;
    assert!(result.success);
    assert!(result.output.contains("REST API for user management"));
    assert!(result.output.contains("Implementation Checklist"));
    // The complexity level appears in the plan structure section
    assert!(result.output.contains("Plan Structure"));

    Ok(())
}

#[tokio::test]
async fn test_edit_command_integration() -> Result<()> {
    let temp_dir = tempdir()?;
    let test_file = temp_dir.path().join("test.txt");
    let initial_content = "Hello, World!\nThis is a test file.\nLine 3";

    fs::write(&test_file, initial_content).await?;

    let registry = create_command_registry().await?;
    let context = create_test_context(
        SandboxLevel::FullAccess,
        false,
        Some(temp_dir.path().to_string_lossy().to_string()),
    );

    // Test search and replace
    let args = serde_json::json!({
        "file_path": test_file.to_string_lossy(),
        "strategy": {
            "type": "SearchReplace",
            "data": { "search": "World", "replace": "Universe" }
        },
        "backup": true
    });

    let result = registry.execute_command("edit", &args, &context).await?;
    assert!(result.success);
    assert!(result.output.contains("Successfully edited file"));

    // Verify the change
    let new_content = fs::read_to_string(&test_file).await?;
    assert!(new_content.contains("Hello, Universe!"));

    Ok(())
}

#[tokio::test]
async fn test_run_command_integration() -> Result<()> {
    let registry = create_command_registry().await?;
    let context = create_test_context(SandboxLevel::FullAccess, false, None);

    // Test simple echo command
    let args = serde_json::json!({
        "command": "echo Hello from Fennec",
        "capture_output": true,
        "timeout_seconds": 5
    });

    let result = registry.execute_command("run", &args, &context).await?;
    assert!(result.success);
    assert!(result.output.contains("Hello from Fennec"));
    assert!(result.output.contains("Exit code: 0"));

    Ok(())
}

#[tokio::test]
async fn test_diff_command_integration() -> Result<()> {
    let registry = create_command_registry().await?;
    let context = create_test_context(SandboxLevel::ReadOnly, false, None);

    // Test text diff
    let args = serde_json::json!({
        "left": "Hello\nWorld\nFrom\nFennec",
        "right": "Hello\nUniverse\nFrom\nFennec",
        "is_file_path": false,
        "format": "unified"
    });

    let result = registry.execute_command("diff", &args, &context).await?;
    assert!(result.success);
    assert!(result.output.contains("-World"));
    assert!(result.output.contains("+Universe"));

    Ok(())
}

#[tokio::test]
async fn test_summarize_command_integration() -> Result<()> {
    let registry = create_command_registry().await?;
    let context = create_test_context(SandboxLevel::ReadOnly, false, None);

    // Test text summarization
    let long_text = "This is a test document.\nIt contains multiple lines.\nEach line has different content.\nSome lines are longer than others and contain more detailed information.\nThe purpose is to test the summarize command functionality.";

    let args = serde_json::json!({
        "target": long_text,
        "is_path": false,
        "max_lines": 10
    });

    let result = registry
        .execute_command("summarize", &args, &context)
        .await?;
    assert!(result.success);
    assert!(result.output.contains("Text Summary"));
    assert!(result.output.contains("Lines:"));
    assert!(result.output.contains("Words:"));
    assert!(result.output.contains("Characters:"));

    Ok(())
}

#[tokio::test]
async fn test_sandbox_security_enforcement() -> Result<()> {
    let registry = create_command_registry().await?;

    // Test that read-only sandbox blocks write operations
    let readonly_context = create_test_context(SandboxLevel::ReadOnly, false, None);

    let edit_args = serde_json::json!({
        "file_path": "/tmp/test.txt",
        "content": "This should fail"
    });

    let result = registry
        .execute_command("edit", &edit_args, &readonly_context)
        .await?;
    assert!(!result.success);
    assert!(result.error.is_some());
    let error_msg = result.error.unwrap();
    assert!(error_msg.contains("WorkspaceWrite") || error_msg.contains("ReadOnly"));

    // Test that read-only sandbox blocks shell execution
    let run_args = serde_json::json!({
        "command": "echo test"
    });

    let result = registry
        .execute_command("run", &run_args, &readonly_context)
        .await?;
    assert!(!result.success);
    assert!(result.error.is_some());
    let error_msg = result.error.unwrap();
    assert!(error_msg.contains("read-only") || error_msg.contains("ReadOnly"));

    Ok(())
}

#[tokio::test]
async fn test_command_preview_functionality() -> Result<()> {
    let registry = create_command_registry().await?;
    let context = create_test_context(SandboxLevel::ReadOnly, false, None);

    // Test plan command preview
    let args = serde_json::json!({
        "task": "Test task",
        "complexity": "simple"
    });

    let preview_context = CommandContext {
        preview_only: true,
        ..context
    };

    let result = registry
        .execute_command("plan", &args, &preview_context)
        .await?;
    assert!(result.success);
    assert!(result.preview.is_some());

    let preview = result.preview.unwrap();
    assert!(preview.description.contains("Test task"));
    assert!(!preview.actions.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_dry_run_functionality() -> Result<()> {
    let registry = create_command_registry().await?;
    let context = create_test_context(SandboxLevel::FullAccess, true, None);

    // Test dry run with edit command
    let temp_path = std::env::temp_dir().join("nonexistent.txt");
    let args = serde_json::json!({
        "file_path": temp_path.to_string_lossy(),
        "strategy": {
            "type": "Replace",
            "data": { "content": "test content" }
        },
        "create_if_missing": true
    });

    let result = registry.execute_command("edit", &args, &context).await?;
    assert!(result.success);
    assert!(result.output.contains("DRY RUN"));

    // Test dry run with run command
    let run_args = serde_json::json!({
        "command": "echo dry run test"
    });

    let result = registry.execute_command("run", &run_args, &context).await?;
    assert!(result.success);
    assert!(result.output.contains("Would execute"));

    Ok(())
}

#[tokio::test]
async fn test_command_validation() -> Result<()> {
    let registry = create_command_registry().await?;
    let context = create_test_context(SandboxLevel::ReadOnly, false, None);

    // Test invalid plan args
    let invalid_args = serde_json::json!({
        "task": "",  // Empty task should fail
        "complexity": "moderate"
    });

    let result = registry
        .execute_command("plan", &invalid_args, &context)
        .await?;
    assert!(!result.success, "Plan command should fail with empty task");
    assert!(
        result.error.is_some(),
        "Plan command should return error message"
    );

    // Test invalid edit args
    let invalid_edit_args = serde_json::json!({
        "file_path": "test.txt"
        // Missing required operation (content, search/replace, or line range)
    });

    let result = registry
        .execute_command("edit", &invalid_edit_args, &context)
        .await?;
    assert!(
        !result.success,
        "Edit command should fail with missing operation"
    );
    assert!(
        result.error.is_some(),
        "Edit command should return error message"
    );

    Ok(())
}

#[tokio::test]
async fn test_command_filtering_by_capabilities() -> Result<()> {
    let registry = create_command_registry().await?;

    // Test filtering by ReadFile capability
    let read_commands = registry
        .list_commands_by_capability(&fennec_core::command::Capability::ReadFile)
        .await;

    // Plan, diff, and summarize commands should be included
    let read_command_names: Vec<String> = read_commands.iter().map(|c| c.name.clone()).collect();
    assert!(read_command_names.contains(&"plan".to_string()));
    assert!(read_command_names.contains(&"diff".to_string()));
    assert!(read_command_names.contains(&"summarize".to_string()));

    // Test filtering by ExecuteShell capability
    let shell_commands = registry
        .list_commands_by_capability(&fennec_core::command::Capability::ExecuteShell)
        .await;

    // run and fix-errors commands should be included
    let shell_command_names: Vec<String> = shell_commands.iter().map(|c| c.name.clone()).collect();
    assert_eq!(shell_commands.len(), 2);
    assert!(shell_command_names.contains(&"run".to_string()));
    assert!(shell_command_names.contains(&"fix-errors".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_command_filtering_by_sandbox_level() -> Result<()> {
    let registry = create_command_registry().await?;

    // Test read-only commands
    let readonly_commands = registry
        .list_commands_for_sandbox(&SandboxLevel::ReadOnly)
        .await;

    let readonly_names: Vec<String> = readonly_commands.iter().map(|c| c.name.clone()).collect();
    assert!(readonly_names.contains(&"plan".to_string()));
    assert!(readonly_names.contains(&"diff".to_string()));
    assert!(readonly_names.contains(&"summarize".to_string()));
    assert!(!readonly_names.contains(&"edit".to_string()));
    assert!(!readonly_names.contains(&"run".to_string()));

    // Test workspace write commands
    let workspace_commands = registry
        .list_commands_for_sandbox(&SandboxLevel::WorkspaceWrite)
        .await;

    let workspace_names: Vec<String> = workspace_commands.iter().map(|c| c.name.clone()).collect();
    assert!(workspace_names.contains(&"plan".to_string()));
    assert!(workspace_names.contains(&"edit".to_string()));
    assert!(workspace_names.contains(&"run".to_string()));
    assert!(workspace_names.contains(&"diff".to_string()));
    assert!(workspace_names.contains(&"summarize".to_string()));

    Ok(())
}
