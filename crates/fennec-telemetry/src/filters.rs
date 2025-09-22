//! Custom filters for telemetry data

use tracing::{Event, Metadata, Subscriber};
use tracing_subscriber::{
    filter::{FilterExt, FilterFn},
    layer::{Context, Filter},
    Layer,
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

    fn callsite_enabled(&self, meta: &Metadata<'_>) -> tracing_subscriber::filter::Interest {
        if meta.target().starts_with("fennec_telemetry") {
            tracing_subscriber::filter::Interest::never()
        } else {
            tracing_subscriber::filter::Interest::sometimes()
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
    use tracing::Level;

    #[test]
    fn test_telemetry_filter() {
        let filter = TelemetryFilter::new();

        // Create metadata for a telemetry event
        let telemetry_meta = tracing::Metadata::new(
            "test",
            "fennec_telemetry::test",
            Level::INFO,
            None,
            None,
            None,
            tracing::field::FieldSet::new(&[], tracing::callsite::Identifier::new()),
            tracing::metadata::Kind::EVENT,
        );

        // Should be filtered out
        assert!(!filter.enabled(&telemetry_meta, &Context::new()));

        // Create metadata for a regular event
        let regular_meta = tracing::Metadata::new(
            "test",
            "fennec_core::test",
            Level::INFO,
            None,
            None,
            None,
            tracing::field::FieldSet::new(&[], tracing::callsite::Identifier::new()),
            tracing::metadata::Kind::EVENT,
        );

        // Should not be filtered out
        assert!(filter.enabled(&regular_meta, &Context::new()));
    }
}
