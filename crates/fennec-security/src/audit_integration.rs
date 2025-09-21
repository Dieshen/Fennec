use crate::audit::{
    utils, AuditEventData, AuditSystem, CommandApprovedData, CommandCompletedData,
    CommandPreviewData, CommandRejectedData, CommandRequestedData, CommandStartedData,
    FileCreateData, FileDeleteData, FileReadData, FileWriteData, PermissionCheckData,
};
use fennec_core::{
    command::{Capability, CommandPreview, CommandResult},
    Result,
};
use std::sync::Arc;
use uuid::Uuid;

/// Auditable wrapper for command execution that logs all audit events
#[derive(Debug)]
pub struct AuditableCommandExecutor {
    audit_system: Arc<AuditSystem>,
    session_id: Uuid,
}

impl AuditableCommandExecutor {
    /// Create a new auditable command executor
    pub fn new(audit_system: Arc<AuditSystem>, session_id: Uuid) -> Self {
        Self {
            audit_system,
            session_id,
        }
    }

    /// Log command request with full metadata
    pub async fn log_command_requested(
        &self,
        command_id: Uuid,
        command_name: &str,
        args: &serde_json::Value,
        capabilities_required: &[Capability],
        sandbox_level: crate::SandboxLevel,
    ) -> Result<()> {
        if let Some(manager) = self.audit_system.get_session(self.session_id).await {
            let event_data = AuditEventData::CommandRequested(CommandRequestedData {
                command_id,
                command_name: command_name.to_string(),
                args_hash: utils::hash_data(args),
                capabilities_required: capabilities_required.to_vec(),
                sandbox_level,
            });

            manager.log_event(event_data, Some(command_id)).await?;
        }
        Ok(())
    }

    /// Log command preview generation
    pub async fn log_command_preview(
        &self,
        command_id: Uuid,
        preview: &CommandPreview,
    ) -> Result<()> {
        if let Some(manager) = self.audit_system.get_session(self.session_id).await {
            let event_data = AuditEventData::CommandPreview(CommandPreviewData {
                command_id,
                preview_hash: utils::preview_hash(preview),
                actions_count: preview.actions.len(),
                requires_approval: preview.requires_approval,
            });

            manager.log_event(event_data, Some(command_id)).await?;
        }
        Ok(())
    }

    /// Log command approval
    pub async fn log_command_approved(
        &self,
        command_id: Uuid,
        approval_method: &str,
        user_decision: &str,
    ) -> Result<()> {
        if let Some(manager) = self.audit_system.get_session(self.session_id).await {
            let event_data = AuditEventData::CommandApproved(CommandApprovedData {
                command_id,
                approval_method: approval_method.to_string(),
                user_decision: user_decision.to_string(),
                approval_timestamp: chrono::Utc::now(),
            });

            manager.log_event(event_data, Some(command_id)).await?;
        }
        Ok(())
    }

    /// Log command rejection
    pub async fn log_command_rejected(
        &self,
        command_id: Uuid,
        rejection_reason: &str,
        user_decision: &str,
    ) -> Result<()> {
        if let Some(manager) = self.audit_system.get_session(self.session_id).await {
            let event_data = AuditEventData::CommandRejected(CommandRejectedData {
                command_id,
                rejection_reason: rejection_reason.to_string(),
                user_decision: user_decision.to_string(),
            });

            manager.log_event(event_data, Some(command_id)).await?;
        }
        Ok(())
    }

    /// Log command execution start
    pub async fn log_command_started(&self, command_id: Uuid, execution_id: Uuid) -> Result<()> {
        if let Some(manager) = self.audit_system.get_session(self.session_id).await {
            let event_data = AuditEventData::CommandStarted(CommandStartedData {
                command_id,
                execution_id,
                start_timestamp: chrono::Utc::now(),
            });

            manager.log_event(event_data, Some(command_id)).await?;
        }
        Ok(())
    }

