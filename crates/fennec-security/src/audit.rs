use fennec_core::{config::Config, Result};
use std::path::PathBuf;

pub struct AuditLogger {
    log_path: PathBuf,
    enabled: bool,
}

impl AuditLogger {
    pub async fn new(config: &Config) -> Result<Self> {
        let log_path = config.security.audit_log_path
            .clone()
            .unwrap_or_else(|| PathBuf::from(".fennec/audit.jsonl"));

        Ok(Self {
            log_path,
            enabled: config.security.audit_log_enabled,
        })
    }
}