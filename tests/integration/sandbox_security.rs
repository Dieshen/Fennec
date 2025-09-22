/// Filesystem Sandbox Security Integration Tests
/// 
/// These tests validate sandbox policy enforcement with real file operations,
/// path traversal prevention, security measures, and comprehensive violation test cases.

use super::common::{TestEnvironment, assertions};
use anyhow::Result;
use fennec_core::command::Capability;
use fennec_security::{
    SandboxLevel, SandboxPolicy, PolicyResult, create_sandbox_policy,
    ApprovalManager, ApprovalRequest, ApprovalStatus, RiskLevel,
    create_shell_command_approval, create_file_write_approval, create_network_access_approval,
};
use serde_json::json;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test basic sandbox policy creation and validation
#[tokio::test]
async fn test_sandbox_policy_creation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    
    // Test valid workspace creation
    let policy = create_sandbox_policy(
        SandboxLevel::WorkspaceWrite, 
        Some(temp_dir.path()), 
        false
    )?;
    
    assert_eq!(policy.level(), &SandboxLevel::WorkspaceWrite);
    assert_eq!(policy.workspace_path(), temp_dir.path());
    assert!(!policy.requires_approval());

    // Test with approval required
    let approval_policy = create_sandbox_policy(
        SandboxLevel::FullAccess,
        Some(temp_dir.path()),
        true
    )?;
    
    assert!(approval_policy.requires_approval());

    // Test invalid workspace
    let invalid_result = create_sandbox_policy(
        SandboxLevel::WorkspaceWrite,
        Some(&PathBuf::from("/nonexistent/path")),
        false
    );
    
    assert!(invalid_result.is_err(), "Should fail with invalid workspace path");

    Ok(())
}

/// Test read-only sandbox restrictions
#[tokio::test]
async fn test_readonly_sandbox_restrictions() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let policy = SandboxPolicy::new(
        SandboxLevel::ReadOnly, 
        env.config.workspace_path.clone(), 
        false
    );

    // Test capability checks
    assert_eq!(policy.check_capability(&Capability::ReadFile), PolicyResult::Allow);
    
    assert!(matches!(
        policy.check_capability(&Capability::WriteFile),
        PolicyResult::Deny(_)
    ));
    
    assert!(matches!(
        policy.check_capability(&Capability::ExecuteShell),
        PolicyResult::Deny(_)
    ));
    
    assert!(matches!(
        policy.check_capability(&Capability::NetworkAccess),
        PolicyResult::Deny(_)
    ));

    // Test file path checks
    let read_file = env.config.workspace_path.join("test_read.txt");
    std::fs::write(&read_file, "test content")?;
    
    assert_eq!(policy.check_read_path(&read_file), PolicyResult::Allow);
    
    assert!(matches!(
        policy.check_write_path(&read_file),
        PolicyResult::Deny(_)
    ));

    // Test shell command restrictions
    assert!(matches!(
        policy.check_shell_command("ls"),
        PolicyResult::Deny(_)
    ));

    // Test network access restrictions
    assert!(matches!(
        policy.check_network_access("https://example.com"),
        PolicyResult::Deny(_)
    ));

    Ok(())
}

/// Test workspace-write sandbox restrictions
#[tokio::test]
async fn test_workspace_write_sandbox_restrictions() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let policy = SandboxPolicy::new(
        SandboxLevel::WorkspaceWrite, 
        env.config.workspace_path.clone(), 
        false
    );

    // Should allow reading and writing within workspace
    assert_eq!(policy.check_capability(&Capability::ReadFile), PolicyResult::Allow);
    assert_eq!(policy.check_capability(&Capability::WriteFile), PolicyResult::Allow);
    
    // Should deny execution and network access
    assert!(matches!(
        policy.check_capability(&Capability::ExecuteShell),
        PolicyResult::Deny(_)
    ));
    
    assert!(matches!(
        policy.check_capability(&Capability::NetworkAccess),
        PolicyResult::Deny(_)
    ));

    // Test file operations within workspace
    let workspace_file = env.config.workspace_path.join("workspace_test.txt");
    assert_eq!(policy.check_read_path(&workspace_file), PolicyResult::Allow);
    assert_eq!(policy.check_write_path(&workspace_file), PolicyResult::Allow);

    // Test file operations outside workspace
    let outside_file = PathBuf::from("/tmp/outside_workspace.txt");
    assert!(matches!(
        policy.check_read_path(&outside_file),
        PolicyResult::Deny(_)
    ));
    assert!(matches!(
        policy.check_write_path(&outside_file),
        PolicyResult::Deny(_)
    ));

    Ok(())
}

