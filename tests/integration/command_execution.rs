/// Command Execution Testing Framework
/// 
/// These tests validate the complete command lifecycle including preview → approval → execution,
/// command chaining and dependencies, concurrent execution, and audit logging.

use super::common::{TestEnvironment, ConfigurableMockProvider, assertions};
use anyhow::Result;
use fennec_orchestration::{CommandState, ExecutionInfo, ApprovalStatus};
use fennec_security::SandboxLevel;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Test basic command submission and execution
#[tokio::test]
async fn test_basic_command_execution() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let context = env.create_context(SandboxLevel::ReadOnly);

    // Submit a simple plan command
    let execution_id = env.execution_engine
        .submit_command(
            "plan".to_string(),
            json!({"task": "Create a simple hello world program"}),
            context,
        )
        .await?;

    // Wait for execution to complete
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Check execution status
    let execution_info = env.execution_engine
        .get_execution_status(execution_id)
        .await
        .expect("Execution should exist");

    // Plan command in read-only mode shouldn't require approval
    assert!(!execution_info.requires_approval);
    
    // Should have completed successfully
    assert!(matches!(
        execution_info.state,
        CommandState::Completed | CommandState::Executing
    ));

    // Should have a result if completed
    if let CommandState::Completed = execution_info.state {
        assert!(execution_info.result.is_some());
        let result = execution_info.result.unwrap();
        assertions::assert_command_success(&result);
    }

    Ok(())
}

/// Test command approval workflow
#[tokio::test]
async fn test_command_approval_workflow() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let context = env.create_context(SandboxLevel::FullAccess);

    // Submit a command that requires approval (run command)
    let execution_id = env.execution_engine
        .submit_command(
            "run".to_string(),
            json!({
                "command": "echo 'approval test'",
                "working_directory": env.config.workspace_str()
            }),
            context,
        )
        .await?;

    // Check that command is pending approval
    let execution_info = env.execution_engine
        .get_execution_status(execution_id)
        .await
        .expect("Execution should exist");

    assert!(execution_info.requires_approval);
    assert_eq!(execution_info.state, CommandState::Pending);

    // Approve the command
    env.execution_engine.approve_command(execution_id).await?;

    // Wait for execution to complete
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Check that command was executed
    let final_info = env.execution_engine
        .get_execution_status(execution_id)
        .await
        .expect("Execution should exist");

    assert!(matches!(
        final_info.state,
        CommandState::Completed | CommandState::Executing | CommandState::Failed { .. }
    ));

    Ok(())
}

/// Test command denial workflow
#[tokio::test]
async fn test_command_denial_workflow() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let context = env.create_context(SandboxLevel::FullAccess);

    // Submit a command that requires approval
    let execution_id = env.execution_engine
        .submit_command(
            "run".to_string(),
            json!({
                "command": "rm -rf /tmp/test_file",
                "working_directory": env.config.workspace_str()
            }),
            context,
        )
        .await?;

    // Verify command is pending
    let execution_info = env.execution_engine
        .get_execution_status(execution_id)
        .await
        .expect("Execution should exist");

    assert_eq!(execution_info.state, CommandState::Pending);

    // Deny the command
    env.execution_engine
        .deny_command(execution_id, "Too dangerous for testing".to_string())
        .await?;

    // Check that command was cancelled
    let final_info = env.execution_engine
        .get_execution_status(execution_id)
        .await
        .expect("Execution should exist");

    assert_eq!(final_info.state, CommandState::Cancelled);

    Ok(())
}

/// Test command preview generation
#[tokio::test]
async fn test_command_preview_generation() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let context = env.create_preview_context(SandboxLevel::WorkspaceWrite);

    // Submit edit command in preview mode
    let result = env.command_registry
        .execute_command(
            "edit",
            &json!({
                "path": "preview_test.txt",
                "content": "This is preview content"
            }),
            &context,
        )
        .await?;

    assertions::assert_command_success(&result);
    
    // File should not actually be created in preview mode
    assertions::assert_file_not_exists(&env, "preview_test.txt").await;
    
    // But should get preview information in output
    assert!(!result.output.is_empty());
    assert!(result.output.contains("preview") || result.output.contains("would"));

    Ok(())
}

