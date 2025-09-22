//! Request correlation and tracing implementation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{Span, Subscriber};
use tracing_subscriber::{layer::Context, Layer};
use uuid::Uuid;

/// Unique identifier for correlating related log entries and operations
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CorrelationId(String);

impl CorrelationId {
    /// Generate a new random correlation ID
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Create a correlation ID from a string
    pub fn from_string(id: String) -> Self {
        Self(id)
    }

    /// Get the correlation ID as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Create a child correlation ID for nested operations
    pub fn child(&self) -> Self {
        Self(format!("{}-{}", self.0, Uuid::new_v4().simple()))
    }
}

impl Default for CorrelationId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for CorrelationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Context for request tracking and correlation
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Unique correlation identifier
    pub correlation_id: CorrelationId,

    /// Request start time
    pub start_time: Instant,

    /// Request timestamp (wall clock time)
    pub timestamp: SystemTime,

    /// Operation name or description
    pub operation: String,

    /// User or session identifier
    pub user_id: Option<String>,

    /// Additional context metadata
    pub metadata: HashMap<String, String>,

    /// Parent correlation ID for nested operations
    pub parent_id: Option<CorrelationId>,
}

impl RequestContext {
    /// Create a new request context
    pub fn new(operation: String) -> Self {
        Self {
            correlation_id: CorrelationId::new(),
            start_time: Instant::now(),
            timestamp: SystemTime::now(),
            operation,
            user_id: None,
            metadata: HashMap::new(),
            parent_id: None,
        }
    }

    /// Create a child context for a nested operation
    pub fn child(&self, operation: String) -> Self {
        Self {
            correlation_id: self.correlation_id.child(),
            start_time: Instant::now(),
            timestamp: SystemTime::now(),
            operation,
            user_id: self.user_id.clone(),
            metadata: self.metadata.clone(),
            parent_id: Some(self.correlation_id.clone()),
        }
    }