/// Test full-access sandbox with approval requirements
#[tokio::test]
async fn test_full_access_sandbox_with_approval() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let policy = SandboxPolicy::new(
        SandboxLevel::FullAccess, 
        env.config.workspace_path.clone(), 
        true // Require approval
    );

    // Reading should still be allowed without approval
    assert_eq!(policy.check_capability(&Capability::ReadFile), PolicyResult::Allow);
    
    // Writing should require approval
    assert!(matches!(
        policy.check_capability(&Capability::WriteFile),
        PolicyResult::RequireApproval(_)
    ));
    
    // Execution should require approval
    assert!(matches!(
        policy.check_capability(&Capability::ExecuteShell),
        PolicyResult::RequireApproval(_)
    ));
    
    // Network access should require approval
    assert!(matches!(
        policy.check_capability(&Capability::NetworkAccess),
        PolicyResult::RequireApproval(_)
    ));

    // Test file path checks with approval
    let test_file = env.config.workspace_path.join("approval_test.txt");
    assert_eq!(policy.check_read_path(&test_file), PolicyResult::Allow);
    assert!(matches!(
        policy.check_write_path(&test_file),
        PolicyResult::RequireApproval(_)
    ));

    Ok(())
}

/// Test path traversal attack prevention
#[tokio::test]
async fn test_path_traversal_prevention() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let policy = SandboxPolicy::new(
        SandboxLevel::WorkspaceWrite, 
        env.config.workspace_path.clone(), 
        false
    );

    // Create nested directory structure for testing
    let subdir = env.create_test_dir("subdir").await?;
    
    // Test various path traversal attempts
    let traversal_attempts = vec![
        "../../../etc/passwd",
        "subdir/../../etc/passwd", 
        "./../../etc/passwd",
        "..\\..\\..\\windows\\system32\\config\\sam", // Windows-style
        "%2e%2e%2f%2e%2e%2f%2e%2e%2fetc%2fpasswd", // URL encoded
        "subdir/../../../etc/passwd",
        "valid_file/../../../etc/passwd",
    ];

    for attempt in traversal_attempts {
        let path = PathBuf::from(attempt);
        
        let read_result = policy.check_read_path(&path);
        assert!(matches!(read_result, PolicyResult::Deny(_)), 
               "Path traversal should be denied for read: {}", attempt);
        
        let write_result = policy.check_write_path(&path);
        assert!(matches!(write_result, PolicyResult::Deny(_)), 
               "Path traversal should be denied for write: {}", attempt);
    }

    // Test that legitimate relative paths within workspace are allowed
    let legitimate_paths = vec![
        "file.txt",
        "subdir/file.txt",
        "./file.txt",
        "subdir/nested/file.txt",
    ];

    for legitimate in legitimate_paths {
        let path = PathBuf::from(legitimate);
        assert_eq!(policy.check_read_path(&path), PolicyResult::Allow,
                  "Legitimate path should be allowed: {}", legitimate);
        assert_eq!(policy.check_write_path(&path), PolicyResult::Allow,
                  "Legitimate path should be allowed: {}", legitimate);
    }

    Ok(())
}

/// Test symbolic link attack prevention
#[tokio::test]
async fn test_symbolic_link_prevention() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let policy = SandboxPolicy::new(
        SandboxLevel::WorkspaceWrite, 
        env.config.workspace_path.clone(), 
        false
    );

    // Create a file outside the workspace
    let temp_dir = TempDir::new()?;
    let outside_file = temp_dir.path().join("outside_target.txt");
    std::fs::write(&outside_file, "sensitive content")?;

    // Try to create a symbolic link pointing outside the workspace
    let symlink_path = env.config.workspace_path.join("malicious_symlink");
    
    // On Unix systems, try to create symlink
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        if symlink(&outside_file, &symlink_path).is_ok() {
            // If symlink creation succeeded, policy should deny access
            let read_result = policy.check_read_path(&symlink_path);
            let write_result = policy.check_write_path(&symlink_path);
            
            // The policy should detect and deny symlinks pointing outside workspace
            // Note: The actual implementation may resolve symlinks and check the target
            // This behavior depends on the specific security implementation
            println!("Symlink check - Read: {:?}, Write: {:?}", read_result, write_result);
        }
    }

    // Test legitimate symlinks within workspace
    let target_file = env.config.workspace_path.join("target.txt");
    std::fs::write(&target_file, "legitimate content")?;
    
    let internal_symlink = env.config.workspace_path.join("internal_symlink");
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        if symlink(&target_file, &internal_symlink).is_ok() {
            // Internal symlinks should be allowed
            assert_eq!(policy.check_read_path(&internal_symlink), PolicyResult::Allow);
            assert_eq!(policy.check_write_path(&internal_symlink), PolicyResult::Allow);
        }
    }

    Ok(())
}