    /// Log command execution completion
    pub async fn log_command_completed(
        &self,
        command_id: Uuid,
        execution_id: Uuid,
        result: &CommandResult,
        duration_ms: u64,
    ) -> Result<()> {
        if let Some(manager) = self.audit_system.get_session(self.session_id).await {
            let event_data = AuditEventData::CommandCompleted(CommandCompletedData {
                command_id,
                execution_id,
                success: result.success,
                duration_ms,
                output_size: result.output.len(),
                error: result.error.clone(),
            });

            manager.log_event(event_data, Some(command_id)).await?;
        }
        Ok(())
    }

    /// Log permission check
    pub async fn log_permission_check(
        &self,
        capability: Capability,
        sandbox_level: crate::SandboxLevel,
        granted: bool,
        reason: Option<&str>,
    ) -> Result<()> {
        if let Some(manager) = self.audit_system.get_session(self.session_id).await {
            let event_data = AuditEventData::PermissionCheck(PermissionCheckData {
                requested_capability: capability,
                sandbox_level,
                granted,
                reason: reason.map(|s| s.to_string()),
            });

            manager.log_event(event_data, None).await?;
        }
        Ok(())
    }

    /// Log file read operation
    pub async fn log_file_read(
        &self,
        path: &str,
        size_bytes: u64,
        checksum: Option<&str>,
    ) -> Result<()> {
        if let Some(manager) = self.audit_system.get_session(self.session_id).await {
            let event_data = AuditEventData::FileRead(FileReadData {
                path: path.to_string(),
                size_bytes,
                checksum: checksum.map(|s| s.to_string()),
                access_time: chrono::Utc::now(),
            });

            manager.log_event(event_data, None).await?;
        }
        Ok(())
    }

    /// Log file write operation
    pub async fn log_file_write(
        &self,
        path: &str,
        size_bytes: u64,
        checksum_before: Option<&str>,
        checksum_after: &str,
        backup_created: bool,
    ) -> Result<()> {
        if let Some(manager) = self.audit_system.get_session(self.session_id).await {
            let event_data = AuditEventData::FileWrite(FileWriteData {
                path: path.to_string(),
                size_bytes,
                checksum_before: checksum_before.map(|s| s.to_string()),
                checksum_after: checksum_after.to_string(),
                backup_created,
            });

            manager.log_event(event_data, None).await?;
        }
        Ok(())
    }

    /// Log file create operation
    pub async fn log_file_create(
        &self,
        path: &str,
        size_bytes: u64,
        checksum_after: &str,
    ) -> Result<()> {
        if let Some(manager) = self.audit_system.get_session(self.session_id).await {
            let event_data = AuditEventData::FileCreate(FileCreateData {
                path: path.to_string(),
                size_bytes,
                checksum_after: checksum_after.to_string(),
            });

            manager.log_event(event_data, None).await?;
        }
        Ok(())
    }

    /// Log file delete operation
    pub async fn log_file_delete(
        &self,
        path: &str,
        size_bytes: u64,
        checksum_before: &str,
        backup_created: bool,
    ) -> Result<()> {
        if let Some(manager) = self.audit_system.get_session(self.session_id).await {
            let event_data = AuditEventData::FileDelete(FileDeleteData {
                path: path.to_string(),
                size_bytes,
                checksum_before: checksum_before.to_string(),
                backup_created,
            });

            manager.log_event(event_data, None).await?;
        }
        Ok(())
    }
}

/// File operations wrapper that automatically logs file operations
#[derive(Debug)]
pub struct AuditableFileOperations {
    executor: Arc<AuditableCommandExecutor>,
}

impl AuditableFileOperations {
    /// Create a new auditable file operations wrapper
    pub fn new(executor: Arc<AuditableCommandExecutor>) -> Self {
        Self { executor }
    }

    /// Read a file with audit logging
    pub async fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        let data = tokio::fs::read(path)
            .await
            .map_err(|e| fennec_core::FennecError::Security {
                message: format!("Failed to read file {}: {}", path, e),
            })?;

