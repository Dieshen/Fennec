use crate::sandbox::SandboxPolicy;
use anyhow::Result;
use fennec_core::command::{CommandPreview, PreviewAction};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};

/// Approval status for operations requiring user consent
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Denied,
    TimedOut,
}

/// Approval request containing details about the operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub operation: String,
    pub description: String,
    pub risk_level: RiskLevel,
    pub details: Vec<String>,
}

/// Risk level classification for operations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::Low => write!(f, "LOW"),
            RiskLevel::Medium => write!(f, "MEDIUM"),
            RiskLevel::High => write!(f, "HIGH"),
            RiskLevel::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Approval manager for handling user consent workflows
#[derive(Debug)]
pub struct ApprovalManager {
    auto_approve_low_risk: bool,
    interactive_mode: bool,
}

impl Default for ApprovalManager {
    fn default() -> Self {
        Self {
            auto_approve_low_risk: false,
            interactive_mode: true,
        }
    }
}

impl ApprovalManager {
    /// Create a new approval manager
    pub fn new(auto_approve_low_risk: bool, interactive_mode: bool) -> Self {
        Self {
            auto_approve_low_risk,
            interactive_mode,
        }
    }

    /// Request approval for an operation
    pub fn request_approval(&self, request: &ApprovalRequest) -> Result<ApprovalStatus> {
        // Auto-approve low risk operations if configured
        if self.auto_approve_low_risk && request.risk_level == RiskLevel::Low {
            return Ok(ApprovalStatus::Approved);
        }

        if !self.interactive_mode {
            // In non-interactive mode, deny all requests that require approval
            return Ok(ApprovalStatus::Denied);
        }

        self.prompt_user_approval(request)
    }

    /// Prompt user for approval via terminal interface
    fn prompt_user_approval(&self, request: &ApprovalRequest) -> Result<ApprovalStatus> {
        println!("\n🛡️  SECURITY APPROVAL REQUIRED");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Operation: {}", request.operation);
        println!(
            "Risk Level: {} {}",
            self.risk_level_emoji(&request.risk_level),
            request.risk_level
        );
        println!("Description: {}", request.description);

        if !request.details.is_empty() {
            println!("\nDetails:");
            for detail in &request.details {
                println!("  • {}", detail);
            }
        }

        println!("\n{}", self.get_risk_warning(&request.risk_level));

        loop {
            print!("\nDo you want to proceed? [y/N/details]: ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim().to_lowercase();

            match input.as_str() {
                "y" | "yes" => {
                    println!("✅ Operation approved by user");
                    return Ok(ApprovalStatus::Approved);
                }
                "n" | "no" | "" => {
                    println!("❌ Operation denied by user");
                    return Ok(ApprovalStatus::Denied);
                }
                "d" | "details" => {
                    self.show_detailed_info(request);
                    continue;
                }
                "?" | "help" => {
                    self.show_help();
                    continue;
                }
                _ => {
                    println!("Invalid input. Please enter 'y' for yes, 'n' for no, or 'details' for more information.");
                    continue;
                }
            }
        }
    }

    /// Show detailed information about the approval request
    fn show_detailed_info(&self, request: &ApprovalRequest) {
        println!("\n📋 DETAILED INFORMATION");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Operation Type: {}", request.operation);
        println!(
            "Risk Assessment: {} ({})",
            request.risk_level,
            self.risk_level_description(&request.risk_level)
        );
        println!("Description: {}", request.description);

        if !request.details.is_empty() {
            println!("\nTechnical Details:");
            for (i, detail) in request.details.iter().enumerate() {
                println!("  {}. {}", i + 1, detail);
            }
        }

        println!("\nSecurity Implications:");
        for implication in self.get_security_implications(&request.risk_level) {
            println!("  ⚠️  {}", implication);
        }
    }

    /// Show help information
    fn show_help(&self) {
        println!("\n❓ APPROVAL HELP");
        println!("━━━━━━━━━━━━━━━━━━");
        println!("Available commands:");
        println!("  y, yes     - Approve the operation");
        println!("  n, no      - Deny the operation (default)");
        println!("  details, d - Show detailed information");
        println!("  help, ?    - Show this help message");
        println!("\nRisk Levels:");
        println!("  🟢 LOW      - Minimal security impact");
        println!("  🟡 MEDIUM   - Moderate security considerations");
        println!("  🟠 HIGH     - Significant security implications");
        println!("  🔴 CRITICAL - Severe security risks");
    }

    /// Get emoji for risk level
    fn risk_level_emoji(&self, risk_level: &RiskLevel) -> &'static str {
        match risk_level {
            RiskLevel::Low => "🟢",
            RiskLevel::Medium => "🟡",
            RiskLevel::High => "🟠",
            RiskLevel::Critical => "🔴",
        }
    }

