use crate::error::{ProviderError, Result};
use crate::mock::MockProviderClient;
use crate::openai::OpenAIClient;
use fennec_core::config::ProviderConfig;
use fennec_core::provider::ProviderClient;
use std::sync::Arc;
use tracing::{info, warn};

/// Factory for creating provider clients
pub struct ProviderClientFactory;

impl ProviderClientFactory {
    /// Create a provider client based on configuration
    pub fn create_client(config: &ProviderConfig) -> Result<Arc<dyn ProviderClient>> {
        // For now, we only support OpenAI, but this can be extended
        // to support other providers based on configuration

        if config.openai_api_key.is_some() {
            info!("Creating OpenAI provider client");
            let client = OpenAIClient::from_provider_config(config)?;
            Ok(Arc::new(client))
        } else {
            info!("No provider credentials found; using mock provider client");
            Ok(Arc::new(MockProviderClient::default()))
        }
    }

    /// Create an OpenAI client specifically
    pub fn create_openai_client(config: &ProviderConfig) -> Result<Arc<OpenAIClient>> {
        info!("Creating OpenAI provider client");
        let client = OpenAIClient::from_provider_config(config)?;
        Ok(Arc::new(client))
    }

    /// Validate provider configuration
    pub fn validate_config(config: &ProviderConfig) -> Result<()> {
        if config.openai_api_key.is_none() {
            info!("No API key supplied; validation will fall back to mock provider");
        }

        if config.default_model.is_empty() {
            return Err(ProviderError::Configuration {
                message: "Default model must be specified".to_string(),
            });
        }

        if config.timeout_seconds == 0 {
            warn!("Timeout is set to 0, this may cause issues");
        }

        // Validate base URL format if provided
        if let Some(base_url) = &config.base_url {
            if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
                return Err(ProviderError::Configuration {
                    message: "Base URL must start with http:// or https://".to_string(),
                });
            }
        }

        Ok(())
    }
}

// Note: ProviderClient trait is already imported and used in this module

#[cfg(test)]
mod tests {
    use super::*;
    use fennec_core::config::ProviderConfig;
    use fennec_core::provider::{ProviderMessage, ProviderRequest};

    #[test]
    fn test_validate_config_missing_api_key() {
        let config = ProviderConfig {
            openai_api_key: None,
            default_model: "gpt-4".to_string(),
            base_url: None,
            timeout_seconds: 30,
        };

        let result = ProviderClientFactory::validate_config(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_config_empty_model() {
        let config = ProviderConfig {
            openai_api_key: Some("test-key".to_string()),
            default_model: String::new(),
            base_url: None,
            timeout_seconds: 30,
        };

        let result = ProviderClientFactory::validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_config_invalid_base_url() {
        let config = ProviderConfig {
            openai_api_key: Some("test-key".to_string()),
            default_model: "gpt-4".to_string(),
            base_url: Some("invalid-url".to_string()),
            timeout_seconds: 30,
        };

        let result = ProviderClientFactory::validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_config_valid() {
        let config = ProviderConfig {
            openai_api_key: Some("test-key".to_string()),
            default_model: "gpt-4".to_string(),
            base_url: Some("https://api.openai.com/v1".to_string()),
            timeout_seconds: 30,
        };

        let result = ProviderClientFactory::validate_config(&config);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_client_without_api_key_uses_mock() {
        let config = ProviderConfig {
            openai_api_key: None,
            default_model: "gpt-4".to_string(),
            base_url: None,
            timeout_seconds: 30,
        };

        let client = ProviderClientFactory::create_client(&config).expect("mock provider");
        let request = ProviderRequest {
            id: uuid::Uuid::new_v4(),
            messages: vec![ProviderMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            model: "mock".to_string(),
            stream: false,
        };

        let response = client.complete(request).await.expect("mock response");
        assert!(response.content.contains("offline mode"));
    }
}
