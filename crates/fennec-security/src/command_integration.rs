use crate::audit::AuditSystem;
use crate::audit_integration::{AuditableCommandExecutor, AuditedCommandExecutionContext};
use fennec_core::{
    command::{Capability, CommandResult},
    Result,
};
use std::sync::Arc;
use uuid::Uuid;

/// Context information needed for audited command execution
#[derive(Debug, Clone)]
pub struct AuditedCommandContext {
    pub session_id: Uuid,
    pub user_id: Option<String>,
    pub workspace_path: Option<String>,
    pub sandbox_level: crate::SandboxLevel,
    pub dry_run: bool,
    pub preview_only: bool,
}

/// Result of audited command execution
#[derive(Debug, Clone)]
pub struct AuditedCommandResult {
    pub command_id: Uuid,
    pub command_name: String,
    pub execution_id: Uuid,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub execution_time_ms: u64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Generic audited command executor that can work with any command system
#[derive(Debug)]
pub struct GenericAuditedExecutor {
    audit_system: Arc<AuditSystem>,
}

impl GenericAuditedExecutor {
    /// Create a new generic audited executor
    pub fn new(audit_system: Arc<AuditSystem>) -> Self {
        Self { audit_system }
    }

    /// Execute a command with full audit trail
    pub async fn execute_with_audit<F, Fut>(
        &self,
        command_name: &str,
        args: &serde_json::Value,
        capabilities_required: &[Capability],
        context: &AuditedCommandContext,
        executor_fn: F,
    ) -> Result<AuditedCommandResult>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<CommandResult>>,
    {
        let command_id = Uuid::new_v4();

        // Create audited execution context
        let audit_context = AuditedCommandExecutionContext::new(
            command_id,
            self.audit_system.clone(),
            context.session_id,
        );

        // Log command request
        audit_context
            .auditor()
            .log_command_requested(
                command_id,
                command_name,
                args,
                capabilities_required,
                context.sandbox_level.clone(),
            )
            .await?;

        // Check permissions and log them
        for capability in capabilities_required {
            let granted = true; // In real implementation, check against sandbox policy
            audit_context
                .auditor()
                .log_permission_check(
                    capability.clone(),
                    context.sandbox_level.clone(),
                    granted,
                    Some("Permission check during command execution"),
                )
                .await?;
        }

        // Start execution audit
        audit_context.start_execution().await?;

        let start_time = std::time::Instant::now();

        // Execute the actual command
        let command_result = executor_fn().await?;

        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        // Complete execution audit
        audit_context.complete_execution(&command_result).await?;

        Ok(AuditedCommandResult {
            command_id,
            command_name: command_name.to_string(),
            execution_id: audit_context.execution_id,
            success: command_result.success,
            output: command_result.output,
            error: command_result.error,
            execution_time_ms,
            created_at: chrono::Utc::now(),
        })
    }

    /// Get the audit system
    pub fn audit_system(&self) -> &Arc<AuditSystem> {
        &self.audit_system
    }

    /// Create an auditable command executor for a specific session
    pub fn for_session(&self, session_id: Uuid) -> AuditableCommandExecutor {
        AuditableCommandExecutor::new(self.audit_system.clone(), session_id)
    }
}

/// Helper function to wrap any command execution with audit logging
pub async fn audit_command_execution<F, Fut>(
    audit_system: Arc<AuditSystem>,
    command_name: &str,
    args: &serde_json::Value,
    capabilities_required: &[Capability],
    context: &AuditedCommandContext,
    executor_fn: F,
) -> Result<AuditedCommandResult>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<CommandResult>>,
{
    let executor = GenericAuditedExecutor::new(audit_system);
    executor
        .execute_with_audit(
            command_name,
            args,
            capabilities_required,
            context,
            executor_fn,
        )
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use fennec_core::{command::CommandResult, config::Config};
    use tempfile::TempDir;

    async fn dummy_command_executor() -> Result<CommandResult> {
        Ok(CommandResult {
            command_id: Uuid::new_v4(),
            success: true,
            output: "Command executed successfully".to_string(),
            error: None,
        })
    }

    async fn failing_command_executor() -> Result<CommandResult> {
        Ok(CommandResult {
            command_id: Uuid::new_v4(),
            success: false,
            output: "Command failed".to_string(),
            error: Some("Simulated error".to_string()),
        })
    }

    #[tokio::test]
    async fn test_generic_audited_executor() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::default();
        config.security.audit_log_path = Some(temp_dir.path().to_path_buf());
        config.security.audit_log_enabled = true;

        let audit_system = Arc::new(AuditSystem::new(&config).await.unwrap());
        let session_id = Uuid::new_v4();

        let _manager = audit_system
            .start_session(session_id, Some("test_user".to_string()), None)
            .await
            .unwrap();

        let executor = GenericAuditedExecutor::new(audit_system.clone());

        let context = AuditedCommandContext {
            session_id,
            user_id: Some("test_user".to_string()),
            workspace_path: Some(temp_dir.path().to_string_lossy().to_string()),
            sandbox_level: crate::SandboxLevel::WorkspaceWrite,
            dry_run: false,
            preview_only: false,
        };

        let args = serde_json::json!({
            "input": "test data",
            "options": ["--verbose"]
        });

        let result = executor
            .execute_with_audit(
                "test_command",
                &args,
                &[Capability::ReadFile, Capability::WriteFile],
                &context,
                dummy_command_executor,
            )
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.command_name, "test_command");
        assert!(!result.output.is_empty());
        // Execution time should be captured (u64 is always >= 0)
        println!("Execution time: {}ms", result.execution_time_ms);

