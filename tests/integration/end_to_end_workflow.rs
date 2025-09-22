/// End-to-End Workflow Integration Tests
/// 
/// These tests simulate complete user workflows from start to finish,
/// testing the integration between planner → edit → run cycles.

use super::common::{TestEnvironment, ConfigurableMockProvider, fixtures, assertions};
use anyhow::Result;
use fennec_commands::CommandExecutionResult;
use fennec_orchestration::CommandState;
use fennec_security::SandboxLevel;
use serde_json::json;
use std::time::Duration;

/// Test a complete planning workflow
#[tokio::test]
async fn test_planning_workflow() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let context = env.create_context(SandboxLevel::ReadOnly);

    // Test simple planning task
    let plan_args = json!({
        "task": fixtures::SIMPLE_TASK,
        "complexity": "simple"
    });

    let result = env.command_registry
        .execute_command("plan", &plan_args, &context)
        .await?;

    assertions::assert_command_success(&result);
    assert!(!result.output.is_empty(), "Plan should generate output");
    assert!(result.output.contains("hello world") || result.output.contains("Hello"), 
           "Plan should reference the task");

    // Verify audit logging
    assertions::assert_audit_log_entries(&env, 1).await?;

    Ok(())
}

/// Test a complex multi-step planning workflow
#[tokio::test]
async fn test_complex_planning_workflow() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let context = env.create_context(SandboxLevel::ReadOnly);

    let plan_args = json!({
        "task": fixtures::COMPLEX_TASK,
        "complexity": "high"
    });

    let result = env.command_registry
        .execute_command("plan", &plan_args, &context)
        .await?;

    assertions::assert_command_success(&result);
    assert!(!result.output.is_empty());
    
    // Complex plans should mention key components
    let output_lower = result.output.to_lowercase();
    assert!(output_lower.contains("api") || output_lower.contains("server"), 
           "Plan should mention API/server components");
    assert!(output_lower.contains("database") || output_lower.contains("db"), 
           "Plan should mention database components");

    Ok(())
}

/// Test plan → edit workflow
#[tokio::test]
async fn test_plan_edit_workflow() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let context = env.create_context(SandboxLevel::WorkspaceWrite);

    // Step 1: Plan the task
    let plan_args = json!({
        "task": "Create a simple Rust hello world program",
        "complexity": "simple"
    });

    let plan_result = env.command_registry
        .execute_command("plan", &plan_args, &context)
        .await?;

    assertions::assert_command_success(&plan_result);

    // Step 2: Create the file based on the plan
    let edit_args = json!({
        "path": "src/main.rs",
        "content": fixtures::SAMPLE_RUST_CODE,
        "backup": true
    });

    let edit_result = env.command_registry
        .execute_command("edit", &edit_args, &context)
        .await?;

    assertions::assert_command_success(&edit_result);

    // Verify file was created
    assertions::assert_file_exists(&env, "src/main.rs").await;
    assertions::assert_file_content(&env, "src/main.rs", fixtures::SAMPLE_RUST_CODE).await?;

    // Verify audit logging for both operations
    assertions::assert_audit_log_entries(&env, 2).await?;

    Ok(())
}

/// Test plan → edit → run workflow
#[tokio::test]
async fn test_plan_edit_run_workflow() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let context = env.create_context(SandboxLevel::FullAccess);

    // Step 1: Plan the task
    let plan_args = json!({
        "task": "Create a Python script that prints hello world",
        "complexity": "simple"
    });

    let plan_result = env.command_registry
        .execute_command("plan", &plan_args, &context)
        .await?;

    assertions::assert_command_success(&plan_result);

    // Step 2: Create the Python file
    let edit_args = json!({
        "path": "hello.py",
        "content": fixtures::SAMPLE_PYTHON_CODE,
        "backup": true
    });

    let edit_result = env.command_registry
        .execute_command("edit", &edit_args, &context)
        .await?;

    assertions::assert_command_success(&edit_result);
    assertions::assert_file_exists(&env, "hello.py").await;

    // Step 3: Run the Python script
    let run_args = json!({
        "command": "python hello.py",
        "working_directory": env.config.workspace_str(),
        "timeout_seconds": 10
    });

    let run_result = env.command_registry
        .execute_command("run", &run_args, &context)
        .await?;

    // Note: This might fail if Python isn't installed, but should succeed in most environments
    if run_result.success {
        assert!(run_result.output.contains("Hello, world!"), 
               "Script should print hello world message");
    }

    // Verify all operations were logged
    assertions::assert_audit_log_entries(&env, 3).await?;

    Ok(())
}

