use fennec_core::{config::Config, Result};
use serde_json::json;
use std::path::PathBuf;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tracing::{debug, error};
use uuid::Uuid;

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

    /// Log a session event
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

    /// Log a user message
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

    /// Log an assistant message
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

    /// Log an error event
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

    /// Log a security event
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_audit_logger_creation() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test_audit.log");

        let logger = AuditLogger::with_path(log_path.clone()).await.unwrap();
        assert_eq!(logger.log_path(), &log_path);
        assert!(logger.is_enabled());
    }

    #[tokio::test]
    async fn test_session_event_logging() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test_audit.log");

        let logger = AuditLogger::with_path(log_path.clone()).await.unwrap();
        let session_id = Uuid::new_v4();

        logger
            .log_session_event(session_id, "test_event", Some("test_metadata"))
            .await
            .unwrap();

        // Verify file was created and has content
        assert!(log_path.exists());
        let content = tokio::fs::read_to_string(&log_path).await.unwrap();
        assert!(content.contains("session_event"));
        assert!(content.contains("test_event"));
        assert!(content.contains(&session_id.to_string()));
    }

    #[tokio::test]
    async fn test_message_logging() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test_audit.log");

        let logger = AuditLogger::with_path(log_path.clone()).await.unwrap();
        let session_id = Uuid::new_v4();

        logger
            .log_user_message(session_id, "Test user message")
            .await
            .unwrap();

        logger
            .log_assistant_message(session_id, "Test assistant message")
            .await
            .unwrap();

        let content = tokio::fs::read_to_string(&log_path).await.unwrap();
        assert!(content.contains("user_message"));
        assert!(content.contains("assistant_message"));
        assert!(content.contains("Test user message"));
        assert!(content.contains("Test assistant message"));
    }
}
