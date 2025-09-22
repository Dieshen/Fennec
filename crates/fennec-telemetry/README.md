# Fennec Telemetry

Comprehensive telemetry, logging, and observability system for Fennec AI Assistant.

## Features

### 🔍 **Structured Logging**
- JSON, Pretty, and Compact output formats
- Configurable log levels (TRACE, DEBUG, INFO, WARN, ERROR)
- File-based and console logging
- Custom formatters with sanitization support

### 🔄 **Log Rotation & Retention**
- Size-based and time-based log rotation
- Configurable retention policies
- Automatic compression of old log files
- Disk space management with size limits

### 🛡️ **Privacy & Security**
- Automatic sanitization of sensitive data (API keys, passwords, PII)
- Configurable redaction patterns
- Field-based filtering
- Audit trail logging

### 📊 **Performance Metrics**
- Request timing and correlation tracking
- Span-based performance measurement
- Prometheus metrics export (optional)
- Custom metrics collection

### 🔗 **Request Correlation**
- Unique correlation IDs for request tracing
- Parent-child relationship tracking
- Cross-service request correlation
- Structured context propagation

## Quick Start

```rust
use fennec_telemetry::{TelemetrySystem, TelemetryConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize with default configuration
    let config = TelemetryConfig::default();
    let _guard = TelemetrySystem::init(config).await?;
    
    // Start logging
    tracing::info!("Application started");
    tracing::warn!(user_id = "123", "User performed action");
    
    Ok(())
}
```

## Configuration

### Basic Configuration

```rust
use fennec_telemetry::{TelemetryConfig, LogLevel, LogFormat};

let mut config = TelemetryConfig::default();

// Configure logging
config.logging.level = LogLevel::Debug;
config.logging.format = LogFormat::Json;
config.logging.file_enabled = true;
config.logging.log_dir = PathBuf::from("./logs");

// Configure metrics
config.metrics.enabled = true;
config.metrics.performance_timing = true;
config.metrics.correlation_tracking = true;

// Configure privacy
config.privacy.sanitize_enabled = true;
config.privacy.redacted_fields = vec!["password", "api_key", "secret"];
```

### TOML Configuration

```toml
[logging]
level = "Info"
format = "Pretty"
console_enabled = true
file_enabled = true
log_dir = "./logs"
log_file_name = "fennec"
max_file_size_mb = 100
include_timestamps = true
include_location = false
include_thread_info = false

[metrics]
enabled = true
performance_timing = true
correlation_tracking = true
prometheus_enabled = false
prometheus_port = 9090
collection_interval_seconds = 60

[retention]
max_files = 10
max_age_days = 30
compress_old_files = true
cleanup_interval_hours = 24
max_total_size_mb = 1024

[privacy]
sanitize_enabled = true
redaction_patterns = [
    "(?i)(api_?key|token|secret|password)\\s*[:=]\\s*['\"]?([a-zA-Z0-9_\\-\\.]+)['\"]?",
    "\\b\\d{4}[\\s\\-]?\\d{4}[\\s\\-]?\\d{4}[\\s\\-]?\\d{4}\\b"
]
redacted_fields = ["password", "api_key", "secret", "token"]
audit_trail = true
```

### Environment Variables

Override configuration with environment variables:

```bash
export FENNEC_TELEMETRY_ENABLED=true
export FENNEC_LOG_LEVEL=DEBUG
export FENNEC_LOG_FORMAT=json
export FENNEC_FILE_LOGGING=true
export FENNEC_LOG_DIR=./logs
export FENNEC_METRICS_ENABLED=true
export FENNEC_SANITIZE_LOGS=true
```

## Advanced Usage

### Request Correlation

```rust
use fennec_telemetry::correlation::{RequestContext, RequestTracker};

// Create request context
let context = RequestContext::new("user_login".to_string())
    .with_user_id("user123".to_string())
    .with_metadata("ip".to_string(), "192.168.1.1".to_string());

// Create span for tracing
let span = context.create_span();
let _enter = span.enter();

tracing::info!("User login started");

// Create child operation
let child_context = context.child("validate_credentials".to_string());
let child_span = child_context.create_span();
let _child_enter = child_span.enter();

tracing::info!("Validating user credentials");

// Complete operations
context.log_completion(true, None);
```

### Performance Metrics

```rust
use fennec_telemetry::metrics::MetricsUtil;

// Time a synchronous operation
let result = MetricsUtil::time_operation("database_query", || {
    // Your database query here
    perform_database_query()
});

// Time an asynchronous operation
let result = MetricsUtil::time_operation_async("api_call", || async {
    // Your async operation here
    make_api_call().await
}).await;

// Using macros
let result = timed!("file_processing", {
    process_file("data.txt")
});

let result = timed_async!("download", {
    download_file("https://example.com/file.zip").await
});
```

