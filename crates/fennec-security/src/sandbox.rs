use anyhow::{anyhow, Result};
use fennec_core::command::Capability;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

impl std::fmt::Display for SandboxLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SandboxLevel::ReadOnly => write!(f, "read-only"),
            SandboxLevel::WorkspaceWrite => write!(f, "workspace-write"),
            SandboxLevel::FullAccess => write!(f, "full-access"),
        }
    }
}

/// Sandbox policy enforcement engine
#[derive(Debug, Clone)]
pub struct SandboxPolicy {
    level: SandboxLevel,
    workspace_path: PathBuf,
    require_approval: bool,
}

/// Result of a sandbox policy check
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyResult {
    Allow,
    Deny(String),
    RequireApproval(String),
}

/// Policy enforcement matrix for different sandbox levels
#[derive(Debug, Clone)]
pub struct PolicyMatrix {
    read_only_capabilities: HashSet<Capability>,
    workspace_write_capabilities: HashSet<Capability>,
    full_access_capabilities: HashSet<Capability>,
}

impl Default for PolicyMatrix {
    fn default() -> Self {
        let mut read_only = HashSet::new();
        read_only.insert(Capability::ReadFile);

        let mut workspace_write = HashSet::new();
        workspace_write.insert(Capability::ReadFile);
        workspace_write.insert(Capability::WriteFile);

        let mut full_access = HashSet::new();
        full_access.insert(Capability::ReadFile);
        full_access.insert(Capability::WriteFile);
        full_access.insert(Capability::ExecuteShell);
        full_access.insert(Capability::NetworkAccess);

        Self {
            read_only_capabilities: read_only,
            workspace_write_capabilities: workspace_write,
            full_access_capabilities: full_access,
        }
    }
}

impl SandboxPolicy {
    /// Create a new sandbox policy
    pub fn new(level: SandboxLevel, workspace_path: PathBuf, require_approval: bool) -> Self {
        Self {
            level,
            workspace_path: workspace_path.canonicalize().unwrap_or(workspace_path),
            require_approval,
        }
    }

    /// Get the current sandbox level
    pub fn level(&self) -> &SandboxLevel {
        &self.level
    }

    /// Get the workspace path
    pub fn workspace_path(&self) -> &Path {
        &self.workspace_path
    }

    /// Check if approval is required
    pub fn requires_approval(&self) -> bool {
        self.require_approval
    }

    /// Check if a capability is allowed by the current sandbox level
    pub fn check_capability(&self, capability: &Capability) -> PolicyResult {
        let matrix = PolicyMatrix::default();

        let allowed_capabilities = match &self.level {
            SandboxLevel::ReadOnly => &matrix.read_only_capabilities,
            SandboxLevel::WorkspaceWrite => &matrix.workspace_write_capabilities,
            SandboxLevel::FullAccess => &matrix.full_access_capabilities,
        };

        if allowed_capabilities.contains(capability) {
            // Check if approval is required for this capability
            if self.require_approval && self.should_require_approval_for_capability(capability) {
                PolicyResult::RequireApproval(format!(
                    "Capability {:?} requires approval in {} mode",
                    capability, self.level
                ))
            } else {
                PolicyResult::Allow
            }
        } else {
            PolicyResult::Deny(format!(
                "Capability {:?} is not allowed in {} mode",
                capability, self.level
            ))
        }
    }

    /// Check if a file path is accessible for reading
    pub fn check_read_path(&self, path: &Path) -> PolicyResult {
        match self.normalize_and_validate_path(path) {
            Ok(normalized_path) => match &self.level {
                SandboxLevel::ReadOnly | SandboxLevel::WorkspaceWrite => {
                    if self.is_within_workspace(&normalized_path) {
                        PolicyResult::Allow
                    } else {
                        PolicyResult::Deny(format!(
                            "Path {} is outside workspace in {} mode",
                            path.display(),
                            self.level
                        ))
                    }
                }
                SandboxLevel::FullAccess => {
                    if self.require_approval && !self.is_within_workspace(&normalized_path) {
                        PolicyResult::RequireApproval(format!(
                            "Reading {} outside workspace requires approval",
                            path.display()
                        ))
                    } else {
                        PolicyResult::Allow
                    }
                }
            },
            Err(e) => PolicyResult::Deny(format!("Invalid path {}: {}", path.display(), e)),
        }
    }

    /// Check if a file path is accessible for writing
    pub fn check_write_path(&self, path: &Path) -> PolicyResult {
        // First check if writing is allowed at all
        if let PolicyResult::Deny(msg) = self.check_capability(&Capability::WriteFile) {
            return PolicyResult::Deny(msg);
        }

        match self.normalize_and_validate_path(path) {
            Ok(normalized_path) => match &self.level {
                SandboxLevel::ReadOnly => {
                    PolicyResult::Deny("File writing is not allowed in read-only mode".to_string())
                }
                SandboxLevel::WorkspaceWrite => {
                    if self.is_within_workspace(&normalized_path) {
                        if self.require_approval {
                            PolicyResult::RequireApproval(format!(
                                "Writing to {} requires approval",
                                path.display()
                            ))
                        } else {
                            PolicyResult::Allow
                        }
                    } else {
                        PolicyResult::Deny(format!(
                            "Path {} is outside workspace in workspace-write mode",
                            path.display()
                        ))
                    }
                }
                SandboxLevel::FullAccess => {
                    if self.require_approval {
                        PolicyResult::RequireApproval(format!(
                            "Writing to {} requires approval",
                            path.display()
                        ))
                    } else {
                        PolicyResult::Allow
                    }
                }
            },
            Err(e) => PolicyResult::Deny(format!("Invalid path {}: {}", path.display(), e)),
        }
    }

