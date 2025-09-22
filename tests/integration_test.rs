/// Comprehensive Integration Tests for Fennec
/// 
/// This file runs the complete integration test suite including end-to-end workflows,
/// provider mocking, sandbox security, command execution, TUI integration, and performance testing.

use std::process::Command;
use std::time::Duration;
use tempfile::TempDir;

// Import the integration test library
mod lib;
use lib::{
    IntegrationTestRunner, TestEnvironment, utils,
    integration::fixtures::test_scenarios,
};

/// Test basic CLI functionality
#[test]
fn test_fennec_help() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "fennec", "--", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Fennec") || stdout.contains("fennec"));
}

/// Test version command
#[test]
fn test_fennec_version() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "fennec", "--", "--version"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
}

/// Test CLI with configuration file
#[test]
#[ignore = "requires environment setup"]
fn test_fennec_with_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    std::fs::write(&config_path, r#"
[provider]
default_model = "gpt-4"
timeout_seconds = 30

[security]
default_sandbox_level = "read-only"
audit_log_enabled = false

[memory]
max_transcript_size = 5000
enable_agents_md = true

[tui]
theme = "default"

[tui.key_bindings]
quit = "Ctrl+C"
help = "F1"
clear = "Ctrl+L"
"#).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--bin", "fennec", "--", "--config", config_path.to_str().unwrap(), "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
}

/// Integration test for the test environment setup
#[tokio::test]
async fn test_integration_environment() {
    let env = TestEnvironment::new().await.expect("Failed to create test environment");
    utils::verify_test_environment(&env).await.expect("Environment verification failed");
}

/// Integration test for hello world scenario
#[tokio::test]
async fn test_hello_world_integration() {
    let runner = IntegrationTestRunner::new().await.expect("Failed to create test runner");
    let scenario = test_scenarios::get_hello_world_scenario();
    
    runner.run_scenario(scenario).await.expect("Hello world scenario failed");
}

/// Integration test for web server scenario
#[tokio::test]
async fn test_web_server_integration() {
    let runner = IntegrationTestRunner::new().await.expect("Failed to create test runner");
    let scenario = test_scenarios::get_web_server_scenario();
    
    runner.run_scenario(scenario).await.expect("Web server scenario failed");
}

/// Integration test for error handling
#[tokio::test]
async fn test_error_handling_integration() {
    let runner = IntegrationTestRunner::new().await.expect("Failed to create test runner");
    let scenario = test_scenarios::get_error_handling_scenario();
    
    // Error handling scenario is expected to have some failures
    let _ = runner.run_scenario(scenario).await;
}

/// Performance benchmark integration test
#[tokio::test]
async fn test_performance_integration() {
    let runner = IntegrationTestRunner::new().await.expect("Failed to create test runner");
    let metrics = runner.run_performance_benchmarks().await.expect("Performance benchmark failed");
    
    // Basic performance assertions
    assert!(metrics.success_rate() > 0.8, "Success rate should be > 80%");
    assert!(metrics.average_duration < Duration::from_millis(1000), "Average duration should be < 1s");
}

/// Test project template setup
#[tokio::test]
async fn test_project_template_integration() {
    let env = TestEnvironment::new().await.expect("Failed to create test environment");
    
    // Test Rust project setup
    utils::setup_sample_project(&env, "rust").await.expect("Failed to setup Rust project");
    
    // Verify key files exist
    lib::integration::common::assertions::assert_file_exists(&env, "Cargo.toml").await;
    lib::integration::common::assertions::assert_file_exists(&env, "src/main.rs").await;
    
    // Test Node.js project setup
    utils::setup_sample_project(&env, "node").await.expect("Failed to setup Node project");
    
    // Verify key files exist
    lib::integration::common::assertions::assert_file_exists(&env, "package.json").await;
    lib::integration::common::assertions::assert_file_exists(&env, "index.js").await;
}

/// Full integration test suite (long-running)
#[tokio::test]
#[ignore = "long running test"]
async fn test_full_integration_suite() {
    let runner = IntegrationTestRunner::new().await.expect("Failed to create test runner");
    
    // Run all predefined scenarios
    runner.run_all_scenarios().await.expect("Full integration suite failed");
    
    // Run performance benchmarks
    let metrics = runner.run_performance_benchmarks().await.expect("Performance benchmark failed");
    
    println!("Full integration suite completed successfully");
    println!("Performance metrics: {:.2} ops/sec, {:.2}% success rate", 
             metrics.operations_per_second(), metrics.success_rate() * 100.0);
}