### Custom Sanitization

```rust
use fennec_telemetry::sanitization::{DataSanitizer, SanitizationPatterns};

// Create sanitizer with custom patterns
let mut patterns = SanitizationPatterns::all_default_patterns();
patterns.push(r"custom_pattern_\d+".to_string());

let sanitizer = DataSanitizer::new(&config.privacy)?;

// Sanitize text
let original = "User password=secret123 logged in";
let sanitized = sanitizer.sanitize_text(original);
// Result: "User password=[REDACTED] logged in"

// Sanitize JSON
let mut json = serde_json::json!({
    "username": "john",
    "password": "secret123",
    "api_key": "sk-1234567890"
});
json = sanitizer.sanitize_json(json);
```

### Log Retention Management

```rust
use fennec_telemetry::retention::RetentionManager;

// Create retention manager
let retention_manager = RetentionManager::new(config.retention).await?;

// Perform manual cleanup
let report = retention_manager
    .perform_cleanup(&log_dir, "fennec")
    .await?;

println!("Cleaned up {} files", report.total_files_removed());
println!("Freed {} MB", report.space_freed_mb());

// Emergency cleanup to specific size
let report = retention_manager
    .emergency_cleanup(&log_dir, "fennec", 100) // 100MB target
    .await?;
```

## CLI Integration

The telemetry system integrates seamlessly with Fennec's CLI:

```bash
# Basic logging control
fennec --log-level debug --log-format json

# File logging
fennec --file-logging --log-dir ./my-logs

# Telemetry control
fennec --telemetry --metrics

# Disable features
fennec --no-telemetry --no-file-logging

# Custom configuration
fennec --telemetry-config ./telemetry.toml

# Disable sanitization (UNSAFE)
fennec --no-sanitize
```

## Security Considerations

### Sensitive Data Protection

The telemetry system automatically sanitizes:

- **API Keys and Tokens**: `api_key=sk-1234567890` → `api_key=[REDACTED]`
- **Passwords**: `password=secret123` → `password=[REDACTED]`
- **Credit Cards**: `4532-1234-5678-9012` → `4532****9012`
- **Email Addresses**: `user@example.com` → `u***@e***.com`
- **SSN Numbers**: `123-45-6789` → `[REDACTED]`

### Custom Sanitization Patterns

Add custom patterns for your specific use case:

```rust
config.privacy.redaction_patterns.extend([
    r"(?i)session_id[:=]\s*([a-f0-9]{32})".to_string(),
    r"(?i)bearer\s+([a-zA-Z0-9_\-\.]+)".to_string(),
]);
```

### Audit Trail

Enable audit logging for compliance:

```rust
config.privacy.audit_trail = true;
config.privacy.audit_log_path = Some(PathBuf::from("./audit.log"));
```

## Performance

### Benchmarks

- **Logging Throughput**: ~100K messages/second (compact format)
- **JSON Formatting**: ~50K messages/second
- **Sanitization Overhead**: ~15% (when enabled)
- **Memory Usage**: ~2MB baseline + log buffers
- **File Rotation**: <1ms for typical log files

### Optimization Tips

1. **Use Compact Format**: For high-throughput scenarios
2. **Disable Location Info**: Reduces overhead significantly
3. **Batch Log Writes**: Automatic buffering optimizes I/O
4. **Configure Retention**: Prevent disk space issues
5. **Selective Sanitization**: Only enable for production

## Troubleshooting

### Common Issues

**Logs not appearing in files:**
```bash
# Check log directory permissions
ls -la ./logs

# Verify configuration
fennec --log-level debug --file-logging
```

**High memory usage:**
```rust
// Reduce retention window
config.retention.max_files = 5;
config.retention.max_age_days = 7;
```

**Slow performance:**
```rust
// Optimize for performance
config.logging.format = LogFormat::Compact;
config.logging.include_location = false;
config.privacy.sanitize_enabled = false; // Only in dev
```

### Debug Mode

Enable debug logging for the telemetry system:

```bash
export RUST_LOG=fennec_telemetry=debug,fennec=info
```

## Examples

See the `examples/` directory for complete examples:

- `basic_usage.rs` - Simple telemetry setup
- `advanced_config.rs` - Complex configuration
- `request_tracing.rs` - Correlation and tracing
- `metrics_collection.rs` - Performance metrics
- `custom_sanitization.rs` - Custom data protection

## Contributing

When adding new telemetry features:

1. **Add comprehensive tests** in `src/tests.rs`
2. **Update configuration** if adding new options
3. **Document security implications** for any data handling
4. **Benchmark performance impact** for core logging paths
5. **Add examples** for new functionality

## License

This crate is part of the Fennec project and is licensed under MIT OR Apache-2.0.