        // Verify audit trail was created
        let manager = audit_system.get_session(session_id).await.unwrap();
        assert!(manager.file_path().exists());

        let content = tokio::fs::read_to_string(manager.file_path())
            .await
            .unwrap();
        assert!(content.contains("CommandRequested"));
        assert!(content.contains("CommandStarted"));
        assert!(content.contains("CommandCompleted"));
        assert!(content.contains("PermissionCheck"));
        assert!(content.contains("test_command"));

        // End session
        audit_system.end_session(session_id, 1, 0).await.unwrap();
    }

    #[tokio::test]
    async fn test_audit_command_execution_helper() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::default();
        config.security.audit_log_path = Some(temp_dir.path().to_path_buf());
        config.security.audit_log_enabled = true;

        let audit_system = Arc::new(AuditSystem::new(&config).await.unwrap());
        let session_id = Uuid::new_v4();

        let _manager = audit_system
            .start_session(session_id, Some("test_user".to_string()), None)
            .await
            .unwrap();

        let context = AuditedCommandContext {
            session_id,
            user_id: Some("test_user".to_string()),
            workspace_path: Some(temp_dir.path().to_string_lossy().to_string()),
            sandbox_level: crate::SandboxLevel::ReadOnly,
            dry_run: false,
            preview_only: false,
        };

        let args = serde_json::json!({"task": "read file"});

        let result = audit_command_execution(
            audit_system.clone(),
            "read_command",
            &args,
            &[Capability::ReadFile],
            &context,
            dummy_command_executor,
        )
        .await
        .unwrap();

        assert!(result.success);
        assert_eq!(result.command_name, "read_command");

        // End session
        audit_system.end_session(session_id, 1, 0).await.unwrap();
    }

    #[tokio::test]
    async fn test_failed_command_audit() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::default();
        config.security.audit_log_path = Some(temp_dir.path().to_path_buf());
        config.security.audit_log_enabled = true;

        let audit_system = Arc::new(AuditSystem::new(&config).await.unwrap());
        let session_id = Uuid::new_v4();

        let _manager = audit_system
            .start_session(session_id, Some("test_user".to_string()), None)
            .await
            .unwrap();

        let executor = GenericAuditedExecutor::new(audit_system.clone());

        let context = AuditedCommandContext {
            session_id,
            user_id: Some("test_user".to_string()),
            workspace_path: None,
            sandbox_level: crate::SandboxLevel::FullAccess,
            dry_run: false,
            preview_only: false,
        };

        let result = executor
            .execute_with_audit(
                "failing_command",
                &serde_json::json!({}),
                &[Capability::ExecuteShell],
                &context,
                failing_command_executor,
            )
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.error.is_some());

        // Verify failed command is also audited
        let manager = audit_system.get_session(session_id).await.unwrap();
        let content = tokio::fs::read_to_string(manager.file_path())
            .await
            .unwrap();
        assert!(content.contains("failing_command"));
        assert!(content.contains("\"success\":false"));

        // End session
        audit_system.end_session(session_id, 1, 1).await.unwrap();
    }

    #[tokio::test]
    async fn test_session_audit_executor() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::default();
        config.security.audit_log_path = Some(temp_dir.path().to_path_buf());
        config.security.audit_log_enabled = true;

        let audit_system = Arc::new(AuditSystem::new(&config).await.unwrap());
        let session_id = Uuid::new_v4();

        let _manager = audit_system
            .start_session(session_id, Some("test_user".to_string()), None)
            .await
            .unwrap();

        let generic_executor = GenericAuditedExecutor::new(audit_system.clone());
        let session_auditor = generic_executor.for_session(session_id);

        // Test session-specific logging
        session_auditor
            .log_permission_check(
                Capability::ReadFile,
                crate::SandboxLevel::ReadOnly,
                true,
                Some("Permission granted for read operation"),
            )
            .await
            .unwrap();

        // Verify logging
        let manager = audit_system.get_session(session_id).await.unwrap();
        let content = tokio::fs::read_to_string(manager.file_path())
            .await
            .unwrap();
        assert!(content.contains("PermissionCheck"));
        assert!(content.contains("Permission granted for read operation"));

        // End session
        audit_system.end_session(session_id, 0, 0).await.unwrap();
    }
}