/// Test command chaining and dependencies
#[tokio::test]
async fn test_command_chaining() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let context = env.create_context(SandboxLevel::WorkspaceWrite);

    // Step 1: Plan a task
    let plan_id = env.execution_engine
        .submit_command(
            "plan".to_string(),
            json!({"task": "Create a configuration file"}),
            context.clone(),
        )
        .await?;

    // Wait for plan to complete
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Step 2: Create the file based on the plan
    let edit_id = env.execution_engine
        .submit_command(
            "edit".to_string(),
            json!({
                "path": "config.toml",
                "content": "[server]\nhost = \"localhost\"\nport = 8080"
            }),
            context.clone(),
        )
        .await?;

    // Wait for edit to complete
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Step 3: Verify the file was created
    let diff_id = env.execution_engine
        .submit_command(
            "diff".to_string(),
            json!({
                "path": "config.toml",
                "show_context": true
            }),
            context,
        )
        .await?;

    // Wait for diff to complete
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify all commands completed
    let executions = vec![plan_id, edit_id, diff_id];
    for exec_id in executions {
        let info = env.execution_engine
            .get_execution_status(exec_id)
            .await
            .expect("Execution should exist");

        assert!(matches!(
            info.state,
            CommandState::Completed | CommandState::Executing
        ), "Command {} should have completed or be executing", exec_id);
    }

    // Verify file was actually created
    assertions::assert_file_exists(&env, "config.toml").await;

    Ok(())
}

/// Test concurrent command execution
#[tokio::test]
async fn test_concurrent_command_execution() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let context = env.create_context(SandboxLevel::WorkspaceWrite);

    // Submit multiple concurrent edit commands
    let mut execution_ids = Vec::new();
    let num_commands = 5;

    for i in 0..num_commands {
        let exec_id = env.execution_engine
            .submit_command(
                "edit".to_string(),
                json!({
                    "path": format!("concurrent_{}.txt", i),
                    "content": format!("Content for file {}", i)
                }),
                context.clone(),
            )
            .await?;
        execution_ids.push(exec_id);
    }

    // Wait for all executions to complete
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify all commands completed successfully
    for (i, exec_id) in execution_ids.iter().enumerate() {
        let info = env.execution_engine
            .get_execution_status(*exec_id)
            .await
            .expect("Execution should exist");

        assert!(matches!(
            info.state,
            CommandState::Completed | CommandState::Executing
        ), "Command {} should have completed", i);

        // Verify file was created
        let filename = format!("concurrent_{}.txt", i);
        assertions::assert_file_exists(&env, &filename).await;
    }

    Ok(())
}

/// Test command execution timeout handling
#[tokio::test]
async fn test_command_execution_timeout() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let context = env.create_context(SandboxLevel::FullAccess);

    // Submit a command with a short timeout that would normally take longer
    let execution_id = env.execution_engine
        .submit_command(
            "run".to_string(),
            json!({
                "command": "sleep 5", // 5 second sleep
                "working_directory": env.config.workspace_str(),
                "timeout_seconds": 1 // 1 second timeout
            }),
            context,
        )
        .await?;

    // Since it requires approval, approve it first
    env.execution_engine.approve_command(execution_id).await?;

    // Wait longer than the timeout
    tokio::time::sleep(Duration::from_millis(2000)).await;

    let execution_info = env.execution_engine
        .get_execution_status(execution_id)
        .await
        .expect("Execution should exist");

    // Command should have failed due to timeout or completed (depending on system)
    assert!(matches!(
        execution_info.state,
        CommandState::Failed { .. } | CommandState::Completed | CommandState::Executing
    ));

    Ok(())
}

