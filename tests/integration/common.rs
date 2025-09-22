use anyhow::Result;
use fennec_commands::{CommandContext, CommandRegistry, create_command_registry};
use fennec_core::config::{Config, ProviderConfig, SecurityConfig, MemoryConfig, TuiConfig};
use fennec_orchestration::{CommandExecutionEngine, DefaultApprovalHandler, BackupManager, BackupRetentionConfig};
use fennec_provider::{MockProviderClient, ProviderClientFactory};
use fennec_security::{AuditLogger, SandboxLevel, create_sandbox_policy};
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// Test configuration for integration tests
#[derive(Debug)]
pub struct TestConfig {
    pub temp_dir: TempDir,
    pub config: Config,
    pub workspace_path: PathBuf,
    pub audit_log_path: PathBuf,
    pub backup_path: PathBuf,
}

impl TestConfig {
    /// Create a new test configuration with temporary directories
    pub fn new() -> Result<Self> {
        let temp_dir = TempDir::new()?;
        let workspace_path = temp_dir.path().join("workspace");
        let audit_log_path = temp_dir.path().join("audit.log");
        let backup_path = temp_dir.path().join("backups");
        
        std::fs::create_dir_all(&workspace_path)?;
        std::fs::create_dir_all(&backup_path)?;

        let config = Config {
            provider: ProviderConfig {
                openai_api_key: None, // Use mock provider
                default_model: "gpt-4".to_string(),
                base_url: None,
                timeout_seconds: 30,
            },
            security: SecurityConfig {
                default_sandbox_level: SandboxLevel::WorkspaceWrite,
                audit_log_enabled: true,
                audit_log_path: Some(audit_log_path.clone()),
                approval_timeout_seconds: 300,
                backup_retention_days: 30,
                max_backups: 100,
            },
            memory: MemoryConfig {
                max_transcript_size: 5000,
                enable_agents_md: true,
                agents_md_path: None,
            },
            tui: TuiConfig {
                theme: "default".to_string(),
                key_bindings: std::collections::HashMap::new(),
            },
        };

        Ok(Self {
            temp_dir,
            config,
            workspace_path,
            audit_log_path,
            backup_path,
        })
    }

    /// Get the workspace path as a string
    pub fn workspace_str(&self) -> &str {
        self.workspace_path.to_str().unwrap()
    }
}

/// Test environment containing all major components
pub struct TestEnvironment {
    pub config: TestConfig,
    pub command_registry: Arc<CommandRegistry>,
    pub execution_engine: Arc<CommandExecutionEngine>,
    pub audit_logger: Arc<AuditLogger>,
    pub session_id: Uuid,
}

impl TestEnvironment {
    /// Create a new test environment with all components initialized
    pub async fn new() -> Result<Self> {
        let config = TestConfig::new()?;
        
        // Create audit logger
        let audit_logger = Arc::new(AuditLogger::with_path(&config.audit_log_path).await?);
        
        // Create command registry
        let command_registry = Arc::new(create_command_registry().await?);
        
        // Create approval handler
        let approval_handler = Arc::new(DefaultApprovalHandler::new(false, false)); // Non-interactive for tests
        
        // Create backup manager
        let backup_manager = Arc::new(BackupManager::new(
            config.backup_path.clone(),
            BackupRetentionConfig::default(),
            audit_logger.clone(),
        ));
        
        // Create execution engine
        let execution_engine = Arc::new(CommandExecutionEngine::new(
            command_registry.clone(),
            approval_handler,
            backup_manager,
            audit_logger.clone(),
            config.config.clone(),
        ));
        
        let session_id = Uuid::new_v4();
        
        Ok(Self {
            config,
            command_registry,
            execution_engine,
            audit_logger,
            session_id,
        })
    }

    /// Create a command context for testing
    pub fn create_context(&self, sandbox_level: SandboxLevel) -> CommandContext {
        CommandContext {
            session_id: self.session_id,
            user_id: None,
            workspace_path: Some(self.config.workspace_path.clone()),
            sandbox_level,
            dry_run: false,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
        }
    }

    /// Create a preview-only context for testing
    pub fn create_preview_context(&self, sandbox_level: SandboxLevel) -> CommandContext {
        CommandContext {
            session_id: self.session_id,
            user_id: None,
            workspace_path: Some(self.config.workspace_path.clone()),
            sandbox_level,
            dry_run: false,
            preview_only: true,
            cancellation_token: CancellationToken::new(),
        }
    }

    /// Create a dry-run context for testing
    pub fn create_dry_run_context(&self, sandbox_level: SandboxLevel) -> CommandContext {
        CommandContext {
            session_id: self.session_id,
            user_id: None,
            workspace_path: Some(self.config.workspace_path.clone()),
            sandbox_level,
            dry_run: true,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
        }
    }