    /// Get risk level description
    fn risk_level_description(&self, risk_level: &RiskLevel) -> &'static str {
        match risk_level {
            RiskLevel::Low => "Minimal security impact",
            RiskLevel::Medium => "Moderate security considerations",
            RiskLevel::High => "Significant security implications",
            RiskLevel::Critical => "Severe security risks",
        }
    }

    /// Get warning message for risk level
    fn get_risk_warning(&self, risk_level: &RiskLevel) -> String {
        match risk_level {
            RiskLevel::Low => "⚪ This operation has minimal security impact.".to_string(),
            RiskLevel::Medium => "🟡 This operation may affect system security. Please review carefully.".to_string(),
            RiskLevel::High => "🟠 WARNING: This operation has significant security implications!".to_string(),
            RiskLevel::Critical => "🔴 DANGER: This operation poses severe security risks! Proceed with extreme caution!".to_string(),
        }
    }

    /// Get security implications for risk level
    fn get_security_implications(&self, risk_level: &RiskLevel) -> Vec<&'static str> {
        match risk_level {
            RiskLevel::Low => vec![
                "Limited scope of impact",
                "Reversible changes",
                "No system-level access required",
            ],
            RiskLevel::Medium => vec![
                "May modify user data or configurations",
                "Could affect application behavior",
                "Limited system resource access",
            ],
            RiskLevel::High => vec![
                "Can access sensitive system resources",
                "May execute arbitrary commands",
                "Could affect system stability",
                "Potential for data loss or corruption",
            ],
            RiskLevel::Critical => vec![
                "Full system access capabilities",
                "Can execute privileged operations",
                "May compromise system security",
                "Potential for irreversible damage",
                "Could affect other users or systems",
            ],
        }
    }
}

/// Helper function to create approval requests for common operations
pub fn create_file_write_approval(path: &str, sandbox_policy: &SandboxPolicy) -> ApprovalRequest {
    let risk_level = if sandbox_policy.workspace_path().join(path).exists() {
        RiskLevel::Medium // Overwriting existing file
    } else {
        RiskLevel::Low // Creating new file
    };

    ApprovalRequest {
        operation: "File Write".to_string(),
        description: format!("Write to file: {}", path),
        risk_level,
        details: vec![
            format!("Target path: {}", path),
            format!("Sandbox level: {}", sandbox_policy.level()),
            format!("Workspace: {}", sandbox_policy.workspace_path().display()),
        ],
    }
}

/// Helper function to create approval requests for shell commands
pub fn create_shell_command_approval(command: &str) -> ApprovalRequest {
    let risk_level = classify_command_risk(command);

    ApprovalRequest {
        operation: "Shell Command Execution".to_string(),
        description: format!("Execute shell command: {}", command),
        risk_level,
        details: vec![
            format!("Command: {}", command),
            "This will execute arbitrary code on your system".to_string(),
            "Ensure you trust the source of this command".to_string(),
        ],
    }
}

