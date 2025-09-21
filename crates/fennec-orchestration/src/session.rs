use fennec_core::{config::Config, Result};
use fennec_security::audit::AuditLogger;

pub struct SessionManager {
    config: Config,
    audit_logger: AuditLogger,
}

impl SessionManager {
    pub async fn new(config: Config, audit_logger: AuditLogger) -> Result<Self> {
        Ok(Self {
            config,
            audit_logger,
        })
    }
}