        let checksum = utils::sha256_checksum(&data);
        self.executor
            .log_file_read(path, data.len() as u64, Some(&checksum))
            .await?;

        Ok(data)
    }

    /// Write a file with audit logging
    pub async fn write_file(&self, path: &str, content: &[u8], create_backup: bool) -> Result<()> {
        let path_buf = std::path::Path::new(path);
        let exists = path_buf.exists();
        let checksum_before = if exists {
            let existing_content =
                tokio::fs::read(path)
                    .await
                    .map_err(|e| fennec_core::FennecError::Security {
                        message: format!("Failed to read existing file {}: {}", path, e),
                    })?;
            Some(utils::sha256_checksum(&existing_content))
        } else {
            None
        };

        // Create backup if requested and file exists
        let backup_created = if create_backup && exists {
            let backup_path = format!("{}.backup.{}", path, chrono::Utc::now().timestamp());
            if let Err(e) = tokio::fs::copy(path, &backup_path).await {
                tracing::warn!("Failed to create backup {}: {}", backup_path, e);
                false
            } else {
                true
            }
        } else {
            false
        };

        // Write the file
        tokio::fs::write(path, content)
            .await
            .map_err(|e| fennec_core::FennecError::Security {
                message: format!("Failed to write file {}: {}", path, e),
            })?;

        let checksum_after = utils::sha256_checksum(content);

        if exists {
            self.executor
                .log_file_write(
                    path,
                    content.len() as u64,
                    checksum_before.as_deref(),
                    &checksum_after,
                    backup_created,
                )
                .await?;
        } else {
            self.executor
                .log_file_create(path, content.len() as u64, &checksum_after)
                .await?;
        }

        Ok(())
    }

    /// Delete a file with audit logging
    pub async fn delete_file(&self, path: &str, create_backup: bool) -> Result<()> {
        let path_buf = std::path::Path::new(path);
        if !path_buf.exists() {
            return Err(fennec_core::FennecError::Security {
                message: format!("File does not exist: {}", path),
            });
        }

        let existing_content =
            tokio::fs::read(path)
                .await
                .map_err(|e| fennec_core::FennecError::Security {
                    message: format!("Failed to read file before deletion {}: {}", path, e),
                })?;

        let checksum_before = utils::sha256_checksum(&existing_content);
        let size_bytes = existing_content.len() as u64;

        // Create backup if requested
        let backup_created = if create_backup {
            let backup_path = format!("{}.deleted.{}", path, chrono::Utc::now().timestamp());
            if let Err(e) = tokio::fs::copy(path, &backup_path).await {
                tracing::warn!(
                    "Failed to create backup before deletion {}: {}",
                    backup_path,
                    e
                );
                false
            } else {
                true
            }
        } else {
            false
        };

        // Delete the file
        tokio::fs::remove_file(path)
            .await
            .map_err(|e| fennec_core::FennecError::Security {
                message: format!("Failed to delete file {}: {}", path, e),
            })?;

        self.executor
            .log_file_delete(path, size_bytes, &checksum_before, backup_created)
            .await?;

        Ok(())
    }
}

/// Integration wrapper for command execution with full audit trail
#[derive(Debug)]
pub struct AuditedCommandExecutionContext {
    pub command_id: Uuid,
    pub execution_id: Uuid,
    pub start_time: std::time::Instant,
    pub auditor: Arc<AuditableCommandExecutor>,
    pub file_ops: AuditableFileOperations,
}

impl AuditedCommandExecutionContext {
    /// Create a new audited execution context
    pub fn new(command_id: Uuid, audit_system: Arc<AuditSystem>, session_id: Uuid) -> Self {
        let execution_id = Uuid::new_v4();
        let auditor = Arc::new(AuditableCommandExecutor::new(audit_system, session_id));
        let file_ops = AuditableFileOperations::new(auditor.clone());

        Self {
            command_id,
            execution_id,
            start_time: std::time::Instant::now(),
            auditor,
            file_ops,
        }
    }