    /// Set the user ID for this request
    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Add metadata to the request context
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Get the elapsed time since request start
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Get timestamp as milliseconds since epoch
    pub fn timestamp_millis(&self) -> u64 {
        self.timestamp
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// Create a tracing span for this request context
    pub fn create_span(&self) -> Span {
        tracing::info_span!(
            "request",
            correlation_id = %self.correlation_id,
            operation = %self.operation,
            user_id = %self.user_id.as_deref().unwrap_or("anonymous"),
            parent_id = %self.parent_id.as_ref().map(|p| p.as_str()).unwrap_or("none"),
            timestamp = self.timestamp_millis()
        )
    }

    /// Log the start of this request
    pub fn log_start(&self) {
        tracing::info!(
            telemetry.event = "request_started",
            correlation_id = %self.correlation_id,
            operation = %self.operation,
            user_id = %self.user_id.as_deref().unwrap_or("anonymous"),
            parent_id = %self.parent_id.as_ref().map(|p| p.as_str()).unwrap_or("none"),
            timestamp = self.timestamp_millis(),
            ?self.metadata,
            "Request started"
        );
    }

    /// Log the completion of this request
    pub fn log_completion(&self, success: bool, error_message: Option<&str>) {
        let elapsed_ms = self.elapsed().as_millis() as u64;

        if success {
            tracing::info!(
                telemetry.event = "request_completed",
                correlation_id = %self.correlation_id,
                operation = %self.operation,
                user_id = %self.user_id.as_deref().unwrap_or("anonymous"),
                elapsed_ms = elapsed_ms,
                success = true,
                "Request completed successfully"
            );
        } else {
            tracing::error!(
                telemetry.event = "request_failed",
                correlation_id = %self.correlation_id,
                operation = %self.operation,
                user_id = %self.user_id.as_deref().unwrap_or("anonymous"),
                elapsed_ms = elapsed_ms,
                success = false,
                error = %error_message.unwrap_or("unknown error"),
                "Request failed"
            );
        }
    }
}

/// Layer that adds correlation tracking to tracing spans
pub struct CorrelationLayer {
    active_requests: Arc<RwLock<HashMap<CorrelationId, RequestContext>>>,
}

impl CorrelationLayer {
    /// Create a new correlation layer
    pub fn new() -> Self {
        Self {
            active_requests: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start tracking a new request
    pub async fn start_request(&self, context: RequestContext) {
        context.log_start();
        let correlation_id = context.correlation_id.clone();
        self.active_requests
            .write()
            .await
            .insert(correlation_id, context);
    }

    /// Complete tracking for a request
    pub async fn complete_request(
        &self,
        correlation_id: &CorrelationId,
        success: bool,
        error_message: Option<&str>,
    ) {
        if let Some(context) = self.active_requests.write().await.remove(correlation_id) {
            context.log_completion(success, error_message);
        }
    }

    /// Get active request context by correlation ID
    pub async fn get_request_context(
        &self,
        correlation_id: &CorrelationId,
    ) -> Option<RequestContext> {
        self.active_requests
            .read()
            .await
            .get(correlation_id)
            .cloned()
    }

    /// Get all active requests
    pub async fn get_active_requests(&self) -> Vec<RequestContext> {
        self.active_requests
            .read()
            .await
            .values()
            .cloned()
            .collect()
    }

    /// Clean up old requests (in case completion wasn't called)
    pub async fn cleanup_old_requests(&self, max_age: Duration) {
        let cutoff = Instant::now() - max_age;
        let mut requests = self.active_requests.write().await;

        let old_requests: Vec<_> = requests
            .iter()
            .filter(|(_, context)| context.start_time < cutoff)
            .map(|(id, context)| (id.clone(), context.clone()))
            .collect();

        for (correlation_id, context) in old_requests {
            requests.remove(&correlation_id);
            tracing::warn!(
                telemetry.event = "request_timeout_cleanup",
                correlation_id = %correlation_id,
                operation = %context.operation,
                elapsed_ms = context.elapsed().as_millis() as u64,
                "Cleaned up old request context"
            );
        }
    }
}

impl Default for CorrelationLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Layer<S> for CorrelationLayer
where
    S: Subscriber + for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
{
    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: Context<'_, S>,
    ) {
        // Extract correlation ID if present in span attributes
        let span = ctx.span(id).expect("Span not found");

        // For now, we'll store basic span information
        // In a full implementation, we would extract and store correlation data
        span.extensions_mut().insert(SpanTiming::new());
    }

    fn on_enter(&self, id: &tracing::span::Id, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(id) {
            if let Some(timing) = span.extensions_mut().get_mut::<SpanTiming>() {
                timing.enter();
            }
        }
    }

    fn on_exit(&self, id: &tracing::span::Id, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(id) {
            if let Some(timing) = span.extensions_mut().get_mut::<SpanTiming>() {
                timing.exit();
            }
        }
    }

    fn on_close(&self, id: tracing::span::Id, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(&id) {
            if let Some(timing) = span.extensions().get::<SpanTiming>() {
                let total_time = timing.total_time();
                let self_time = timing.self_time();

                tracing::debug!(
                    telemetry.event = "span_completed",
                    span_name = %span.name(),
                    total_time_ms = total_time.as_millis() as u64,
                    self_time_ms = self_time.as_millis() as u64,
                    "Span timing recorded"
                );
            }
        }
    }
}

/// Tracks timing information for spans
#[derive(Debug)]
struct SpanTiming {
    start_time: Instant,
    total_time: Duration,
    self_time: Duration,
    entered_at: Option<Instant>,
}

impl SpanTiming {
    fn new() -> Self {
        Self {
            start_time: Instant::now(),
            total_time: Duration::ZERO,
            self_time: Duration::ZERO,
            entered_at: None,
        }
    }

    fn enter(&mut self) {
        self.entered_at = Some(Instant::now());
    }

    fn exit(&mut self) {
        if let Some(entered_at) = self.entered_at.take() {
            let duration = entered_at.elapsed();
            self.self_time += duration;
        }
    }

    fn total_time(&self) -> Duration {
        self.start_time.elapsed()
    }

    fn self_time(&self) -> Duration {
        let mut self_time = self.self_time;
        if let Some(entered_at) = self.entered_at {
            self_time += entered_at.elapsed();
        }
        self_time
    }
}

/// Utility for creating and managing request contexts
pub struct RequestTracker {
    correlation_layer: Arc<CorrelationLayer>,
}

impl RequestTracker {
    /// Create a new request tracker
    pub fn new(correlation_layer: Arc<CorrelationLayer>) -> Self {
        Self { correlation_layer }
    }

    /// Start tracking a new request
    pub async fn start_request(&self, operation: String) -> RequestGuard {
        let context = RequestContext::new(operation);
        let correlation_id = context.correlation_id.clone();

        self.correlation_layer.start_request(context).await;

        RequestGuard {
            correlation_id,
            correlation_layer: Arc::clone(&self.correlation_layer),
            completed: false,
        }
    }

    /// Start tracking a child request
    pub async fn start_child_request(
        &self,
        parent_id: &CorrelationId,
        operation: String,
    ) -> Option<RequestGuard> {
        if let Some(parent_context) = self.correlation_layer.get_request_context(parent_id).await {
            let context = parent_context.child(operation);
            let correlation_id = context.correlation_id.clone();

            self.correlation_layer.start_request(context).await;

            Some(RequestGuard {
                correlation_id,
                correlation_layer: Arc::clone(&self.correlation_layer),
                completed: false,
            })
        } else {
            None
        }
    }
}

/// RAII guard for request tracking that ensures completion is logged
pub struct RequestGuard {
    correlation_id: CorrelationId,
    correlation_layer: Arc<CorrelationLayer>,
    completed: bool,
}

impl RequestGuard {
    /// Get the correlation ID for this request
    pub fn correlation_id(&self) -> &CorrelationId {
        &self.correlation_id
    }