    /// Check if a shell command is allowed
    pub fn check_shell_command(&self, command: &str) -> PolicyResult {
        // First check if shell execution is allowed at all
        if let PolicyResult::Deny(msg) = self.check_capability(&Capability::ExecuteShell) {
            return PolicyResult::Deny(msg);
        }

        match &self.level {
            SandboxLevel::ReadOnly | SandboxLevel::WorkspaceWrite => PolicyResult::Deny(format!(
                "Shell command execution is not allowed in {} mode",
                self.level
            )),
            SandboxLevel::FullAccess => {
                if self.require_approval || self.is_dangerous_command(command) {
                    PolicyResult::RequireApproval(format!(
                        "Executing shell command '{}' requires approval",
                        command
                    ))
                } else {
                    PolicyResult::Allow
                }
            }
        }
    }

    /// Check if network access is allowed
    pub fn check_network_access(&self, url: &str) -> PolicyResult {
        // First check if network access is allowed at all
        if let PolicyResult::Deny(msg) = self.check_capability(&Capability::NetworkAccess) {
            return PolicyResult::Deny(msg);
        }

        match &self.level {
            SandboxLevel::ReadOnly | SandboxLevel::WorkspaceWrite => PolicyResult::Deny(format!(
                "Network access is not allowed in {} mode",
                self.level
            )),
            SandboxLevel::FullAccess => {
                if self.require_approval {
                    PolicyResult::RequireApproval(format!(
                        "Network access to {} requires approval",
                        url
                    ))
                } else {
                    PolicyResult::Allow
                }
            }
        }
    }

    /// Normalize and validate a path to prevent path traversal attacks
    fn normalize_and_validate_path(&self, path: &Path) -> Result<PathBuf> {
        // Convert to absolute path if relative
        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.workspace_path.join(path)
        };

        // Canonicalize to resolve any .. or . components
        let canonical_path = absolute_path.canonicalize().or_else(|_| {
            // If canonicalize fails (e.g., file doesn't exist), manually resolve
            self.resolve_path_components(&absolute_path)
        })?;

        // Check for path traversal attempts
        if canonical_path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(anyhow!("Path traversal detected: {}", path.display()));
        }

        Ok(canonical_path)
    }

    /// Manually resolve path components to handle non-existent paths
    fn resolve_path_components(&self, path: &Path) -> Result<PathBuf> {
        let mut resolved = PathBuf::new();

        for component in path.components() {
            match component {
                std::path::Component::ParentDir => {
                    if !resolved.pop() {
                        return Err(anyhow!(
                            "Path traversal attempt: too many parent directory references"
                        ));
                    }
                }
                std::path::Component::CurDir => {
                    // Skip current directory references
                }
                other => {
                    resolved.push(other);
                }
            }
        }

        Ok(resolved)
    }

    /// Check if a path is within the workspace
    fn is_within_workspace(&self, path: &Path) -> bool {
        path.starts_with(&self.workspace_path)
    }

    /// Check if approval should be required for a specific capability
    fn should_require_approval_for_capability(&self, capability: &Capability) -> bool {
        match capability {
            Capability::WriteFile | Capability::ExecuteShell | Capability::NetworkAccess => true,
            Capability::ReadFile => false, // Reading generally doesn't require approval
        }
    }

    /// Check if a shell command is considered dangerous
    fn is_dangerous_command(&self, command: &str) -> bool {
        let dangerous_commands = [
            "rm",
            "del",
            "format",
            "mkfs",
            "dd",
            "fdisk",
            "parted",
            "sudo",
            "su",
            "chmod",
            "chown",
            "passwd",
            "userdel",
            "useradd",
            "systemctl",
            "service",
            "reboot",
            "shutdown",
            "halt",
            "poweroff",
            "iptables",
            "ufw",
            "firewall",
            "netsh",
            "route",
            "curl",
            "wget",
            "nc",
            "netcat",
            "telnet",
            "ssh",
            "scp",
            "rsync",
        ];

        let command_lower = command.to_lowercase();
        dangerous_commands.iter().any(|&dangerous| {
            command_lower.starts_with(dangerous)
                || command_lower.contains(&format!(" {}", dangerous))
        })
    }
}

/// Validate working directory and create sandbox policy
pub fn create_sandbox_policy(
    level: SandboxLevel,
    working_dir: Option<&Path>,
    require_approval: bool,
) -> Result<SandboxPolicy> {
    let workspace_path = match working_dir {
        Some(dir) => {
            if !dir.exists() {
                return Err(anyhow!(
                    "Working directory does not exist: {}",
                    dir.display()
                ));
            }
            if !dir.is_dir() {
                return Err(anyhow!(
                    "Working directory is not a directory: {}",
                    dir.display()
                ));
            }
            dir.to_path_buf()
        }
        None => std::env::current_dir()?,
    };

    Ok(SandboxPolicy::new(level, workspace_path, require_approval))
}
