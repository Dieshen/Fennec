# Fennec Provider Architecture

## Provider Abstraction Layer

Fennec implements a clean provider abstraction that allows seamless integration with multiple LLM providers while maintaining consistent behavior and streaming capabilities.

## Core Provider Traits

### StreamingProvider Trait
**Purpose**: Define common interface for all LLM providers
**Key Methods**:
```rust
#[async_trait]
pub trait StreamingProvider: Send + Sync {
    async fn stream_completion(
        &self,
        request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<CompletionChunk>>>>>;

    async fn get_models(&self) -> Result<Vec<ModelInfo>>;

    fn supports_streaming(&self) -> bool;

    fn provider_name(&self) -> &'static str;
}
```

### Provider Configuration
**Configuration Structure**:
```rust
pub struct ProviderConfig {
    pub provider_type: ProviderType,
    pub api_key: String,
    pub base_url: Option<String>,
    pub model: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f64>,
    pub timeout: Duration,
    pub retry_config: RetryConfig,
}

pub enum ProviderType {
    OpenAI,
    Anthropic,     // Planned
    OpenRouter,    // Planned
    Ollama,        // Planned
    Azure,         // Planned
}
```

## Current Provider Implementations

### OpenAI Provider
**Status**: ✅ Fully Implemented
**Features**:
- **Streaming Support**: Real-time response streaming
- **Multiple Models**: GPT-3.5, GPT-4, GPT-4 Turbo support
- **Error Handling**: Comprehensive error mapping and retry logic
- **Rate Limiting**: Built-in rate limiting and backoff
- **Function Calling**: Support for tool/function calling

**Implementation Details**:
```rust
pub struct OpenAIProvider {
    client: reqwest::Client,
    config: OpenAIConfig,
    rate_limiter: RateLimiter,
}

impl StreamingProvider for OpenAIProvider {
    async fn stream_completion(
        &self,
        request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<CompletionChunk>>>>> {
        // Implementation with Server-Sent Events (SSE)
        let stream = self.client
            .post(&self.config.api_url())
            .headers(self.build_headers()?)
            .json(&self.build_request(request)?)
            .send()
            .await?
            .bytes_stream()
            .map(|chunk| self.parse_sse_chunk(chunk));

        Ok(Box::pin(stream))
    }
}
```

**Configuration Example**:
```toml
[provider.openai]
api_key = "${OPENAI_API_KEY}"
model = "gpt-4"
max_tokens = 4096
temperature = 0.7
timeout = "30s"

[provider.openai.retry]
max_attempts = 3
initial_delay = "1s"
max_delay = "30s"
```

## Planned Provider Implementations

### Anthropic (Claude)
**Status**: 🚧 Planned
**Target Features**:
- **Claude Models**: Claude-3, Claude-3.5, Claude-4 support
- **Streaming API**: Native streaming response support
- **Message Format**: Anthropic's message-based API
- **Safety Features**: Built-in safety and content filtering

**Planned Implementation**:
```rust
pub struct AnthropicProvider {
    client: reqwest::Client,
    config: AnthropicConfig,
}

// Configuration
[provider.anthropic]
api_key = "${ANTHROPIC_API_KEY}"
model = "claude-3-sonnet"
max_tokens = 4096
```

### OpenRouter
**Status**: 🚧 Planned
**Target Features**:
- **Multiple Models**: Access to various models through single API
- **Model Selection**: Dynamic model selection based on task
- **Cost Optimization**: Automatic model selection for cost efficiency
- **Fallback Support**: Fallback to alternative models on failure

**Planned Implementation**:
```rust
pub struct OpenRouterProvider {
    client: reqwest::Client,
    config: OpenRouterConfig,
    model_selector: ModelSelector,
}
```

### Ollama (Local LLMs)
**Status**: 🚧 Planned
**Target Features**:
- **Local Execution**: Run LLMs locally without internet
- **Model Management**: Download and manage local models
- **Privacy**: Complete privacy with no external API calls
- **Performance**: Optimized for local hardware

**Planned Implementation**:
```rust
pub struct OllamaProvider {
    client: reqwest::Client,
    config: OllamaConfig,
    model_manager: ModelManager,
}

// Configuration
[provider.ollama]
base_url = "http://localhost:11434"
model = "codellama"
gpu_layers = 35
```

### Azure OpenAI
**Status**: 🚧 Planned
**Target Features**:
- **Enterprise Integration**: Azure AD authentication
- **Compliance**: Enterprise compliance and data governance
- **Regional Deployment**: Region-specific deployments
- **Managed Service**: Fully managed Azure service

## Provider Management

### Provider Registry
**Dynamic Loading**:
```rust
pub struct ProviderRegistry {
    providers: HashMap<ProviderType, Box<dyn StreamingProvider>>,
    default_provider: ProviderType,
}

impl ProviderRegistry {
    pub fn register_provider(
        &mut self,
        provider_type: ProviderType,
        provider: Box<dyn StreamingProvider>,
    ) {
        self.providers.insert(provider_type, provider);
    }

    pub async fn get_provider(
        &self,
        provider_type: Option<ProviderType>,
    ) -> Result<&dyn StreamingProvider> {
        let provider_type = provider_type.unwrap_or(self.default_provider);
        self.providers.get(&provider_type)
            .ok_or(ProviderError::NotFound(provider_type))
    }
}
```

