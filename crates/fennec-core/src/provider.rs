use crate::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRequest {
    pub id: Uuid,
    pub messages: Vec<ProviderMessage>,
    pub model: String,
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderResponse {
    pub id: Uuid,
    pub content: String,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

pub trait ProviderClient: Send + Sync {
    async fn complete(&self, request: ProviderRequest) -> Result<ProviderResponse>;
    async fn stream(&self, request: ProviderRequest) -> Result<Box<dyn futures::Stream<Item = Result<String>> + Unpin + Send>>;
}