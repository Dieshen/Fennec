use fennec_core::{
    config::Config,
    provider::{ProviderClient, ProviderMessage, ProviderRequest},
    session::Session,
    transcript::{MessageRole, Transcript},
    Result,
};
use fennec_provider::ProviderClientFactory;
use fennec_security::audit::AuditLogger;
use futures::Stream;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

/// Session management and orchestration
pub struct SessionManager {
    config: Config,
    audit_logger: AuditLogger,
    provider_client: Arc<dyn ProviderClient>,
    current_session: Arc<RwLock<Option<Session>>>,
    current_transcript: Arc<RwLock<Option<Transcript>>>,
}

impl SessionManager {
    /// Create a new SessionManager
    #[instrument(skip(config, audit_logger))]
    pub async fn new(config: Config, audit_logger: AuditLogger) -> Result<Self> {
        info!("Initializing SessionManager");

        // Validate provider configuration
        fennec_provider::ProviderClientFactory::validate_config(&config.provider).map_err(|e| {
            fennec_core::FennecError::Provider {
                message: format!("Provider configuration validation failed: {}", e),
            }
        })?;

        // Create provider client
        let provider_client =
            ProviderClientFactory::create_client(&config.provider).map_err(|e| {
                fennec_core::FennecError::Provider {
                    message: format!("Failed to create provider client: {}", e),
                }
            })?;

        info!("Provider client created successfully");

        Ok(Self {
            config,
            audit_logger,
            provider_client,
            current_session: Arc::new(RwLock::new(None)),
            current_transcript: Arc::new(RwLock::new(None)),
        })
    }

    /// Start a new chat session
    #[instrument(skip(self))]
    pub async fn start_session(&self) -> Result<Uuid> {
        info!("Starting new chat session");

        let session = Session::new();
        let session_id = session.id;
        let transcript = Transcript::new(session_id);

        // Store the session and transcript
        {
            let mut current_session = self.current_session.write().await;
            *current_session = Some(session);
        }

        {
            let mut current_transcript = self.current_transcript.write().await;
            *current_transcript = Some(transcript);
        }

        // Log session start
        self.audit_logger
            .log_session_event(session_id, "session_started", None)
            .await?;

        info!("Session started with ID: {}", session_id);
        Ok(session_id)
    }

    /// End the current session
    #[instrument(skip(self))]
    pub async fn end_session(&self) -> Result<()> {
        let session_id = {
            let session_guard = self.current_session.read().await;
            session_guard.as_ref().map(|s| s.id)
        };

        if let Some(session_id) = session_id {
            info!("Ending session: {}", session_id);

            // Log session end
            self.audit_logger
                .log_session_event(session_id, "session_ended", None)
                .await?;

            // Clear current session and transcript
            {
                let mut current_session = self.current_session.write().await;
                *current_session = None;
            }

            {
                let mut current_transcript = self.current_transcript.write().await;
                *current_transcript = None;
            }

            info!("Session ended: {}", session_id);
        } else {
            warn!("Attempted to end session when no session is active");
        }

        Ok(())
    }

    /// Send a message and get a response
    #[instrument(skip(self, content), fields(content_len = content.len()))]
    pub async fn send_message(&self, content: String) -> Result<String> {
        let session_id = self.ensure_active_session().await?;

        info!("Processing message in session: {}", session_id);

        // Add user message to transcript
        self.add_message_to_transcript(MessageRole::User, content.clone())
            .await?;

        // Log user message
        self.audit_logger
            .log_user_message(session_id, &content)
            .await?;

        // Get conversation context
        let messages = self.get_conversation_context().await?;

        // Create provider request
        let request = ProviderRequest {
            id: Uuid::new_v4(),
            messages,
            model: self.config.provider.default_model.clone(),
            stream: false,
        };

        // Send to provider
        debug!("Sending request to provider");
        match self.provider_client.complete(request).await {
            Ok(response) => {
                info!("Received response from provider");

                // Add assistant response to transcript
                self.add_message_to_transcript(MessageRole::Assistant, response.content.clone())
                    .await?;

                // Log assistant response
                self.audit_logger
                    .log_assistant_message(session_id, &response.content)
                    .await?;

                // Log usage if available
                if let Some(usage) = &response.usage {
                    debug!(
                        "Token usage - prompt: {}, completion: {}, total: {}",
                        usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
                    );
                }

                Ok(response.content)
            }
            Err(e) => {
                error!("Provider error: {}", e);

                // Log error
                self.audit_logger
                    .log_error_event(session_id, &format!("Provider error: {}", e))
                    .await?;

                Err(e)
            }
        }
    }

    /// Send a message and get a streaming response
    #[instrument(skip(self, content), fields(content_len = content.len()))]
    pub async fn send_message_stream(
        &self,
        content: String,
    ) -> Result<Box<dyn Stream<Item = Result<String>> + Unpin + Send>> {
        let session_id = self.ensure_active_session().await?;

        info!("Processing streaming message in session: {}", session_id);

        // Add user message to transcript
        self.add_message_to_transcript(MessageRole::User, content.clone())
            .await?;

        // Log user message
        self.audit_logger
            .log_user_message(session_id, &content)
            .await?;

        // Get conversation context
        let messages = self.get_conversation_context().await?;

        // Create provider request
        let request = ProviderRequest {
            id: Uuid::new_v4(),
            messages,
            model: self.config.provider.default_model.clone(),
            stream: true,
        };

        // Send to provider
        debug!("Sending streaming request to provider");
        let stream = self.provider_client.stream(request).await?;

        info!("Streaming response initiated");
        Ok(stream)
    }

    /// Get the current session ID
    pub async fn current_session_id(&self) -> Option<Uuid> {
        let session_guard = self.current_session.read().await;
        session_guard.as_ref().map(|s| s.id)
    }