    /// Mark the request as successfully completed
    pub async fn complete_success(mut self) {
        self.correlation_layer
            .complete_request(&self.correlation_id, true, None)
            .await;
        self.completed = true;
    }

    /// Mark the request as failed with an error message
    pub async fn complete_error(mut self, error_message: &str) {
        self.correlation_layer
            .complete_request(&self.correlation_id, false, Some(error_message))
            .await;
        self.completed = true;
    }

    /// Get the request context
    pub async fn get_context(&self) -> Option<RequestContext> {
        self.correlation_layer
            .get_request_context(&self.correlation_id)
            .await
    }
}

impl Drop for RequestGuard {
    fn drop(&mut self) {
        if !self.completed {
            // Use a blocking approach for the drop implementation
            // In a real implementation, you might want to use a different strategy
            tracing::warn!(
                correlation_id = %self.correlation_id,
                "Request guard dropped without explicit completion"
            );
        }
    }
}

/// Macro for creating a traced request span
#[macro_export]
macro_rules! traced_request {
    ($operation:expr) => {{
        let context = $crate::correlation::RequestContext::new($operation.to_string());
        let span = context.create_span();
        let _guard = span.enter();
        context
    }};

    ($operation:expr, $user_id:expr) => {{
        let context = $crate::correlation::RequestContext::new($operation.to_string())
            .with_user_id($user_id.to_string());
        let span = context.create_span();
        let _guard = span.enter();
        context
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    #[test]
    fn test_correlation_id_generation() {
        let id1 = CorrelationId::new();
        let id2 = CorrelationId::new();

        assert_ne!(id1, id2);
        assert!(!id1.as_str().is_empty());
    }

    #[test]
    fn test_correlation_id_child() {
        let parent = CorrelationId::new();
        let child = parent.child();

        assert!(child.as_str().starts_with(parent.as_str()));
        assert_ne!(parent, child);
    }

    #[test]
    fn test_request_context_creation() {
        let context = RequestContext::new("test_operation".to_string());

        assert_eq!(context.operation, "test_operation");
        assert!(context.user_id.is_none());
        assert!(context.parent_id.is_none());
        assert!(context.metadata.is_empty());
    }

    #[test]
    fn test_request_context_child() {
        let parent =
            RequestContext::new("parent_op".to_string()).with_user_id("user123".to_string());

        let child = parent.child("child_op".to_string());

        assert_eq!(child.operation, "child_op");
        assert_eq!(child.user_id, Some("user123".to_string()));
        assert_eq!(child.parent_id, Some(parent.correlation_id.clone()));
    }

    #[tokio::test]
    async fn test_correlation_layer() {
        let layer = CorrelationLayer::new();
        let context = RequestContext::new("test_request".to_string());
        let correlation_id = context.correlation_id.clone();

        // Start tracking
        layer.start_request(context).await;

        // Verify it's tracked
        let active_requests = layer.get_active_requests().await;
        assert_eq!(active_requests.len(), 1);

        // Complete tracking
        layer.complete_request(&correlation_id, true, None).await;

        // Verify it's removed
        let active_requests = layer.get_active_requests().await;
        assert_eq!(active_requests.len(), 0);
    }

    #[tokio::test]
    async fn test_request_guard() {
        let layer = Arc::new(CorrelationLayer::new());
        let tracker = RequestTracker::new(layer);

        {
            let mut guard = tracker.start_request("test_operation".to_string()).await;

            // Verify the request is being tracked
            let context = guard.get_context().await.unwrap();
            assert_eq!(context.operation, "test_operation");

            // Complete successfully
            guard.complete_success().await;
        }

        // Verify no active requests remain
        let active_requests = tracker.correlation_layer.get_active_requests().await;
        assert_eq!(active_requests.len(), 0);
    }

    #[tokio::test]
    async fn test_cleanup_old_requests() {
        let layer = CorrelationLayer::new();
        let context = RequestContext::new("old_request".to_string());

        layer.start_request(context).await;

        // Simulate old request
        sleep(Duration::from_millis(10)).await;

        // Cleanup requests older than 5ms
        layer.cleanup_old_requests(Duration::from_millis(5)).await;

        // Should have been cleaned up
        let active_requests = layer.get_active_requests().await;
        assert_eq!(active_requests.len(), 0);
    }
}
