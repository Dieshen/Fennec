use anyhow::Result;
use fennec_commands::{CommandContext, CommandExecutionResult, CommandRegistry};
use fennec_core::{command::CommandPreview, config::Config};
use fennec_security::{
    approval::{
        ApprovalManager, ApprovalRequest, ApprovalStatus as SecurityApprovalStatus, RiskLevel,
    },
    audit::AuditLogger,
    SandboxLevel,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Duration;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// State of a command execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CommandState {
    /// Command is pending approval or execution
    Pending,
    /// Command has been approved and is ready for execution
    Approved,
    /// Command is currently executing
    Executing,
    /// Command execution completed successfully
    Completed,
    /// Command execution failed
    Failed { reason: String },
    /// Command was cancelled before execution
    Cancelled,
    /// Command approval timed out
    ApprovalTimeout,
}

/// Information about a command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionInfo {
    pub id: Uuid,
    pub command_name: String,
    pub args: serde_json::Value,
    pub state: CommandState,
    pub preview: Option<CommandPreview>,
    pub result: Option<CommandExecutionResult>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub requires_approval: bool,
    pub approval_timeout: Option<Duration>,
    pub backup_info: Option<BackupInfo>,
    pub session_id: Uuid,
}

/// Information about a backup created for an execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub id: Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub affected_files: Vec<PathBuf>,
    pub backup_path: PathBuf,
    pub description: String,
}

/// Configuration for backup retention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupRetentionConfig {
    pub max_backups: usize,
    pub max_age_days: u64,
    pub cleanup_interval_hours: u64,
}

impl Default for BackupRetentionConfig {
    fn default() -> Self {
        Self {
            max_backups: 100,
            max_age_days: 30,
            cleanup_interval_hours: 24,
        }
    }
}

/// Approval status for command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalStatus {
    Pending,
    Approved {
        approved_at: chrono::DateTime<chrono::Utc>,
    },
    Denied {
        reason: String,
    },
    Timeout,
}

impl From<SecurityApprovalStatus> for ApprovalStatus {
    fn from(status: SecurityApprovalStatus) -> Self {
        match status {
            SecurityApprovalStatus::Pending => ApprovalStatus::Pending,
            SecurityApprovalStatus::Approved => ApprovalStatus::Approved {
                approved_at: chrono::Utc::now(),
            },
            SecurityApprovalStatus::Denied => ApprovalStatus::Denied {
                reason: "User denied the operation".to_string(),
            },
            SecurityApprovalStatus::TimedOut => ApprovalStatus::Timeout,
        }
    }
}

/// Trait for handling approval workflows
#[async_trait::async_trait]
pub trait ApprovalHandler: Send + Sync {
    /// Request approval for a command execution
    async fn request_approval(
        &self,
        execution_info: &ExecutionInfo,
        timeout: Duration,
    ) -> Result<ApprovalStatus>;

    /// Check if a command requires approval based on sandbox level and command type
    fn requires_approval(&self, command_name: &str, sandbox_level: &SandboxLevel) -> bool;
}

/// Default approval handler that integrates with the security approval system
pub struct DefaultApprovalHandler {
    approval_rules: HashMap<String, SandboxLevel>,
    approval_manager: ApprovalManager,
}

impl Default for DefaultApprovalHandler {
    fn default() -> Self {
        let mut approval_rules = HashMap::new();

        // Commands that always require approval regardless of sandbox level
        approval_rules.insert("run".to_string(), SandboxLevel::ReadOnly);

        // Commands that require approval in certain sandbox levels
        approval_rules.insert("edit".to_string(), SandboxLevel::WorkspaceWrite);

        Self {
            approval_rules,
            approval_manager: ApprovalManager::default(),
        }
    }
}

impl DefaultApprovalHandler {
    pub fn new(auto_approve_low_risk: bool, interactive_mode: bool) -> Self {
        let mut approval_rules = HashMap::new();
        approval_rules.insert("run".to_string(), SandboxLevel::ReadOnly);
        approval_rules.insert("edit".to_string(), SandboxLevel::WorkspaceWrite);

        Self {
            approval_rules,
            approval_manager: ApprovalManager::new(auto_approve_low_risk, interactive_mode),
        }
    }

