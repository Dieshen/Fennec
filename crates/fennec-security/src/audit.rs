use fennec_core::{command::Capability, config::Config, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tokio::{
    fs::{File, OpenOptions},
    io::AsyncWriteExt,
    sync::RwLock,
};
use tracing::{debug, error, warn};
use uuid::Uuid;

/// Metadata common to all audit events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEventMetadata {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event_id: Uuid,
    pub session_id: Uuid,
    pub sequence_number: u64,
    pub correlation_id: Option<Uuid>,
    pub user_id: Option<String>,
    pub workspace_path: Option<String>,
}

/// Session lifecycle events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStartData {
    pub user_id: Option<String>,
    pub workspace_path: Option<String>,
    pub config_snapshot: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEndData {
    pub duration_ms: u64,
    pub command_count: u64,
    pub error_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionPauseData {
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionResumeData {
    pub previous_pause_duration_ms: Option<u64>,
}

/// Command execution events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRequestedData {
    pub command_id: Uuid,
    pub command_name: String,
    pub args_hash: String,
    pub capabilities_required: Vec<Capability>,
    pub sandbox_level: crate::SandboxLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPreviewData {
    pub command_id: Uuid,
    pub preview_hash: String,
    pub actions_count: usize,
    pub requires_approval: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandApprovedData {
    pub command_id: Uuid,
    pub approval_method: String,
    pub user_decision: String,
    pub approval_timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRejectedData {
    pub command_id: Uuid,
    pub rejection_reason: String,
    pub user_decision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandStartedData {
    pub command_id: Uuid,
    pub execution_id: Uuid,
    pub start_timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandCompletedData {
    pub command_id: Uuid,
    pub execution_id: Uuid,
    pub success: bool,
    pub duration_ms: u64,
    pub output_size: usize,
    pub error: Option<String>,
}

/// File operation events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReadData {
    pub path: String,
    pub size_bytes: u64,
    pub checksum: Option<String>,
    pub access_time: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileWriteData {
    pub path: String,
    pub size_bytes: u64,
    pub checksum_before: Option<String>,
    pub checksum_after: String,
    pub backup_created: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDeleteData {
    pub path: String,
    pub size_bytes: u64,
    pub checksum_before: String,
    pub backup_created: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCreateData {
    pub path: String,
    pub size_bytes: u64,
    pub checksum_after: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryCreateData {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryDeleteData {
    pub path: String,
    pub recursive: bool,
    pub files_affected: u32,
}

/// Security events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionCheckData {
    pub requested_capability: Capability,
    pub sandbox_level: crate::SandboxLevel,
    pub granted: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxViolationData {
    pub attempted_action: String,
    pub blocked_reason: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequiredData {
    pub command_id: Uuid,
    pub approval_reason: String,
    pub security_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityWarningData {
    pub warning_type: String,
    pub details: String,
    pub action_taken: String,
}

/// Error events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandErrorData {
    pub command_id: Uuid,
    pub error_type: String,
    pub error_message: String,
    pub stack_trace: Option<String>,
    pub recovery_attempted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemErrorData {
    pub component: String,
    pub error_type: String,
    pub error_message: String,
    pub impact_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationErrorData {
    pub validation_type: String,
    pub input_data_hash: String,
    pub error_details: String,
}

/// Comprehensive audit event enum
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", content = "event_data")]
pub enum AuditEventData {
    // Session events
    SessionStart(SessionStartData),
    SessionEnd(SessionEndData),
    SessionPause(SessionPauseData),
    SessionResume(SessionResumeData),

    // Command events
    CommandRequested(CommandRequestedData),
    CommandPreview(CommandPreviewData),
    CommandApproved(CommandApprovedData),
    CommandRejected(CommandRejectedData),
    CommandStarted(CommandStartedData),
    CommandCompleted(CommandCompletedData),

    // File operation events
    FileRead(FileReadData),
    FileWrite(FileWriteData),
    FileDelete(FileDeleteData),
    FileCreate(FileCreateData),
    DirectoryCreate(DirectoryCreateData),
    DirectoryDelete(DirectoryDeleteData),

    // Security events
    PermissionCheck(PermissionCheckData),
    SandboxViolation(SandboxViolationData),
    ApprovalRequired(ApprovalRequiredData),
    SecurityWarning(SecurityWarningData),

    // Error events
    CommandError(CommandErrorData),
    SystemError(SystemErrorData),
    ValidationError(ValidationErrorData),
}

/// Complete audit event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub metadata: AuditEventMetadata,
    #[serde(flatten)]
    pub data: AuditEventData,
}

/// Session-specific audit file manager
#[derive(Debug)]
pub struct SessionAuditManager {
    session_id: Uuid,
    file_path: PathBuf,
    file: Option<File>,
    sequence_counter: AtomicU64,
    started_at: chrono::DateTime<chrono::Utc>,
    user_id: Option<String>,
    workspace_path: Option<String>,
    enabled: bool,
}

impl SessionAuditManager {
    /// Create a new session audit manager
    pub async fn new(
        session_id: Uuid,
        base_audit_path: &PathBuf,
        user_id: Option<String>,
        workspace_path: Option<String>,
        enabled: bool,
    ) -> Result<Self> {
        let started_at = chrono::Utc::now();
        let date_str = started_at.format("%Y-%m-%d").to_string();
        let timestamp_str = started_at.format("%Y%m%dT%H%M%SZ").to_string();

        // Create session-specific audit file path
        let session_dir = base_audit_path.join("sessions").join(date_str);
        let file_path = session_dir.join(format!(
            "fennec-audit-{}-{}.jsonl",
            session_id, timestamp_str
        ));

        let mut manager = Self {
            session_id,
            file_path,
            file: None,
            sequence_counter: AtomicU64::new(1),
            started_at,
            user_id,
            workspace_path,
            enabled,
        };

        if enabled {
            manager.initialize_file().await?;
            manager.log_session_start().await?;
        }

        Ok(manager)
    }

    /// Initialize the audit file
    async fn initialize_file(&mut self) -> Result<()> {
        // Create directory if it doesn't exist
        if let Some(parent) = self.file_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                fennec_core::FennecError::Security {
                    message: format!("Failed to create audit directory: {}", e),
                }
            })?;
        }

        // Open file for append
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)
            .await
            .map_err(|e| fennec_core::FennecError::Security {
                message: format!("Failed to open audit file: {}", e),
            })?;

        self.file = Some(file);
        Ok(())
    }

    /// Log session start event
    async fn log_session_start(&self) -> Result<()> {
        let event_data = AuditEventData::SessionStart(SessionStartData {
            user_id: self.user_id.clone(),
            workspace_path: self.workspace_path.clone(),
            config_snapshot: None, // TODO: Add config snapshot if needed
        });

        self.write_event(event_data, None).await
    }

    /// Log an audit event
    pub async fn log_event(
        &self,
        event_data: AuditEventData,
        correlation_id: Option<Uuid>,
    ) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        self.write_event(event_data, correlation_id).await
    }

    /// Write an event to the audit file
    async fn write_event(
        &self,
        event_data: AuditEventData,
        correlation_id: Option<Uuid>,
    ) -> Result<()> {
        let metadata = AuditEventMetadata {
            timestamp: chrono::Utc::now(),
            event_id: Uuid::new_v4(),
            session_id: self.session_id,
            sequence_number: self.sequence_counter.fetch_add(1, Ordering::SeqCst),
            correlation_id,
            user_id: self.user_id.clone(),
            workspace_path: self.workspace_path.clone(),
        };

        let event = AuditEvent {
            metadata,
            data: event_data,
        };

        let event_json =
            serde_json::to_string(&event).map_err(|e| fennec_core::FennecError::Security {
                message: format!("Failed to serialize audit event: {}", e),
            })?;

        if let Some(ref mut file) = &mut self.file.as_ref() {
            // Write as JSONL (one JSON object per line)
            let mut file_clone =
                file.try_clone()
                    .await
                    .map_err(|e| fennec_core::FennecError::Security {
                        message: format!("Failed to clone file handle: {}", e),
                    })?;

            file_clone
                .write_all(event_json.as_bytes())
                .await
                .map_err(|e| fennec_core::FennecError::Security {
                    message: format!("Failed to write audit event: {}", e),
                })?;

            file_clone
                .write_all(b"\n")
                .await
                .map_err(|e| fennec_core::FennecError::Security {
                    message: format!("Failed to write newline: {}", e),
                })?;

            file_clone
                .flush()
                .await
                .map_err(|e| fennec_core::FennecError::Security {
                    message: format!("Failed to flush audit file: {}", e),
                })?;

            debug!("Audit event written: {}", event.metadata.event_id);
        }

        Ok(())
    }

    /// Finalize the session and log session end event
    pub async fn finalize_session(&self, command_count: u64, error_count: u64) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let duration_ms = (chrono::Utc::now() - self.started_at).num_milliseconds() as u64;

        let event_data = AuditEventData::SessionEnd(SessionEndData {
            duration_ms,
            command_count,
            error_count,
        });

        self.write_event(event_data, None).await
    }

    /// Get the session ID
    pub fn session_id(&self) -> Uuid {
        self.session_id
    }

    /// Get the audit file path
    pub fn file_path(&self) -> &PathBuf {
        &self.file_path
    }

    /// Check if audit logging is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// Global audit system that manages multiple sessions
#[derive(Debug)]
pub struct AuditSystem {
    base_audit_path: PathBuf,
    sessions: Arc<RwLock<HashMap<Uuid, Arc<SessionAuditManager>>>>,
    enabled: bool,
}

impl AuditSystem {
    /// Create a new audit system
    pub async fn new(config: &Config) -> Result<Self> {
        let base_audit_path = config
            .security
            .audit_log_path
            .clone()
            .unwrap_or_else(|| PathBuf::from(".fennec/audit"));

        let system = Self {
            base_audit_path,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            enabled: config.security.audit_log_enabled,
        };

        // Create base directory
        if system.enabled {
            tokio::fs::create_dir_all(&system.base_audit_path)
                .await
                .map_err(|e| fennec_core::FennecError::Security {
                    message: format!("Failed to create audit base directory: {}", e),
                })?;
        }

        Ok(system)
    }

    /// Start a new audit session
    pub async fn start_session(
        &self,
        session_id: Uuid,
        user_id: Option<String>,
        workspace_path: Option<String>,
    ) -> Result<Arc<SessionAuditManager>> {
        let manager = Arc::new(
            SessionAuditManager::new(
                session_id,
                &self.base_audit_path,
                user_id,
                workspace_path,
                self.enabled,
            )
            .await?,
        );

        if self.enabled {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id, manager.clone());
        }

        Ok(manager)
    }

    /// Get an existing session audit manager
    pub async fn get_session(&self, session_id: Uuid) -> Option<Arc<SessionAuditManager>> {
        let sessions = self.sessions.read().await;
        sessions.get(&session_id).cloned()
    }

    /// End a session and clean up
    pub async fn end_session(
        &self,
        session_id: Uuid,
        command_count: u64,
        error_count: u64,
    ) -> Result<()> {
        if let Some(manager) = self.get_session(session_id).await {
            manager.finalize_session(command_count, error_count).await?;
        }

        let mut sessions = self.sessions.write().await;
        sessions.remove(&session_id);

        Ok(())
    }

    /// Check if audit logging is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get the base audit path
    pub fn base_audit_path(&self) -> &PathBuf {
        &self.base_audit_path
    }
}

/// Audit query filters and parameters
#[derive(Debug, Clone, Default)]
pub struct AuditQueryFilter {
    pub session_id: Option<Uuid>,
    pub event_types: Option<Vec<String>>,
    pub date_from: Option<chrono::DateTime<chrono::Utc>>,
    pub date_to: Option<chrono::DateTime<chrono::Utc>>,
    pub user_id: Option<String>,
    pub command_id: Option<Uuid>,
    pub workspace_path: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Query result container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditQueryResult {
    pub events: Vec<AuditEvent>,
    pub total_count: usize,
    pub filtered_count: usize,
    pub query_duration_ms: u64,
}

/// Audit query interface for compliance reporting
#[derive(Debug)]
pub struct AuditQueryEngine {
    base_audit_path: PathBuf,
}

impl AuditQueryEngine {
    /// Create a new audit query engine
    pub fn new(base_audit_path: PathBuf) -> Self {
        Self { base_audit_path }
    }

    /// Query audit events with filters
    pub async fn query_events(&self, filter: AuditQueryFilter) -> Result<AuditQueryResult> {
        let start_time = std::time::Instant::now();
        let mut events = Vec::new();
        let mut total_count = 0;

        // Determine which files to scan based on date range
        let file_paths = self.get_relevant_files(&filter).await?;

        for file_path in file_paths {
            if let Ok(file_content) = tokio::fs::read_to_string(&file_path).await {
                for (line_num, line) in file_content.lines().enumerate() {
                    total_count += 1;

                    if line.trim().is_empty() {
                        continue;
                    }

                    match serde_json::from_str::<AuditEvent>(line) {
                        Ok(event) => {
                            if self.matches_filter(&event, &filter) {
                                events.push(event);
                            }
                        }
                        Err(e) => {
                            warn!(
                                "Failed to parse audit event at {}:{}: {}",
                                file_path.display(),
                                line_num + 1,
                                e
                            );
                        }
                    }
                }
            }
        }

        // Apply sorting and pagination
        events.sort_by(|a, b| a.metadata.timestamp.cmp(&b.metadata.timestamp));

        let filtered_count = events.len();

        // Apply pagination
        if let Some(offset) = filter.offset {
            if offset < events.len() {
                events = events.into_iter().skip(offset).collect();
            } else {
                events.clear();
            }
        }

        if let Some(limit) = filter.limit {
            events.truncate(limit);
        }

        let query_duration_ms = start_time.elapsed().as_millis() as u64;

        Ok(AuditQueryResult {
            events,
            total_count,
            filtered_count,
            query_duration_ms,
        })
    }

    /// Get session summary for a specific session
    pub async fn get_session_summary(&self, session_id: Uuid) -> Result<SessionSummary> {
        let filter = AuditQueryFilter {
            session_id: Some(session_id),
            ..Default::default()
        };

        let result = self.query_events(filter).await?;
        let mut summary = SessionSummary {
            session_id,
            start_time: None,
            end_time: None,
            duration_ms: None,
            command_count: 0,
            error_count: 0,
            file_operations: 0,
            security_events: 0,
            user_id: None,
            workspace_path: None,
        };

        for event in result.events {
            // Extract user_id and workspace_path from metadata
            if summary.user_id.is_none() {
                summary.user_id = event.metadata.user_id.clone();
            }
            if summary.workspace_path.is_none() {
                summary.workspace_path = event.metadata.workspace_path.clone();
            }

            match &event.data {
                AuditEventData::SessionStart(_) => {
                    summary.start_time = Some(event.metadata.timestamp);
                }
                AuditEventData::SessionEnd(data) => {
                    summary.end_time = Some(event.metadata.timestamp);
                    summary.duration_ms = Some(data.duration_ms);
                    summary.command_count = data.command_count;
                    summary.error_count = data.error_count;
                }
                AuditEventData::CommandRequested(_) => {
                    summary.command_count += 1;
                }
                AuditEventData::CommandError(_)
                | AuditEventData::SystemError(_)
                | AuditEventData::ValidationError(_) => {
                    summary.error_count += 1;
                }
                AuditEventData::FileRead(_)
                | AuditEventData::FileWrite(_)
                | AuditEventData::FileDelete(_)
                | AuditEventData::FileCreate(_) => {
                    summary.file_operations += 1;
                }
                AuditEventData::PermissionCheck(_)
                | AuditEventData::SandboxViolation(_)
                | AuditEventData::SecurityWarning(_) => {
                    summary.security_events += 1;
                }
                _ => {}
            }
        }

        Ok(summary)
    }

    /// Get command execution trail for a specific command
    pub async fn get_command_trail(&self, command_id: Uuid) -> Result<CommandTrail> {
        let filter = AuditQueryFilter {
            command_id: Some(command_id),
            ..Default::default()
        };

        let result = self.query_events(filter).await?;
        let mut trail = CommandTrail {
            command_id,
            events: Vec::new(),
            timeline: Vec::new(),
        };

        for event in result.events {
            let event_type = match &event.data {
                AuditEventData::CommandRequested(_) => "Requested",
                AuditEventData::CommandPreview(_) => "Preview Generated",
                AuditEventData::CommandApproved(_) => "Approved",
                AuditEventData::CommandRejected(_) => "Rejected",
                AuditEventData::CommandStarted(_) => "Started",
                AuditEventData::CommandCompleted(_) => "Completed",
                AuditEventData::CommandError(_) => "Error",
                _ => continue,
            };

            trail.timeline.push(CommandTimelineEntry {
                timestamp: event.metadata.timestamp,
                event_type: event_type.to_string(),
                description: format!("{:?}", event.data),
            });

            trail.events.push(event);
        }

        trail.timeline.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        Ok(trail)
    }

    /// Export audit logs for compliance reporting
    pub async fn export_audit_logs(
        &self,
        filter: AuditQueryFilter,
        format: ExportFormat,
    ) -> Result<String> {
        let result = self.query_events(filter).await?;

        match format {
            ExportFormat::Json => serde_json::to_string_pretty(&result.events).map_err(|e| {
                fennec_core::FennecError::Security {
                    message: format!("Failed to serialize audit events to JSON: {}", e),
                }
            }),
            ExportFormat::Csv => {
                let mut csv_output = String::new();
                csv_output.push_str("timestamp,event_id,session_id,sequence_number,event_type,user_id,workspace_path\n");

                for event in result.events {
                    let event_type = match &event.data {
                        AuditEventData::SessionStart(_) => "SessionStart",
                        AuditEventData::SessionEnd(_) => "SessionEnd",
                        AuditEventData::CommandRequested(_) => "CommandRequested",
                        AuditEventData::CommandCompleted(_) => "CommandCompleted",
                        AuditEventData::FileWrite(_) => "FileWrite",
                        AuditEventData::SecurityWarning(_) => "SecurityWarning",
                        _ => "Other",
                    };

                    csv_output.push_str(&format!(
                        "{},{},{},{},{},{},{}\n",
                        event.metadata.timestamp.to_rfc3339(),
                        event.metadata.event_id,
                        event.metadata.session_id,
                        event.metadata.sequence_number,
                        event_type,
                        event.metadata.user_id.as_deref().unwrap_or(""),
                        event.metadata.workspace_path.as_deref().unwrap_or("")
                    ));
                }

                Ok(csv_output)
            }
        }
    }

    /// Get relevant audit files based on filter criteria
    async fn get_relevant_files(&self, filter: &AuditQueryFilter) -> Result<Vec<PathBuf>> {
        let sessions_dir = self.base_audit_path.join("sessions");

        if !sessions_dir.exists() {
            return Ok(Vec::new());
        }

        let mut file_paths = Vec::new();

        // If session_id is specified, try to find the specific file
        if let Some(session_id) = filter.session_id {
            // Scan all date directories for the session file
            let mut date_dirs = tokio::fs::read_dir(&sessions_dir).await.map_err(|e| {
                fennec_core::FennecError::Security {
                    message: format!("Failed to read sessions directory: {}", e),
                }
            })?;

            while let Some(entry) =
                date_dirs
                    .next_entry()
                    .await
                    .map_err(|e| fennec_core::FennecError::Security {
                        message: format!("Failed to read directory entry: {}", e),
                    })?
            {
                if entry
                    .file_type()
                    .await
                    .map_err(|e| fennec_core::FennecError::Security {
                        message: format!("Failed to get file type: {}", e),
                    })?
                    .is_dir()
                {
                    let mut files = tokio::fs::read_dir(entry.path()).await.map_err(|e| {
                        fennec_core::FennecError::Security {
                            message: format!("Failed to read date directory: {}", e),
                        }
                    })?;

                    while let Some(file) = files.next_entry().await.map_err(|e| {
                        fennec_core::FennecError::Security {
                            message: format!("Failed to read file entry: {}", e),
                        }
                    })? {
                        let file_name = file.file_name();
                        let file_name_str = file_name.to_string_lossy();

                        if file_name_str.starts_with(&format!("fennec-audit-{}", session_id))
                            && file_name_str.ends_with(".jsonl")
                        {
                            file_paths.push(file.path());
                        }
                    }
                }
            }
        } else {
            // Scan all files, optionally filtered by date range
            let mut date_dirs = tokio::fs::read_dir(&sessions_dir).await.map_err(|e| {
                fennec_core::FennecError::Security {
                    message: format!("Failed to read sessions directory: {}", e),
                }
            })?;

            while let Some(entry) =
                date_dirs
                    .next_entry()
                    .await
                    .map_err(|e| fennec_core::FennecError::Security {
                        message: format!("Failed to read directory entry: {}", e),
                    })?
            {
                if entry
                    .file_type()
                    .await
                    .map_err(|e| fennec_core::FennecError::Security {
                        message: format!("Failed to get file type: {}", e),
                    })?
                    .is_dir()
                {
                    // Check if this date directory is within our date range
                    let dir_name = entry.file_name();
                    let dir_name_str = dir_name.to_string_lossy();

                    if let Some(date_from) = filter.date_from {
                        if let Ok(dir_date) =
                            chrono::NaiveDate::parse_from_str(&dir_name_str, "%Y-%m-%d")
                        {
                            let dir_datetime = dir_date.and_hms_opt(0, 0, 0).unwrap().and_utc();
                            if dir_datetime < date_from {
                                continue;
                            }
                        }
                    }

                    if let Some(date_to) = filter.date_to {
                        if let Ok(dir_date) =
                            chrono::NaiveDate::parse_from_str(&dir_name_str, "%Y-%m-%d")
                        {
                            let dir_datetime = dir_date.and_hms_opt(23, 59, 59).unwrap().and_utc();
                            if dir_datetime > date_to {
                                continue;
                            }
                        }
                    }

                    let mut files = tokio::fs::read_dir(entry.path()).await.map_err(|e| {
                        fennec_core::FennecError::Security {
                            message: format!("Failed to read date directory: {}", e),
                        }
                    })?;

                    while let Some(file) = files.next_entry().await.map_err(|e| {
                        fennec_core::FennecError::Security {
                            message: format!("Failed to read file entry: {}", e),
                        }
                    })? {
                        let file_name = file.file_name();
                        let file_name_str = file_name.to_string_lossy();

                        if file_name_str.starts_with("fennec-audit-")
                            && file_name_str.ends_with(".jsonl")
                        {
                            file_paths.push(file.path());
                        }
                    }
                }
            }
        }

        file_paths.sort();
        Ok(file_paths)
    }

    /// Check if an event matches the filter criteria
    fn matches_filter(&self, event: &AuditEvent, filter: &AuditQueryFilter) -> bool {
        // Session ID filter
        if let Some(session_id) = filter.session_id {
            if event.metadata.session_id != session_id {
                return false;
            }
        }

        // Event type filter
        if let Some(event_types) = &filter.event_types {
            let event_type = match &event.data {
                AuditEventData::SessionStart(_) => "SessionStart",
                AuditEventData::SessionEnd(_) => "SessionEnd",
                AuditEventData::CommandRequested(_) => "CommandRequested",
                AuditEventData::CommandPreview(_) => "CommandPreview",
                AuditEventData::CommandApproved(_) => "CommandApproved",
                AuditEventData::CommandRejected(_) => "CommandRejected",
                AuditEventData::CommandStarted(_) => "CommandStarted",
                AuditEventData::CommandCompleted(_) => "CommandCompleted",
                AuditEventData::FileRead(_) => "FileRead",
                AuditEventData::FileWrite(_) => "FileWrite",
                AuditEventData::FileDelete(_) => "FileDelete",
                AuditEventData::FileCreate(_) => "FileCreate",
                AuditEventData::DirectoryCreate(_) => "DirectoryCreate",
                AuditEventData::DirectoryDelete(_) => "DirectoryDelete",
                AuditEventData::PermissionCheck(_) => "PermissionCheck",
                AuditEventData::SandboxViolation(_) => "SandboxViolation",
                AuditEventData::ApprovalRequired(_) => "ApprovalRequired",
                AuditEventData::SecurityWarning(_) => "SecurityWarning",
                AuditEventData::CommandError(_) => "CommandError",
                AuditEventData::SystemError(_) => "SystemError",
                AuditEventData::ValidationError(_) => "ValidationError",
                _ => "Unknown",
            };

            if !event_types.contains(&event_type.to_string()) {
                return false;
            }
        }

        // Date range filter
        if let Some(date_from) = filter.date_from {
            if event.metadata.timestamp < date_from {
                return false;
            }
        }

        if let Some(date_to) = filter.date_to {
            if event.metadata.timestamp > date_to {
                return false;
            }
        }

        // User ID filter
        if let Some(user_id) = &filter.user_id {
            if event.metadata.user_id.as_ref() != Some(user_id) {
                return false;
            }
        }

        // Command ID filter
        if let Some(command_id) = filter.command_id {
            let event_command_id = match &event.data {
                AuditEventData::CommandRequested(data) => Some(data.command_id),
                AuditEventData::CommandPreview(data) => Some(data.command_id),
                AuditEventData::CommandApproved(data) => Some(data.command_id),
                AuditEventData::CommandRejected(data) => Some(data.command_id),
                AuditEventData::CommandStarted(data) => Some(data.command_id),
                AuditEventData::CommandCompleted(data) => Some(data.command_id),
                AuditEventData::CommandError(data) => Some(data.command_id),
                _ => None,
            };

            if event_command_id != Some(command_id) {
                return false;
            }
        }

        // Workspace path filter
        if let Some(workspace_path) = &filter.workspace_path {
            if event.metadata.workspace_path.as_ref() != Some(workspace_path) {
                return false;
            }
        }

        true
    }
}

/// Session summary for reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: Uuid,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub duration_ms: Option<u64>,
    pub command_count: u64,
    pub error_count: u64,
    pub file_operations: u64,
    pub security_events: u64,
    pub user_id: Option<String>,
    pub workspace_path: Option<String>,
}

/// Command execution trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandTrail {
    pub command_id: Uuid,
    pub events: Vec<AuditEvent>,
    pub timeline: Vec<CommandTimelineEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandTimelineEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event_type: String,
    pub description: String,
}

/// Export format options
#[derive(Debug, Clone)]
pub enum ExportFormat {
    Json,
    Csv,
}

/// Utility functions for generating checksums and hashes
pub mod utils {
    use ring::digest::{digest, SHA256};
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    /// Generate a SHA256 checksum of data
    pub fn sha256_checksum(data: &[u8]) -> String {
        let digest = digest(&SHA256, data);
        hex::encode(digest.as_ref())
    }

    /// Generate a hash of any hashable data (for args, etc.)
    pub fn hash_data<T: Hash>(data: &T) -> String {
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Generate a preview hash from command preview
    pub fn preview_hash(preview: &fennec_core::command::CommandPreview) -> String {
        hash_data(&(
            preview.command_id,
            &preview.description,
            &preview.actions,
            preview.requires_approval,
        ))
    }
}

// Legacy compatibility - keeping the old AuditLogger for existing code
pub use self::legacy::AuditLogger;

mod legacy {
    use super::*;
    use serde_json::json;

    /// Legacy audit logger for backward compatibility
    pub struct AuditLogger {
        log_path: PathBuf,
        enabled: bool,
    }

    impl AuditLogger {
        /// Create a new AuditLogger from config
        pub async fn new(config: &Config) -> Result<Self> {
            let log_path = config
                .security
                .audit_log_path
                .clone()
                .unwrap_or_else(|| PathBuf::from(".fennec/audit.jsonl"));

            let logger = Self {
                log_path,
                enabled: config.security.audit_log_enabled,
            };

            // Create directory if it doesn't exist
            if logger.enabled {
                if let Some(parent) = logger.log_path.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
            }

            Ok(logger)
        }

        /// Create a new AuditLogger with a specific path (for testing)
        pub async fn with_path<P: Into<PathBuf>>(log_path: P) -> Result<Self> {
            let log_path = log_path.into();

            // Create directory if it doesn't exist
            if let Some(parent) = log_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }

            Ok(Self {
                log_path,
                enabled: true,
            })
        }

        /// Log a session event (legacy)
        pub async fn log_session_event(
            &self,
            session_id: Uuid,
            event_type: &str,
            metadata: Option<&str>,
        ) -> Result<()> {
            if !self.enabled {
                return Ok(());
            }

            let event = json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "event_type": "session_event",
                "session_id": session_id,
                "details": {
                    "action": event_type,
                    "metadata": metadata
                }
            });

            self.write_log_entry(&event).await
        }

        /// Log a user message (legacy)
        pub async fn log_user_message(&self, session_id: Uuid, content: &str) -> Result<()> {
            if !self.enabled {
                return Ok(());
            }

            let event = json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "event_type": "user_message",
                "session_id": session_id,
                "details": {
                    "content_length": content.len(),
                    "content_preview": content.chars().take(100).collect::<String>()
                }
            });

            self.write_log_entry(&event).await
        }

        /// Log an assistant message (legacy)
        pub async fn log_assistant_message(&self, session_id: Uuid, content: &str) -> Result<()> {
            if !self.enabled {
                return Ok(());
            }

            let event = json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "event_type": "assistant_message",
                "session_id": session_id,
                "details": {
                    "content_length": content.len(),
                    "content_preview": content.chars().take(100).collect::<String>()
                }
            });

            self.write_log_entry(&event).await
        }

        /// Log an error event (legacy)
        pub async fn log_error_event(&self, session_id: Uuid, error_message: &str) -> Result<()> {
            if !self.enabled {
                return Ok(());
            }

            let event = json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "event_type": "error",
                "session_id": session_id,
                "details": {
                    "error_message": error_message
                }
            });

            self.write_log_entry(&event).await
        }

        /// Log a security event (legacy)
        pub async fn log_security_event(
            &self,
            session_id: Option<Uuid>,
            event_type: &str,
            details: &str,
        ) -> Result<()> {
            if !self.enabled {
                return Ok(());
            }

            let event = json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "event_type": "security_event",
                "session_id": session_id,
                "details": {
                    "security_event_type": event_type,
                    "details": details
                }
            });

            self.write_log_entry(&event).await
        }

        /// Write a log entry to the audit log file
        async fn write_log_entry(&self, event: &serde_json::Value) -> Result<()> {
            match self.serialize_and_write(event).await {
                Ok(()) => {
                    debug!("Audit log entry written successfully");
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to write audit log entry: {}", e);
                    Err(e)
                }
            }
        }

        /// Serialize and write the event to file
        async fn serialize_and_write(&self, event: &serde_json::Value) -> Result<()> {
            let log_line =
                serde_json::to_string(event).map_err(|e| fennec_core::FennecError::Security {
                    message: format!("Failed to serialize audit log entry: {}", e),
                })?;

            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.log_path)
                .await
                .map_err(|e| fennec_core::FennecError::Security {
                    message: format!("Failed to open audit log file: {}", e),
                })?;

            file.write_all(log_line.as_bytes()).await.map_err(|e| {
                fennec_core::FennecError::Security {
                    message: format!("Failed to write to audit log file: {}", e),
                }
            })?;

            file.write_all(b"\n")
                .await
                .map_err(|e| fennec_core::FennecError::Security {
                    message: format!("Failed to write newline to audit log file: {}", e),
                })?;

            file.flush()
                .await
                .map_err(|e| fennec_core::FennecError::Security {
                    message: format!("Failed to flush audit log file: {}", e),
                })?;

            Ok(())
        }

        /// Check if audit logging is enabled
        pub fn is_enabled(&self) -> bool {
            self.enabled
        }

        /// Get the log file path
        pub fn log_path(&self) -> &PathBuf {
            &self.log_path
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fennec_core::command::CommandPreview;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_session_audit_manager() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();
        let session_id = Uuid::new_v4();

        let manager = SessionAuditManager::new(
            session_id,
            &base_path,
            Some("test_user".to_string()),
            Some("/test/workspace".to_string()),
            true,
        )
        .await
        .unwrap();

        assert_eq!(manager.session_id(), session_id);
        assert!(manager.is_enabled());

        // Test logging a command event
        let command_id = Uuid::new_v4();
        let event_data = AuditEventData::CommandRequested(CommandRequestedData {
            command_id,
            command_name: "test_command".to_string(),
            args_hash: "abc123".to_string(),
            capabilities_required: vec![Capability::ReadFile],
            sandbox_level: crate::SandboxLevel::ReadOnly,
        });

        manager.log_event(event_data, None).await.unwrap();

        // Verify file exists and has content
        assert!(manager.file_path().exists());
        let content = tokio::fs::read_to_string(manager.file_path())
            .await
            .unwrap();
        assert!(content.contains("SessionStart"));
        assert!(content.contains("CommandRequested"));
        assert!(content.contains(&session_id.to_string()));
        assert!(content.contains(&command_id.to_string()));
    }

    #[tokio::test]
    async fn test_audit_system() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::default();
        config.security.audit_log_path = Some(temp_dir.path().to_path_buf());
        config.security.audit_log_enabled = true;

        let audit_system = AuditSystem::new(&config).await.unwrap();
        assert!(audit_system.is_enabled());

        let session_id = Uuid::new_v4();
        let manager = audit_system
            .start_session(session_id, Some("test_user".to_string()), None)
            .await
            .unwrap();

        assert_eq!(manager.session_id(), session_id);

        // Test ending session
        audit_system.end_session(session_id, 5, 1).await.unwrap();

        // Verify session end event was logged
        let content = tokio::fs::read_to_string(manager.file_path())
            .await
            .unwrap();
        assert!(content.contains("SessionEnd"));
        assert!(content.contains("\"command_count\":5"));
        assert!(content.contains("\"error_count\":1"));
    }

    #[tokio::test]
    async fn test_utils() {
        // Test checksum generation
        let data = b"test data";
        let checksum = utils::sha256_checksum(data);
        assert!(!checksum.is_empty());

        // Test hash generation
        let test_string = "test";
        let hash = utils::hash_data(&test_string);
        assert!(!hash.is_empty());

        // Test preview hash
        let preview = CommandPreview {
            command_id: Uuid::new_v4(),
            description: "Test preview".to_string(),
            actions: vec![],
            requires_approval: false,
        };
        let preview_hash = utils::preview_hash(&preview);
        assert!(!preview_hash.is_empty());
    }

    #[tokio::test]
    async fn test_legacy_compatibility() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test_audit.log");

        let logger = AuditLogger::with_path(log_path.clone()).await.unwrap();
        let session_id = Uuid::new_v4();

        logger
            .log_session_event(session_id, "test_event", Some("test_metadata"))
            .await
            .unwrap();

        assert!(log_path.exists());
        let content = tokio::fs::read_to_string(&log_path).await.unwrap();
        assert!(content.contains("session_event"));
        assert!(content.contains("test_event"));
        assert!(content.contains(&session_id.to_string()));
    }
}
