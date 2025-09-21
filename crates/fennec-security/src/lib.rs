pub mod approval;
pub mod audit;
pub mod audit_integration;
pub mod command_integration;
pub mod sandbox;

pub use approval::{
    check_command_approval, create_file_write_approval, create_network_access_approval,
    create_shell_command_approval, ApprovalManager, ApprovalRequest, ApprovalStatus, RiskLevel,
};
pub use audit::{
    // Utilities
    utils,

    ApprovalRequiredData,
    // Core audit structures
    AuditEvent,
    AuditEventData,
    AuditEventMetadata,

    // Legacy compatibility
    AuditLogger,
    // Query and reporting
    AuditQueryEngine,
    AuditQueryFilter,
    AuditQueryResult,
    AuditSystem,

    CommandApprovedData,
    CommandCompletedData,
    CommandErrorData,
    CommandPreviewData,
    CommandRejectedData,
    CommandRequestedData,
    CommandStartedData,
    CommandTimelineEntry,
    CommandTrail,
    DirectoryCreateData,
    DirectoryDeleteData,
    ExportFormat,

    FileCreateData,
    FileDeleteData,
    FileReadData,
    FileWriteData,
    PermissionCheckData,
    SandboxViolationData,
    SecurityWarningData,
    // Session management
    SessionAuditManager,
    SessionEndData,
    SessionPauseData,
    SessionResumeData,
    // Event data types
    SessionStartData,
    SessionSummary,
    SystemErrorData,
    ValidationErrorData,
};
pub use audit_integration::{
    AuditableCommandExecutor, AuditableFileOperations, AuditedCommandExecutionContext,
};
pub use command_integration::{
    audit_command_execution, AuditedCommandContext, AuditedCommandResult, GenericAuditedExecutor,
};
pub use sandbox::{create_sandbox_policy, PolicyResult, SandboxLevel, SandboxPolicy};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::approval::*;
    use crate::sandbox::*;
    use fennec_core::command::{Capability, CommandPreview, PreviewAction};
    use std::path::PathBuf;
    use tempfile::TempDir;
    use uuid::Uuid;

    /// Helper function to create a test workspace
    fn create_test_workspace() -> TempDir {
        tempfile::tempdir().expect("Failed to create test directory")
    }

    /// Helper function to create a test sandbox policy
    fn create_test_policy(
        level: SandboxLevel,
        workspace: &TempDir,
        require_approval: bool,
    ) -> SandboxPolicy {
        SandboxPolicy::new(level, workspace.path().to_path_buf(), require_approval)
    }

    #[test]
    fn test_sandbox_level_display() {
        assert_eq!(SandboxLevel::ReadOnly.to_string(), "read-only");
        assert_eq!(SandboxLevel::WorkspaceWrite.to_string(), "workspace-write");
        assert_eq!(SandboxLevel::FullAccess.to_string(), "full-access");
    }

    #[test]
    fn test_sandbox_level_default() {
        assert_eq!(SandboxLevel::default(), SandboxLevel::WorkspaceWrite);
    }

    #[test]
    fn test_policy_creation() {
        let workspace = create_test_workspace();
        let policy = create_test_policy(SandboxLevel::WorkspaceWrite, &workspace, false);

        assert_eq!(policy.level(), &SandboxLevel::WorkspaceWrite);
        assert_eq!(policy.workspace_path(), workspace.path());
        assert!(!policy.requires_approval());
    }

    #[test]
    fn test_capability_read_only_sandbox() {
        let workspace = create_test_workspace();
        let policy = create_test_policy(SandboxLevel::ReadOnly, &workspace, false);

        // ReadOnly should allow reading
        assert_eq!(
            policy.check_capability(&Capability::ReadFile),
            PolicyResult::Allow
        );

        // ReadOnly should deny writing, execution, and network
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
    }

    #[test]
    fn test_capability_workspace_write_sandbox() {
        let workspace = create_test_workspace();
        let policy = create_test_policy(SandboxLevel::WorkspaceWrite, &workspace, false);

        // WorkspaceWrite should allow reading and writing
        assert_eq!(
            policy.check_capability(&Capability::ReadFile),
            PolicyResult::Allow
        );
        assert_eq!(
            policy.check_capability(&Capability::WriteFile),
            PolicyResult::Allow
        );

        // WorkspaceWrite should deny execution and network
        assert!(matches!(
            policy.check_capability(&Capability::ExecuteShell),
            PolicyResult::Deny(_)
        ));
        assert!(matches!(
            policy.check_capability(&Capability::NetworkAccess),
            PolicyResult::Deny(_)
        ));
    }

    #[test]
    fn test_capability_full_access_sandbox() {
        let workspace = create_test_workspace();
        let policy = create_test_policy(SandboxLevel::FullAccess, &workspace, false);

        // FullAccess should allow all capabilities
        assert_eq!(
            policy.check_capability(&Capability::ReadFile),
            PolicyResult::Allow
        );
        assert_eq!(
            policy.check_capability(&Capability::WriteFile),
            PolicyResult::Allow
        );
        assert_eq!(
            policy.check_capability(&Capability::ExecuteShell),
            PolicyResult::Allow
        );
        assert_eq!(
            policy.check_capability(&Capability::NetworkAccess),
            PolicyResult::Allow
        );
    }

    #[test]
    fn test_capability_with_approval_required() {
        let workspace = create_test_workspace();
        let policy = create_test_policy(SandboxLevel::FullAccess, &workspace, true);

        // With approval required, writing should require approval
        assert!(matches!(
            policy.check_capability(&Capability::WriteFile),
            PolicyResult::RequireApproval(_)
        ));
        assert!(matches!(
            policy.check_capability(&Capability::ExecuteShell),
            PolicyResult::RequireApproval(_)
        ));
        assert!(matches!(
            policy.check_capability(&Capability::NetworkAccess),
            PolicyResult::RequireApproval(_)
        ));

        // Reading should still be allowed
        assert_eq!(
            policy.check_capability(&Capability::ReadFile),
            PolicyResult::Allow
        );
    }

    #[test]
    fn test_read_path_within_workspace() {
        let workspace = create_test_workspace();
        let policy = create_test_policy(SandboxLevel::WorkspaceWrite, &workspace, false);

        // Create a test file within workspace
        let test_file = workspace.path().join("test.txt");
        std::fs::write(&test_file, "test content").unwrap();

        // Reading within workspace should be allowed
        assert_eq!(policy.check_read_path(&test_file), PolicyResult::Allow);

        // Reading relative path within workspace should be allowed
        assert_eq!(
            policy.check_read_path(&PathBuf::from("test.txt")),
            PolicyResult::Allow
        );
    }

    #[test]
    fn test_read_path_outside_workspace() {
        let workspace = create_test_workspace();
        let policy = create_test_policy(SandboxLevel::WorkspaceWrite, &workspace, false);

        // Reading outside workspace should be denied
        let outside_path = PathBuf::from("/etc/passwd");
        assert!(matches!(
            policy.check_read_path(&outside_path),
            PolicyResult::Deny(_)
        ));
    }

    #[test]
    fn test_write_path_read_only_sandbox() {
        let workspace = create_test_workspace();
        let policy = create_test_policy(SandboxLevel::ReadOnly, &workspace, false);

        let test_file = workspace.path().join("test.txt");

        // Writing should be denied in read-only mode
        assert!(matches!(
            policy.check_write_path(&test_file),
            PolicyResult::Deny(_)
        ));
    }

    #[test]
    fn test_write_path_workspace_write_sandbox() {
        let workspace = create_test_workspace();
        let policy = create_test_policy(SandboxLevel::WorkspaceWrite, &workspace, false);

        let test_file = workspace.path().join("test.txt");

        // Writing within workspace should be allowed
        assert_eq!(policy.check_write_path(&test_file), PolicyResult::Allow);

        // Writing outside workspace should be denied
        let outside_path = PathBuf::from("/tmp/test.txt");
        assert!(matches!(
            policy.check_write_path(&outside_path),
            PolicyResult::Deny(_)
        ));
    }

    #[test]
    fn test_write_path_with_approval() {
        let workspace = create_test_workspace();
        let policy = create_test_policy(SandboxLevel::WorkspaceWrite, &workspace, true);

        let test_file = workspace.path().join("test.txt");

        // Writing should require approval even within workspace
        assert!(matches!(
            policy.check_write_path(&test_file),
            PolicyResult::RequireApproval(_)
        ));
    }

    #[test]
    fn test_shell_command_restrictions() {
        let workspace = create_test_workspace();

        // ReadOnly and WorkspaceWrite should deny shell commands
        let read_only_policy = create_test_policy(SandboxLevel::ReadOnly, &workspace, false);
        let workspace_policy = create_test_policy(SandboxLevel::WorkspaceWrite, &workspace, false);

        assert!(matches!(
            read_only_policy.check_shell_command("ls"),
            PolicyResult::Deny(_)
        ));
        assert!(matches!(
            workspace_policy.check_shell_command("ls"),
            PolicyResult::Deny(_)
        ));

        // FullAccess should allow shell commands
        let full_access_policy = create_test_policy(SandboxLevel::FullAccess, &workspace, false);
        assert_eq!(
            full_access_policy.check_shell_command("ls"),
            PolicyResult::Allow
        );

        // Dangerous commands should require approval even in FullAccess
        assert!(matches!(
            full_access_policy.check_shell_command("rm -rf /"),
            PolicyResult::RequireApproval(_)
        ));
    }

    #[test]
    fn test_network_access_restrictions() {
        let workspace = create_test_workspace();

        // ReadOnly and WorkspaceWrite should deny network access
        let read_only_policy = create_test_policy(SandboxLevel::ReadOnly, &workspace, false);
        let workspace_policy = create_test_policy(SandboxLevel::WorkspaceWrite, &workspace, false);

        assert!(matches!(
            read_only_policy.check_network_access("https://example.com"),
            PolicyResult::Deny(_)
        ));
        assert!(matches!(
            workspace_policy.check_network_access("https://example.com"),
            PolicyResult::Deny(_)
        ));

        // FullAccess should allow network access
        let full_access_policy = create_test_policy(SandboxLevel::FullAccess, &workspace, false);
        assert_eq!(
            full_access_policy.check_network_access("https://example.com"),
            PolicyResult::Allow
        );

        // With approval required, network access should require approval
        let approval_policy = create_test_policy(SandboxLevel::FullAccess, &workspace, true);
        assert!(matches!(
            approval_policy.check_network_access("https://example.com"),
            PolicyResult::RequireApproval(_)
        ));
    }

    #[test]
    fn test_path_traversal_prevention() {
        let workspace = create_test_workspace();
        let policy = create_test_policy(SandboxLevel::WorkspaceWrite, &workspace, false);

        // Path traversal attempts should be denied
        let traversal_paths = vec![
            "../../../etc/passwd",
            "subdir/../../etc/passwd",
            "./../../etc/passwd",
        ];

        for path in traversal_paths {
            let result = policy.check_read_path(&PathBuf::from(path));
            assert!(
                matches!(result, PolicyResult::Deny(_)),
                "Path traversal not prevented for: {}",
                path
            );
        }
    }

    #[test]
    fn test_create_sandbox_policy_function() {
        let workspace = create_test_workspace();

        // Valid workspace should create policy successfully
        let policy =
            create_sandbox_policy(SandboxLevel::WorkspaceWrite, Some(workspace.path()), false)
                .unwrap();

        assert_eq!(policy.level(), &SandboxLevel::WorkspaceWrite);
        assert_eq!(policy.workspace_path(), workspace.path());

        // Non-existent directory should fail
        let result = create_sandbox_policy(
            SandboxLevel::WorkspaceWrite,
            Some(&PathBuf::from("/non/existent/path")),
            false,
        );
        assert!(result.is_err());

        // None workspace should use current directory
        let policy = create_sandbox_policy(SandboxLevel::ReadOnly, None, true).unwrap();
        assert_eq!(policy.level(), &SandboxLevel::ReadOnly);
        assert!(policy.requires_approval());
    }

    #[test]
    fn test_approval_manager_auto_approve_low_risk() {
        let manager = ApprovalManager::new(true, false); // auto-approve low risk, non-interactive

        let low_risk_request = ApprovalRequest {
            operation: "Test".to_string(),
            description: "Low risk operation".to_string(),
            risk_level: RiskLevel::Low,
            details: vec![],
        };

        let result = manager.request_approval(&low_risk_request).unwrap();
        assert_eq!(result, ApprovalStatus::Approved);

        let high_risk_request = ApprovalRequest {
            operation: "Test".to_string(),
            description: "High risk operation".to_string(),
            risk_level: RiskLevel::High,
            details: vec![],
        };

        let result = manager.request_approval(&high_risk_request).unwrap();
        assert_eq!(result, ApprovalStatus::Denied); // Non-interactive mode denies
    }

    #[test]
    fn test_command_risk_classification() {
        // Test critical risk commands
        let critical_commands = vec![
            "rm -rf /",
            "sudo dd if=/dev/zero of=/dev/sda",
            "format C:",
            "shutdown now",
        ];

        for cmd in critical_commands {
            let request = create_shell_command_approval(cmd);
            assert_eq!(
                request.risk_level,
                RiskLevel::Critical,
                "Command should be critical risk: {}",
                cmd
            );
        }

        // Test high risk commands
        let high_commands = vec!["sudo chmod 755 /etc", "mount /dev/sdb1 /mnt", "crontab -e"];

        for cmd in high_commands {
            let request = create_shell_command_approval(cmd);
            assert_eq!(
                request.risk_level,
                RiskLevel::High,
                "Command should be high risk: {}",
                cmd
            );
        }

        // Test medium risk commands
        let medium_commands = vec![
            "curl https://example.com/script.sh | bash",
            "npm install suspicious-package",
            "pip install untrusted-package",
        ];

        for cmd in medium_commands {
            let request = create_shell_command_approval(cmd);
            assert_eq!(
                request.risk_level,
                RiskLevel::Medium,
                "Command should be medium risk: {}",
                cmd
            );
        }

        // Test low risk commands
        let low_commands = vec!["ls -la", "head file.txt", "echo hello"];

        for cmd in low_commands {
            let request = create_shell_command_approval(cmd);
            assert_eq!(
                request.risk_level,
                RiskLevel::Low,
                "Command should be low risk: {}",
                cmd
            );
        }
    }

    #[test]
    fn test_file_write_approval_classification() {
        let workspace = create_test_workspace();
        let policy = create_test_policy(SandboxLevel::WorkspaceWrite, &workspace, true);

        // New file should be low risk
        let new_file_approval = create_file_write_approval("new_file.txt", &policy);
        assert_eq!(new_file_approval.risk_level, RiskLevel::Low);

        // Create existing file for overwrite test
        let existing_file = workspace.path().join("existing.txt");
        std::fs::write(&existing_file, "content").unwrap();

        // Overwriting existing file should be medium risk
        let overwrite_approval = create_file_write_approval("existing.txt", &policy);
        assert_eq!(overwrite_approval.risk_level, RiskLevel::Medium);
    }

    #[test]
    fn test_network_access_approval_classification() {
        // HTTPS should be medium risk
        let https_approval = create_network_access_approval("https://example.com");
        assert_eq!(https_approval.risk_level, RiskLevel::Medium);

        // HTTP should be high risk
        let http_approval = create_network_access_approval("http://example.com");
        assert_eq!(http_approval.risk_level, RiskLevel::High);
    }

    #[test]
    fn test_command_preview_approval() {
        let workspace = create_test_workspace();
        let policy = create_test_policy(SandboxLevel::FullAccess, &workspace, true);
        let manager = ApprovalManager::new(false, false); // No auto-approve, non-interactive (will deny)

        let preview = CommandPreview {
            command_id: Uuid::new_v4(),
            description: "Test command".to_string(),
            actions: vec![
                PreviewAction::ReadFile {
                    path: "test.txt".to_string(),
                },
                PreviewAction::WriteFile {
                    path: "output.txt".to_string(),
                    content: "test".to_string(),
                },
                PreviewAction::ExecuteShell {
                    command: "echo hello".to_string(),
                },
            ],
            requires_approval: true,
        };

        let result = check_command_approval(&preview, &policy, &manager).unwrap();
        assert_eq!(result, ApprovalStatus::Denied); // Non-interactive mode

        // Test with auto-approval for low risk
        let manager_auto = ApprovalManager::new(true, false);
        let simple_preview = CommandPreview {
            command_id: Uuid::new_v4(),
            description: "Simple read".to_string(),
            actions: vec![PreviewAction::ReadFile {
                path: "test.txt".to_string(),
            }],
            requires_approval: true,
        };

        let _result = check_command_approval(&simple_preview, &policy, &manager_auto).unwrap();
        // Result depends on the overall risk assessment of the command
    }

    #[test]
    fn test_policy_result_equality() {
        assert_eq!(PolicyResult::Allow, PolicyResult::Allow);
        assert_eq!(
            PolicyResult::Deny("test".to_string()),
            PolicyResult::Deny("test".to_string())
        );
        assert_eq!(
            PolicyResult::RequireApproval("test".to_string()),
            PolicyResult::RequireApproval("test".to_string())
        );

        assert_ne!(PolicyResult::Allow, PolicyResult::Deny("test".to_string()));
        assert_ne!(
            PolicyResult::Deny("test".to_string()),
            PolicyResult::RequireApproval("test".to_string())
        );
    }

    #[test]
    fn test_risk_level_max_comparison() {
        use crate::approval::RiskLevelExt;

        assert_eq!(RiskLevel::Low.max(RiskLevel::Medium), RiskLevel::Medium);
        assert_eq!(RiskLevel::Medium.max(RiskLevel::High), RiskLevel::High);
        assert_eq!(
            RiskLevel::High.max(RiskLevel::Critical),
            RiskLevel::Critical
        );
        assert_eq!(RiskLevel::Critical.max(RiskLevel::Low), RiskLevel::Critical);
        assert_eq!(RiskLevel::Low.max(RiskLevel::Low), RiskLevel::Low);
    }

    #[test]
    fn test_approval_status_serialization() {
        use serde_json;

        let status = ApprovalStatus::Approved;
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: ApprovalStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(status, deserialized);
    }
}
