pub mod client;
pub mod error;
pub mod models;
pub mod openai;
pub mod streaming;
pub mod mock;

#[cfg(test)]
mod integration_test;

// Re-export commonly used types
pub use client::ProviderClientFactory;
pub use error::{ProviderError, Result};
pub use fennec_core::provider::ProviderClient;
pub use mock::MockProviderClient;
pub use openai::{OpenAIClient, OpenAIConfig};

#[cfg(test)]
mod integration_tests {
    use super::*;
    use fennec_core::config::ProviderConfig;

    #[tokio::test]
    async fn test_client_factory_integration() {
        // This test would require a valid API key, so it's disabled by default
        // You can enable it for manual testing with a real API key

        if std::env::var("OPENAI_API_KEY").is_err() {
            return; // Skip test if no API key is provided
        }

        let config = ProviderConfig {
            openai_api_key: std::env::var("OPENAI_API_KEY").ok(),
            default_model: "gpt-3.5-turbo".to_string(),
            base_url: None,
            timeout_seconds: 30,
        };

        let validation_result = ProviderClientFactory::validate_config(&config);
        assert!(validation_result.is_ok());

        let client = ProviderClientFactory::create_client(&config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_provider_message_conversion() {
        use crate::models::ChatMessage;
        use fennec_core::provider::ProviderMessage;

        let provider_msg = ProviderMessage {
            role: "user".to_string(),
            content: "Hello, world!".to_string(),
        };

        let chat_msg: ChatMessage = provider_msg.clone().into();
        assert_eq!(chat_msg.role, provider_msg.role);
        assert_eq!(chat_msg.content, provider_msg.content);
        assert!(chat_msg.name.is_none());

        let converted_back: ProviderMessage = chat_msg.into();
        assert_eq!(converted_back.role, provider_msg.role);
        assert_eq!(converted_back.content, provider_msg.content);
    }
}
