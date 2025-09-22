use crate::error::{ProviderError, Result};
use crate::models::*;
use crate::streaming::SseStream;
use fennec_core::config::ProviderConfig;
use fennec_core::provider::{ProviderClient, ProviderRequest, ProviderResponse};
use futures::{Stream, StreamExt};
use reqwest::{header, Client, Response};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::{sleep, timeout};
use tracing::{debug, error, info, instrument, warn};

/// OpenAI API client configuration
#[derive(Debug, Clone)]
pub struct OpenAIConfig {
    pub api_key: String,
    pub base_url: String,
    pub timeout: Duration,
    pub max_retries: u32,
    pub initial_retry_delay: Duration,
    pub max_retry_delay: Duration,
    pub max_concurrent_requests: usize,
}

impl Default for OpenAIConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://api.openai.com/v1".to_string(),
            timeout: Duration::from_secs(30),
            max_retries: 3,
            initial_retry_delay: Duration::from_millis(500),
            max_retry_delay: Duration::from_secs(60),
            max_concurrent_requests: 10,
        }
    }
}

impl From<&ProviderConfig> for OpenAIConfig {
    fn from(config: &ProviderConfig) -> Self {
        Self {
            api_key: config.openai_api_key.clone().unwrap_or_default(),
            base_url: config
                .base_url
                .clone()
                .unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
            timeout: Duration::from_secs(config.timeout_seconds),
            ..Default::default()
        }
    }
}

/// OpenAI API client
pub struct OpenAIClient {
    client: Client,
    config: OpenAIConfig,
    semaphore: Arc<Semaphore>,
}