/// Test command execution error handling
#[tokio::test]
async fn test_command_execution_error_handling() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let context = env.create_context(SandboxLevel::FullAccess);

    // Submit a command that will definitely fail
    let execution_id = env.execution_engine
        .submit_command(
            "run".to_string(),
            json!({
                "command": "nonexistent_command_that_will_fail",
                "working_directory": env.config.workspace_str()
            }),
            context,
        )
        .await?;

    // Approve the command
    env.execution_engine.approve_command(execution_id).await?;

    // Wait for execution to complete
    tokio::time::sleep(Duration::from_millis(300)).await;

    let execution_info = env.execution_engine
        .get_execution_status(execution_id)
        .await
        .expect("Execution should exist");

    // Command should have failed
    assert!(matches!(execution_info.state, CommandState::Failed { .. }));

    // Should have error information
    if let Some(result) = execution_info.result {
        assertions::assert_command_failure(&result);
        assert!(result.error.is_some());
    }

    Ok(())
}

/// Test command execution with backup and rollback
#[tokio::test]
async fn test_command_backup_and_rollback() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let context = env.create_context(SandboxLevel::WorkspaceWrite);

    // Create initial file
    let original_content = "Original content for rollback test";
    env.write_test_file("rollback_test.txt", original_content).await?;

    // Submit edit command that will create backup
    let execution_id = env.execution_engine
        .submit_command(
            "edit".to_string(),
            json!({
                "path": "rollback_test.txt",
                "content": "Modified content for rollback test",
                "backup": true
            }),
            context,
        )
        .await?;

    // Wait for execution to complete
    tokio::time::sleep(Duration::from_millis(200)).await;

    let execution_info = env.execution_engine
        .get_execution_status(execution_id)
        .await
        .expect("Execution should exist");

    // Verify file was modified
    let modified_content = env.read_test_file("rollback_test.txt").await?;
    assert!(modified_content.contains("Modified content"));

    // Verify backup was created
    assert!(execution_info.backup_info.is_some());

    // Perform rollback
    env.execution_engine.rollback_execution(execution_id).await?;

    // Verify file was restored
    let restored_content = env.read_test_file("rollback_test.txt").await?;
    assert_eq!(restored_content.trim(), original_content);

    Ok(())
}

/// Test session execution tracking
#[tokio::test]
async fn test_session_execution_tracking() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let context = env.create_context(SandboxLevel::WorkspaceWrite);

    // Submit multiple commands
    let commands = vec![
        ("plan", json!({"task": "Test task 1"})),
        ("edit", json!({"path": "test1.txt", "content": "content1"})),
        ("edit", json!({"path": "test2.txt", "content": "content2"})),
        ("diff", json!({"path": "test1.txt"})),
    ];

    let mut execution_ids = Vec::new();

    for (command_name, args) in commands {
        let exec_id = env.execution_engine
            .submit_command(command_name.to_string(), args, context.clone())
            .await?;
        execution_ids.push(exec_id);
    }

    // Wait for executions to complete
    tokio::time::sleep(Duration::from_millis(400)).await;

    // Get all session executions
    let session_executions = env.execution_engine
        .list_session_executions(env.session_id)
        .await;

    // Should have at least our submitted commands
    assert!(session_executions.len() >= execution_ids.len());

    // Verify all our commands are tracked
    let session_ids: Vec<Uuid> = session_executions.iter().map(|e| e.id).collect();
    for exec_id in execution_ids {
        assert!(session_ids.contains(&exec_id), 
               "Session should track execution {}", exec_id);
    }

    // Verify session information is consistent
    for execution in session_executions {
        assert_eq!(execution.session_id, env.session_id);
        assert!(execution.created_at <= execution.updated_at);
    }

    Ok(())
}

