use crate::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub provider: ProviderConfig,
    pub security: SecurityConfig,
    pub memory: MemoryConfig,
    pub tui: TuiConfig,
    #[cfg(feature = "telemetry")]
    pub telemetry: Option<TelemetryConfigRef>,
}

#[cfg(feature = "telemetry")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfigRef {
    pub config_path: Option<PathBuf>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Which provider to use: "openai", "anthropic", "openrouter"
    #[serde(default = "default_provider")]
    pub provider: String,

    // OpenAI configuration
    pub openai_api_key: Option<String>,

    // Anthropic configuration
    pub anthropic_api_key: Option<String>,

    // OpenRouter configuration
    pub openrouter_api_key: Option<String>,

    pub default_model: String,
    pub base_url: Option<String>,
    pub timeout_seconds: u64,
}

fn default_provider() -> String {
    "openai".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub default_sandbox_level: String,
    pub audit_log_enabled: bool,
    pub audit_log_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub storage_path: PathBuf,
    pub max_transcript_size: usize,
    pub enable_agents_md: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiConfig {
    pub theme: String,
    pub key_bindings: KeyBindings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBindings {
    pub quit: String,
    pub help: String,
    pub clear: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            provider: ProviderConfig {
                provider: "openai".to_string(),
                openai_api_key: None,
                anthropic_api_key: None,
                openrouter_api_key: None,
                default_model: "gpt-4".to_string(),
                base_url: None,
                timeout_seconds: 30,
            },
            security: SecurityConfig {
                default_sandbox_level: "workspace-write".to_string(),
                audit_log_enabled: true,
                audit_log_path: None,
            },
            memory: MemoryConfig {
                storage_path: PathBuf::from(".fennec"),
                max_transcript_size: 10_000,
                enable_agents_md: true,
            },
            tui: TuiConfig {
                theme: "default".to_string(),
                key_bindings: KeyBindings {
                    quit: "Ctrl+C".to_string(),
                    help: "F1".to_string(),
                    clear: "Ctrl+L".to_string(),
                },
            },
            #[cfg(feature = "telemetry")]
            telemetry: Some(TelemetryConfigRef {
                config_path: None,
                enabled: true,
            }),
        }
    }
}

impl Config {
    pub async fn load(config_path: Option<&Path>) -> Result<Self> {
        let config_file = match config_path {
            Some(path) => path.to_path_buf(),
            None => Self::default_config_path()?,
        };

        if config_file.exists() {
            info!("Loading config from: {}", config_file.display());
            let content = tokio::fs::read_to_string(&config_file).await.map_err(|e| {
                crate::FennecError::FileRead {
                    path: config_file.display().to_string(),
                    source: e,
                }
            })?;
            let mut config: Config =
                toml::from_str(&content).map_err(|e| crate::FennecError::ConfigLoadFailed {
                    path: config_file.display().to_string(),
                    source: Box::new(e),
                })?;

            // Override with environment variables
            config.load_env_overrides();
            Ok(config)
        } else {
            info!("No config file found, using defaults");
            let mut config = Self::default();
            config.load_env_overrides();
            Ok(config)
        }
    }

    fn default_config_path() -> Result<PathBuf> {
        let project_dirs = ProjectDirs::from("com", "fennec", "fennec").ok_or_else(|| {
            crate::FennecError::ConfigInvalid {
                issue: "Could not determine config directory".to_string(),
                suggestion: "Ensure your system has proper home directory permissions".to_string(),
            }
        })?;

        Ok(project_dirs.config_dir().join("config.toml"))
    }

    fn load_env_overrides(&mut self) {
        // Provider selection
        if let Ok(provider) = std::env::var("FENNEC_PROVIDER") {
            self.provider.provider = provider;
        }

        // OpenAI
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            self.provider.openai_api_key = Some(api_key);
        }
        if let Ok(base_url) = std::env::var("OPENAI_BASE_URL") {
            self.provider.base_url = Some(base_url);
        }