/// Helper function to create approval requests for network access
pub fn create_network_access_approval(url: &str) -> ApprovalRequest {
    let risk_level = if url.starts_with("https://") {
        RiskLevel::Medium
    } else {
        RiskLevel::High // Non-HTTPS connections are riskier
    };

    ApprovalRequest {
        operation: "Network Access".to_string(),
        description: format!("Access network resource: {}", url),
        risk_level,
        details: vec![
            format!("URL: {}", url),
            "This will send data over the network".to_string(),
            "Ensure you trust the destination".to_string(),
        ],
    }
}

/// Classify the risk level of a shell command
fn classify_command_risk(command: &str) -> RiskLevel {
    let command_lower = command.to_lowercase();

    // Critical risk commands
    let critical_patterns = [
        "rm -rf",
        "del /s",
        "format",
        "mkfs",
        "dd if=",
        "fdisk",
        "parted",
        "sudo rm",
        "sudo dd",
        "chmod 777",
        "chown root",
        "passwd root",
        "userdel",
        "systemctl stop",
        "service stop",
        "shutdown",
        "reboot",
        "halt",
        "poweroff",
        "iptables -F",
        "ufw disable",
        "route delete",
    ];

    // High risk commands
    let high_patterns = [
        "sudo",
        "su -",
        "chmod",
        "chown",
        "passwd",
        "useradd",
        "usermod",
        "systemctl",
        "service",
        "mount",
        "umount",
        "crontab",
        "at ",
        "iptables",
        "ufw",
        "firewall",
        "netsh",
        "route add",
        "curl -X POST",
        "wget -O",
        "nc -l",
        "netcat -l",
        "ssh",
        "scp",
        "rsync",
    ];

    // Medium risk commands
    let medium_patterns = [
        "curl",
        "wget",
        "git clone",
        "npm install",
        "pip install",
        "apt install",
        "yum install",
        "brew install",
        "make install",
        "./configure",
        "make",
        "gcc",
        "g++",
        "rustc",
        "cargo build",
        "go build",
        "python -c",
    ];

    if critical_patterns
        .iter()
        .any(|&pattern| command_lower.contains(pattern))
    {
        RiskLevel::Critical
    } else if high_patterns
        .iter()
        .any(|&pattern| command_lower.contains(pattern))
    {
        RiskLevel::High
    } else if medium_patterns
        .iter()
        .any(|&pattern| command_lower.contains(pattern))
    {
        RiskLevel::Medium
    } else {
        RiskLevel::Low
    }
}

/// Process a command preview and check if approval is needed
pub fn check_command_approval(
    preview: &CommandPreview,
    sandbox_policy: &SandboxPolicy,
    approval_manager: &ApprovalManager,
) -> Result<ApprovalStatus> {
    if !preview.requires_approval {
        return Ok(ApprovalStatus::Approved);
    }

    // Create approval request based on the command actions
    let request = create_approval_request_from_preview(preview, sandbox_policy);
    approval_manager.request_approval(&request)
}

/// Create approval request from command preview
fn create_approval_request_from_preview(
    preview: &CommandPreview,
    sandbox_policy: &SandboxPolicy,
) -> ApprovalRequest {
    let mut details = vec![
        format!("Command ID: {}", preview.command_id),
        format!("Sandbox level: {}", sandbox_policy.level()),
    ];

    let mut risk_level = RiskLevel::Low;
    let mut operation_types = Vec::new();

    for action in &preview.actions {
        match action {
            PreviewAction::ReadFile { path } => {
                details.push(format!("Read file: {}", path));
                operation_types.push("File Reading");
            }
            PreviewAction::WriteFile { path, .. } => {
                details.push(format!("Write file: {}", path));
                operation_types.push("File Writing");
                risk_level = risk_level.max(RiskLevel::Medium);
            }
            PreviewAction::ExecuteShell { command } => {
                details.push(format!("Execute: {}", command));
                operation_types.push("Shell Execution");
                risk_level = risk_level.max(classify_command_risk(command));
            }
        }
    }

    let operation = if operation_types.is_empty() {
        "Unknown Operation".to_string()
    } else {
        operation_types.join(", ")
    };

    ApprovalRequest {
        operation,
        description: preview.description.clone(),
        risk_level,
        details,
    }
}