    /// Start command execution with audit logging
    pub async fn start_execution(&self) -> Result<()> {
        self.auditor
            .log_command_started(self.command_id, self.execution_id)
            .await
    }

    /// Complete command execution with audit logging
    pub async fn complete_execution(&self, result: &CommandResult) -> Result<()> {
        let duration_ms = self.start_time.elapsed().as_millis() as u64;
        self.auditor
            .log_command_completed(self.command_id, self.execution_id, result, duration_ms)
            .await
    }

    /// Get the auditable command executor
    pub fn auditor(&self) -> &AuditableCommandExecutor {
        &self.auditor
    }

    /// Get the auditable file operations
    pub fn file_ops(&self) -> &AuditableFileOperations {
        &self.file_ops
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::AuditSystem;
    use fennec_core::config::Config;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_auditable_command_executor() {
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

        let executor = AuditableCommandExecutor::new(audit_system.clone(), session_id);
        let command_id = Uuid::new_v4();

        // Test logging command request
        executor
            .log_command_requested(
                command_id,
                "test_command",
                &serde_json::json!({"arg": "value"}),
                &[Capability::ReadFile],
                crate::SandboxLevel::ReadOnly,
            )
            .await
            .unwrap();

        // Test logging permission check
        executor
            .log_permission_check(
                Capability::ReadFile,
                crate::SandboxLevel::ReadOnly,
                true,
                Some("Capability allowed in read-only mode"),
            )
            .await
            .unwrap();

        // Verify events were logged
        let manager = audit_system.get_session(session_id).await.unwrap();
        assert!(manager.file_path().exists());

        let content = tokio::fs::read_to_string(manager.file_path())
            .await
            .unwrap();
        assert!(content.contains("CommandRequested"));
        assert!(content.contains("PermissionCheck"));
        assert!(content.contains(&command_id.to_string()));
    }

    #[tokio::test]
    async fn test_auditable_file_operations() {
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

        let executor = Arc::new(AuditableCommandExecutor::new(
            audit_system.clone(),
            session_id,
        ));
        let file_ops = AuditableFileOperations::new(executor);

        // Test file operations
        let test_file = temp_dir.path().join("test.txt");
        let test_content = b"Hello, World!";

        // Write file (create)
        file_ops
            .write_file(test_file.to_str().unwrap(), test_content, true)
            .await
            .unwrap();

        // Read file
        let read_content = file_ops
            .read_file(test_file.to_str().unwrap())
            .await
            .unwrap();
        assert_eq!(read_content, test_content);

        // Write file again (overwrite)
        let new_content = b"Updated content";
        file_ops
            .write_file(test_file.to_str().unwrap(), new_content, true)
            .await
            .unwrap();

        // Delete file
        file_ops
            .delete_file(test_file.to_str().unwrap(), true)
            .await
            .unwrap();

        // Verify audit events were logged
        let manager = audit_system.get_session(session_id).await.unwrap();
        let content = tokio::fs::read_to_string(manager.file_path())
            .await
            .unwrap();
        assert!(content.contains("FileCreate"));
        assert!(content.contains("FileRead"));
        assert!(content.contains("FileWrite"));
        assert!(content.contains("FileDelete"));
    }

    #[tokio::test]
    async fn test_audited_command_execution_context() {
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

        let command_id = Uuid::new_v4();
        let context =
            AuditedCommandExecutionContext::new(command_id, audit_system.clone(), session_id);

        // Start execution
        context.start_execution().await.unwrap();

        // Simulate some work
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Complete execution
        let result = fennec_core::command::CommandResult {
            command_id,
            success: true,
            output: "Command completed successfully".to_string(),
            error: None,
        };
        context.complete_execution(&result).await.unwrap();

        // Verify audit trail
        let manager = audit_system.get_session(session_id).await.unwrap();
        let content = tokio::fs::read_to_string(manager.file_path())
            .await
            .unwrap();
        assert!(content.contains("CommandStarted"));
        assert!(content.contains("CommandCompleted"));
        assert!(content.contains(&context.execution_id.to_string()));
    }
}
