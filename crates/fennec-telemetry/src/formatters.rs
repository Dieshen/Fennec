//! Custom formatters for telemetry output

use crate::sanitization::DataSanitizer;
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fmt;
use tracing::{Event, Subscriber};
use tracing_subscriber::{
    fmt::{format::Writer, time::FormatTime, FmtContext, FormatEvent, FormatFields},
    registry::LookupSpan,
};

/// Custom JSON formatter with sanitization support
pub struct SanitizedJsonFormatter {
    sanitizer: Option<DataSanitizer>,
    include_timestamps: bool,
    include_location: bool,
    include_thread_info: bool,
}

impl SanitizedJsonFormatter {
    pub fn new() -> Self {
        Self {
            sanitizer: None,
            include_timestamps: true,
            include_location: false,
            include_thread_info: false,
        }
    }

    pub fn with_sanitizer(mut self, sanitizer: DataSanitizer) -> Self {
        self.sanitizer = Some(sanitizer);
        self
    }

    pub fn with_timestamps(mut self, include: bool) -> Self {
        self.include_timestamps = include;
        self
    }

    pub fn with_location(mut self, include: bool) -> Self {
        self.include_location = include;
        self
    }

    pub fn with_thread_info(mut self, include: bool) -> Self {
        self.include_thread_info = include;
        self
    }
}

impl<S, N> FormatEvent<S, N> for SanitizedJsonFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let mut fields = HashMap::new();
        let mut visitor = JsonVisitor::new(&mut fields);
        event.record(&mut visitor);

        let metadata = event.metadata();
        let mut json_event = json!({
            "level": metadata.level().to_string(),
            "target": metadata.target(),
            "message": visitor.message.unwrap_or_else(|| "".to_string()),
            "fields": fields,
        });

        if self.include_timestamps {
            json_event["timestamp"] = json!(Utc::now().to_rfc3339());
        }

        if self.include_location {
            if let Some(file) = metadata.file() {
                json_event["file"] = json!(file);
            }
            if let Some(line) = metadata.line() {
                json_event["line"] = json!(line);
            }
        }

        if self.include_thread_info {
            json_event["thread"] = json!(std::thread::current().name().unwrap_or("unnamed"));
            json_event["thread_id"] = json!(format!("{:?}", std::thread::current().id()));
        }

        // Add span information
        if let Some(scope) = ctx.event_scope() {
            let mut spans = Vec::new();
            for span in scope.from_root() {
                spans.push(json!({
                    "name": span.name(),
                    "target": span.metadata().target(),
                }));
            }
            if !spans.is_empty() {
                json_event["spans"] = json!(spans);
            }
        }

        // Apply sanitization if configured
        if let Some(ref sanitizer) = self.sanitizer {
            json_event = sanitizer.sanitize_json(json_event);
        }

        writeln!(writer, "{}", serde_json::to_string(&json_event).unwrap())?;
        Ok(())
    }
}

/// Custom compact formatter for high-performance logging
pub struct CompactFormatter {
    sanitizer: Option<DataSanitizer>,
    include_timestamps: bool,
}

impl CompactFormatter {
    pub fn new() -> Self {
        Self {
            sanitizer: None,
            include_timestamps: true,
        }
    }

    pub fn with_sanitizer(mut self, sanitizer: DataSanitizer) -> Self {
        self.sanitizer = Some(sanitizer);
        self
    }

    pub fn with_timestamps(mut self, include: bool) -> Self {
        self.include_timestamps = include;
        self
    }
}

impl<S, N> FormatEvent<S, N> for CompactFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let metadata = event.metadata();

        // Timestamp
        if self.include_timestamps {
            write!(writer, "{} ", Utc::now().format("%H:%M:%S%.3f"))?;
        }

        // Level
        write!(writer, "{:5} ", metadata.level())?;

        // Target (truncated)
        let target = metadata.target();
        let short_target = if target.len() > 20 {
            &target[target.len() - 20..]
        } else {
            target
        };
        write!(writer, "{:20} ", short_target)?;

        // Current span name
        if let Some(scope) = ctx.event_scope() {
            if let Some(span) = scope.into_iter().next() {
                write!(writer, "[{}] ", span.name())?;
            }
        }

        // Message and fields
        let mut fields = HashMap::new();
        let mut visitor = JsonVisitor::new(&mut fields);
        event.record(&mut visitor);

        let mut message = visitor.message.unwrap_or_else(|| "".to_string());

        // Apply sanitization
        if let Some(ref sanitizer) = self.sanitizer {
            message = sanitizer.sanitize_text(&message);
        }

        write!(writer, "{}", message)?;

        // Add important fields inline
        for (key, value) in fields {
            if key != "message" && !key.starts_with("telemetry.") {
                let value_str = match value {
                    Value::String(s) => s,
                    other => other.to_string(),
                };

                let sanitized_value = if let Some(ref sanitizer) = self.sanitizer {
                    sanitizer.sanitize_text(&value_str)
                } else {
                    value_str
                };

                write!(writer, " {}={}", key, sanitized_value)?;
            }
        }

        writeln!(writer)?;
        Ok(())
    }
}

