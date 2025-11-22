//! Performance metrics and instrumentation

use crate::{config::MetricsConfig, Error, Result};
use metrics::{
    counter, histogram, Counter, Gauge, Histogram, Key, KeyName, Metadata, Recorder,
    SharedString, Unit,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{debug, info, Subscriber};
use tracing_subscriber::{layer::Context, Layer};

/// Layer that collects performance metrics from tracing events
pub struct MetricsLayer {
    config: MetricsConfig,
    recorder: Arc<FennecMetricsRecorder>,
}

impl MetricsLayer {
    /// Create a new metrics layer
    pub fn new(config: MetricsConfig) -> Result<Self> {
        let recorder = Arc::new(FennecMetricsRecorder::new());

        // Install the global metrics recorder
        metrics::set_global_recorder((*recorder).clone()).map_err(|e| Error::System {
            message: format!("Failed to install metrics recorder: {}", e),
        })?;

        Ok(Self { config, recorder })
    }

    /// Get the metrics recorder for direct access
    pub fn recorder(&self) -> Arc<FennecMetricsRecorder> {
        Arc::clone(&self.recorder)
    }
}

impl<S> Layer<S> for MetricsLayer
where
    S: Subscriber + for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
{
    fn on_new_span(
        &self,
        _attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: Context<'_, S>,
    ) {
        if self.config.performance_timing {
            if let Some(span) = ctx.span(id) {
                span.extensions_mut().insert(SpanMetrics::new());
            }
        }
    }

    fn on_enter(&self, id: &tracing::span::Id, ctx: Context<'_, S>) {
        if self.config.performance_timing {
            if let Some(span) = ctx.span(id) {
                if let Some(metrics) = span.extensions_mut().get_mut::<SpanMetrics>() {
                    metrics.enter();
                }

                // Increment span entry counter
                counter!("fennec.spans.entered").increment(1);
            }
        }
    }

    fn on_exit(&self, id: &tracing::span::Id, ctx: Context<'_, S>) {
        if self.config.performance_timing {
            if let Some(span) = ctx.span(id) {
                if let Some(metrics) = span.extensions_mut().get_mut::<SpanMetrics>() {
                    metrics.exit();
                }
            }
        }
    }

    fn on_close(&self, id: tracing::span::Id, ctx: Context<'_, S>) {
        if self.config.performance_timing {
            if let Some(span) = ctx.span(&id) {
                if let Some(metrics) = span.extensions().get::<SpanMetrics>() {
                    let span_name = span.name();
                    let total_time = metrics.total_time();
                    let self_time = metrics.self_time();

                    // Record span duration histogram
                    histogram!("fennec.span.duration", "span" => span_name.to_string())
                        .record(total_time.as_millis() as f64);

                    // Record self time histogram
                    histogram!("fennec.span.self_time", "span" => span_name.to_string())
                        .record(self_time.as_millis() as f64);

                    // Increment completed spans counter
                    counter!("fennec.spans.completed", "span" => span_name.to_string())
                        .increment(1);

                    debug!(
                        telemetry.event = "span_metrics_recorded",
                        span_name = span_name,
                        total_time_ms = total_time.as_millis() as u64,
                        self_time_ms = self_time.as_millis() as u64,
                        "Span performance metrics recorded"
                    );
                }
            }
        }
    }

    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        // Count events by level
        let level = event.metadata().level();
        counter!("fennec.events.total", "level" => level.to_string()).increment(1);

        // Count specific telemetry events
        let mut visitor = MetricsEventVisitor::new();
        event.record(&mut visitor);

        if let Some(event_type) = visitor.telemetry_event {
            counter!("fennec.telemetry.events", "event_type" => event_type).increment(1);
        }
    }
}

/// Metrics recorder implementation for Fennec
#[derive(Clone)]
pub struct FennecMetricsRecorder {
    counters: Arc<Mutex<HashMap<Key, u64>>>,
    gauges: Arc<Mutex<HashMap<Key, f64>>>,
    histograms: Arc<Mutex<HashMap<Key, Vec<f64>>>>,
    start_time: Instant,
}

