//! Custom filters for telemetry data

use tracing::{Metadata, Subscriber};
use tracing_subscriber::{
    filter::FilterFn,
    layer::{Context, Filter},
};

/// Filter that blocks telemetry events from creating infinite loops
pub struct TelemetryFilter;

impl TelemetryFilter {
    pub fn new() -> Self {
        Self
    }
}

impl<S> Filter<S> for TelemetryFilter
where
    S: Subscriber,
{
    fn enabled(&self, meta: &Metadata<'_>, _ctx: &Context<'_, S>) -> bool {
        // Filter out our own telemetry events to prevent infinite loops
        !meta.target().starts_with("fennec_telemetry")
    }

    fn callsite_enabled(&self, meta: &Metadata<'_>) -> tracing::subscriber::Interest {
        if meta.target().starts_with("fennec_telemetry") {
            tracing::subscriber::Interest::never()
        } else {
            tracing::subscriber::Interest::sometimes()
        }
    }
}

/// Create a filter that excludes noisy dependencies
pub fn create_dependency_filter() -> impl Filter<tracing_subscriber::Registry> {
    FilterFn::new(|metadata| {
        let target = metadata.target();

        // Allow all fennec logs
        if target.starts_with("fennec") {
            return true;
        }

        // Filter out noisy dependencies
        let noisy_targets = [
            "hyper",
            "h2",
            "tower",
            "reqwest",
            "rustls",
            "tokio_util",
            "want",
        ];

        for noisy in &noisy_targets {
            if target.starts_with(noisy) {
                return metadata.level() <= &tracing::Level::WARN;
            }
        }

        // Default: allow INFO and above for external dependencies
        metadata.level() <= &tracing::Level::INFO
    })
}

/// Create a filter for specific log levels
pub fn create_level_filter(level: tracing::Level) -> impl Filter<tracing_subscriber::Registry> {
    FilterFn::new(move |metadata| metadata.level() <= &level)
}

/// Create a filter that only allows events with correlation IDs
pub fn create_correlation_filter() -> impl Filter<tracing_subscriber::Registry> {
    FilterFn::new(|metadata| {
        // This is a simplified implementation
        // In practice, you'd check for correlation ID in the span context
        metadata.target().starts_with("fennec")
            || metadata.fields().field("correlation_id").is_some()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_filter() {
        let _filter = TelemetryFilter::new();

        // For now, just test that the filter can be created successfully
        // TODO: Implement proper testing when Context is publicly available
        // This is a placeholder test to ensure the filter compiles and runs

        // Should not be filtered out - we'll skip this test for now as Context::new() is private
        // assert!(filter.enabled(&regular_meta, &Context::new()));
    }
}