/// Extension trait for RiskLevel to support max comparison
pub trait RiskLevelExt {
    fn max(self, other: Self) -> Self;
}

impl RiskLevelExt for RiskLevel {
    fn max(self, other: Self) -> Self {
        use RiskLevel::*;
        match (self, other) {
            (Critical, _) | (_, Critical) => Critical,
            (High, _) | (_, High) => High,
            (Medium, _) | (_, Medium) => Medium,
            (Low, Low) => Low,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_approval_status_values() {
        assert!(matches!(ApprovalStatus::Pending, ApprovalStatus::Pending));
        assert!(matches!(ApprovalStatus::Approved, ApprovalStatus::Approved));
        assert!(matches!(ApprovalStatus::Denied, ApprovalStatus::Denied));
        assert!(matches!(ApprovalStatus::TimedOut, ApprovalStatus::TimedOut));
    }

    #[test]
    fn test_approval_status_eq() {
        assert_eq!(ApprovalStatus::Pending, ApprovalStatus::Pending);
        assert_eq!(ApprovalStatus::Approved, ApprovalStatus::Approved);
        assert_ne!(ApprovalStatus::Approved, ApprovalStatus::Denied);
    }

    #[test]
    fn test_risk_level_display() {
        assert_eq!(RiskLevel::Low.to_string(), "LOW");
        assert_eq!(RiskLevel::Medium.to_string(), "MEDIUM");
        assert_eq!(RiskLevel::High.to_string(), "HIGH");
        assert_eq!(RiskLevel::Critical.to_string(), "CRITICAL");
    }

    #[test]
    fn test_risk_level_eq() {
        assert_eq!(RiskLevel::Low, RiskLevel::Low);
        assert_eq!(RiskLevel::High, RiskLevel::High);
        assert_ne!(RiskLevel::Low, RiskLevel::High);
    }

    #[test]
    fn test_risk_level_max() {
        assert_eq!(RiskLevel::Low.max(RiskLevel::Medium), RiskLevel::Medium);
        assert_eq!(RiskLevel::High.max(RiskLevel::Low), RiskLevel::High);
        assert_eq!(
            RiskLevel::Critical.max(RiskLevel::High),
            RiskLevel::Critical
        );
        assert_eq!(RiskLevel::Medium.max(RiskLevel::Medium), RiskLevel::Medium);
    }

    #[test]
    fn test_approval_manager_default() {
        let manager = ApprovalManager::default();
        assert!(!manager.auto_approve_low_risk);
        assert!(manager.interactive_mode);
    }

    #[test]
    fn test_approval_manager_new() {
        let manager = ApprovalManager::new(true, false);
        assert!(manager.auto_approve_low_risk);
        assert!(!manager.interactive_mode);
    }

    #[test]
    fn test_auto_approve_low_risk() {
        let manager = ApprovalManager::new(true, true);
        let request = ApprovalRequest {
            operation: "Test".to_string(),
            description: "Test operation".to_string(),
            risk_level: RiskLevel::Low,
            details: vec![],
        };

        let status = manager.request_approval(&request).unwrap();
        assert_eq!(status, ApprovalStatus::Approved);
    }

    #[test]
    fn test_non_interactive_denies() {
        let manager = ApprovalManager::new(false, false);
        let request = ApprovalRequest {
            operation: "Test".to_string(),
            description: "Test operation".to_string(),
            risk_level: RiskLevel::Medium,
            details: vec![],
        };

        let status = manager.request_approval(&request).unwrap();
        assert_eq!(status, ApprovalStatus::Denied);
    }

    #[test]
    fn test_risk_level_emoji() {
        let manager = ApprovalManager::default();
        assert_eq!(manager.risk_level_emoji(&RiskLevel::Low), "🟢");
        assert_eq!(manager.risk_level_emoji(&RiskLevel::Medium), "🟡");
        assert_eq!(manager.risk_level_emoji(&RiskLevel::High), "🟠");
        assert_eq!(manager.risk_level_emoji(&RiskLevel::Critical), "🔴");
    }

    #[test]
    fn test_risk_level_description() {
        let manager = ApprovalManager::default();
        assert_eq!(
            manager.risk_level_description(&RiskLevel::Low),
            "Minimal security impact"
        );
        assert_eq!(
            manager.risk_level_description(&RiskLevel::Medium),
            "Moderate security considerations"
        );
        assert_eq!(
            manager.risk_level_description(&RiskLevel::High),
            "Significant security implications"
        );
        assert_eq!(
            manager.risk_level_description(&RiskLevel::Critical),
            "Severe security risks"
        );
    }

    #[test]
    fn test_get_risk_warning() {
        let manager = ApprovalManager::default();
        assert!(manager
            .get_risk_warning(&RiskLevel::Low)
            .contains("minimal"));
        assert!(manager
            .get_risk_warning(&RiskLevel::Medium)
            .contains("security"));
        assert!(manager
            .get_risk_warning(&RiskLevel::High)
            .contains("WARNING"));
        assert!(manager
            .get_risk_warning(&RiskLevel::Critical)
            .contains("DANGER"));
    }

    #[test]
    fn test_get_security_implications() {
        let manager = ApprovalManager::default();

        let low_impl = manager.get_security_implications(&RiskLevel::Low);
        assert!(!low_impl.is_empty());
        assert!(low_impl.iter().any(|s| s.contains("Limited")));

        let critical_impl = manager.get_security_implications(&RiskLevel::Critical);
        assert!(!critical_impl.is_empty());
        assert!(critical_impl
            .iter()
            .any(|s| s.contains("Full system access")));
    }

    #[test]
    fn test_classify_command_risk_critical() {
        assert_eq!(classify_command_risk("rm -rf /"), RiskLevel::Critical);
        assert_eq!(
            classify_command_risk("sudo dd if=/dev/zero of=/dev/sda"),
            RiskLevel::Critical
        );
        assert_eq!(
            classify_command_risk("chmod 777 /etc/passwd"),
            RiskLevel::Critical
        );
        assert_eq!(
            classify_command_risk("shutdown -h now"),
            RiskLevel::Critical
        );
    }

    #[test]
    fn test_classify_command_risk_high() {
        assert_eq!(classify_command_risk("sudo apt update"), RiskLevel::High);
        assert_eq!(classify_command_risk("chmod +x script.sh"), RiskLevel::High);
        assert_eq!(
            classify_command_risk("systemctl restart nginx"),
            RiskLevel::High
        );
        assert_eq!(classify_command_risk("ssh user@server"), RiskLevel::High);
    }

    #[test]
    fn test_classify_command_risk_medium() {
        assert_eq!(
            classify_command_risk("curl https://example.com"),
            RiskLevel::Medium
        );
        assert_eq!(
            classify_command_risk("npm install package"),
            RiskLevel::Medium
        );
        assert_eq!(classify_command_risk("git clone repo"), RiskLevel::Medium);
        assert_eq!(
            classify_command_risk("cargo build --release"),
            RiskLevel::Medium
        );
    }

    #[test]
    fn test_classify_command_risk_low() {
        assert_eq!(classify_command_risk("ls -la"), RiskLevel::Low);
        assert_eq!(classify_command_risk("echo hello"), RiskLevel::Low);
        assert_eq!(classify_command_risk("pwd"), RiskLevel::Low);
        assert_eq!(classify_command_risk("cd /tmp"), RiskLevel::Low);
    }

    #[test]
    fn test_create_shell_command_approval() {
        let request = create_shell_command_approval("rm -rf /tmp/test");
        assert_eq!(request.operation, "Shell Command Execution");
        assert!(request.description.contains("rm -rf"));
        assert_eq!(request.risk_level, RiskLevel::Critical);
        assert!(!request.details.is_empty());
    }

    #[test]
    fn test_create_network_access_approval_https() {
        let request = create_network_access_approval("https://example.com");
        assert_eq!(request.operation, "Network Access");
        assert_eq!(request.risk_level, RiskLevel::Medium);
        assert!(request.description.contains("https://example.com"));
    }

    #[test]
    fn test_create_network_access_approval_http() {
        let request = create_network_access_approval("http://example.com");
        assert_eq!(request.operation, "Network Access");
        assert_eq!(request.risk_level, RiskLevel::High);
    }

    #[test]
    fn test_approval_request_clone() {
        let request = ApprovalRequest {
            operation: "Test".to_string(),
            description: "Test desc".to_string(),
            risk_level: RiskLevel::Low,
            details: vec!["detail1".to_string()],
        };

        let cloned = request.clone();
        assert_eq!(cloned.operation, request.operation);
        assert_eq!(cloned.risk_level, request.risk_level);
    }

    #[test]
    fn test_approval_request_serialization() {
        let request = ApprovalRequest {
            operation: "Test".to_string(),
            description: "Test desc".to_string(),
            risk_level: RiskLevel::Medium,
            details: vec!["detail1".to_string()],
        };

        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: ApprovalRequest = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.operation, request.operation);
        assert_eq!(deserialized.risk_level, request.risk_level);
    }

    #[test]
    fn test_approval_status_serialization() {
        let status = ApprovalStatus::Approved;
        let serialized = serde_json::to_string(&status).unwrap();
        let deserialized: ApprovalStatus = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, ApprovalStatus::Approved);
    }

