use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SandboxLevel {
    ReadOnly,
    WorkspaceWrite,
    FullAccess,
}

impl Default for SandboxLevel {
    fn default() -> Self {
        Self::WorkspaceWrite
    }
}