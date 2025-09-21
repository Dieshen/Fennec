use async_trait::async_trait;
use fennec_core::provider::{
    ProviderClient, ProviderMessage, ProviderRequest, ProviderResponse, Usage,
};
use fennec_core::Result;
use futures::stream;
use uuid::Uuid;

/// Simple fallback provider that echoes the last user message.
/// Useful for development when no external provider credentials are configured.
#[derive(Debug, Default)]
pub struct MockProviderClient;

#[async_trait]
impl ProviderClient for MockProviderClient {
    async fn complete(&self, request: ProviderRequest) -> Result<ProviderResponse> {
        let reply = generate_reply(&request.messages)?;
        Ok(ProviderResponse {
            id: Uuid::new_v4(),
            content: reply,
            usage: Some(Usage {
                prompt_tokens: request.messages.len() as u32 * 10,
                completion_tokens: 12,
                total_tokens: request.messages.len() as u32 * 10 + 12,
            }),
        })
    }

    async fn stream(
        &self,
        request: ProviderRequest,
    ) -> Result<Box<dyn futures::Stream<Item = Result<String>> + Unpin + Send>> {
        let reply = generate_reply(&request.messages)?;
        // Simulate incremental output.
        let parts: Vec<Result<String>> = reply
            .split_whitespace()
            .map(|word| Ok(format!("{} ", word)))
            .collect();
        Ok(Box::new(stream::iter(parts)))
    }
}

fn generate_reply(messages: &[ProviderMessage]) -> Result<String> {
    let fallback = "I'm running without a configured provider. Set OPENAI_API_KEY or another provider to get live responses.".to_string();
    let Some(last) = messages.iter().rev().find(|m| m.role == "user") else {
        return Ok(fallback);
    };

    Ok(format!(
        "(offline mode) I received: \"{}\"",
        last.content.trim()
    ))
}
