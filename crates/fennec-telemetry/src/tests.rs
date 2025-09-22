//! Comprehensive tests for the telemetry system

#[cfg(test)]
mod integration_tests {
    use crate::{
        config::{LogFormat, LogLevel, TelemetryConfig},
        correlation::RequestContext,
        metrics::MetricsUtil,
        retention::RetentionManager,
        system::TelemetrySystem,
    };
    use std::time::Duration;
    use tempfile::TempDir;
    use tokio::time::sleep;

    #[tokio::test]
    #[cfg_attr(
        target_os = "windows",
        ignore = "File logging test is flaky on Windows"
    )]
    async fn test_full_telemetry_system_initialization() {
        let temp_dir = TempDir::new().unwrap();

        let mut config = TelemetryConfig::default();
        config.logging.log_dir = temp_dir.path().to_path_buf();
        config.logging.file_enabled = true;
        config.metrics.enabled = true;
        config.privacy.sanitize_enabled = true;

        let _guard = TelemetrySystem::init(config).await.unwrap();

        // Test that logging works
        tracing::info!(
            telemetry.event = "test_event",
            user_id = "test_user",
            operation = "test_operation",
            "Test message for integration test"
        );

        tracing::warn!(
            correlation_id = "test-correlation-123",
            "Test warning with correlation"
        );

        tracing::error!(error = "Test error", code = 500, "Test error message");

        // Allow time for async operations
        sleep(Duration::from_millis(100)).await;

        // Verify log file was created
        let log_file = temp_dir.path().join("fennec.log");
        assert!(log_file.exists());
    }

    #[tokio::test]
    #[cfg_attr(
        target_os = "windows",
        ignore = "File logging test is flaky on Windows"
    )]
    async fn test_telemetry_with_different_formats() {
        for format in [LogFormat::Json, LogFormat::Pretty, LogFormat::Compact] {
            let temp_dir = TempDir::new().unwrap();

            let mut config = TelemetryConfig::default();
            config.logging.log_dir = temp_dir.path().to_path_buf();
            config.logging.format = format;
            config.logging.file_enabled = true;
            config.logging.console_enabled = false; // Disable console to avoid conflicts

            let _guard = TelemetrySystem::init(config).await.unwrap();

            tracing::info!(
                test_format = ?format,
                "Testing format integration"
            );

            sleep(Duration::from_millis(50)).await;

            let log_file = temp_dir.path().join("fennec.log");
            assert!(log_file.exists());

            let content = tokio::fs::read_to_string(&log_file).await.unwrap();
            assert!(!content.is_empty());
        }
    }

    #[tokio::test]
    async fn test_request_correlation_workflow() {
        let temp_dir = TempDir::new().unwrap();

        let mut config = TelemetryConfig::default();
        config.logging.log_dir = temp_dir.path().to_path_buf();
        config.metrics.correlation_tracking = true;

        let _guard = TelemetrySystem::init(config).await.unwrap();

        // Create a request context
        let context = RequestContext::new("test_operation".to_string())
            .with_user_id("test_user".to_string())
            .with_metadata("key".to_string(), "value".to_string());

        let span = context.create_span();
        let _enter = span.enter();

        tracing::info!("Starting test operation");

        // Simulate some work
        sleep(Duration::from_millis(10)).await;

        tracing::info!("Operation in progress");

        // Create child operation
        let child_context = context.child("child_operation".to_string());
        let child_span = child_context.create_span();
        let _child_enter = child_span.enter();

        tracing::info!("Child operation running");

        drop(_child_enter);
        drop(child_span);

        tracing::info!("Test operation completed");

        drop(_enter);
        drop(span);

        context.log_completion(true, None);
    }

    #[tokio::test]
    #[cfg_attr(
        target_os = "windows",
        ignore = "File logging test is flaky on Windows"
    )]
    async fn test_data_sanitization_integration() {
        let temp_dir = TempDir::new().unwrap();

        let mut config = TelemetryConfig::default();
        config.logging.log_dir = temp_dir.path().to_path_buf();
        config.privacy.sanitize_enabled = true;
        config.logging.format = LogFormat::Json; // Use JSON for easier parsing

        let _guard = TelemetrySystem::init(config).await.unwrap();

        // Log potentially sensitive information
        tracing::info!(
            api_key = "sk-1234567890abcdef",
            password = "secretpassword123",
            email = "user@example.com",
            credit_card = "4532-1234-5678-9012",
            "Test message with sensitive data"
        );

        sleep(Duration::from_millis(100)).await;

        let log_file = temp_dir.path().join("fennec.log");
        let content = tokio::fs::read_to_string(&log_file).await.unwrap();

        // Verify sensitive data was redacted
        assert!(!content.contains("sk-1234567890abcdef"));
        assert!(!content.contains("secretpassword123"));
        assert!(content.contains("[REDACTED]") || content.contains("u***@e***.com"));
    }

    #[tokio::test]
    #[cfg_attr(
        target_os = "windows",
        ignore = "File logging test is flaky on Windows"
    )]
    async fn test_log_rotation_integration() {
        let temp_dir = TempDir::new().unwrap();

        let mut config = TelemetryConfig::default();
        config.logging.log_dir = temp_dir.path().to_path_buf();
        config.logging.max_file_size_mb = 1; // Very small size to trigger rotation
        config.logging.file_enabled = true;

        let _guard = TelemetrySystem::init(config).await.unwrap();

        // Write enough data to trigger rotation
        for i in 0..1000 {
            tracing::info!(
                iteration = i,
                data = format!("This is a long log message with iteration {} that should help fill up the log file to trigger rotation. Adding more text to increase the size of each log entry.", i),
                "Test rotation message"
            );
        }

        sleep(Duration::from_millis(200)).await;

        // Check if log files exist
        let log_file = temp_dir.path().join("fennec.log");
        assert!(log_file.exists());

        // There might be rotated files as well
        let entries: Vec<_> = std::fs::read_dir(temp_dir.path())
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        let log_files: Vec<_> = entries
            .iter()
            .filter(|entry| {
                entry.file_name().to_string_lossy().contains("fennec")
                    && entry.file_name().to_string_lossy().ends_with(".log")
            })
            .collect();

        assert!(!log_files.is_empty());
    }

    #[tokio::test]
    async fn test_metrics_integration() {
        let temp_dir = TempDir::new().unwrap();

        let mut config = TelemetryConfig::default();
        config.logging.log_dir = temp_dir.path().to_path_buf();
        config.metrics.enabled = true;
        config.metrics.performance_timing = true;

        let _guard = TelemetrySystem::init(config).await.unwrap();

        // Test timed operations
        let result = MetricsUtil::time_operation("test_sync_operation", || {
            std::thread::sleep(Duration::from_millis(10));
            42
        });
        assert_eq!(result, 42);

        let result = MetricsUtil::time_operation_async("test_async_operation", || async {
            sleep(Duration::from_millis(10)).await;
            84
        })
        .await;
        assert_eq!(result, 84);

        // Test span metrics
        let span = tracing::info_span!("test_span");
        let _enter = span.enter();

        sleep(Duration::from_millis(5)).await;
        tracing::info!("Inside test span");

        drop(_enter);
        drop(span);

        sleep(Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn test_retention_policy_integration() {
        let temp_dir = TempDir::new().unwrap();

        let mut config = TelemetryConfig::default();
        config.logging.log_dir = temp_dir.path().to_path_buf();
        config.retention.max_files = 3;
        config.retention.max_age_days = 1;
        config.retention.compress_old_files = true;

        let _guard = TelemetrySystem::init(config.clone()).await.unwrap();

        // Create several log files manually to test retention
        for i in 0..5 {
            let file_name = format!("fennec_{:02}.log", i);
            let file_path = temp_dir.path().join(&file_name);
            tokio::fs::write(&file_path, format!("Test log content {}", i))
                .await
                .unwrap();

            // Set different modification times
            let time = std::time::SystemTime::now() - Duration::from_secs(i * 3600);
            filetime::set_file_mtime(&file_path, filetime::FileTime::from_system_time(time))
                .unwrap();
        }

        // Test retention manager
        let retention_manager = RetentionManager::new(config.retention).await.unwrap();
        let report = retention_manager
            .perform_cleanup(temp_dir.path(), "fennec")
            .await
            .unwrap();

        // Should have removed some files
        assert!(report.total_files_removed() > 0 || report.files_compressed > 0);
    }

    #[tokio::test]
    async fn test_error_handling_and_recovery() {
        // Test with invalid log directory permissions (if possible)
        let temp_dir = TempDir::new().unwrap();
        let invalid_dir = temp_dir
            .path()
            .join("nonexistent")
            .join("deeply")
            .join("nested");

        let mut config = TelemetryConfig::default();
        config.logging.log_dir = invalid_dir;
        config.logging.file_enabled = true;

        // This should still succeed (creating directories)
        let _guard = TelemetrySystem::init(config).await.unwrap();

        tracing::info!("Test message in auto-created directory");
        sleep(Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn test_telemetry_disabled_mode() {
        let temp_dir = TempDir::new().unwrap();

        let mut config = TelemetryConfig::default();
        config.enabled = false;
        config.logging.log_dir = temp_dir.path().to_path_buf();

        let _guard = TelemetrySystem::init(config).await.unwrap();

        tracing::info!("This message should be filtered out");
        tracing::error!("This error should also be filtered out");

        sleep(Duration::from_millis(100)).await;

        // With telemetry disabled, less should be logged
    }

    #[tokio::test]
    #[cfg_attr(
        target_os = "windows",
        ignore = "File logging test is flaky on Windows"
    )]
    async fn test_concurrent_logging() {
        let temp_dir = TempDir::new().unwrap();

        let mut config = TelemetryConfig::default();
        config.logging.log_dir = temp_dir.path().to_path_buf();
        config.logging.file_enabled = true;

        let _guard = TelemetrySystem::init(config).await.unwrap();

        // Spawn multiple concurrent logging tasks
        let mut handles = Vec::new();

        for i in 0..10 {
            let handle = tokio::spawn(async move {
                for j in 0..50 {
                    tracing::info!(
                        task_id = i,
                        iteration = j,
                        "Concurrent logging test message"
                    );

                    if j % 10 == 0 {
                        sleep(Duration::from_millis(1)).await;
                    }
                }
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }

        sleep(Duration::from_millis(100)).await;

        let log_file = temp_dir.path().join("fennec.log");
        assert!(log_file.exists());

        let content = tokio::fs::read_to_string(&log_file).await.unwrap();
        assert!(!content.is_empty());

        // Should have messages from all tasks
        let line_count = content.lines().count();
        assert!(line_count >= 400); // 10 tasks * 50 messages - some might be filtered
    }

    #[test]
    fn test_configuration_validation() {
        let mut config = TelemetryConfig::default();

        // Valid configuration should pass
        assert!(config.validate().is_ok());

        // Invalid retention settings
        config.retention.max_files = 0;
        assert!(config.validate().is_err());

        config.retention.max_files = 5;
        config.retention.max_age_days = 0;
        assert!(config.validate().is_err());

        // Invalid regex patterns
        config.retention.max_age_days = 30;
        config.privacy.redaction_patterns = vec!["[invalid regex".to_string()];
        assert!(config.validate().is_err());
    }

    #[tokio::test]
    async fn test_environment_override_integration() {
        // Set environment variables
        std::env::set_var("FENNEC_LOG_LEVEL", "DEBUG");
        std::env::set_var("FENNEC_LOG_FORMAT", "json");
        std::env::set_var("FENNEC_FILE_LOGGING", "true");

        let mut config = TelemetryConfig::default();
        config.load_env_overrides();

        assert!(matches!(config.logging.level, LogLevel::Debug));
        assert!(matches!(config.logging.format, LogFormat::Json));
        assert!(config.logging.file_enabled);

        // Clean up
        std::env::remove_var("FENNEC_LOG_LEVEL");
        std::env::remove_var("FENNEC_LOG_FORMAT");
        std::env::remove_var("FENNEC_FILE_LOGGING");
    }
}