    #[test]
    fn test_risk_level_serialization() {
        let risk = RiskLevel::High;
        let serialized = serde_json::to_string(&risk).unwrap();
        let deserialized: RiskLevel = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, RiskLevel::High);
    }

    #[test]
    fn test_check_command_approval_not_required() {
        use fennec_core::command::CommandPreview;
        use uuid::Uuid;

        let preview = CommandPreview {
            command_id: Uuid::new_v4(),
            description: "Test".to_string(),
            actions: vec![],
            requires_approval: false,
        };

        let sandbox =
            SandboxPolicy::new(crate::SandboxLevel::ReadOnly, PathBuf::from("/tmp"), false);
        let manager = ApprovalManager::new(false, false);

        let status = check_command_approval(&preview, &sandbox, &manager).unwrap();
        assert_eq!(status, ApprovalStatus::Approved);
    }

    #[test]
    fn test_classify_command_risk_case_insensitive() {
        assert_eq!(classify_command_risk("RM -RF /"), RiskLevel::Critical);
        assert_eq!(classify_command_risk("SUDO apt update"), RiskLevel::High);
        assert_eq!(
            classify_command_risk("CURL http://example.com"),
            RiskLevel::Medium
        );
    }

    #[test]
    fn test_risk_level_debug() {
        let risk = RiskLevel::Critical;
        let debug = format!("{:?}", risk);
        assert!(debug.contains("Critical"));
    }

    #[test]
    fn test_approval_status_debug() {
        let status = ApprovalStatus::Approved;
        let debug = format!("{:?}", status);
        assert!(debug.contains("Approved"));
    }

    #[test]
    fn test_approval_status_clone() {
        let status = ApprovalStatus::Pending;
        let cloned = status.clone();
        assert_eq!(cloned, ApprovalStatus::Pending);
    }

    #[test]
    fn test_risk_level_clone() {
        let risk = RiskLevel::Medium;
        let cloned = risk.clone();
        assert_eq!(cloned, RiskLevel::Medium);
    }
}
