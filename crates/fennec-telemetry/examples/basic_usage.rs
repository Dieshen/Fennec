//! Basic telemetry usage example
//!
//! This example demonstrates the basic setup and usage of the Fennec telemetry system.
//!
//! Run with: cargo run --example basic_usage

use fennec_telemetry::{
    correlation::RequestContext, metrics::MetricsUtil, timed, timed_async, LogFormat, LogLevel,
    TelemetryConfig, TelemetrySystem,
};
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🚀 Starting Fennec Telemetry Basic Usage Example");

    // Create a custom telemetry configuration
    let mut config = TelemetryConfig::default();

    // Configure logging
    config.logging.level = LogLevel::Info;
    config.logging.format = LogFormat::Pretty;
    config.logging.file_enabled = true;
    config.logging.console_enabled = true;
    config.logging.log_dir = PathBuf::from("./example_logs");
    config.logging.include_timestamps = true;

    // Configure metrics
    config.metrics.enabled = true;
    config.metrics.performance_timing = true;
    config.metrics.correlation_tracking = true;

    // Configure privacy (enable sanitization)
    config.privacy.sanitize_enabled = true;

    println!("📋 Initializing telemetry system...");

    // Initialize the telemetry system
    let _guard = TelemetrySystem::init(config).await?;

    println!("✅ Telemetry system initialized successfully!");

    // Basic logging examples
    tracing::info!("Starting basic logging examples");

    tracing::debug!("This is a debug message");
    tracing::info!("This is an info message");
    tracing::warn!("This is a warning message");
    tracing::error!("This is an error message");

    // Structured logging with fields
    tracing::info!(
        user_id = "user123",
        action = "login",
        ip_address = "192.168.1.100",
        success = true,
        "User login event"
    );

    // Demonstrate telemetry events
    tracing::info!(
        telemetry.event = "application_started",
        version = env!("CARGO_PKG_VERSION"),
        "Application has started successfully"
    );

    // Request correlation example
    println!("🔗 Demonstrating request correlation...");

    let request_context = RequestContext::new("user_registration".to_string())
        .with_user_id("new_user_456".to_string())
        .with_metadata("source".to_string(), "web_app".to_string())
        .with_metadata("referrer".to_string(), "https://example.com".to_string());

    let span = request_context.create_span();
    let _span_guard = span.enter();

    request_context.log_start();

    tracing::info!("Processing user registration");

    // Simulate some work
    sleep(Duration::from_millis(100)).await;

    // Create a child operation
    let validation_context = request_context.child("email_validation".to_string());
    let validation_span = validation_context.create_span();
    let _validation_guard = validation_span.enter();

    tracing::info!("Validating email address");
    sleep(Duration::from_millis(50)).await;

    drop(_validation_guard);
    drop(validation_span);

    tracing::info!("Registration completed successfully");

    request_context.log_completion(true, None);

    drop(_span_guard);
    drop(span);

    // Performance metrics examples
    println!("📊 Demonstrating performance metrics...");

    // Timed synchronous operation
    let result = MetricsUtil::time_operation("data_processing", || {
        // Simulate some CPU-intensive work
        std::thread::sleep(Duration::from_millis(75));
        "processed_data"
    });

    tracing::info!(result = result, "Synchronous operation completed");

    // Timed asynchronous operation
    let result = MetricsUtil::time_operation_async("api_call", || async {
        // Simulate an async API call
        sleep(Duration::from_millis(120)).await;
        "api_response"
    })
    .await;

    tracing::info!(result = result, "Asynchronous operation completed");

    // Using the timed macros
    let computation_result = timed!("complex_computation", {
        let mut sum = 0;
        for i in 0..1000000 {
            sum += i;
        }
        sum
    });

    tracing::info!(result = computation_result, "Complex computation completed");

    let async_result = timed_async!("async_computation", {
        let mut results = Vec::new();
        for i in 0..10 {
            sleep(Duration::from_millis(10)).await;
            results.push(i * 2);
        }
        results.len()
    });

    tracing::info!(result = async_result, "Async computation completed");

    // Demonstrate sensitive data sanitization
    println!("🛡️ Demonstrating data sanitization...");

    // These should be automatically sanitized in the logs
    tracing::warn!(
        api_key = "sk-1234567890abcdef",
        user_email = "user@example.com",
        "Simulating potentially sensitive data logging"
    );

    tracing::error!(
        password = "supersecret123",
        token = "bearer_token_12345",
        credit_card = "4532-1234-5678-9012",
        error = "Authentication failed",
        "Simulating error with sensitive data"
    );

    // Error handling example
    println!("❌ Demonstrating error handling...");

    let error_context = RequestContext::new("file_processing".to_string())
        .with_metadata("file_name".to_string(), "important_data.txt".to_string());

    error_context.log_start();

    // Simulate an error scenario
    sleep(Duration::from_millis(30)).await;

    tracing::error!(
        telemetry.event = "file_processing_failed",
        error = "File not found",
        file_path = "/tmp/important_data.txt",
        "Failed to process file"
    );

    error_context.log_completion(false, Some("File not found"));

    // Nested span example
    println!("🎯 Demonstrating nested spans...");

    let outer_span = tracing::info_span!("request_handler", request_id = "req_789");
    let _outer_guard = outer_span.enter();

    tracing::info!("Handling incoming request");

    {
        let auth_span = tracing::info_span!("authentication", user_id = "user789");
        let _auth_guard = auth_span.enter();

        tracing::info!("Authenticating user");
        sleep(Duration::from_millis(25)).await;
        tracing::info!("User authenticated successfully");
    }

    {
        let db_span = tracing::info_span!("database_query", table = "users");
        let _db_guard = db_span.enter();

        tracing::info!("Executing database query");
        sleep(Duration::from_millis(40)).await;
        tracing::info!("Database query completed");
    }

    tracing::info!("Request handled successfully");

    // Wait a bit for all async operations to complete
    sleep(Duration::from_millis(200)).await;

    println!("📈 Getting telemetry statistics...");
    let stats = TelemetrySystem::get_stats().await;
    println!("Telemetry Stats: {:?}", stats);

    println!("💾 Flushing telemetry data...");
    TelemetrySystem::flush().await?;

    println!("✨ Example completed successfully!");
    println!("📁 Check './example_logs/' directory for log files");

    Ok(())
}
