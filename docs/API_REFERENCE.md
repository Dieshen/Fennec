# Fennec API Reference

Complete API documentation for Fennec AI Assistant, covering all public interfaces, types, and extension points.

## Table of Contents

1. [Core Types](#core-types)
2. [Command System](#command-system)
3. [Provider Interface](#provider-interface)
4. [Memory System](#memory-system)
5. [Security System](#security-system)
6. [Configuration](#configuration)
7. [TUI Components](#tui-components)
8. [Extension Points](#extension-points)

## Core Types

### Result and Error Types

```rust
// fennec-core/src/error.rs
pub type Result<T> = std::result::Result<T, FennecError>;

#[derive(thiserror::Error, Debug)]
pub enum FennecError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Command execution failed: {0}")]
    Command(String),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Security violation: {0}")]
    Security(String),

    #[error("Memory system error: {0}")]
    Memory(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
```

### Session Types

```rust
// fennec-core/src/session.rs
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionId(pub Uuid);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub transcript: Vec<Message>,
    pub context: SessionContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    pub workspace_path: Option<PathBuf>,
    pub sandbox_level: SandboxLevel,
    pub memory_context: MemoryContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub role: MessageRole,
    pub content: String,
    pub metadata: MessageMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}
```

### Provider Types

```rust
// fennec-core/src/provider.rs
use async_trait::async_trait;

#[async_trait]
pub trait ProviderClient: Send + Sync {
    async fn stream_completion(
        &self,
        request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<CompletionChunk>>>>>;

    async fn get_models(&self) -> Result<Vec<ModelInfo>>;

    fn provider_name(&self) -> &str;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionChunk {
    pub content: Option<String>,
    pub finish_reason: Option<FinishReason>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FinishReason {
    Stop,
    Length,
    ContentFilter,
    ToolCalls,
}
```

## Command System

### Command Registration

```rust
// fennec-commands/src/registry.rs
use async_trait::async_trait;

#[async_trait]
pub trait CommandExecutor: Send + Sync {
    async fn execute(
        &self,
        args: &serde_json::Value,
        context: &CommandContext,
    ) -> Result<CommandExecutionResult>;

    fn descriptor(&self) -> CommandDescriptor;
}

#[derive(Debug, Clone)]
pub struct CommandDescriptor {
    pub name: String,
    pub description: String,
    pub capabilities: Vec<Capability>,
    pub requires_approval: bool,
    pub preview_mode: PreviewMode,
}

#[derive(Debug, Clone)]
pub struct CommandContext {
    pub session_id: SessionId,
    pub user_id: Option<String>,
    pub workspace_path: Option<PathBuf>,
    pub sandbox_level: SandboxLevel,
    pub dry_run: bool,
    pub preview_only: bool,
    pub cancellation_token: CancellationToken,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecutionResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub preview: Option<CommandPreview>,
    pub metadata: CommandMetadata,
}
```

### Built-in Commands

#### Plan Command

```rust
// fennec-commands/src/plan.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanArgs {
    pub task: String,
    pub complexity: Option<Complexity>,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Complexity {
    Simple,
    Moderate,
    Complex,
    Expert,
}

pub struct PlanCommand;

#[async_trait]
impl CommandExecutor for PlanCommand {
    async fn execute(
        &self,
        args: &serde_json::Value,
        context: &CommandContext,
    ) -> Result<CommandExecutionResult> {
        let plan_args: PlanArgs = serde_json::from_value(args.clone())?;

        // Generate implementation plan using LLM provider
        let plan = self.generate_plan(&plan_args, context).await?;

        Ok(CommandExecutionResult {
            success: true,
            output: plan.to_string(),
            error: None,
            preview: Some(plan.into()),
            metadata: CommandMetadata::default(),
        })
    }

    fn descriptor(&self) -> CommandDescriptor {
        CommandDescriptor {
            name: "plan".to_string(),
            description: "Generate structured implementation plans".to_string(),
            capabilities: vec![Capability::ReadFile, Capability::ProviderAccess],
            requires_approval: false,
            preview_mode: PreviewMode::Always,
        }
    }
}
```

#### Edit Command

```rust
// fennec-commands/src/edit.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditArgs {
    pub file_path: String,
    pub instruction: String,
    pub backup: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditResult {
    pub file_path: String,
    pub diff: String,
    pub backup_path: Option<String>,
    pub changes_applied: bool,
}

pub struct EditCommand {
    file_operations: Arc<FileOperations>,
}

#[async_trait]
impl CommandExecutor for EditCommand {
    async fn execute(
        &self,
        args: &serde_json::Value,
        context: &CommandContext,
    ) -> Result<CommandExecutionResult> {
        let edit_args: EditArgs = serde_json::from_value(args.clone())?;

        // Generate file edit using LLM
        let edit_request = self.prepare_edit_request(&edit_args, context).await?;

        // Apply edit with preview
        let result = self.file_operations
            .apply_edit(edit_request, context.dry_run)
            .await?;

        Ok(CommandExecutionResult {
            success: result.success,
            output: serde_json::to_string_pretty(&result)?,
            error: result.error,
            preview: Some(result.preview),
            metadata: CommandMetadata::from_edit(&result),
        })
    }

    fn descriptor(&self) -> CommandDescriptor {
        CommandDescriptor {
            name: "edit".to_string(),
            description: "Make precise file edits with diff previews".to_string(),
            capabilities: vec![
                Capability::ReadFile,
                Capability::WriteFile,
                Capability::ProviderAccess,
            ],
            requires_approval: true,
            preview_mode: PreviewMode::Always,
        }
    }
}
```

#### Run Command

```rust
// fennec-commands/src/run.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunArgs {
    pub command: String,
    pub working_directory: Option<String>,
    pub environment: Option<HashMap<String, String>>,
    pub timeout: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration: Duration,
}

pub struct RunCommand;

#[async_trait]
impl CommandExecutor for RunCommand {
    async fn execute(
        &self,
        args: &serde_json::Value,
        context: &CommandContext,
    ) -> Result<CommandExecutionResult> {
        let run_args: RunArgs = serde_json::from_value(args.clone())?;

        // Security validation
        self.validate_command(&run_args.command, context)?;

        // Execute command with timeout
        let result = self.execute_shell_command(&run_args, context).await?;

        Ok(CommandExecutionResult {
            success: result.exit_code == 0,
            output: result.stdout,
            error: if result.exit_code != 0 { Some(result.stderr) } else { None },
            preview: None,
            metadata: CommandMetadata::from_run(&result),
        })
    }

    fn descriptor(&self) -> CommandDescriptor {
        CommandDescriptor {
            name: "run".to_string(),
            description: "Execute shell commands safely".to_string(),
            capabilities: vec![Capability::ExecuteShell],
            requires_approval: true,
            preview_mode: PreviewMode::OnRequest,
        }
    }
}
```

### File Operations

```rust
// fennec-commands/src/file_ops.rs
pub struct FileOperations {
    config: FileOperationsConfig,
}

impl FileOperations {
    pub fn new(config: FileOperationsConfig) -> Self {
        Self { config }
    }

    pub async fn read_file(&self, path: &Path) -> Result<String> {
        // Validate path is within sandbox
        self.validate_read_path(path)?;

        tokio::fs::read_to_string(path)
            .await
            .map_err(FennecError::from)
    }

    pub async fn write_file(
        &self,
        path: &Path,
        content: &str,
        create_backup: bool,
    ) -> Result<FileWriteResult> {
        // Validate path is within sandbox
        self.validate_write_path(path)?;

        // Create backup if requested
        let backup_path = if create_backup && path.exists() {
            Some(self.create_backup(path).await?)
        } else {
            None
        };

        // Write file atomically
        let temp_path = path.with_extension("tmp");
        tokio::fs::write(&temp_path, content).await?;
        tokio::fs::rename(&temp_path, path).await?;

        Ok(FileWriteResult {
            path: path.to_path_buf(),
            backup_path,
            bytes_written: content.len(),
        })
    }

    pub async fn generate_diff(
        &self,
        original: &str,
        modified: &str,
        context_lines: usize,
    ) -> Result<String> {
        use similar::{ChangeTag, TextDiff};

        let diff = TextDiff::from_lines(original, modified);
        let mut result = String::new();

        for (idx, group) in diff.grouped_ops(context_lines).iter().enumerate() {
            if idx > 0 {
                result.push_str("@@ ... @@\n");
            }

            for op in group {
                for change in diff.iter_inline_changes(op) {
                    let (sign, s) = match change.tag() {
                        ChangeTag::Delete => ("-", "removed"),
                        ChangeTag::Insert => ("+", "added"),
                        ChangeTag::Equal => (" ", "context"),
                    };

                    result.push_str(&format!("{}{}", sign, change));
                }
            }
        }

        Ok(result)
    }
}
```

## Provider Interface

### OpenAI Provider Implementation

```rust
// fennec-provider/src/openai.rs
pub struct OpenAIProvider {
    client: reqwest::Client,
    config: OpenAIConfig,
}

impl OpenAIProvider {
    pub fn new(config: OpenAIConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout))
            .build()?;

        Ok(Self { client, config })
    }
}

#[async_trait]
impl ProviderClient for OpenAIProvider {
    async fn stream_completion(
        &self,
        request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<CompletionChunk>>>>> {
        let url = format!("{}/chat/completions", self.config.base_url);

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(FennecError::Provider(
                format!("API request failed: {}", response.status())
            ));
        }

        let stream = response
            .bytes_stream()
            .map_err(|e| FennecError::Provider(e.to_string()))
            .and_then(|chunk| async move {
                self.parse_streaming_chunk(&chunk).await
            });

        Ok(Box::pin(stream))
    }

    async fn get_models(&self) -> Result<Vec<ModelInfo>> {
        let url = format!("{}/models", self.config.base_url);

        let response: ModelsResponse = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .send()
            .await?
            .json()
            .await?;

        Ok(response.data)
    }

    fn provider_name(&self) -> &str {
        "openai"
    }
}

#[derive(Debug, Clone)]
pub struct OpenAIConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub timeout: u64,
}
```

### Creating Custom Providers

```rust
// Example custom provider implementation
pub struct CustomProvider {
    // Custom provider state
}

#[async_trait]
impl ProviderClient for CustomProvider {
    async fn stream_completion(
        &self,
        request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<CompletionChunk>>>>> {
        // Implement streaming completion logic
        todo!("Implement custom provider streaming")
    }

    async fn get_models(&self) -> Result<Vec<ModelInfo>> {
        // Return available models
        todo!("Implement model listing")
    }

    fn provider_name(&self) -> &str {
        "custom"
    }
}

// Register custom provider
pub fn register_custom_provider(registry: &mut ProviderRegistry) -> Result<()> {
    let provider = CustomProvider::new()?;
    registry.register("custom", Box::new(provider))?;
    Ok(())
}
```

## Memory System

### Memory Service Interface

```rust
// fennec-memory/src/lib.rs
#[async_trait]
pub trait MemoryService: Send + Sync {
    async fn store_session(&self, session: &Session) -> Result<()>;

    async fn load_session(&self, session_id: &SessionId) -> Result<Option<Session>>;

    async fn search_context(
        &self,
        query: &str,
        max_results: usize,
    ) -> Result<Vec<MemoryEntry>>;

    async fn update_memory_files(
        &self,
        updates: HashMap<String, String>,
    ) -> Result<()>;

    async fn get_project_context(&self) -> Result<ProjectContext>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub source: MemorySource,
    pub content: String,
    pub relevance_score: f32,
    pub timestamp: DateTime<Utc>,
    pub metadata: MemoryMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemorySource {
    AgentsMd,
    ClaudeMemoryBank,
    SessionTranscript,
    GitHistory,
    ProjectFile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    pub workspace_path: PathBuf,
    pub agents_md: Option<String>,
    pub project_brief: Option<String>,
    pub active_context: Option<String>,
    pub git_info: Option<GitInfo>,
}
```

### Memory Adapters

```rust
// fennec-memory/src/adapters/agents_md.rs
pub struct AgentsMdAdapter {
    workspace_path: PathBuf,
}

impl AgentsMdAdapter {
    pub fn new(workspace_path: PathBuf) -> Self {
        Self { workspace_path }
    }

    pub async fn load_agents_md(&self) -> Result<Option<String>> {
        let agents_path = self.workspace_path.join("AGENTS.md");

        if agents_path.exists() {
            let content = tokio::fs::read_to_string(&agents_path).await?;
            Ok(Some(content))
        } else {
            Ok(None)
        }
    }

    pub async fn parse_guidelines(&self, content: &str) -> Result<ProjectGuidelines> {
        // Parse AGENTS.md content into structured guidelines
        let guidelines = ProjectGuidelines::from_markdown(content)?;
        Ok(guidelines)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectGuidelines {
    pub coding_standards: Vec<CodingStandard>,
    pub architecture_decisions: Vec<ArchitectureDecision>,
    pub security_requirements: Vec<SecurityRequirement>,
    pub development_workflow: Option<String>,
}
```

## Security System

### Sandbox Policies

```rust
// fennec-security/src/sandbox.rs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SandboxLevel {
    ReadOnly,
    WorkspaceWrite,
    FullAccess,
}

#[derive(Debug, Clone)]
pub struct SandboxPolicy {
    level: SandboxLevel,
    workspace_path: PathBuf,
    requires_approval: bool,
}

impl SandboxPolicy {
    pub fn new(
        level: SandboxLevel,
        workspace_path: PathBuf,
        requires_approval: bool,
    ) -> Self {
        Self {
            level,
            workspace_path,
            requires_approval,
        }
    }

    pub fn check_capability(&self, capability: &Capability) -> PolicyResult {
        match (&self.level, capability) {
            (SandboxLevel::ReadOnly, Capability::ReadFile) => PolicyResult::Allow,
            (SandboxLevel::ReadOnly, _) => PolicyResult::Deny(
                "Operation not allowed in read-only mode".to_string()
            ),

            (SandboxLevel::WorkspaceWrite, Capability::ReadFile) => PolicyResult::Allow,
            (SandboxLevel::WorkspaceWrite, Capability::WriteFile) => {
                if self.requires_approval {
                    PolicyResult::RequireApproval("File write requires approval".to_string())
                } else {
                    PolicyResult::Allow
                }
            },
            (SandboxLevel::WorkspaceWrite, Capability::ExecuteShell) => {
                PolicyResult::Deny("Shell execution not allowed in workspace-write mode".to_string())
            },

            (SandboxLevel::FullAccess, _) => {
                if self.requires_approval {
                    PolicyResult::RequireApproval("Operation requires approval".to_string())
                } else {
                    PolicyResult::Allow
                }
            },
        }
    }

    pub fn check_read_path(&self, path: &Path) -> PolicyResult {
        if self.is_path_allowed(path) {
            PolicyResult::Allow
        } else {
            PolicyResult::Deny(format!(
                "Path '{}' is outside workspace boundaries",
                path.display()
            ))
        }
    }

    pub fn check_write_path(&self, path: &Path) -> PolicyResult {
        match self.level {
            SandboxLevel::ReadOnly => PolicyResult::Deny(
                "Write operations not allowed in read-only mode".to_string()
            ),
            _ => {
                if self.is_path_allowed(path) {
                    if self.requires_approval {
                        PolicyResult::RequireApproval(format!(
                            "Write to '{}' requires approval",
                            path.display()
                        ))
                    } else {
                        PolicyResult::Allow
                    }
                } else {
                    PolicyResult::Deny(format!(
                        "Path '{}' is outside workspace boundaries",
                        path.display()
                    ))
                }
            }
        }
    }

    fn is_path_allowed(&self, path: &Path) -> bool {
        // Canonicalize paths to prevent traversal attacks
        let canonical_path = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                // If path doesn't exist, check parent directory
                let parent = path.parent().unwrap_or(Path::new("."));
                match parent.canonicalize() {
                    Ok(p) => p.join(path.file_name().unwrap_or_default()),
                    Err(_) => return false,
                }
            }
        };

        canonical_path.starts_with(&self.workspace_path)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PolicyResult {
    Allow,
    Deny(String),
    RequireApproval(String),
}
```

### Approval System

```rust
// fennec-security/src/approval.rs
pub struct ApprovalManager {
    auto_approve_low_risk: bool,
    interactive_mode: bool,
}

impl ApprovalManager {
    pub fn new(auto_approve_low_risk: bool, interactive_mode: bool) -> Self {
        Self {
            auto_approve_low_risk,
            interactive_mode,
        }
    }

    pub fn request_approval(&self, request: &ApprovalRequest) -> Result<ApprovalStatus> {
        match request.risk_level {
            RiskLevel::Low if self.auto_approve_low_risk => {
                Ok(ApprovalStatus::Approved)
            },
            _ if self.interactive_mode => {
                self.prompt_user_approval(request)
            },
            _ => {
                // Non-interactive mode denies by default
                Ok(ApprovalStatus::Denied)
            }
        }
    }

    fn prompt_user_approval(&self, request: &ApprovalRequest) -> Result<ApprovalStatus> {
        // In actual implementation, this would interact with the TUI
        // For API documentation, we show the structure
        println!("Approval Required:");
        println!("Operation: {}", request.operation);
        println!("Risk Level: {:?}", request.risk_level);
        println!("Description: {}", request.description);

        for detail in &request.details {
            println!("  - {}", detail);
        }

        // Interactive prompt would go here
        Ok(ApprovalStatus::Approved) // Placeholder
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub operation: String,
    pub description: String,
    pub risk_level: RiskLevel,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ApprovalStatus {
    Approved,
    Denied,
    Pending,
}
```

### Audit System

```rust
// fennec-security/src/audit.rs
pub struct AuditLogger {
    log_path: PathBuf,
    session_id: SessionId,
}

impl AuditLogger {
    pub async fn new(config: &Config) -> Result<Self> {
        let log_path = config.audit_log_path.clone();
        let session_id = SessionId::new();

        // Ensure log directory exists
        if let Some(parent) = log_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        Ok(Self { log_path, session_id })
    }

    pub async fn log_event(&self, event: AuditEvent) -> Result<()> {
        let event_data = AuditEventData {
            timestamp: Utc::now(),
            session_id: self.session_id.clone(),
            event,
        };

        let log_line = serde_json::to_string(&event_data)?;

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .await?;

        use tokio::io::AsyncWriteExt;
        file.write_all(log_line.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;

        Ok(())
    }

    pub async fn log_command_execution(
        &self,
        command: &str,
        args: &serde_json::Value,
        result: &CommandExecutionResult,
        approval_status: Option<ApprovalStatus>,
    ) -> Result<()> {
        let event = AuditEvent::CommandExecution {
            command: command.to_string(),
            args: args.clone(),
            success: result.success,
            approval_status,
            capabilities_used: vec![], // Would be populated from command descriptor
        };

        self.log_event(event).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEventData {
    pub timestamp: DateTime<Utc>,
    pub session_id: SessionId,
    pub event: AuditEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuditEvent {
    SessionStart {
        workspace_path: Option<PathBuf>,
        sandbox_level: SandboxLevel,
    },
    SessionEnd {
        duration: Duration,
        commands_executed: u32,
    },
    CommandExecution {
        command: String,
        args: serde_json::Value,
        success: bool,
        approval_status: Option<ApprovalStatus>,
        capabilities_used: Vec<Capability>,
    },
    FileOperation {
        operation: FileOperation,
        path: PathBuf,
        success: bool,
    },
    SecurityViolation {
        attempted_operation: String,
        violation_type: String,
        blocked: bool,
    },
}
```

## Configuration

### Configuration Types

```rust
// fennec-core/src/config.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub provider: ProviderConfig,
    pub security: SecurityConfig,
    pub memory: MemoryConfig,
    pub ui: UiConfig,
    pub audit: AuditConfig,
}

impl Config {
    pub async fn load(config_path: Option<&Path>) -> Result<Self> {
        let config_content = if let Some(path) = config_path {
            tokio::fs::read_to_string(path).await?
        } else {
            // Load from default locations
            Self::load_from_default_locations().await?
        };

        let mut config: Config = toml::from_str(&config_content)?;

        // Substitute environment variables
        config.substitute_env_vars()?;

        Ok(config)
    }

    fn substitute_env_vars(&mut self) -> Result<()> {
        // Substitute ${VAR} patterns in configuration
        if let Some(api_key) = &mut self.provider.openai.api_key {
            *api_key = Self::substitute_env_var(api_key)?;
        }

        Ok(())
    }

    fn substitute_env_var(value: &str) -> Result<String> {
        if value.starts_with("${") && value.ends_with("}") {
            let var_name = &value[2..value.len()-1];
            std::env::var(var_name)
                .map_err(|_| FennecError::Config(
                    format!("Environment variable '{}' not found", var_name)
                ))
        } else {
            Ok(value.to_string())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub default: String,
    pub openai: OpenAIProviderConfig,
    // Future provider configs...
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIProviderConfig {
    pub api_key: Option<String>,
    pub base_url: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub timeout: u64,
}

impl Default for OpenAIProviderConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4".to_string(),
            max_tokens: 4096,
            temperature: 0.1,
            timeout: 30,
        }
    }
}
```

## TUI Components

### Application Structure

```rust
// fennec-tui/src/app.rs
pub struct App {
    session_manager: Arc<SessionManager>,
    sandbox_policy: SandboxPolicy,
    approval_manager: ApprovalManager,
    state: AppState,
}

impl App {
    pub async fn new_with_security(
        session_manager: SessionManager,
        sandbox_policy: SandboxPolicy,
        approval_manager: ApprovalManager,
    ) -> Result<Self> {
        Ok(Self {
            session_manager: Arc::new(session_manager),
            sandbox_policy,
            approval_manager,
            state: AppState::default(),
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        // Initialize terminal
        let mut terminal = self.setup_terminal()?;

        // Event loop
        loop {
            // Render UI
            terminal.draw(|f| self.ui(f))?;

            // Handle events
            if let Event::Key(key) = event::read()? {
                if self.handle_key_event(key).await? {
                    break; // Exit requested
                }
            }
        }

        // Cleanup terminal
        self.restore_terminal()?;

        Ok(())
    }

    fn ui(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(f.size());

        // Chat panel
        self.render_chat_panel(f, chunks[0]);

        // Preview panel
        self.render_preview_panel(f, chunks[1]);

        // Status bar
        self.render_status_bar(f);
    }
}

#[derive(Debug, Default)]
pub struct AppState {
    pub input: String,
    pub messages: Vec<DisplayMessage>,
    pub preview_content: Option<String>,
    pub status: String,
    pub mode: AppMode,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Chat,
    Preview,
    Approval,
}
```

## Extension Points

### Creating Custom Commands

```rust
// Example: Custom deployment command
use fennec_commands::{CommandExecutor, CommandDescriptor, CommandContext, CommandExecutionResult};
use async_trait::async_trait;

pub struct DeployCommand {
    deployment_config: DeploymentConfig,
}

impl DeployCommand {
    pub fn new(config: DeploymentConfig) -> Self {
        Self {
            deployment_config: config,
        }
    }
}

#[async_trait]
impl CommandExecutor for DeployCommand {
    async fn execute(
        &self,
        args: &serde_json::Value,
        context: &CommandContext,
    ) -> Result<CommandExecutionResult> {
        let deploy_args: DeployArgs = serde_json::from_value(args.clone())?;

        // Validate deployment target
        self.validate_deployment_target(&deploy_args.target)?;

        // Execute deployment steps
        let result = self.execute_deployment(&deploy_args, context).await?;

        Ok(CommandExecutionResult {
            success: result.success,
            output: result.output,
            error: result.error,
            preview: None,
            metadata: CommandMetadata::default(),
        })
    }

    fn descriptor(&self) -> CommandDescriptor {
        CommandDescriptor {
            name: "deploy".to_string(),
            description: "Deploy application to target environment".to_string(),
            capabilities: vec![
                Capability::ExecuteShell,
                Capability::NetworkAccess,
                Capability::ReadFile,
            ],
            requires_approval: true,
            preview_mode: PreviewMode::Always,
        }
    }
}

// Register custom command
pub fn register_deploy_command(registry: &mut CommandRegistry) -> Result<()> {
    let config = DeploymentConfig::load()?;
    let command = DeployCommand::new(config);
    registry.register(Box::new(command))?;
    Ok(())
}
```

### Plugin System (Future)

```rust
// Planned plugin interface
pub trait FennecPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn commands(&self) -> Vec<Box<dyn CommandExecutor>>;
    fn memory_adapters(&self) -> Vec<Box<dyn MemoryAdapter>>;
    fn providers(&self) -> Vec<Box<dyn ProviderClient>>;
}

pub struct PluginManager {
    loaded_plugins: HashMap<String, Box<dyn FennecPlugin>>,
}

impl PluginManager {
    pub fn load_plugin(&mut self, plugin_path: &Path) -> Result<()> {
        // Dynamic plugin loading (future implementation)
        todo!("Plugin loading not yet implemented")
    }

    pub fn register_commands(&self, registry: &mut CommandRegistry) -> Result<()> {
        for plugin in self.loaded_plugins.values() {
            for command in plugin.commands() {
                registry.register(command)?;
            }
        }
        Ok(())
    }
}
```

This API reference provides comprehensive documentation of all public interfaces and extension points in Fennec. Use this as a guide for extending functionality, integrating with external systems, or contributing to the project.