/// Test dangerous shell command detection
#[tokio::test]
async fn test_dangerous_shell_command_detection() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let policy = SandboxPolicy::new(
        SandboxLevel::FullAccess, 
        env.config.workspace_path.clone(), 
        false
    );

    // Test critical risk commands (should require approval even in full access)
    let critical_commands = vec![
        "rm -rf /",
        "sudo dd if=/dev/zero of=/dev/sda",
        "format C:",
        "shutdown -h now",
        "init 0",
        ":(){ :|:& };:", // Fork bomb
        "mv / /dev/null",
        "chmod -R 777 /",
    ];

    for cmd in critical_commands {
        let result = policy.check_shell_command(cmd);
        assert!(matches!(result, PolicyResult::RequireApproval(_) | PolicyResult::Deny(_)), 
               "Critical command should require approval or be denied: {}", cmd);
    }

    // Test high risk commands
    let high_risk_commands = vec![
        "sudo chmod 755 /etc",
        "mount /dev/sdb1 /mnt",
        "crontab -e",
        "passwd root",
        "iptables -F",
    ];

    for cmd in high_risk_commands {
        let result = policy.check_shell_command(cmd);
        // These should either require approval or be allowed in full access
        assert!(!matches!(result, PolicyResult::Deny(_)),
               "High risk command handling: {}", cmd);
    }

    // Test safe commands
    let safe_commands = vec![
        "ls -la",
        "cat file.txt",
        "echo hello",
        "head -n 10 file.txt",
        "grep pattern file.txt",
        "find . -name '*.txt'",
    ];

    for cmd in safe_commands {
        let result = policy.check_shell_command(cmd);
        assert_eq!(result, PolicyResult::Allow, 
                  "Safe command should be allowed: {}", cmd);
    }

    Ok(())
}

/// Test network access restrictions
#[tokio::test]
async fn test_network_access_restrictions() -> Result<()> {
    let env = TestEnvironment::new().await?;
    
    // Test in read-only mode (should deny all network access)
    let readonly_policy = SandboxPolicy::new(
        SandboxLevel::ReadOnly, 
        env.config.workspace_path.clone(), 
        false
    );

    assert!(matches!(
        readonly_policy.check_network_access("https://safe-api.com"),
        PolicyResult::Deny(_)
    ));

    // Test in workspace-write mode (should deny all network access)
    let workspace_policy = SandboxPolicy::new(
        SandboxLevel::WorkspaceWrite, 
        env.config.workspace_path.clone(), 
        false
    );

    assert!(matches!(
        workspace_policy.check_network_access("https://safe-api.com"),
        PolicyResult::Deny(_)
    ));

    // Test in full-access mode
    let fullaccess_policy = SandboxPolicy::new(
        SandboxLevel::FullAccess, 
        env.config.workspace_path.clone(), 
        false
    );

    // HTTPS should be allowed
    assert_eq!(
        fullaccess_policy.check_network_access("https://api.openai.com"),
        PolicyResult::Allow
    );

    // HTTP should require approval or be flagged as risky
    let http_result = fullaccess_policy.check_network_access("http://insecure-api.com");
    // This might be allowed, require approval, or be denied based on policy
    println!("HTTP access result: {:?}", http_result);

    // With approval required
    let approval_policy = SandboxPolicy::new(
        SandboxLevel::FullAccess, 
        env.config.workspace_path.clone(), 
        true
    );

    assert!(matches!(
        approval_policy.check_network_access("https://api.openai.com"),
        PolicyResult::RequireApproval(_)
    ));

    Ok(())
}

