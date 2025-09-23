# Fennec Security Model

## Three-Tier Sandbox Architecture

Fennec implements a **three-tier security model** designed to provide enterprise-grade security while maintaining developer productivity.

### Sandbox Levels

#### 1. Read-Only (`read-only`)
**Purpose**: Safe exploration and analysis
**Capabilities**:
- ✅ File reading and code analysis
- ✅ Repository browsing and git history
- ✅ Static analysis and documentation generation
- ❌ No write operations
- ❌ No command execution
- ❌ No network access (except LLM provider)

**Use Cases**:
- Code exploration and learning
- Security audits and code review
- Documentation generation
- Safe onboarding for new team members

#### 2. Workspace-Write (`workspace-write`) - Default
**Purpose**: Standard development workflow within project boundaries
**Capabilities**:
- ✅ Read/write access within project workspace
- ✅ Limited shell command execution (safe commands only)
- ✅ File editing and creation within workspace
- ✅ Git operations within workspace
- ❌ System-wide file access
- ❌ Network access beyond workspace tools
- ❌ Dangerous command execution

**Use Cases**:
- Daily development workflow
- Code editing and refactoring
- Running tests and builds
- Git operations and version control

#### 3. Danger-Full-Access (`danger-full-access`)
**Purpose**: Full system access for advanced operations
**Capabilities**:
- ✅ Full system access with all capabilities
- ✅ Unrestricted command execution
- ✅ Network access and external tool integration
- ✅ System configuration changes
- ⚠️ Requires explicit approval for dangerous operations
- ⚠️ Enhanced audit logging

**Use Cases**:
- System administration and DevOps
- CI/CD setup and deployment
- Package installation and system updates
- Infrastructure management

## Security Components

### Path Traversal Protection
**Implementation**: `fennec-security` crate
**Features**:
- Canonical path resolution to prevent `../` attacks
- Workspace boundary enforcement
- Symlink validation and resolution
- Hidden file and directory protection

**Example Protections**:
```rust
// Blocked attempts
/etc/passwd              // System file access
../../../secret.key      // Path traversal
~/.ssh/id_rsa           // Home directory access
/proc/meminfo           // System information
```

### Command Filtering & Risk Assessment
**Risk Classifications**:
- **Safe**: `ls`, `cat`, `grep`, `git status`
- **Moderate**: `cargo build`, `npm install`, `git commit`
- **Dangerous**: `rm -rf`, `sudo`, `curl | sh`, `dd`

**Filtering Logic**:
```rust
pub enum CommandRisk {
    Safe,       // Auto-approved
    Moderate,   // Contextual approval
    Dangerous,  // Always requires approval
    Blocked,    // Never allowed
}
```

### Approval Workflows
**Approval Types**:
1. **Automatic** - Safe commands in appropriate sandbox
2. **Contextual** - Based on command, arguments, and current context
3. **Interactive** - User prompt with risk explanation
4. **Blocked** - Commands that are never allowed

**Approval UI**:
```
⚠️  APPROVAL REQUIRED
Command: rm src/sensitive.rs
Risk Level: MODERATE
Reason: File deletion outside undo scope

Preview what will happen:
• Delete file: src/sensitive.rs (1,247 lines)
• Cannot be undone without git

[A]pprove  [D]eny  [V]iew File  [?]Help
```

### Capability-Based Permissions
**Capability System**:
```rust
pub struct Capabilities {
    pub read_files: bool,
    pub write_files: bool,
    pub execute_commands: bool,
    pub network_access: bool,
    pub git_operations: bool,
}
```

**Command Declarations**:
```rust
impl Command for EditCommand {
    fn required_capabilities() -> Capabilities {
        Capabilities {
            read_files: true,
            write_files: true,
            execute_commands: false,
            network_access: false,
            git_operations: false,
        }
    }
}
```

### Comprehensive Audit System
**Audit Trail Structure**:
```json
{
  "timestamp": "2025-09-23T10:30:00Z",
  "session_id": "uuid-here",
  "command": "edit",
  "sandbox_level": "workspace-write",
  "approval_status": "approved",
  "risk_level": "moderate",
  "files_accessed": ["src/main.rs"],
  "commands_executed": [],
  "outcome": "success",
  "user_id": "developer@company.com"
}
```

**Audit Features**:
- **Complete traceability** of all privileged operations
- **Command-level tracking** with approval status and outcomes
- **Security event logging** with risk classification
- **Session management** with pause/resume capabilities
- **JSON format** for easy parsing and analysis

## Security Best Practices

### Credential Management
- **Never hardcode** API keys or secrets in code
- **Environment variables** for development (.env files)
- **OS keyring integration** for production deployments
- **Credential rotation** support and documentation

### Input Validation
- **Sanitize all inputs** before processing
- **Path validation** with canonical resolution
- **Command argument filtering** and validation
- **Content type validation** for file operations

### Error Handling
- **Fail securely** - errors don't leak sensitive information
- **Detailed logging** for debugging without PII exposure
- **Graceful degradation** when security constraints are hit
- **Clear error messages** for users and administrators

### Network Security
- **TLS enforcement** for all external communications
- **Certificate validation** for LLM provider connections
- **Rate limiting** to prevent abuse and DoS
- **Request/response sanitization** to prevent injection

## Integration with Development Workflow

### Git Integration
- **Sandbox-aware** git operations
- **Workspace boundary** enforcement for git commands
- **Commit signing** support and validation
- **Branch protection** and merge policies

### CI/CD Security
- **Secrets management** in pipeline integration
- **Audit trail** preservation in automated environments
- **Role-based access** for different pipeline stages
- **Compliance reporting** for security teams

### Enterprise Features
- **RBAC integration** with existing identity systems
- **Policy enforcement** at organizational level
- **Compliance reporting** (SOX, HIPAA, etc.)
- **Security monitoring** and alerting integration

## Configuration Examples

### Secure Development Team
```toml
[security]
default_sandbox = "workspace-write"
require_approval = true
audit_logging = true
allowed_commands = ["cargo", "git", "npm", "yarn"]
blocked_patterns = ["rm -rf", "sudo", "curl.*sh"]
```

### DevOps Team
```toml
[security]
default_sandbox = "danger-full-access"
require_approval = false  # For experienced DevOps
audit_logging = true
elevated_commands = ["docker", "kubectl", "terraform"]
approval_timeout = 300    # 5 minutes
```

### Security Team
```toml
[security]
default_sandbox = "read-only"
require_approval = true
audit_logging = true
audit_retention_days = 2555  # 7 years
compliance_mode = "sox"      # SOX compliance
```

---

*This security model provides enterprise-grade protection while maintaining developer productivity and workflow integration.*