    /// Create an approval request from execution info
    fn create_approval_request(&self, execution_info: &ExecutionInfo) -> ApprovalRequest {
        let risk_level = self.assess_risk_level(execution_info);
        let mut details = vec![
            format!("Command: {}", execution_info.command_name),
            format!("Execution ID: {}", execution_info.id),
            format!("Session ID: {}", execution_info.session_id),
        ];

        // Add preview details if available
        if let Some(preview) = &execution_info.preview {
            details.push(format!("Preview: {}", preview.description));
            for action in &preview.actions {
                match action {
                    fennec_core::command::PreviewAction::ReadFile { path } => {
                        details.push(format!("Will read file: {}", path));
                    }
                    fennec_core::command::PreviewAction::WriteFile { path, content } => {
                        details.push(format!(
                            "Will write to file: {} ({} bytes)",
                            path,
                            content.len()
                        ));
                    }
                    fennec_core::command::PreviewAction::ExecuteShell { command } => {
                        details.push(format!("Will execute: {}", command));
                    }
                }
            }
        }

        ApprovalRequest {
            operation: format!("{} Command", execution_info.command_name.to_uppercase()),
            description: format!("Execute '{}' command", execution_info.command_name),
            risk_level,
            details,
        }
    }

    /// Assess risk level for an execution
    fn assess_risk_level(&self, execution_info: &ExecutionInfo) -> RiskLevel {
        match execution_info.command_name.as_str() {
            "run" => RiskLevel::High,    // Shell execution is always high risk
            "edit" => RiskLevel::Medium, // File editing is medium risk
            "plan" | "summarize" | "diff" => RiskLevel::Low, // Read-only operations are low risk
            _ => RiskLevel::Medium,      // Unknown commands default to medium risk
        }
    }
}

#[async_trait::async_trait]
impl ApprovalHandler for DefaultApprovalHandler {
    async fn request_approval(
        &self,
        execution_info: &ExecutionInfo,
        timeout: Duration,
    ) -> Result<ApprovalStatus> {
        let approval_request = self.create_approval_request(execution_info);

        // Use tokio::time::timeout to implement timeout functionality
        match tokio::time::timeout(timeout, async {
            self.approval_manager.request_approval(&approval_request)
        })
        .await
        {
            Ok(Ok(security_status)) => Ok(security_status.into()),
            Ok(Err(e)) => Err(e),
            Err(_) => Ok(ApprovalStatus::Timeout),
        }
    }

    fn requires_approval(&self, command_name: &str, sandbox_level: &SandboxLevel) -> bool {
        match self.approval_rules.get(command_name) {
            Some(required_level) => match (required_level, sandbox_level) {
                (SandboxLevel::ReadOnly, _) => true,
                (SandboxLevel::WorkspaceWrite, SandboxLevel::ReadOnly) => false,
                (SandboxLevel::WorkspaceWrite, _) => true,
                (SandboxLevel::FullAccess, SandboxLevel::FullAccess) => true,
                (SandboxLevel::FullAccess, _) => false,
            },
            None => false, // Unknown commands don't require approval by default
        }
    }
}

/// Backup manager for creating and managing file backups
pub struct BackupManager {
    backup_root: PathBuf,
    retention_config: BackupRetentionConfig,
    audit_logger: Arc<AuditLogger>,
}

impl BackupManager {
    pub fn new(
        backup_root: PathBuf,
        retention_config: BackupRetentionConfig,
        audit_logger: Arc<AuditLogger>,
    ) -> Self {
        Self {
            backup_root,
            retention_config,
            audit_logger,
        }
    }