    /// Write a test file to the workspace
    pub async fn write_test_file(&self, path: &str, content: &str) -> Result<PathBuf> {
        let file_path = self.config.workspace_path.join(path);
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&file_path, content).await?;
        Ok(file_path)
    }

    /// Read a test file from the workspace
    pub async fn read_test_file(&self, path: &str) -> Result<String> {
        let file_path = self.config.workspace_path.join(path);
        Ok(tokio::fs::read_to_string(file_path).await?)
    }

    /// Create a subdirectory in the workspace
    pub async fn create_test_dir(&self, path: &str) -> Result<PathBuf> {
        let dir_path = self.config.workspace_path.join(path);
        tokio::fs::create_dir_all(&dir_path).await?;
        Ok(dir_path)
    }

    /// List files in the workspace
    pub async fn list_workspace_files(&self) -> Result<Vec<String>> {
        let mut files = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.config.workspace_path).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(name) = path.file_name() {
                if let Some(name_str) = name.to_str() {
                    files.push(name_str.to_string());
                }
            }
        }
        
        files.sort();
        Ok(files)
    }

    /// Get the number of audit log entries
    pub async fn audit_log_count(&self) -> Result<usize> {
        if self.config.audit_log_path.exists() {
            let content = tokio::fs::read_to_string(&self.config.audit_log_path).await?;
            Ok(content.lines().count())
        } else {
            Ok(0)
        }
    }

    /// Check if a backup exists for a given execution
    pub fn backup_exists(&self, backup_id: &Uuid) -> bool {
        let backup_dir = self.config.backup_path.join(backup_id.to_string());
        backup_dir.exists()
    }
}

/// Utility functions for assertions and test helpers
pub mod assertions {
    use super::*;
    use fennec_commands::CommandExecutionResult;
    use fennec_orchestration::{CommandState, ExecutionInfo};

    /// Assert that a command execution was successful
    pub fn assert_command_success(result: &CommandExecutionResult) {
        assert!(result.success, "Command should succeed but failed: {:?}", result.error);
    }

    /// Assert that a command execution failed
    pub fn assert_command_failure(result: &CommandExecutionResult) {
        assert!(!result.success, "Command should fail but succeeded");
    }

    /// Assert that execution info has the expected state
    pub fn assert_execution_state(execution: &ExecutionInfo, expected_state: &CommandState) {
        assert_eq!(&execution.state, expected_state, 
                  "Execution state mismatch. Expected: {:?}, Actual: {:?}", 
                  expected_state, execution.state);
    }

    /// Assert that a file exists in the workspace
    pub async fn assert_file_exists(env: &TestEnvironment, path: &str) {
        let file_path = env.config.workspace_path.join(path);
        assert!(file_path.exists(), "File should exist: {}", path);
    }

    /// Assert that a file does not exist in the workspace
    pub async fn assert_file_not_exists(env: &TestEnvironment, path: &str) {
        let file_path = env.config.workspace_path.join(path);
        assert!(!file_path.exists(), "File should not exist: {}", path);
    }

    /// Assert that a file contains specific content
    pub async fn assert_file_content(env: &TestEnvironment, path: &str, expected: &str) -> Result<()> {
        let content = env.read_test_file(path).await?;
        assert_eq!(content.trim(), expected.trim(), 
                  "File content mismatch for {}", path);
        Ok(())
    }

    /// Assert that audit log contains expected number of entries
    pub async fn assert_audit_log_entries(env: &TestEnvironment, min_count: usize) -> Result<()> {
        let count = env.audit_log_count().await?;
        assert!(count >= min_count, 
               "Expected at least {} audit log entries, found {}", min_count, count);
        Ok(())
    }
}

/// Test data and fixtures
pub mod fixtures {
    /// Sample code files for testing
    pub const SAMPLE_RUST_CODE: &str = r#"
fn main() {
    println!("Hello, world!");
}
"#;

    pub const SAMPLE_PYTHON_CODE: &str = r#"
def hello():
    print("Hello, world!")

if __name__ == "__main__":
    hello()
"#;

    pub const SAMPLE_JAVASCRIPT_CODE: &str = r#"
function hello() {
    console.log("Hello, world!");
}

hello();
"#;

    /// Sample configuration files
    pub const SAMPLE_TOML_CONFIG: &str = r#"
[server]
host = "localhost"
port = 8080

[database]
url = "sqlite://data.db"
max_connections = 10
"#;

    pub const SAMPLE_JSON_CONFIG: &str = r#"
{
    "name": "test-project",
    "version": "1.0.0",
    "dependencies": {
        "lodash": "^4.17.21",
        "express": "^4.18.0"
    }
}
"#;

    /// Sample tasks for planning tests
    pub const SIMPLE_TASK: &str = "Create a simple hello world program";
    
    pub const COMPLEX_TASK: &str = r#"
Create a web server with the following features:
1. REST API endpoints for user management
2. SQLite database integration
3. JWT authentication
4. Input validation and error handling
5. Unit tests with 80% coverage
"#;

    pub const REFACTORING_TASK: &str = r#"
Refactor the existing codebase to:
1. Extract common utilities into separate modules
2. Improve error handling throughout
3. Add comprehensive documentation
4. Optimize performance bottlenecks
"#;
}

/// Mock provider with configurable responses for testing
pub struct ConfigurableMockProvider {
    responses: Arc<RwLock<Vec<String>>>,
    current_index: Arc<RwLock<usize>>,
    delay_ms: u64,
    should_error: bool,
}