    /// Get the current transcript
    pub async fn current_transcript(&self) -> Option<Transcript> {
        let transcript_guard = self.current_transcript.read().await;
        transcript_guard.clone()
    }

    /// Clear the current conversation history
    #[instrument(skip(self))]
    pub async fn clear_conversation(&self) -> Result<()> {
        let session_id = self.ensure_active_session().await?;

        info!("Clearing conversation history for session: {}", session_id);

        {
            let mut transcript_guard = self.current_transcript.write().await;
            if let Some(transcript) = transcript_guard.as_mut() {
                transcript.messages.clear();
            }
        }

        // Log conversation clear
        self.audit_logger
            .log_session_event(session_id, "conversation_cleared", None)
            .await?;

        info!("Conversation history cleared");
        Ok(())
    }

    /// Get conversation statistics
    pub async fn conversation_stats(&self) -> Option<ConversationStats> {
        let transcript_guard = self.current_transcript.read().await;
        if let Some(transcript) = transcript_guard.as_ref() {
            let user_messages = transcript
                .messages
                .iter()
                .filter(|m| matches!(m.role, MessageRole::User))
                .count();
            let assistant_messages = transcript
                .messages
                .iter()
                .filter(|m| matches!(m.role, MessageRole::Assistant))
                .count();
            let total_characters: usize = transcript.messages.iter().map(|m| m.content.len()).sum();

            Some(ConversationStats {
                total_messages: transcript.messages.len(),
                user_messages,
                assistant_messages,
                total_characters,
            })
        } else {
            None
        }
    }

    /// Set model for current session
    pub async fn set_model(&self, model: String) -> Result<()> {
        let session_id = self.ensure_active_session().await?;

        info!("Setting model to '{}' for session: {}", model, session_id);

        // In a full implementation, you might validate the model exists
        // For now, we'll just log it
        self.audit_logger
            .log_session_event(session_id, "model_changed", Some(&model))
            .await?;

        Ok(())
    }

    /// Ensure there's an active session, creating one if needed
    async fn ensure_active_session(&self) -> Result<Uuid> {
        let session_guard = self.current_session.read().await;
        if let Some(session) = session_guard.as_ref() {
            Ok(session.id)
        } else {
            drop(session_guard); // Release read lock before calling start_session
            self.start_session().await
        }
    }

    /// Add a message to the current transcript
    async fn add_message_to_transcript(&self, role: MessageRole, content: String) -> Result<()> {
        let mut transcript_guard = self.current_transcript.write().await;
        if let Some(transcript) = transcript_guard.as_mut() {
            transcript.add_message(role, content);
            Ok(())
        } else {
            Err(fennec_core::FennecError::Session {
                message: "No active transcript".to_string(),
            })
        }
    }

    /// Get conversation context for the provider
    async fn get_conversation_context(&self) -> Result<Vec<ProviderMessage>> {
        let transcript_guard = self.current_transcript.read().await;
        if let Some(transcript) = transcript_guard.as_ref() {
            let messages = transcript
                .messages
                .iter()
                .map(|msg| ProviderMessage {
                    role: match msg.role {
                        MessageRole::User => "user".to_string(),
                        MessageRole::Assistant => "assistant".to_string(),
                        MessageRole::System => "system".to_string(),
                    },
                    content: msg.content.clone(),
                })
                .collect();

            Ok(messages)
        } else {
            Ok(vec![])
        }
    }
}

/// Statistics about the current conversation
#[derive(Debug, Clone)]
pub struct ConversationStats {
    pub total_messages: usize,
    pub user_messages: usize,
    pub assistant_messages: usize,
    pub total_characters: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use fennec_core::config::{Config, ProviderConfig};
    use tempfile::TempDir;

    async fn create_test_session_manager() -> Result<(SessionManager, TempDir)> {
        let temp_dir = TempDir::new().unwrap();
        let audit_log_path = temp_dir.path().join("audit.log");

        let config = Config {
            provider: ProviderConfig {
                openai_api_key: Some("test-key".to_string()),
                default_model: "gpt-3.5-turbo".to_string(),
                base_url: None,
                timeout_seconds: 30,
            },
            ..Default::default()
        };

        let audit_logger = AuditLogger::with_path(audit_log_path).await?;

        // Note: This will fail in tests without a real API key
        // In practice, you'd use a mock provider for testing
        match SessionManager::new(config, audit_logger).await {
            Ok(manager) => Ok((manager, temp_dir)),
            Err(_) => {
                // Skip test if no valid API key
                panic!("Test requires valid OpenAI API key in config");
            }
        }
    }

    #[tokio::test]
    async fn test_session_lifecycle() {
        if std::env::var("OPENAI_API_KEY").is_err() {
            return; // Skip test if no API key
        }

        let (manager, _temp_dir) = create_test_session_manager().await.unwrap();

        // Test session creation
        let session_id = manager.start_session().await.unwrap();
        assert_eq!(manager.current_session_id().await, Some(session_id));

        // Test session end
        manager.end_session().await.unwrap();
        assert_eq!(manager.current_session_id().await, None);
    }

    #[tokio::test]
    async fn test_conversation_stats() {
        if std::env::var("OPENAI_API_KEY").is_err() {
            return; // Skip test if no API key
        }

        let (manager, _temp_dir) = create_test_session_manager().await.unwrap();
        manager.start_session().await.unwrap();

        // Initially no stats
        assert!(manager.conversation_stats().await.is_some());

        // After clearing, should still have stats structure
        manager.clear_conversation().await.unwrap();
        let stats = manager.conversation_stats().await.unwrap();
        assert_eq!(stats.total_messages, 0);
    }
}