    /// Create a backup for files that will be modified by a command
    pub async fn create_backup(
        &self,
        files: &[PathBuf],
        description: String,
    ) -> Result<BackupInfo> {
        let backup_id = Uuid::new_v4();
        let timestamp = chrono::Utc::now();
        let backup_dir = self
            .backup_root
            .join(timestamp.format("%Y-%m-%d").to_string())
            .join(backup_id.to_string());

        tokio::fs::create_dir_all(&backup_dir).await?;

        let mut backed_up_files = Vec::new();

        for file_path in files {
            if file_path.exists() {
                let relative_path = file_path.strip_prefix("/").unwrap_or(file_path);
                let backup_file = backup_dir.join(relative_path);

                if let Some(parent) = backup_file.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }

                // Retry copy operation on Windows file locking issues
                let mut attempts = 0;
                let max_attempts = 5;
                loop {
                    match tokio::fs::copy(file_path, &backup_file).await {
                        Ok(_) => break,
                        Err(e) if attempts < max_attempts && e.raw_os_error() == Some(32) => {
                            attempts += 1;
                            tokio::time::sleep(tokio::time::Duration::from_millis(50 * attempts))
                                .await;
                        }
                        Err(e) => return Err(e.into()),
                    }
                }
                backed_up_files.push(file_path.clone());

                debug!(
                    "Backed up file: {} -> {}",
                    file_path.display(),
                    backup_file.display()
                );
            }
        }

        let backup_info = BackupInfo {
            id: backup_id,
            timestamp,
            affected_files: backed_up_files,
            backup_path: backup_dir,
            description: description.clone(),
        };

        // Write backup metadata
        let metadata_file = backup_info.backup_path.join("metadata.json");
        let metadata_json = serde_json::to_string_pretty(&backup_info)?;
        tokio::fs::write(&metadata_file, metadata_json).await?;

        // Log backup creation
        self.audit_logger
            .log_security_event(
                None,
                "backup_created",
                &format!("Backup created: {} ({})", backup_id, description),
            )
            .await?;

        info!(
            "Created backup: {} with {} files",
            backup_id,
            backup_info.affected_files.len()
        );

        Ok(backup_info)
    }

    /// Restore files from a backup
    pub async fn restore_backup(&self, backup_info: &BackupInfo) -> Result<()> {
        if !backup_info.backup_path.exists() {
            return Err(anyhow::anyhow!(
                "Backup path does not exist: {}",
                backup_info.backup_path.display()
            ));
        }

        for file_path in &backup_info.affected_files {
            let relative_path = file_path.strip_prefix("/").unwrap_or(file_path);
            let backup_file = backup_info.backup_path.join(relative_path);

            if backup_file.exists() {
                if let Some(parent) = file_path.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }

                // Retry copy operation on Windows file locking issues
                let mut attempts = 0;
                let max_attempts = 3;
                loop {
                    match tokio::fs::copy(&backup_file, file_path).await {
                        Ok(_) => break,
                        Err(e) if attempts < max_attempts && e.raw_os_error() == Some(32) => {
                            attempts += 1;
                            tokio::time::sleep(tokio::time::Duration::from_millis(10 * attempts))
                                .await;
                        }
                        Err(e) => return Err(e.into()),
                    }
                }
                debug!(
                    "Restored file: {} -> {}",
                    backup_file.display(),
                    file_path.display()
                );
            }
        }

        // Log backup restoration
        self.audit_logger
            .log_security_event(
                None,
                "backup_restored",
                &format!(
                    "Backup restored: {} ({})",
                    backup_info.id, backup_info.description
                ),
            )
            .await?;

        info!(
            "Restored backup: {} with {} files",
            backup_info.id,
            backup_info.affected_files.len()
        );

        Ok(())
    }

    /// Clean up old backups based on retention policy
    pub async fn cleanup_backups(&self) -> Result<()> {
        if !self.backup_root.exists() {
            return Ok(());
        }

        let cutoff_date =
            chrono::Utc::now() - chrono::Duration::days(self.retention_config.max_age_days as i64);
        let mut backup_count = 0;
        let mut cleaned_count = 0;

        let mut entries = tokio::fs::read_dir(&self.backup_root).await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                let mut day_entries = tokio::fs::read_dir(entry.path()).await?;
                while let Some(backup_entry) = day_entries.next_entry().await? {
                    if backup_entry.file_type().await?.is_dir() {
                        backup_count += 1;

                        // Check metadata for creation time
                        let metadata_file = backup_entry.path().join("metadata.json");
                        if let Ok(metadata_content) =
                            tokio::fs::read_to_string(&metadata_file).await
                        {
                            if let Ok(backup_info) =
                                serde_json::from_str::<BackupInfo>(&metadata_content)
                            {
                                if backup_info.timestamp < cutoff_date
                                    || backup_count > self.retention_config.max_backups
                                {
                                    tokio::fs::remove_dir_all(backup_entry.path()).await?;
                                    cleaned_count += 1;

                                    debug!("Cleaned up backup: {}", backup_info.id);
                                }
                            }
                        }
                    }
                }
            }
        }

        if cleaned_count > 0 {
            info!("Cleaned up {} old backups", cleaned_count);
        }

        Ok(())
    }
}