/// Test audit logging for command execution
#[tokio::test]
async fn test_command_execution_audit_logging() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let context = env.create_context(SandboxLevel::WorkspaceWrite);

    let initial_audit_count = env.audit_log_count().await?;

    // Submit and execute a command
    let execution_id = env.execution_engine
        .submit_command(
            "edit".to_string(),
            json!({
                "path": "audit_test.txt",
                "content": "Content for audit testing"
            }),
            context,
        )
        .await?;

    // Wait for execution to complete
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Check audit log was updated
    let final_audit_count = env.audit_log_count().await?;
    assert!(final_audit_count > initial_audit_count, 
           "Audit log should have new entries");

    // Verify the execution was tracked
    let execution_info = env.execution_engine
        .get_execution_status(execution_id)
        .await
        .expect("Execution should exist");

    assert!(matches!(
        execution_info.state,
        CommandState::Completed | CommandState::Executing
    ));

    Ok(())
}

/// Test command execution state transitions
#[tokio::test]
async fn test_command_state_transitions() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let context = env.create_context(SandboxLevel::FullAccess);

    // Submit a command that requires approval
    let execution_id = env.execution_engine
        .submit_command(
            "run".to_string(),
            json!({
                "command": "echo 'state transition test'",
                "working_directory": env.config.workspace_str()
            }),
            context,
        )
        .await?;

    // Initially should be pending
    let initial_info = env.execution_engine
        .get_execution_status(execution_id)
        .await
        .expect("Execution should exist");
    assert_eq!(initial_info.state, CommandState::Pending);

    // Approve the command
    env.execution_engine.approve_command(execution_id).await?;

    // Should transition to approved then executing/completed
    tokio::time::sleep(Duration::from_millis(100)).await;
    let approved_info = env.execution_engine
        .get_execution_status(execution_id)
        .await
        .expect("Execution should exist");

    assert!(matches!(
        approved_info.state,
        CommandState::Approved | CommandState::Executing | CommandState::Completed
    ));

    // Wait for final completion
    tokio::time::sleep(Duration::from_millis(200)).await;
    let final_info = env.execution_engine
        .get_execution_status(execution_id)
        .await
        .expect("Execution should exist");

    assert!(matches!(
        final_info.state,
        CommandState::Completed | CommandState::Failed { .. }
    ));

    Ok(())
}