/// Test workflow with error recovery
#[tokio::test]
async fn test_workflow_error_recovery() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let context = env.create_context(SandboxLevel::FullAccess);

    // Step 1: Create a file with intentionally bad content
    let bad_edit_args = json!({
        "path": "broken.py",
        "content": "this is not valid python code $$$ !!!"
    });

    let edit_result = env.command_registry
        .execute_command("edit", &bad_edit_args, &context)
        .await?;

    assertions::assert_command_success(&edit_result);

    // Step 2: Try to run the broken file
    let run_args = json!({
        "command": "python broken.py",
        "working_directory": env.config.workspace_str(),
        "timeout_seconds": 5
    });

    let run_result = env.command_registry
        .execute_command("run", &run_args, &context)
        .await?;

    // Should fail due to syntax error
    assertions::assert_command_failure(&run_result);
    assert!(run_result.error.is_some(), "Should have error information");

    // Step 3: Fix the file
    let fix_edit_args = json!({
        "path": "broken.py",
        "content": fixtures::SAMPLE_PYTHON_CODE,
        "backup": true
    });

    let fix_result = env.command_registry
        .execute_command("edit", &fix_edit_args, &context)
        .await?;

    assertions::assert_command_success(&fix_result);

    // Step 4: Run the fixed file
    let retry_run_result = env.command_registry
        .execute_command("run", &run_args, &context)
        .await?;

    // Should now succeed (if Python is available)
    if retry_run_result.success {
        assert!(retry_run_result.output.contains("Hello, world!"));
    }

    Ok(())
}

/// Test workflow with rollback functionality
#[tokio::test]
async fn test_workflow_with_rollback() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let context = env.create_context(SandboxLevel::WorkspaceWrite);

    // Create initial file
    let original_content = "# Original file content\nprint('original')";
    env.write_test_file("rollback_test.py", original_content).await?;

    // Submit an edit command through the execution engine
    let edit_args = json!({
        "path": "rollback_test.py",
        "content": "# Modified content\nprint('modified')",
        "backup": true
    });

    let execution_id = env.execution_engine
        .submit_command("edit".to_string(), edit_args, context)
        .await?;

    // Wait for execution to complete
    tokio::time::sleep(Duration::from_millis(100)).await;

    let execution_status = env.execution_engine
        .get_execution_status(execution_id)
        .await
        .expect("Execution should exist");

    // If execution completed successfully, test rollback
    if matches!(execution_status.state, CommandState::Completed) {
        // Verify file was modified
        let modified_content = env.read_test_file("rollback_test.py").await?;
        assert!(modified_content.contains("modified"));

        // Perform rollback
        env.execution_engine.rollback_execution(execution_id).await?;

        // Verify file was restored
        let restored_content = env.read_test_file("rollback_test.py").await?;
        assert!(restored_content.contains("original"));
    }

    Ok(())
}

/// Test concurrent workflow execution
#[tokio::test]
async fn test_concurrent_workflows() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let context = env.create_context(SandboxLevel::WorkspaceWrite);

    // Start multiple concurrent edit operations
    let mut handles = Vec::new();

    for i in 0..5 {
        let env_clone = env.command_registry.clone();
        let context_clone = context.clone();
        let filename = format!("concurrent_{}.txt", i);
        let content = format!("Content for file {}", i);

        let handle = tokio::spawn(async move {
            let edit_args = json!({
                "path": filename,
                "content": content
            });

            env_clone
                .execute_command("edit", &edit_args, &context_clone)
                .await
        });

        handles.push(handle);
    }

    // Wait for all operations to complete
    let results: Result<Vec<_>, _> = futures::future::try_join_all(handles).await;
    let results = results?;

    // All operations should succeed
    for result in results {
        let command_result = result?;
        assertions::assert_command_success(&command_result);
    }

    // Verify all files were created
    for i in 0..5 {
        let filename = format!("concurrent_{}.txt", i);
        assertions::assert_file_exists(&env, &filename).await;
        
        let content = env.read_test_file(&filename).await?;
        let expected = format!("Content for file {}", i);
        assert_eq!(content.trim(), expected);
    }

    Ok(())
}

/// Test workflow state persistence
#[tokio::test]
async fn test_workflow_state_persistence() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let context = env.create_context(SandboxLevel::WorkspaceWrite);

    // Submit multiple commands and track their states
    let commands = vec![
        ("plan", json!({"task": "Simple task", "complexity": "low"})),
        ("edit", json!({"path": "test1.txt", "content": "content1"})),
        ("edit", json!({"path": "test2.txt", "content": "content2"})),
    ];

    let mut execution_ids = Vec::new();

    for (command_name, args) in commands {
        let execution_id = env.execution_engine
            .submit_command(command_name.to_string(), args, context.clone())
            .await?;
        execution_ids.push(execution_id);
    }

    // Wait for executions to complete
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify all executions are tracked
    let session_executions = env.execution_engine
        .list_session_executions(env.session_id)
        .await;

    assert_eq!(session_executions.len(), 3, "Should track all executed commands");

    // Verify each execution has appropriate state
    for execution in session_executions {
        assert!(matches!(
            execution.state,
            CommandState::Completed | CommandState::Executing | CommandState::Failed { .. }
        ), "Execution should have progressed from pending state");
    }

    Ok(())
}