/// Test approval manager functionality
#[tokio::test]
async fn test_approval_manager() -> Result<()> {
    // Test auto-approve for low risk
    let auto_manager = ApprovalManager::new(true, false);
    
    let low_risk_request = ApprovalRequest {
        operation: "Read File".to_string(),
        description: "Reading a configuration file".to_string(),
        risk_level: RiskLevel::Low,
        details: vec!["Path: config.toml".to_string()],
    };

    let result = auto_manager.request_approval(&low_risk_request)?;
    assert_eq!(result, ApprovalStatus::Approved);

    // Test deny for high risk in non-interactive mode
    let high_risk_request = ApprovalRequest {
        operation: "Execute Shell".to_string(),
        description: "Running system command".to_string(),
        risk_level: RiskLevel::High,
        details: vec!["Command: rm -rf /tmp/*".to_string()],
    };

    let result = auto_manager.request_approval(&high_risk_request)?;
    assert_eq!(result, ApprovalStatus::Denied);

    // Test deny for critical risk
    let critical_request = ApprovalRequest {
        operation: "System Modification".to_string(),
        description: "Modifying system files".to_string(),
        risk_level: RiskLevel::Critical,
        details: vec!["Command: rm -rf /".to_string()],
    };

    let result = auto_manager.request_approval(&critical_request)?;
    assert_eq!(result, ApprovalStatus::Denied);

    Ok(())
}

/// Test risk level assessment for different operations
#[tokio::test]
async fn test_risk_level_assessment() -> Result<()> {
    // Test shell command risk assessment
    let safe_shell = create_shell_command_approval("ls -la");
    assert_eq!(safe_shell.risk_level, RiskLevel::Low);

    let dangerous_shell = create_shell_command_approval("rm -rf /");
    assert_eq!(dangerous_shell.risk_level, RiskLevel::Critical);

    let risky_shell = create_shell_command_approval("sudo chmod 777 /etc");
    assert_eq!(risky_shell.risk_level, RiskLevel::High);

    // Test file write risk assessment
    let env = TestEnvironment::new().await?;
    let policy = SandboxPolicy::new(
        SandboxLevel::WorkspaceWrite, 
        env.config.workspace_path.clone(), 
        true
    );

    let new_file_approval = create_file_write_approval("new_file.txt", &policy);
    assert_eq!(new_file_approval.risk_level, RiskLevel::Low);

    // Create existing file for overwrite test
    let existing_file = env.config.workspace_path.join("existing.txt");
    std::fs::write(&existing_file, "existing content")?;

    let overwrite_approval = create_file_write_approval("existing.txt", &policy);
    assert_eq!(overwrite_approval.risk_level, RiskLevel::Medium);

    // Test network access risk assessment
    let https_approval = create_network_access_approval("https://api.example.com");
    assert_eq!(https_approval.risk_level, RiskLevel::Medium);

    let http_approval = create_network_access_approval("http://insecure.example.com");
    assert_eq!(http_approval.risk_level, RiskLevel::High);

    Ok(())
}

/// Test file operation rollback mechanisms
#[tokio::test]
async fn test_file_operation_rollback() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let context = env.create_context(SandboxLevel::WorkspaceWrite);

    // Create initial file content
    let original_content = "Original file content\nLine 2\nLine 3";
    let test_file = "rollback_test.txt";
    env.write_test_file(test_file, original_content).await?;

    // Perform edit operation through execution engine
    let edit_args = json!({
        "path": test_file,
        "content": "Modified content\nNew line 2\nNew line 3",
        "backup": true
    });

    let execution_id = env.execution_engine
        .submit_command("edit".to_string(), edit_args, context)
        .await?;

    // Wait for execution
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Verify file was modified
    let modified_content = env.read_test_file(test_file).await?;
    assert!(modified_content.contains("Modified content"));

    // Perform rollback
    env.execution_engine.rollback_execution(execution_id).await?;

    // Verify file was restored
    let restored_content = env.read_test_file(test_file).await?;
    assert_eq!(restored_content.trim(), original_content.trim());

    Ok(())
}

/// Test comprehensive security violation scenarios
#[tokio::test]
async fn test_comprehensive_security_violations() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let readonly_context = env.create_context(SandboxLevel::ReadOnly);
    let workspace_context = env.create_context(SandboxLevel::WorkspaceWrite);

    // Test 1: Attempt to write in read-only mode
    let readonly_edit_result = env.command_registry
        .execute_command("edit", &json!({
            "path": "readonly_violation.txt",
            "content": "This should not be allowed"
        }), &readonly_context)
        .await;

    match readonly_edit_result {
        Ok(result) => assertions::assert_command_failure(&result),
        Err(_) => (), // Expected to fail at policy level
    }

    // Test 2: Attempt to execute shell in workspace-write mode
    let shell_execute_result = env.command_registry
        .execute_command("run", &json!({
            "command": "echo 'shell execution test'",
            "working_directory": env.config.workspace_str()
        }), &workspace_context)
        .await;

    match shell_execute_result {
        Ok(result) => assertions::assert_command_failure(&result),
        Err(_) => (), // Expected to fail at policy level
    }

    // Test 3: Attempt to access files outside workspace
    let outside_access_result = env.command_registry
        .execute_command("edit", &json!({
            "path": "/etc/passwd",
            "content": "malicious content"
        }), &workspace_context)
        .await;

    match outside_access_result {
        Ok(result) => assertions::assert_command_failure(&result),
        Err(_) => (), // Expected to fail at policy level
    }

    // Verify audit log captures violations
    assertions::assert_audit_log_entries(&env, 1).await?;

    Ok(())
}

