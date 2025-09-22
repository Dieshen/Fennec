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
    pub openai_api_key: Option<String>,
    pub default_model: String,
    pub base_url: Option<String>,
    pub timeout_seconds: u64,
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
                openai_api_key: None,
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
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            self.provider.openai_api_key = Some(api_key);
        }

        if let Ok(base_url) = std::env::var("OPENAI_BASE_URL") {
            self.provider.base_url = Some(base_url);
        }

        if let Ok(model) = std::env::var("FENNEC_DEFAULT_MODEL") {
            self.provider.default_model = model;
        }
    }
}