impl ConfigurableMockProvider {
    /// Create a new mock provider with predefined responses
    pub fn new(responses: Vec<String>) -> Self {
        Self {
            responses: Arc::new(RwLock::new(responses)),
            current_index: Arc::new(RwLock::new(0)),
            delay_ms: 0,
            should_error: false,
        }
    }

    /// Create a mock provider that simulates errors
    pub fn with_errors() -> Self {
        Self {
            responses: Arc::new(RwLock::new(vec!["Error response".to_string()])),
            current_index: Arc::new(RwLock::new(0)),
            delay_ms: 0,
            should_error: true,
        }
    }

    /// Create a mock provider with simulated latency
    pub fn with_delay(responses: Vec<String>, delay_ms: u64) -> Self {
        Self {
            responses: Arc::new(RwLock::new(responses)),
            current_index: Arc::new(RwLock::new(0)),
            delay_ms,
            should_error: false,
        }
    }

    /// Add more responses to the provider
    pub async fn add_responses(&self, mut new_responses: Vec<String>) {
        let mut responses = self.responses.write().await;
        responses.append(&mut new_responses);
    }

    /// Reset the provider to start from the first response
    pub async fn reset(&self) {
        let mut index = self.current_index.write().await;
        *index = 0;
    }
}

#[async_trait::async_trait]
impl fennec_core::provider::ProviderClient for ConfigurableMockProvider {
    async fn complete(&self, request: fennec_core::provider::ProviderRequest) -> fennec_core::Result<fennec_core::provider::ProviderResponse> {
        use fennec_core::provider::{ProviderResponse, Usage};
        
        if self.delay_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;
        }

        if self.should_error {
            return Err(fennec_core::FennecError::Provider("Mock error".to_string()));
        }

        let responses = self.responses.read().await;
        let mut index = self.current_index.write().await;
        
        let response_content = if responses.is_empty() {
            "Default mock response".to_string()
        } else {
            let content = responses[*index % responses.len()].clone();
            *index += 1;
            content
        };

        Ok(ProviderResponse {
            id: Uuid::new_v4(),
            content: response_content,
            usage: Some(Usage {
                prompt_tokens: request.messages.len() as u32 * 10,
                completion_tokens: 20,
                total_tokens: request.messages.len() as u32 * 10 + 20,
            }),
        })
    }

    async fn stream(
        &self,
        request: fennec_core::provider::ProviderRequest,
    ) -> fennec_core::Result<Box<dyn futures::Stream<Item = fennec_core::Result<String>> + Unpin + Send>> {
        use futures::stream;
        
        if self.delay_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;
        }

        if self.should_error {
            return Err(fennec_core::FennecError::Provider("Mock stream error".to_string()));
        }

        let response = self.complete(request).await?;
        let words: Vec<fennec_core::Result<String>> = response.content
            .split_whitespace()
            .map(|word| Ok(format!("{} ", word)))
            .collect();

        Ok(Box::new(stream::iter(words)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_test_environment_creation() {
        let env = TestEnvironment::new().await.unwrap();
        
        // Test that workspace exists
        assert!(env.config.workspace_path.exists());
        
        // Test context creation
        let context = env.create_context(SandboxLevel::ReadOnly);
        assert_eq!(context.session_id, env.session_id);
        assert_eq!(context.sandbox_level, SandboxLevel::ReadOnly);
        assert!(!context.dry_run);
        assert!(!context.preview_only);
    }

    #[tokio::test]
    async fn test_file_operations() {
        let env = TestEnvironment::new().await.unwrap();
        
        // Test writing and reading files
        let content = "Hello, test!";
        env.write_test_file("test.txt", content).await.unwrap();
        
        let read_content = env.read_test_file("test.txt").await.unwrap();
        assert_eq!(read_content, content);
        
        // Test listing files
        let files = env.list_workspace_files().await.unwrap();
        assert!(files.contains(&"test.txt".to_string()));
    }

    #[tokio::test]
    async fn test_configurable_mock_provider() {
        let responses = vec![
            "First response".to_string(),
            "Second response".to_string(),
        ];
        let provider = ConfigurableMockProvider::new(responses);
        
        // Test cycling through responses
        let request = fennec_core::provider::ProviderRequest {
            messages: vec![],
            model: "test".to_string(),
            temperature: None,
            max_tokens: None,
            stream: false,
        };
        
        let response1 = provider.complete(request.clone()).await.unwrap();
        assert_eq!(response1.content, "First response");
        
        let response2 = provider.complete(request.clone()).await.unwrap();
        assert_eq!(response2.content, "Second response");
        
        let response3 = provider.complete(request).await.unwrap();
        assert_eq!(response3.content, "First response"); // Should cycle back
    }

    #[tokio::test]
    async fn test_error_mock_provider() {
        let provider = ConfigurableMockProvider::with_errors();
        
        let request = fennec_core::provider::ProviderRequest {
            messages: vec![],
            model: "test".to_string(),
            temperature: None,
            max_tokens: None,
            stream: false,
        };
        
        let result = provider.complete(request).await;
        assert!(result.is_err());
    }
}