impl OpenAIClient {
    pub fn new(config: OpenAIConfig) -> Result<Self> {
        if config.api_key.is_empty() {
            return Err(ProviderError::ConfigurationMissing {
                provider: "openai".to_string(),
            });
        }

        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {}", config.api_key)).map_err(|e| {
                ProviderError::ConfigurationInvalid {
                    provider: "openai".to_string(),
                    setting: "api_key".to_string(),
                    issue: format!("Invalid format: {}", e),
                }
            })?,
        );
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static("fennec/0.1.0"),
        );

        let client = Client::builder()
            .timeout(config.timeout)
            .default_headers(headers)
            .build()
            .map_err(|e| ProviderError::ConfigurationInvalid {
                provider: "openai".to_string(),
                setting: "http_client".to_string(),
                issue: format!("Failed to create HTTP client: {}", e),
            })?;

        let semaphore = Arc::new(Semaphore::new(config.max_concurrent_requests));

        Ok(Self {
            client,
            config,
            semaphore,
        })
    }

    pub fn from_provider_config(config: &ProviderConfig) -> Result<Self> {
        Self::new(config.into())
    }

    #[instrument(skip(self, request), fields(model = %request.model))]
    pub async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|e| ProviderError::Generic {
                message: format!("Failed to acquire semaphore: {}", e),
                provider: "openai".to_string(),
                context: None,
            })?;

        let url = format!("{}/chat/completions", self.config.base_url);
        debug!("Making chat completion request to: {}", url);

        self.retry_with_backoff(|| async {
            let response = timeout(
                self.config.timeout,
                self.client.post(&url).json(&request).send(),
            )
            .await
            .map_err(|_| ProviderError::Timeout {
                operation: "chat_completion".to_string(),
                timeout_ms: self.config.timeout.as_millis() as u64,
            })?
            .map_err(|e| ProviderError::Http {
                operation: "chat_completion_request".to_string(),
                source: e,
            })?;

            self.handle_response(response).await
        })
        .await
    }

    #[instrument(skip(self, request), fields(model = %request.model))]
    pub async fn chat_completion_stream(
        &self,
        mut request: ChatCompletionRequest,
    ) -> Result<impl Stream<Item = Result<ChatCompletionChunk>>> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|e| ProviderError::Generic {
                message: format!("Failed to acquire semaphore: {}", e),
                provider: "openai".to_string(),
                context: None,
            })?;

        request.stream = Some(true);
        let url = format!("{}/chat/completions", self.config.base_url);
        debug!("Making streaming chat completion request to: {}", url);

        let response = timeout(
            self.config.timeout,
            self.client.post(&url).json(&request).send(),
        )
        .await
        .map_err(|_| ProviderError::Timeout {
            operation: "stream_chat_completion".to_string(),
            timeout_ms: self.config.timeout.as_millis() as u64,
        })?
        .map_err(|e| ProviderError::Http {
            operation: "stream_chat_completion_request".to_string(),
            source: e,
        })?;

        if !response.status().is_success() {
            let error = self.parse_error_response(response).await?;
            return Err(error);
        }

        let sse_stream = SseStream::new(response);
        Ok(sse_stream.parse_events())
    }

    pub async fn list_models(&self) -> Result<ModelsResponse> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|e| ProviderError::Generic {
                message: format!("Failed to acquire semaphore: {}", e),
                provider: "openai".to_string(),
                context: None,
            })?;

        let url = format!("{}/models", self.config.base_url);
        debug!("Listing models from: {}", url);

        self.retry_with_backoff(|| async {
            let response = timeout(self.config.timeout, self.client.get(&url).send())
                .await
                .map_err(|_| ProviderError::Timeout {
                    operation: "embed".to_string(),
                    timeout_ms: self.config.timeout.as_millis() as u64,
                })?
                .map_err(|e| ProviderError::Http {
                    operation: "embed_request".to_string(),
                    source: e,
                })?;

            self.handle_response(response).await
        })
        .await
    }

    async fn handle_response<T>(&self, response: Response) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let status = response.status();

        if status.is_success() {
            let response_text = response.text().await.map_err(|e| ProviderError::Http {
                operation: "read_response_text".to_string(),
                source: e,
            })?;
            debug!("Received response: {}", response_text);

            serde_json::from_str(&response_text).map_err(|e| {
                error!("Failed to parse response: {}, text: {}", e, response_text);
                ProviderError::Json {
                    operation: "parse_response".to_string(),
                    source: e,
                }
            })
        } else {
            let error = self.parse_error_response(response).await?;
            Err(error)
        }
    }

    async fn parse_error_response(&self, response: Response) -> Result<ProviderError> {
        let status_code = response.status().as_u16();
        let response_text = response.text().await.map_err(|e| ProviderError::Http {
            operation: "read_error_response_text".to_string(),
            source: e,
        })?;

        // Try to parse as OpenAI error format
        if let Ok(openai_error) = serde_json::from_str::<OpenAIError>(&response_text) {
            let error_details = openai_error.error;

            match status_code {
                401 => Err(ProviderError::AuthenticationFailed {
                    provider: "openai".to_string(),
                    reason: error_details.message,
                }),
                429 => {
                    // Extract retry-after from the error message if present
                    let retry_after = self
                        .extract_retry_after(&error_details.message)
                        .unwrap_or(60);
                    Err(ProviderError::RateLimit {
                        provider: "openai".to_string(),
                        message: error_details.message,
                        retry_after,
                        daily_limit: None,
                        current_usage: None,
                    })
                }
                400 => Err(ProviderError::InvalidRequest {
                    field: "request".to_string(),
                    issue: error_details.message,
                }),
                404 if error_details.error_type == "model_not_found" => {
                    Err(ProviderError::ModelNotFound {
                        model: error_details.param.unwrap_or_default(),
                    })
                }
                413 => Err(ProviderError::TokenLimit {
                    used: 0,  // We don't have exact usage information from OpenAI error
                    limit: 0, // We don't have exact limit information from OpenAI error
                    suggestion: error_details.message,
                }),
                500..=599 => Err(ProviderError::ServerError {
                    provider: "openai".to_string(),
                    status_code,
                    message: error_details.message,
                    is_temporary: true,
                }),
                _ => Err(ProviderError::Generic {
                    message: error_details.message,
                    provider: "openai".to_string(),
                    context: None,
                }),
            }
        } else {
            // Fallback for non-standard error formats
            match status_code {
                401 => Err(ProviderError::AuthenticationFailed {
                    provider: "openai".to_string(),
                    reason: "Authentication failed".to_string(),
                }),
                429 => Err(ProviderError::RateLimit {
                    provider: "openai".to_string(),
                    message: "Rate limit exceeded".to_string(),
                    retry_after: 60,
                    daily_limit: None,
                    current_usage: None,
                }),
                500..=599 => Err(ProviderError::ServerError {
                    provider: "openai".to_string(),
                    status_code,
                    message: response_text,
                    is_temporary: true,
                }),
                _ => Err(ProviderError::Generic {
                    message: format!("HTTP {}: {}", status_code, response_text),
                    provider: "openai".to_string(),
                    context: Some(format!("status_code: {}", status_code)),
                }),
            }
        }
    }

    fn extract_retry_after(&self, message: &str) -> Option<u64> {
        // Simple regex-free extraction for retry-after
        if message.contains("Please try again in ") {
            // This is a simplified approach - in practice you might use regex
            // or more sophisticated parsing
            Some(60) // Default to 60 seconds
        } else {
            None
        }
    }

    async fn retry_with_backoff<F, Fut, T>(&self, mut operation: F) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut attempt = 0;
        let mut delay = self.config.initial_retry_delay;

        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(err) if attempt >= self.config.max_retries || !err.is_retryable() => {
                    return Err(err);
                }
                Err(err) => {
                    attempt += 1;
                    warn!(
                        "Request failed (attempt {}/{}): {}",
                        attempt, self.config.max_retries, err
                    );

                    // Use error-specific retry delay if available
                    let retry_delay = err.retry_after().map(Duration::from_secs).unwrap_or(delay);

                    info!("Retrying in {:?}", retry_delay);
                    sleep(retry_delay).await;

                    // Exponential backoff with jitter
                    delay = std::cmp::min(delay * 2, self.config.max_retry_delay);
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl ProviderClient for OpenAIClient {
    #[instrument(skip(self, request), fields(request_id = %request.id, model = %request.model))]
    async fn complete(&self, request: ProviderRequest) -> fennec_core::Result<ProviderResponse> {
        info!("Processing completion request for model: {}", request.model);

        let chat_request = ChatCompletionRequest {
            model: request.model.clone(),
            messages: request.messages.into_iter().map(Into::into).collect(),
            stream: Some(false),
            max_tokens: Some(4096), // Default max tokens
            temperature: Some(0.7), // Default temperature
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            user: None,
        };

        let response = self.chat_completion(chat_request).await?;

        if let Some(choice) = response.choices.first() {
            Ok(ProviderResponse {
                id: request.id,
                content: choice.message.content.clone(),
                usage: response.usage.map(Into::into),
            })
        } else {
            Err(ProviderError::Generic {
                message: "No choices in response".to_string(),
                provider: "openai".to_string(),
                context: None,
            }
            .into())
        }
    }

    #[instrument(skip(self, request), fields(request_id = %request.id, model = %request.model))]
    async fn stream(
        &self,
        request: ProviderRequest,
    ) -> fennec_core::Result<Box<dyn Stream<Item = fennec_core::Result<String>> + Unpin + Send>>
    {
        info!("Processing streaming request for model: {}", request.model);

        let chat_request = ChatCompletionRequest {
            model: request.model.clone(),
            messages: request.messages.into_iter().map(Into::into).collect(),
            stream: Some(true),
            max_tokens: Some(4096),
            temperature: Some(0.7),
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            user: None,
        };

        let stream = self.chat_completion_stream(chat_request).await?;

        let content_stream = stream
            .filter_map(|chunk_result| async move {
                match chunk_result {
                    Ok(chunk) => {
                        if let Some(choice) = chunk.choices.first() {
                            if let Some(content) = &choice.delta.content {
                                Some(Ok(content.clone()))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    Err(e) => Some(Err(e.into())),
                }
            })
            .boxed();

        Ok(Box::new(content_stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_openai_config_from_provider_config() {
        let provider_config = ProviderConfig {
            openai_api_key: Some("test-key".to_string()),
            default_model: "gpt-4".to_string(),
            base_url: Some("https://api.test.com/v1".to_string()),
            timeout_seconds: 60,
        };

        let openai_config = OpenAIConfig::from(&provider_config);
        assert_eq!(openai_config.api_key, "test-key");
        assert_eq!(openai_config.base_url, "https://api.test.com/v1");
        assert_eq!(openai_config.timeout, Duration::from_secs(60));
    }

    #[tokio::test]
    async fn test_client_creation_fails_without_api_key() {
        let config = OpenAIConfig {
            api_key: String::new(),
            ..Default::default()
        };

        let result = OpenAIClient::new(config);
        assert!(result.is_err());

        if let Err(ProviderError::ConfigurationMissing { provider }) = result {
            assert_eq!(provider, "openai");
        } else {
            panic!("Expected ConfigurationMissing error");
        }
    }
}