/// Custom time formatter
pub struct PreciseTimeFormatter;

impl FormatTime for PreciseTimeFormatter {
    fn format_time(&self, w: &mut Writer<'_>) -> fmt::Result {
        let now: DateTime<Utc> = Utc::now();
        write!(w, "{}", now.format("%Y-%m-%d %H:%M:%S%.6f UTC"))
    }
}

/// Visitor for extracting fields into JSON format
struct JsonVisitor<'a> {
    fields: &'a mut HashMap<String, Value>,
    message: Option<String>,
}

impl<'a> JsonVisitor<'a> {
    fn new(fields: &'a mut HashMap<String, Value>) -> Self {
        Self {
            fields,
            message: None,
        }
    }
}

impl<'a> tracing::field::Visit for JsonVisitor<'a> {
    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        self.fields.insert(field.name().to_string(), json!(value));
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.fields.insert(field.name().to_string(), json!(value));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.fields.insert(field.name().to_string(), json!(value));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields.insert(field.name().to_string(), json!(value));
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        } else {
            self.fields.insert(field.name().to_string(), json!(value));
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        self.fields
            .insert(field.name().to_string(), json!(format!("{:?}", value)));
    }
}

/// Utility for creating structured log entries
pub struct StructuredLogger;

impl StructuredLogger {
    /// Create a structured info log
    pub fn info(event: &str, fields: HashMap<String, Value>) {
        tracing::info!(telemetry.event = event, ?fields, "Structured log event");
    }

    /// Create a structured error log
    pub fn error(event: &str, error: &str, fields: HashMap<String, Value>) {
        tracing::error!(
            telemetry.event = event,
            error = error,
            ?fields,
            "Structured error event"
        );
    }

    /// Create a structured performance log
    pub fn performance(operation: &str, duration_ms: u64, fields: HashMap<String, Value>) {
        tracing::info!(
            telemetry.event = "performance",
            operation = operation,
            duration_ms = duration_ms,
            ?fields,
            "Performance measurement"
        );
    }

    /// Create a structured audit log
    pub fn audit(action: &str, user_id: Option<&str>, fields: HashMap<String, Value>) {
        tracing::warn!(
            telemetry.event = "audit",
            action = action,
            user_id = user_id.unwrap_or("anonymous"),
            ?fields,
            "Audit trail event"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PrivacyConfig;
    use std::io;
    use tracing_subscriber::fmt::MakeWriter;

    struct TestWriter {
        buf: Vec<u8>,
    }

    impl TestWriter {
        fn new() -> Self {
            Self { buf: Vec::new() }
        }

        fn contents(&self) -> String {
            String::from_utf8_lossy(&self.buf).to_string()
        }
    }

    impl io::Write for TestWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.buf.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl MakeWriter<'_> for TestWriter {
        type Writer = Self;

        fn make_writer(&self) -> Self::Writer {
            Self::new()
        }
    }

    #[test]
    fn test_precise_time_formatter() {
        let formatter = PreciseTimeFormatter;
        let mut writer = Writer::new(&mut Vec::new());

        // Just test that it doesn't panic
        formatter.format_time(&mut writer).unwrap();
    }

    #[test]
    fn test_json_visitor() {
        let mut fields = HashMap::new();
        let mut visitor = JsonVisitor::new(&mut fields);

        visitor.record_str(
            &tracing::field::Field::new("test", tracing::callsite::Identifier::new()),
            "value",
        );
        visitor.record_i64(
            &tracing::field::Field::new("number", tracing::callsite::Identifier::new()),
            42,
        );
        visitor.record_bool(
            &tracing::field::Field::new("flag", tracing::callsite::Identifier::new()),
            true,
        );

        assert_eq!(fields.get("test"), Some(&json!("value")));
        assert_eq!(fields.get("number"), Some(&json!(42)));
        assert_eq!(fields.get("flag"), Some(&json!(true)));
    }

    #[test]
    fn test_structured_logger() {
        let mut fields = HashMap::new();
        fields.insert("key".to_string(), json!("value"));
        fields.insert("count".to_string(), json!(42));

        // These should not panic
        StructuredLogger::info("test_event", fields.clone());
        StructuredLogger::error("test_error", "Something went wrong", fields.clone());
        StructuredLogger::performance("test_op", 100, fields.clone());
        StructuredLogger::audit("test_action", Some("user123"), fields);
    }
}