impl FennecMetricsRecorder {
    /// Create a new metrics recorder
    pub fn new() -> Self {
        Self {
            counters: Arc::new(Mutex::new(HashMap::new())),
            gauges: Arc::new(Mutex::new(HashMap::new())),
            histograms: Arc::new(Mutex::new(HashMap::new())),
            start_time: Instant::now(),
        }
    }

    /// Get current counter values
    pub fn get_counters(&self) -> HashMap<Key, u64> {
        self.counters.lock().unwrap().clone()
    }

    /// Get current gauge values
    pub fn get_gauges(&self) -> HashMap<Key, f64> {
        self.gauges.lock().unwrap().clone()
    }

    /// Get histogram samples
    pub fn get_histograms(&self) -> HashMap<Key, Vec<f64>> {
        self.histograms.lock().unwrap().clone()
    }

    /// Get metrics summary
    pub fn get_summary(&self) -> MetricsSummary {
        let counters = self.get_counters();
        let gauges = self.get_gauges();
        let histograms = self.get_histograms();

        let uptime = self.start_time.elapsed();
        let total_events = counters.values().sum();
        let total_spans = counters
            .iter()
            .filter(|(key, _)| key.name().starts_with("fennec.spans"))
            .map(|(_, value)| *value)
            .sum();

        MetricsSummary {
            uptime_seconds: uptime.as_secs(),
            total_events,
            total_spans,
            counter_count: counters.len(),
            gauge_count: gauges.len(),
            histogram_count: histograms.len(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.counters.lock().unwrap().clear();
        self.gauges.lock().unwrap().clear();
        self.histograms.lock().unwrap().clear();

        info!(
            telemetry.event = "metrics_reset",
            "All metrics have been reset"
        );
    }

    /// Export metrics in Prometheus format
    #[cfg(feature = "prometheus")]
    pub fn export_prometheus(&self) -> String {
        let mut output = String::new();

        // Export counters
        for (key, value) in self.get_counters() {
            output.push_str(&format!(
                "# TYPE {} counter\n{}{} {}\n",
                key.name(),
                key.name(),
                format_labels(key.labels()),
                value
            ));
        }

        // Export gauges
        for (key, value) in self.get_gauges() {
            output.push_str(&format!(
                "# TYPE {} gauge\n{}{} {}\n",
                key.name(),
                key.name(),
                format_labels(key.labels()),
                value
            ));
        }

        // Export histograms (simplified - just count and sum)
        for (key, samples) in self.get_histograms() {
            let count = samples.len();
            let sum: f64 = samples.iter().sum();

            output.push_str(&format!(
                "# TYPE {} histogram\n{}_count{} {}\n{}_sum{} {}\n",
                key.name(),
                key.name(),
                format_labels(key.labels()),
                count,
                key.name(),
                format_labels(key.labels()),
                sum
            ));
        }

        output
    }
}

impl Recorder for FennecMetricsRecorder {
    fn describe_counter(&self, _key: KeyName, _unit: Option<Unit>, _description: SharedString) {
        // Description is stored but not used in this implementation
    }

    fn describe_gauge(&self, _key: KeyName, _unit: Option<Unit>, _description: SharedString) {
        // Description is stored but not used in this implementation
    }

    fn describe_histogram(&self, _key: KeyName, _unit: Option<Unit>, _description: SharedString) {
        // Description is stored but not used in this implementation
    }

    fn register_counter(&self, key: &Key, _metadata: &Metadata<'_>) -> Counter {
        let counters = Arc::clone(&self.counters);
        let key = key.clone();

        // Initialize counter in map
        counters.lock().unwrap().insert(key.clone(), 0);

        // Use AtomicU64 for the counter implementation
        let atomic = std::sync::atomic::AtomicU64::new(0);
        Counter::from_arc(Arc::new(atomic))
    }

    fn register_gauge(&self, key: &Key, _metadata: &Metadata<'_>) -> Gauge {
        let gauges = Arc::clone(&self.gauges);
        let key = key.clone();

        // Initialize gauge in map
        gauges.lock().unwrap().insert(key, 0.0);

        // Use AtomicU64 for the gauge implementation (will store f64 bits)
        let atomic = std::sync::atomic::AtomicU64::new(0);
        Gauge::from_arc(Arc::new(atomic))
    }

    fn register_histogram(&self, key: &Key, _metadata: &Metadata<'_>) -> Histogram {
        let histograms = Arc::clone(&self.histograms);
        let key = key.clone();

        // Initialize histogram in map
        histograms.lock().unwrap().insert(key.clone(), Vec::new());

        // Create a simple histogram implementation
        let histogram_impl = SimpleHistogramImpl { histograms, key };

        Histogram::from_arc(Arc::new(histogram_impl))
    }
}

/// Simple histogram implementation that stores values
struct SimpleHistogramImpl {
    histograms: Arc<Mutex<HashMap<Key, Vec<f64>>>>,
    key: Key,
}

impl metrics::HistogramFn for SimpleHistogramImpl {
    fn record(&self, value: f64) {
        if let Ok(mut histograms) = self.histograms.lock() {
            histograms
                .entry(self.key.clone())
                .or_insert_with(Vec::new)
                .push(value);
        }
    }
}

/// Tracks timing metrics for spans
#[derive(Debug)]
struct SpanMetrics {
    start_time: Instant,
    #[allow(dead_code)]
    total_time: Duration,
    self_time: Duration,
    entered_at: Option<Instant>,
    entry_count: u32,
}

impl SpanMetrics {
    fn new() -> Self {
        Self {
            start_time: Instant::now(),
            total_time: Duration::ZERO,
            self_time: Duration::ZERO,
            entered_at: None,
            entry_count: 0,
        }
    }

    fn enter(&mut self) {
        self.entered_at = Some(Instant::now());
        self.entry_count += 1;
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

/// Visitor for extracting telemetry events from log records
struct MetricsEventVisitor {
    telemetry_event: Option<String>,
}

impl MetricsEventVisitor {
    fn new() -> Self {
        Self {
            telemetry_event: None,
        }
    }
}

impl tracing::field::Visit for MetricsEventVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "telemetry.event" {
            self.telemetry_event = Some(value.to_string());
        }
    }

    fn record_debug(&mut self, _field: &tracing::field::Field, _value: &dyn std::fmt::Debug) {
        // Not implemented for this visitor
    }
}

/// Summary of current metrics
#[derive(Debug, Clone)]
pub struct MetricsSummary {
    pub uptime_seconds: u64,
    pub total_events: u64,
    pub total_spans: u64,
    pub counter_count: usize,
    pub gauge_count: usize,
    pub histogram_count: usize,
    pub timestamp: u64,
}

/// Utility functions for metrics
pub struct MetricsUtil;

impl MetricsUtil {
    /// Record operation timing
    pub fn time_operation<F, R>(operation_name: &str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start = Instant::now();
        let result = f();
        let duration = start.elapsed();

        histogram!("fennec.operation.duration", "operation" => operation_name.to_string())
            .record(duration.as_millis() as f64);

        counter!("fennec.operations.completed", "operation" => operation_name.to_string())
            .increment(1);

        result
    }

    /// Record operation timing async
    pub async fn time_operation_async<F, Fut, R>(operation_name: &str, f: F) -> R
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = R>,
    {
        let start = Instant::now();
        let result = f().await;
        let duration = start.elapsed();

        histogram!("fennec.operation.duration", "operation" => operation_name.to_string())
            .record(duration.as_millis() as f64);

        counter!("fennec.operations.completed", "operation" => operation_name.to_string())
            .increment(1);

        result
    }

    /// Record memory usage
    pub fn record_memory_usage() {
        // This is a simplified implementation
        // In a real system, you would use proper memory measurement
        #[cfg(target_os = "linux")]
        {
            if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
                for line in status.lines() {
                    if line.starts_with("VmRSS:") {
                        if let Some(kb_str) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = kb_str.parse::<f64>() {
                                metrics::gauge!("fennec.memory.rss_kb").set(kb);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Record system load
    pub fn record_system_load() {
        // This is a simplified implementation
        #[cfg(target_os = "linux")]
        {
            if let Ok(loadavg) = std::fs::read_to_string("/proc/loadavg") {
                if let Some(load1_str) = loadavg.split_whitespace().next() {
                    if let Ok(load1) = load1_str.parse::<f64>() {
                        metrics::gauge!("fennec.system.load1").set(load1);
                    }
                }
            }
        }
    }
}

#[cfg(feature = "prometheus")]
fn format_labels(labels: std::slice::Iter<'_, metrics::Label>) -> String {
    let label_pairs: Vec<String> = labels
        .map(|label| format!("{}=\"{}\"", label.key(), label.value()))
        .collect();
    if label_pairs.is_empty() {
        String::new()
    } else {
        format!("{{{}}}", label_pairs.join(","))
    }
}

/// Macros for convenient metrics recording
#[macro_export]
macro_rules! timed {
    ($operation:expr, $code:block) => {
        $crate::metrics::MetricsUtil::time_operation($operation, || $code)
    };
}

#[macro_export]
macro_rules! timed_async {
    ($operation:expr, $code:block) => {
        $crate::metrics::MetricsUtil::time_operation_async($operation, || async move $code).await
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    #[test]
    fn test_metrics_recorder() {
        let recorder = FennecMetricsRecorder::new();

        // Test counter
        let counter = recorder.register_counter(
            &Key::from_name("test_counter"),
            &Metadata::new(
                "test",
                metrics::Level::INFO,
                Some("test_counter_description"),
            ),
        );
        counter.increment(5);

        // Test gauge
        let gauge = recorder.register_gauge(
            &Key::from_name("test_gauge"),
            &Metadata::new("test", metrics::Level::INFO, Some("test_gauge_description")),
        );
        gauge.set(42.0);
    }

    #[test]
    fn test_span_metrics() {
        let mut metrics = SpanMetrics::new();

        // Simulate span lifecycle
        metrics.enter();
        std::thread::sleep(Duration::from_millis(10));
        metrics.exit();

        assert!(metrics.self_time() >= Duration::from_millis(10));
        assert!(metrics.total_time() >= Duration::from_millis(10));
        assert_eq!(metrics.entry_count, 1);
    }

    #[test]
    fn test_metrics_util_timing() {
        let result = MetricsUtil::time_operation("test_operation", || {
            std::thread::sleep(Duration::from_millis(10));
            42
        });

        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_metrics_util_timing_async() {
        let result = MetricsUtil::time_operation_async("test_async_operation", || async {
            sleep(Duration::from_millis(10)).await;
            42
        })
        .await;

        assert_eq!(result, 42);
    }

    #[test]
    fn test_metrics_summary() {
        let recorder = FennecMetricsRecorder::new();

        // Add some test metrics
        let counter = recorder.register_counter(
            &Key::from_name("test_counter"),
            &Metadata::new(
                "test",
                metrics::Level::INFO,
                Some("test_counter_description"),
            ),
        );
        counter.increment(10);

        let summary = recorder.get_summary();

        assert_eq!(summary.counter_count, 1);
        assert_eq!(summary.gauge_count, 0);
        assert_eq!(summary.histogram_count, 0);
    }

    #[cfg(feature = "prometheus")]
    #[test]
    fn test_prometheus_export() {
        let recorder = FennecMetricsRecorder::new();

        // Add test metrics
        let counter = recorder.register_counter(
            &Key::from_name("test_counter"),
            &Metadata::new(
                "test",
                metrics::Level::INFO,
                Some("test_counter_description"),
            ),
        );
        counter.increment(5);

        let prometheus_output = recorder.export_prometheus();

        assert!(prometheus_output.contains("test_counter"));
    }
}