/// Main command execution engine
pub struct CommandExecutionEngine {
    command_registry: Arc<CommandRegistry>,
    approval_handler: Arc<dyn ApprovalHandler>,
    backup_manager: Arc<BackupManager>,
    audit_logger: Arc<AuditLogger>,
    executions: Arc<RwLock<HashMap<Uuid, ExecutionInfo>>>,
    config: Config,
}

impl CommandExecutionEngine {
    /// Create a new command execution engine
    pub fn new(
        command_registry: Arc<CommandRegistry>,
        approval_handler: Arc<dyn ApprovalHandler>,
        backup_manager: Arc<BackupManager>,
        audit_logger: Arc<AuditLogger>,
        config: Config,
    ) -> Self {
        Self {
            command_registry,
            approval_handler,
            backup_manager,
            audit_logger,
            executions: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Submit a command for execution
    pub async fn submit_command(
        &self,
        command_name: String,
        args: serde_json::Value,
        context: CommandContext,
    ) -> Result<Uuid> {
        let execution_id = Uuid::new_v4();
        let now = chrono::Utc::now();

        // Check if command exists
        let command = self
            .command_registry
            .get_command(&command_name)
            .await
            .ok_or_else(|| anyhow::anyhow!("Command '{}' not found", command_name))?;

        // Generate preview
        let preview = command.preview(&args, &context).await?;

        // Determine if approval is required
        let requires_approval = self
            .approval_handler
            .requires_approval(&command_name, &context.sandbox_level);

        let execution_info = ExecutionInfo {
            id: execution_id,
            command_name: command_name.clone(),
            args,
            state: CommandState::Pending,
            preview: Some(preview),
            result: None,
            created_at: now,
            updated_at: now,
            requires_approval,
            approval_timeout: if requires_approval {
                Some(Duration::from_secs(300)) // 5 minutes default
            } else {
                None
            },
            backup_info: None,
            session_id: context.session_id,
        };

        // Store execution info
        {
            let mut executions = self.executions.write().await;
            executions.insert(execution_id, execution_info.clone());
        }

        // Log command submission
        self.audit_logger
            .log_security_event(
                Some(context.session_id),
                "command_submitted",
                &format!(
                    "Command '{}' submitted for execution: {}",
                    command_name, execution_id
                ),
            )
            .await?;

        info!(
            "Command '{}' submitted with execution ID: {}",
            command_name, execution_id
        );

        // If no approval required, execute immediately
        if !requires_approval {
            tokio::spawn({
                let engine = self.clone_arc();
                let context = context.clone();
                async move {
                    if let Err(e) = engine.execute_command_internal(execution_id, context).await {
                        error!("Failed to execute command {}: {}", execution_id, e);
                    }
                }
            });
        }

        Ok(execution_id)
    }

    /// Approve a pending command execution
    pub async fn approve_command(&self, execution_id: Uuid) -> Result<()> {
        let mut execution_info = {
            let mut executions = self.executions.write().await;
            executions
                .get_mut(&execution_id)
                .ok_or_else(|| anyhow::anyhow!("Execution {} not found", execution_id))?
                .clone()
        };

        if execution_info.state != CommandState::Pending {
            return Err(anyhow::anyhow!(
                "Execution {} is not in pending state (current: {:?})",
                execution_id,
                execution_info.state
            ));
        }

        // Update state to approved
        execution_info.state = CommandState::Approved;
        execution_info.updated_at = chrono::Utc::now();
        let session_id = execution_info.session_id;

        {
            let mut executions = self.executions.write().await;
            executions.insert(execution_id, execution_info);
        }

        // Log approval
        self.audit_logger
            .log_security_event(
                Some(session_id),
                "command_approved",
                &format!("Command execution approved: {}", execution_id),
            )
            .await?;

        info!("Command execution approved: {}", execution_id);

        // Start execution
        tokio::spawn({
            let engine = self.clone_arc();
            let context = CommandContext {
                session_id,
                user_id: None,
                workspace_path: None,
                sandbox_level: SandboxLevel::WorkspaceWrite, // TODO: Get from execution context
                dry_run: false,
                preview_only: false,
                cancellation_token: tokio_util::sync::CancellationToken::new(),
            action_log: None,
            };
            async move {
                if let Err(e) = engine.execute_command_internal(execution_id, context).await {
                    error!("Failed to execute approved command {}: {}", execution_id, e);
                }
            }
        });

        Ok(())
    }

    /// Deny a pending command execution
    pub async fn deny_command(&self, execution_id: Uuid, reason: String) -> Result<()> {
        let mut execution_info = {
            let mut executions = self.executions.write().await;
            executions
                .get_mut(&execution_id)
                .ok_or_else(|| anyhow::anyhow!("Execution {} not found", execution_id))?
                .clone()
        };

        if execution_info.state != CommandState::Pending {
            return Err(anyhow::anyhow!(
                "Execution {} is not in pending state (current: {:?})",
                execution_id,
                execution_info.state
            ));
        }

        // Update state to cancelled
        execution_info.state = CommandState::Cancelled;
        execution_info.updated_at = chrono::Utc::now();

        {
            let mut executions = self.executions.write().await;
            executions.insert(execution_id, execution_info.clone());
        }

        // Log denial
        self.audit_logger
            .log_security_event(
                Some(execution_info.session_id),
                "command_denied",
                &format!("Command execution denied: {} - {}", execution_id, reason),
            )
            .await?;

        info!("Command execution denied: {} - {}", execution_id, reason);

        Ok(())
    }

    /// Get execution status
    pub async fn get_execution_status(&self, execution_id: Uuid) -> Option<ExecutionInfo> {
        let executions = self.executions.read().await;
        executions.get(&execution_id).cloned()
    }

    /// List all executions for a session
    pub async fn list_session_executions(&self, session_id: Uuid) -> Vec<ExecutionInfo> {
        let executions = self.executions.read().await;
        executions
            .values()
            .filter(|exec| exec.session_id == session_id)
            .cloned()
            .collect()
    }

    /// Rollback a command execution using its backup
    pub async fn rollback_execution(&self, execution_id: Uuid) -> Result<()> {
        let execution_info = {
            let executions = self.executions.read().await;
            executions
                .get(&execution_id)
                .ok_or_else(|| anyhow::anyhow!("Execution {} not found", execution_id))?
                .clone()
        };

        if let Some(backup_info) = &execution_info.backup_info {
            self.backup_manager.restore_backup(backup_info).await?;

            // Log rollback
            self.audit_logger
                .log_security_event(
                    Some(execution_info.session_id),
                    "command_rollback",
                    &format!("Command execution rolled back: {}", execution_id),
                )
                .await?;

            info!("Command execution rolled back: {}", execution_id);
        } else {
            return Err(anyhow::anyhow!(
                "No backup available for execution {}",
                execution_id
            ));
        }

        Ok(())
    }

    /// Internal command execution logic
    async fn execute_command_internal(
        &self,
        execution_id: Uuid,
        context: CommandContext,
    ) -> Result<()> {
        let execution_info = {
            let executions = self.executions.read().await;
            executions
                .get(&execution_id)
                .ok_or_else(|| anyhow::anyhow!("Execution {} not found", execution_id))?
                .clone()
        };

        // Update state to executing
        {
            let mut executions = self.executions.write().await;
            if let Some(exec) = executions.get_mut(&execution_id) {
                exec.state = CommandState::Executing;
                exec.updated_at = chrono::Utc::now();
            }
        }

        // Create backup if needed for destructive operations
        let backup_info = if self.is_destructive_command(&execution_info.command_name) {
            // Extract file paths from preview
            let affected_files = self.extract_affected_files(&execution_info)?;
            if !affected_files.is_empty() {
                let backup = self
                    .backup_manager
                    .create_backup(
                        &affected_files,
                        format!("Pre-execution backup for {}", execution_info.command_name),
                    )
                    .await?;
                Some(backup)
            } else {
                None
            }
        } else {
            None
        };

        // Update execution info with backup
        if let Some(ref backup) = backup_info {
            let mut executions = self.executions.write().await;
            if let Some(exec) = executions.get_mut(&execution_id) {
                exec.backup_info = Some(backup.clone());
            }
        }

        // Execute the command
        let result = self
            .command_registry
            .execute_command(&execution_info.command_name, &execution_info.args, &context)
            .await?;

        // Update execution state based on result
        let error_message = result.error.as_deref().unwrap_or("Unknown error");
        let final_state = if result.success {
            CommandState::Completed
        } else {
            CommandState::Failed {
                reason: error_message.to_string(),
            }
        };

        {
            let mut executions = self.executions.write().await;
            if let Some(exec) = executions.get_mut(&execution_id) {
                exec.state = final_state;
                exec.result = Some(result.clone());
                exec.updated_at = chrono::Utc::now();
            }
        }

        // Log execution completion
        self.audit_logger
            .log_security_event(
                Some(execution_info.session_id),
                "command_executed",
                &format!(
                    "Command execution completed: {} - Success: {}",
                    execution_id, result.success
                ),
            )
            .await?;

        if result.success {
            info!("Command execution completed successfully: {}", execution_id);
        } else {
            warn!(
                "Command execution failed: {} - {}",
                execution_id, error_message
            );
        }

        Ok(())
    }

    /// Check if a command is destructive and requires backup
    fn is_destructive_command(&self, command_name: &str) -> bool {
        matches!(command_name, "edit" | "run")
    }

    /// Extract affected file paths from execution info
    fn extract_affected_files(&self, execution_info: &ExecutionInfo) -> Result<Vec<PathBuf>> {
        if let Some(preview) = &execution_info.preview {
            let mut files = Vec::new();
            for action in &preview.actions {
                match action {
                    fennec_core::command::PreviewAction::WriteFile { path, .. } => {
                        files.push(PathBuf::from(path));
                    }
                    fennec_core::command::PreviewAction::ReadFile { .. } => {
                        // Read operations don't need backup
                    }
                    fennec_core::command::PreviewAction::ExecuteShell { .. } => {
                        // Shell commands might affect multiple files - would need more sophisticated analysis
                    }
                }
            }
            Ok(files)
        } else {
            Ok(Vec::new())
        }
    }

    /// Helper to clone the engine as Arc for async tasks
    fn clone_arc(&self) -> Arc<CommandExecutionEngine> {
        Arc::new(CommandExecutionEngine {
            command_registry: self.command_registry.clone(),
            approval_handler: self.approval_handler.clone(),
            backup_manager: self.backup_manager.clone(),
            audit_logger: self.audit_logger.clone(),
            executions: self.executions.clone(),
            config: self.config.clone(),
        })
    }
}

impl Clone for CommandExecutionEngine {
    fn clone(&self) -> Self {
        Self {
            command_registry: self.command_registry.clone(),
            approval_handler: self.approval_handler.clone(),
            backup_manager: self.backup_manager.clone(),
            audit_logger: self.audit_logger.clone(),
            executions: self.executions.clone(),
            config: self.config.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fennec_commands::create_command_registry;
    use tempfile::TempDir;
    use tokio_util::sync::CancellationToken;

    async fn create_test_engine() -> Result<(CommandExecutionEngine, TempDir)> {
        let temp_dir = TempDir::new().unwrap();
        let audit_log_path = temp_dir.path().join("audit.log");
        let backup_path = temp_dir.path().join("backups");

        let config = Config::default();
        let command_registry = Arc::new(create_command_registry().await?);
        let approval_handler = Arc::new(DefaultApprovalHandler::default());
        let audit_logger = Arc::new(AuditLogger::with_path(audit_log_path).await?);
        let backup_manager = Arc::new(BackupManager::new(
            backup_path,
            BackupRetentionConfig::default(),
            audit_logger.clone(),
        ));

        let engine = CommandExecutionEngine::new(
            command_registry,
            approval_handler,
            backup_manager,
            audit_logger,
            config,
        );

        Ok((engine, temp_dir))
    }

    #[tokio::test]
    async fn test_command_submission() {
        let (engine, _temp_dir) = create_test_engine().await.unwrap();

        let context = CommandContext {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: None,
            sandbox_level: SandboxLevel::ReadOnly,
            dry_run: false,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
            action_log: None,
        };

        let execution_id = engine
            .submit_command(
                "plan".to_string(),
                serde_json::json!({"task": "Test task"}),
                context,
            )
            .await
            .unwrap();

        let status = engine.get_execution_status(execution_id).await.unwrap();
        assert_eq!(status.command_name, "plan");
        assert!(!status.requires_approval); // plan command shouldn't require approval in ReadOnly
    }

    #[tokio::test]
    async fn test_approval_workflow() {
        let (engine, _temp_dir) = create_test_engine().await.unwrap();

        let context = CommandContext {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: None,
            sandbox_level: SandboxLevel::FullAccess,
            dry_run: false,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
            action_log: None,
        };

        // Submit a command that requires approval
        let execution_id = engine
            .submit_command(
                "run".to_string(),
                serde_json::json!({"command": "echo test"}),
                context,
            )
            .await
            .unwrap();

        let status = engine.get_execution_status(execution_id).await.unwrap();
        assert!(status.requires_approval);
        assert_eq!(status.state, CommandState::Pending);

        // Approve the command
        engine.approve_command(execution_id).await.unwrap();

        // Give some time for async execution
        tokio::time::sleep(Duration::from_millis(100)).await;

        let status = engine.get_execution_status(execution_id).await.unwrap();
        assert!(matches!(
            status.state,
            CommandState::Executing | CommandState::Completed | CommandState::Failed { .. }
        ));
    }

    #[tokio::test]
    #[cfg_attr(target_os = "windows", ignore = "Flaky on Windows due to file locking")]
    async fn test_backup_creation() {
        let temp_dir = TempDir::new().unwrap();
        let audit_log_path = temp_dir.path().join("audit.log");
        let backup_path = temp_dir.path().join("backups");
        let test_file = temp_dir
            .path()
            .join(format!("test_{}.txt", uuid::Uuid::new_v4()));

        // Create a test file
        tokio::fs::write(&test_file, "test content").await.unwrap();

        // Add a small delay to ensure file handle is released on Windows after write
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let audit_logger = Arc::new(AuditLogger::with_path(audit_log_path).await.unwrap());
        let backup_manager =
            BackupManager::new(backup_path, BackupRetentionConfig::default(), audit_logger);

        let backup_info = backup_manager
            .create_backup(&[test_file.clone()], "Test backup".to_string())
            .await
            .unwrap();

        assert_eq!(backup_info.affected_files.len(), 1);
        assert_eq!(backup_info.affected_files[0], test_file);
        assert!(backup_info.backup_path.exists());

        // Test restoration
        tokio::fs::write(&test_file, "modified content")
            .await
            .unwrap();

        // Add a small delay to ensure file handle is released on Windows
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        backup_manager.restore_backup(&backup_info).await.unwrap();

        let restored_content = tokio::fs::read_to_string(&test_file).await.unwrap();
        assert_eq!(restored_content, "test content");
    }
}
