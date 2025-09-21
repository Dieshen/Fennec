use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub capabilities: Vec<Capability>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Capability {
    ReadFile,
    WriteFile,
    ExecuteShell,
    NetworkAccess,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPreview {
    pub command_id: Uuid,
    pub description: String,
    pub actions: Vec<PreviewAction>,
    pub requires_approval: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PreviewAction {
    ReadFile { path: String },
    WriteFile { path: String, content: String },
    ExecuteShell { command: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub command_id: Uuid,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}