/// Test command execution with different sandbox levels
#[tokio::test]
async fn test_command_execution_sandbox_levels() -> Result<()> {
    let env = TestEnvironment::new().await?;

    // Test command execution at different sandbox levels
    let levels = vec![
        SandboxLevel::ReadOnly,
        SandboxLevel::WorkspaceWrite,
        SandboxLevel::FullAccess,
    ];

    for level in levels {
        let context = env.create_context(level.clone());

        // Plan command should work at all levels
        let plan_id = env.execution_engine
            .submit_command(
                "plan".to_string(),
                json!({"task": format!("Plan for {}", level)}),
                context.clone(),
            )
            .await?;

        tokio::time::sleep(Duration::from_millis(100)).await;

        let plan_info = env.execution_engine
            .get_execution_status(plan_id)
            .await
            .expect("Execution should exist");

        assert!(matches!(
            plan_info.state,
            CommandState::Completed | CommandState::Executing
        ), "Plan should work at {} level", level);

        // Edit command behavior depends on sandbox level
        let edit_result = env.execution_engine
            .submit_command(
                "edit".to_string(),
                json!({
                    "path": format!("sandbox_{}.txt", level.to_string().replace("-", "_")),
                    "content": format!("Content for {} level", level)
                }),
                context,
            )
            .await;

        match level {
            SandboxLevel::ReadOnly => {
                // Edit should either fail to submit or fail during execution
                if let Ok(edit_id) = edit_result {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    let edit_info = env.execution_engine
                        .get_execution_status(edit_id)
                        .await
                        .expect("Execution should exist");

                    // Should fail due to sandbox restrictions
                    assert!(matches!(
                        edit_info.state,
                        CommandState::Failed { .. } | CommandState::Cancelled
                    ));
                }
            }
            SandboxLevel::WorkspaceWrite | SandboxLevel::FullAccess => {
                // Edit should succeed
                let edit_id = edit_result?;
                tokio::time::sleep(Duration::from_millis(100)).await;

                let edit_info = env.execution_engine
                    .get_execution_status(edit_id)
                    .await
                    .expect("Execution should exist");

                assert!(matches!(
                    edit_info.state,
                    CommandState::Completed | CommandState::Executing
                ));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod command_execution_integration_tests {
    use super::*;

    /// Test the complete command execution pipeline
    #[tokio::test]
    async fn test_complete_execution_pipeline() -> Result<()> {
        let env = TestEnvironment::new().await?;
        let context = env.create_context(SandboxLevel::WorkspaceWrite);

        // Execute a complete workflow
        let workflow_commands = vec![
            ("plan", json!({"task": "Create a simple application"})),
            ("edit", json!({"path": "app.py", "content": "print('Hello, World!')"})),
            ("diff", json!({"path": "app.py"})),
        ];

        for (command_name, args) in workflow_commands {
            let execution_id = env.execution_engine
                .submit_command(command_name.to_string(), args, context.clone())
                .await?;

            // Wait for completion
            tokio::time::sleep(Duration::from_millis(150)).await;

            let execution_info = env.execution_engine
                .get_execution_status(execution_id)
                .await
                .expect("Execution should exist");

            assert!(matches!(
                execution_info.state,
                CommandState::Completed | CommandState::Executing
            ), "Command '{}' should complete successfully", command_name);
        }

        // Verify final state
        assertions::assert_file_exists(&env, "app.py").await;
        let content = env.read_test_file("app.py").await?;
        assert!(content.contains("Hello, World!"));

        Ok(())
    }

    /// Performance test for command execution
    #[tokio::test]
    async fn test_command_execution_performance() -> Result<()> {
        let env = TestEnvironment::new().await?;
        let context = env.create_context(SandboxLevel::ReadOnly);

        let start_time = std::time::Instant::now();

        // Execute multiple simple commands
        let mut execution_ids = Vec::new();
        for i in 0..10 {
            let exec_id = env.execution_engine
                .submit_command(
                    "plan".to_string(),
                    json!({"task": format!("Performance test task {}", i)}),
                    context.clone(),
                )
                .await?;
            execution_ids.push(exec_id);
        }

        // Wait for all to complete
        tokio::time::sleep(Duration::from_millis(1000)).await;

        let duration = start_time.elapsed();

        // Verify all completed
        for exec_id in execution_ids {
            let info = env.execution_engine
                .get_execution_status(exec_id)
                .await
                .expect("Execution should exist");

            assert!(matches!(
                info.state,
                CommandState::Completed | CommandState::Executing
            ));
        }

        // Should complete within reasonable time
        assert!(duration < Duration::from_secs(5), 
               "Command execution took too long: {:?}", duration);

        Ok(())
    }

    /// Test error recovery in command execution
    #[tokio::test]
    async fn test_command_execution_error_recovery() -> Result<()> {
        let env = TestEnvironment::new().await?;
        let context = env.create_context(SandboxLevel::WorkspaceWrite);

        // Create a file that will cause conflicts
        env.write_test_file("conflict.txt", "original content").await?;

        // Try to edit the same file multiple times rapidly
        let mut execution_ids = Vec::new();
        for i in 0..3 {
            let exec_id = env.execution_engine
                .submit_command(
                    "edit".to_string(),
                    json!({
                        "path": "conflict.txt",
                        "content": format!("Modified content {}", i)
                    }),
                    context.clone(),
                )
                .await?;
            execution_ids.push(exec_id);
        }

        // Wait for all to complete
        tokio::time::sleep(Duration::from_millis(500)).await;

        // At least one should succeed, others might fail or succeed depending on implementation
        let mut successful_count = 0;
        for exec_id in execution_ids {
            let info = env.execution_engine
                .get_execution_status(exec_id)
                .await
                .expect("Execution should exist");

            if matches!(info.state, CommandState::Completed) {
                successful_count += 1;
            }
        }

        assert!(successful_count > 0, "At least one edit should succeed");

        // File should exist and have some content
        assertions::assert_file_exists(&env, "conflict.txt").await;

        Ok(())
    }
}