        // Anthropic
        if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
            self.provider.anthropic_api_key = Some(api_key);
        }
        if let Ok(base_url) = std::env::var("ANTHROPIC_BASE_URL") {
            self.provider.base_url = Some(base_url);
        }

        // OpenRouter
        if let Ok(api_key) = std::env::var("OPENROUTER_API_KEY") {
            self.provider.openrouter_api_key = Some(api_key);
        }

        // General settings
        if let Ok(model) = std::env::var("FENNEC_DEFAULT_MODEL") {
            self.provider.default_model = model;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tokio::fs;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.provider.default_model, "gpt-4");
        assert_eq!(config.provider.timeout_seconds, 30);
        assert_eq!(config.security.default_sandbox_level, "workspace-write");
        assert!(config.security.audit_log_enabled);
        assert_eq!(config.memory.max_transcript_size, 10_000);
        assert!(config.memory.enable_agents_md);
        assert_eq!(config.tui.theme, "default");
        assert_eq!(config.tui.key_bindings.quit, "Ctrl+C");
    }

    #[test]
    fn test_provider_config_default() {
        let config = Config::default();
        assert!(config.provider.openai_api_key.is_none());
        assert!(config.provider.base_url.is_none());
        assert_eq!(config.provider.timeout_seconds, 30);
    }

    #[test]
    fn test_security_config_default() {
        let config = Config::default();
        assert_eq!(config.security.default_sandbox_level, "workspace-write");
        assert!(config.security.audit_log_enabled);
        assert!(config.security.audit_log_path.is_none());
    }

    #[test]
    fn test_memory_config_default() {
        let config = Config::default();
        assert_eq!(config.memory.storage_path, PathBuf::from(".fennec"));
        assert_eq!(config.memory.max_transcript_size, 10_000);
        assert!(config.memory.enable_agents_md);
    }

    #[test]
    fn test_tui_config_default() {
        let config = Config::default();
        assert_eq!(config.tui.theme, "default");
        assert_eq!(config.tui.key_bindings.quit, "Ctrl+C");
        assert_eq!(config.tui.key_bindings.help, "F1");
        assert_eq!(config.tui.key_bindings.clear, "Ctrl+L");
    }

    #[cfg(feature = "telemetry")]
    #[test]
    fn test_telemetry_config_default() {
        let config = Config::default();
        assert!(config.telemetry.is_some());
        let telemetry = config.telemetry.unwrap();
        assert!(telemetry.enabled);
        assert!(telemetry.config_path.is_none());
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let serialized = toml::to_string(&config).unwrap();
        assert!(serialized.contains("default_model"));
        assert!(serialized.contains("gpt-4"));
    }

    #[test]
    fn test_config_deserialization() {
        let toml_str = r#"
            [provider]
            default_model = "gpt-4-turbo"
            timeout_seconds = 60

            [security]
            default_sandbox_level = "read-only"
            audit_log_enabled = false

            [memory]
            storage_path = ".fennec-data"
            max_transcript_size = 20000
            enable_agents_md = false

            [tui]
            theme = "dark"

            [tui.key_bindings]
            quit = "q"
            help = "h"
            clear = "c"
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.provider.default_model, "gpt-4-turbo");
        assert_eq!(config.provider.timeout_seconds, 60);
        assert_eq!(config.security.default_sandbox_level, "read-only");
        assert!(!config.security.audit_log_enabled);
        assert_eq!(config.memory.storage_path, PathBuf::from(".fennec-data"));
        assert_eq!(config.memory.max_transcript_size, 20000);
        assert!(!config.memory.enable_agents_md);
        assert_eq!(config.tui.theme, "dark");
        assert_eq!(config.tui.key_bindings.quit, "q");
    }

    #[tokio::test]
    async fn test_load_with_nonexistent_file() {
        let temp_path = PathBuf::from("/tmp/nonexistent_config.toml");
        let config = Config::load(Some(&temp_path)).await.unwrap();
        // Should return default config
        assert_eq!(config.provider.default_model, "gpt-4");
    }

    #[tokio::test]
    async fn test_load_with_existing_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");

        let toml_content = r#"
            [provider]
            default_model = "gpt-4-turbo"
            timeout_seconds = 120

            [security]
            default_sandbox_level = "strict"
            audit_log_enabled = true

            [memory]
            storage_path = "/tmp/fennec"
            max_transcript_size = 50000
            enable_agents_md = true

            [tui]
            theme = "light"

            [tui.key_bindings]
            quit = "Ctrl+Q"
            help = "F2"
            clear = "Ctrl+K"
        "#;

        fs::write(&config_path, toml_content).await.unwrap();

        let config = Config::load(Some(&config_path)).await.unwrap();
        assert_eq!(config.provider.default_model, "gpt-4-turbo");
        assert_eq!(config.provider.timeout_seconds, 120);
        assert_eq!(config.security.default_sandbox_level, "strict");
        assert_eq!(config.memory.max_transcript_size, 50000);
        assert_eq!(config.tui.theme, "light");
        assert_eq!(config.tui.key_bindings.help, "F2");
    }

    #[tokio::test]
    async fn test_load_with_invalid_toml() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("invalid_config.toml");

        fs::write(&config_path, "invalid toml content {{{")
            .await
            .unwrap();

        let result = Config::load(Some(&config_path)).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::FennecError::ConfigLoadFailed { .. }
        ));
    }

    #[tokio::test]
    async fn test_load_env_overrides() {
        // Set environment variables
        env::set_var("OPENAI_API_KEY", "test-api-key");
        env::set_var("OPENAI_BASE_URL", "https://test.openai.com");
        env::set_var("FENNEC_DEFAULT_MODEL", "gpt-3.5-turbo");

        let mut config = Config::default();
        config.load_env_overrides();

        assert_eq!(
            config.provider.openai_api_key,
            Some("test-api-key".to_string())
        );
        assert_eq!(
            config.provider.base_url,
            Some("https://test.openai.com".to_string())
        );
        assert_eq!(config.provider.default_model, "gpt-3.5-turbo");

        // Clean up
        env::remove_var("OPENAI_API_KEY");
        env::remove_var("OPENAI_BASE_URL");
        env::remove_var("FENNEC_DEFAULT_MODEL");
    }

    #[test]
    fn test_default_config_path() {
        let result = Config::default_config_path();
        // Should succeed on most systems
        if result.is_ok() {
            let path = result.unwrap();
            assert!(path.to_string_lossy().contains("fennec"));
            assert!(path.to_string_lossy().ends_with("config.toml"));
        }
    }

    #[test]
    fn test_key_bindings_clone() {
        let bindings = KeyBindings {
            quit: "q".to_string(),
            help: "h".to_string(),
            clear: "c".to_string(),
        };
        let cloned = bindings.clone();
        assert_eq!(bindings.quit, cloned.quit);
        assert_eq!(bindings.help, cloned.help);
        assert_eq!(bindings.clear, cloned.clear);
    }

    #[test]
    fn test_config_clone() {
        let config = Config::default();
        let cloned = config.clone();
        assert_eq!(config.provider.default_model, cloned.provider.default_model);
        assert_eq!(config.tui.theme, cloned.tui.theme);
    }
}