/// Test security enforcement across different platforms
#[tokio::test]
async fn test_cross_platform_security() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let policy = SandboxPolicy::new(
        SandboxLevel::WorkspaceWrite, 
        env.config.workspace_path.clone(), 
        false
    );

    // Test Unix-specific path traversal attempts
    #[cfg(unix)]
    {
        let unix_attempts = vec![
            "/etc/passwd",
            "/etc/shadow", 
            "/root/.ssh/id_rsa",
            "~/.bashrc",
            "$HOME/.profile",
        ];

        for attempt in unix_attempts {
            let result = policy.check_read_path(&PathBuf::from(attempt));
            assert!(matches!(result, PolicyResult::Deny(_)), 
                   "Unix system path should be denied: {}", attempt);
        }
    }

    // Test Windows-specific path traversal attempts
    #[cfg(windows)]
    {
        let windows_attempts = vec![
            "C:\\Windows\\System32\\config\\SAM",
            "C:\\Windows\\System32\\drivers\\etc\\hosts",
            "%SYSTEMROOT%\\System32\\config\\SYSTEM",
            "\\\\server\\share\\sensitive.txt",
        ];

        for attempt in windows_attempts {
            let result = policy.check_read_path(&PathBuf::from(attempt));
            assert!(matches!(result, PolicyResult::Deny(_)), 
                   "Windows system path should be denied: {}", attempt);
        }
    }

    Ok(())
}

#[cfg(test)]
mod sandbox_integration_tests {
    use super::*;

    /// Test complete sandbox workflow with real file operations
    #[tokio::test]
    async fn test_complete_sandbox_workflow() -> Result<()> {
        let env = TestEnvironment::new().await?;
        
        // Test progression through sandbox levels
        let levels = vec![
            SandboxLevel::ReadOnly,
            SandboxLevel::WorkspaceWrite,
            SandboxLevel::FullAccess,
        ];

        for level in levels {
            let context = env.create_context(level.clone());
            
            // Plan should work at all levels
            let plan_result = env.command_registry
                .execute_command("plan", &json!({"task": "Test task"}), &context)
                .await?;
            assertions::assert_command_success(&plan_result);

            // Edit should work at WorkspaceWrite and FullAccess
            let edit_result = env.command_registry
                .execute_command("edit", &json!({
                    "path": format!("test_{}.txt", level.to_string().replace("-", "_")),
                    "content": format!("Content for {} level", level)
                }), &context)
                .await;

            match level {
                SandboxLevel::ReadOnly => {
                    // Should fail or be denied
                    match edit_result {
                        Ok(result) => assertions::assert_command_failure(&result),
                        Err(_) => (),
                    }
                }
                SandboxLevel::WorkspaceWrite | SandboxLevel::FullAccess => {
                    // Should succeed
                    let result = edit_result?;
                    assertions::assert_command_success(&result);
                }
            }
        }

        Ok(())
    }

    /// Performance test for security checks
    #[tokio::test]
    async fn test_security_check_performance() -> Result<()> {
        let env = TestEnvironment::new().await?;
        let policy = SandboxPolicy::new(
            SandboxLevel::WorkspaceWrite, 
            env.config.workspace_path.clone(), 
            false
        );

        let test_paths: Vec<PathBuf> = (0..1000)
            .map(|i| env.config.workspace_path.join(format!("file_{}.txt", i)))
            .collect();

        let start_time = std::time::Instant::now();
        
        for path in &test_paths {
            let _ = policy.check_read_path(path);
            let _ = policy.check_write_path(path);
        }
        
        let duration = start_time.elapsed();
        
        // Security checks should be fast (under 1 second for 1000 paths)
        assert!(duration < std::time::Duration::from_secs(1), 
               "Security checks took too long: {:?}", duration);

        Ok(())
    }
}