### Provider Selection
**Selection Strategies**:
- **Manual**: User-specified provider for session
- **Automatic**: Best provider based on task requirements
- **Fallback**: Automatic fallback on provider failure
- **Cost-Optimized**: Cheapest provider for task requirements

**Selection Logic**:
```rust
pub struct ProviderSelector {
    registry: ProviderRegistry,
    selection_strategy: SelectionStrategy,
}

pub enum SelectionStrategy {
    Manual(ProviderType),
    Automatic,
    CostOptimized,
    Fallback(Vec<ProviderType>),
}
```

## Streaming and Response Processing

### Streaming Architecture
**Server-Sent Events (SSE)**:
```rust
pub struct CompletionChunk {
    pub content: Option<String>,
    pub role: Option<Role>,
    pub finish_reason: Option<FinishReason>,
    pub usage: Option<TokenUsage>,
    pub metadata: ChunkMetadata,
}

pub enum FinishReason {
    Stop,
    Length,
    ContentFilter,
    ToolCalls,
}
```

**Stream Processing**:
- **Real-time Updates**: Immediate UI updates as content arrives
- **Error Handling**: Graceful handling of stream interruptions
- **Backpressure**: Handle slow consumer scenarios
- **Reconnection**: Automatic reconnection on network issues

### Response Aggregation
**Chunk Assembly**:
```rust
pub struct ResponseAggregator {
    chunks: Vec<CompletionChunk>,
    current_content: String,
    metadata: ResponseMetadata,
}

impl ResponseAggregator {
    pub fn add_chunk(&mut self, chunk: CompletionChunk) {
        if let Some(content) = chunk.content {
            self.current_content.push_str(&content);
        }
        self.chunks.push(chunk);
    }

    pub fn finalize(self) -> CompletionResponse {
        CompletionResponse {
            content: self.current_content,
            metadata: self.metadata,
            usage: self.calculate_usage(),
        }
    }
}
```

## Error Handling and Reliability

### Error Types
```rust
#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("Provider not found: {0:?}")]
    NotFound(ProviderType),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Rate limit exceeded: retry after {retry_after:?}")]
    RateLimit { retry_after: Option<Duration> },

    #[error("Model not available: {model}")]
    ModelUnavailable { model: String },

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Streaming error: {0}")]
    Streaming(String),
}
```

### Retry Logic
**Exponential Backoff**:
```rust
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_factor: f64,
}

impl RetryConfig {
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let delay = self.initial_delay.as_secs_f64()
            * self.backoff_factor.powi(attempt as i32);
        Duration::from_secs_f64(delay.min(self.max_delay.as_secs_f64()))
    }
}
```

### Circuit Breaker Pattern
**Failure Detection**:
```rust
pub struct CircuitBreaker {
    state: CircuitState,
    failure_count: u32,
    threshold: u32,
    timeout: Duration,
    last_failure: Option<Instant>,
}

pub enum CircuitState {
    Closed,    // Normal operation
    Open,      // Failing, reject requests
    HalfOpen,  // Testing if service recovered
}
```

## Performance and Monitoring

### Metrics Collection
**Provider Metrics**:
```rust
pub struct ProviderMetrics {
    pub request_count: u64,
    pub success_count: u64,
    pub error_count: u64,
    pub avg_response_time: Duration,
    pub token_usage: TokenUsage,
    pub rate_limit_hits: u64,
}
```

**Performance Monitoring**:
- **Response Times**: Track request/response latency
- **Token Usage**: Monitor token consumption and costs
- **Error Rates**: Track error patterns and recovery
- **Throughput**: Measure requests per second

### Caching Strategy
**Response Caching**:
```rust
pub struct ResponseCache {
    cache: HashMap<RequestHash, CachedResponse>,
    ttl: Duration,
    max_size: usize,
}

pub struct CachedResponse {
    response: CompletionResponse,
    timestamp: Instant,
    access_count: u64,
}
```

**Cache Policies**:
- **TTL-based**: Time-to-live expiration
- **LRU**: Least recently used eviction
- **Size-limited**: Maximum cache size limits
- **Semantic**: Cache based on request similarity

## Security Considerations

### API Key Management
**Secure Storage**:
- **Environment Variables**: Development and CI environments
- **OS Keyring**: Production deployments
- **Encrypted Storage**: Optional encryption for sensitive projects
- **Rotation Support**: Automatic key rotation capabilities

### Request/Response Filtering
**Content Filtering**:
- **Input Sanitization**: Remove sensitive data from requests
- **Response Filtering**: Filter sensitive content from responses
- **Audit Logging**: Log all provider interactions
- **Privacy Controls**: Opt-out of data collection

### Network Security
**TLS and Certificates**:
- **Certificate Validation**: Strict certificate validation
- **TLS 1.3**: Modern TLS version enforcement
- **Proxy Support**: Corporate proxy integration
- **Timeout Handling**: Prevent hanging requests

---

*This provider architecture ensures reliable, performant, and secure integration with multiple LLM providers while maintaining a consistent developer experience.*