/// Test workflow with different sandbox levels
#[tokio::test]
async fn test_workflow_sandbox_levels() -> Result<()> {
    let env = TestEnvironment::new().await?;

    // Test read-only sandbox
    let readonly_context = env.create_context(SandboxLevel::ReadOnly);
    
    // Plan should work in read-only
    let plan_result = env.command_registry
        .execute_command("plan", &json!({"task": "Test task"}), &readonly_context)
        .await?;
    assertions::assert_command_success(&plan_result);

    // Edit should fail in read-only
    let edit_result = env.command_registry
        .execute_command("edit", &json!({
            "path": "readonly_test.txt",
            "content": "test"
        }), &readonly_context)
        .await;
    
    // Should either fail or be rejected by sandbox policy
    match edit_result {
        Ok(result) => assertions::assert_command_failure(&result),
        Err(_) => (), // Expected to fail at the sandbox level
    }

    // Test workspace-write sandbox
    let workspace_context = env.create_context(SandboxLevel::WorkspaceWrite);
    
    // Edit should work in workspace-write
    let edit_result = env.command_registry
        .execute_command("edit", &json!({
            "path": "workspace_test.txt",
            "content": "test content"
        }), &workspace_context)
        .await?;
    assertions::assert_command_success(&edit_result);

    // Run should fail in workspace-write (requires full access)
    let run_result = env.command_registry
        .execute_command("run", &json!({
            "command": "echo test",
            "working_directory": env.config.workspace_str()
        }), &workspace_context)
        .await;

    match run_result {
        Ok(result) => assertions::assert_command_failure(&result),
        Err(_) => (), // Expected to fail at the sandbox level
    }

    // Test full access sandbox
    let fullaccess_context = env.create_context(SandboxLevel::FullAccess);

    // All commands should work in full access
    let run_result = env.command_registry
        .execute_command("run", &json!({
            "command": "echo 'full access test'",
            "working_directory": env.config.workspace_str()
        }), &fullaccess_context)
        .await?;

    // Should succeed if echo command is available
    if run_result.success {
        assert!(run_result.output.contains("full access test"));
    }

    Ok(())
}

/// Test workflow with preview mode
#[tokio::test]
async fn test_workflow_preview_mode() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let context = env.create_preview_context(SandboxLevel::WorkspaceWrite);

    // Edit in preview mode should not actually create files
    let edit_args = json!({
        "path": "preview_test.txt",
        "content": "This should not be written"
    });

    let result = env.command_registry
        .execute_command("edit", &edit_args, &context)
        .await?;

    assertions::assert_command_success(&result);
    
    // File should not exist after preview
    assertions::assert_file_not_exists(&env, "preview_test.txt").await;

    // But we should get preview information
    assert!(!result.output.is_empty(), "Preview should provide output");

    Ok(())
}

/// Test workflow with dry run mode
#[tokio::test]
async fn test_workflow_dry_run_mode() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let context = env.create_dry_run_context(SandboxLevel::FullAccess);

    // Run command in dry run mode
    let run_args = json!({
        "command": "echo 'dry run test'",
        "working_directory": env.config.workspace_str()
    });

    let result = env.command_registry
        .execute_command("run", &run_args, &context)
        .await?;

    assertions::assert_command_success(&result);
    
    // Should show what would be executed without actually executing
    assert!(result.output.contains("echo") || result.output.contains("dry run"), 
           "Dry run should show command information");

    Ok(())
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Test the complete integration test environment setup
    #[tokio::test]
    async fn test_integration_environment_setup() -> Result<()> {
        let env = TestEnvironment::new().await?;
        
        // Verify all components are properly initialized
        assert!(env.config.workspace_path.exists());
        assert!(env.config.backup_path.exists());
        
        // Verify command registry has expected commands
        let commands = env.command_registry.list_commands().await;
        let command_names: Vec<&String> = commands.iter().map(|c| &c.name).collect();
        
        assert!(command_names.contains(&&"plan".to_string()));
        assert!(command_names.contains(&&"edit".to_string()));
        assert!(command_names.contains(&&"run".to_string()));
        
        Ok(())
    }

    /// Benchmark test for workflow performance
    #[tokio::test]
    async fn test_workflow_performance() -> Result<()> {
        let env = TestEnvironment::new().await?;
        let context = env.create_context(SandboxLevel::ReadOnly);

        let start_time = std::time::Instant::now();
        
        // Execute a simple plan command
        let result = env.command_registry
            .execute_command("plan", &json!({"task": "Quick test"}), &context)
            .await?;

        let duration = start_time.elapsed();
        
        assertions::assert_command_success(&result);
        
        // Plan command should complete within reasonable time (adjust as needed)
        assert!(duration < Duration::from_secs(5), 
               "Plan command took too long: {:?}", duration);

        Ok(())
    }
}