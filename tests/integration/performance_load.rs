/// Performance and Load Testing Suite
/// 
/// These tests validate system performance under load, memory usage, resource management,
/// concurrent user scenarios, and benchmark critical paths.

use super::common::{TestEnvironment, ConfigurableMockProvider, assertions};
use anyhow::Result;
use fennec_orchestration::CommandState;
use fennec_security::SandboxLevel;
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use uuid::Uuid;

/// Performance metrics collection
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub operation_count: usize,
    pub total_duration: Duration,
    pub average_duration: Duration,
    pub min_duration: Duration,
    pub max_duration: Duration,
    pub success_count: usize,
    pub failure_count: usize,
    pub memory_usage_mb: f64,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            operation_count: 0,
            total_duration: Duration::ZERO,
            average_duration: Duration::ZERO,
            min_duration: Duration::MAX,
            max_duration: Duration::ZERO,
            success_count: 0,
            failure_count: 0,
            memory_usage_mb: 0.0,
        }
    }

    pub fn add_measurement(&mut self, duration: Duration, success: bool) {
        self.operation_count += 1;
        self.total_duration += duration;
        
        if duration < self.min_duration {
            self.min_duration = duration;
        }
        if duration > self.max_duration {
            self.max_duration = duration;
        }
        
        if success {
            self.success_count += 1;
        } else {
            self.failure_count += 1;
        }
        
        self.average_duration = self.total_duration / self.operation_count as u32;
    }

    pub fn success_rate(&self) -> f64 {
        if self.operation_count == 0 {
            0.0
        } else {
            self.success_count as f64 / self.operation_count as f64
        }
    }

    pub fn operations_per_second(&self) -> f64 {
        if self.total_duration.is_zero() {
            0.0
        } else {
            self.operation_count as f64 / self.total_duration.as_secs_f64()
        }
    }
}

/// Load test configuration
#[derive(Debug, Clone)]
pub struct LoadTestConfig {
    pub concurrent_users: usize,
    pub operations_per_user: usize,
    pub test_duration: Duration,
    pub ramp_up_duration: Duration,
    pub operation_delay: Duration,
}

impl Default for LoadTestConfig {
    fn default() -> Self {
        Self {
            concurrent_users: 10,
            operations_per_user: 100,
            test_duration: Duration::from_secs(60),
            ramp_up_duration: Duration::from_secs(10),
            operation_delay: Duration::from_millis(100),
        }
    }
}

/// Performance test for basic command execution
#[tokio::test]
async fn test_command_execution_performance() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let context = env.create_context(SandboxLevel::ReadOnly);
    
    let mut metrics = PerformanceMetrics::new();
    let num_operations = 100;
    
    for i in 0..num_operations {
        let start_time = Instant::now();
        
        let result = env.command_registry
            .execute_command(
                "plan",
                &json!({"task": format!("Performance test task {}", i)}),
                &context,
            )
            .await;
        
        let duration = start_time.elapsed();
        let success = result.is_ok() && result.as_ref().unwrap().success;
        
        metrics.add_measurement(duration, success);
    }
    
    // Performance assertions
    assert!(metrics.success_rate() > 0.95, "Success rate should be > 95%, got {}", metrics.success_rate());
    assert!(metrics.average_duration < Duration::from_millis(500), 
           "Average duration should be < 500ms, got {:?}", metrics.average_duration);
    assert!(metrics.max_duration < Duration::from_secs(2), 
           "Max duration should be < 2s, got {:?}", metrics.max_duration);
    assert!(metrics.operations_per_second() > 2.0, 
           "Should handle > 2 ops/sec, got {:.2}", metrics.operations_per_second());
    
    println!("Command execution performance: {:.2} ops/sec, avg: {:?}", 
             metrics.operations_per_second(), metrics.average_duration);
    
    Ok(())
}

/// Performance test for concurrent command execution
#[tokio::test]
async fn test_concurrent_command_performance() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let context = env.create_context(SandboxLevel::WorkspaceWrite);
    
    let num_concurrent = 20;
    let operations_per_thread = 10;
    let semaphore = Arc::new(Semaphore::new(num_concurrent));
    
    let start_time = Instant::now();
    let success_count = Arc::new(AtomicUsize::new(0));
    let failure_count = Arc::new(AtomicUsize::new(0));
    
    let mut handles = Vec::new();
    
    for thread_id in 0..num_concurrent {
        let env_clone = env.command_registry.clone();
        let context_clone = context.clone();
        let semaphore_clone = semaphore.clone();
        let success_count_clone = success_count.clone();
        let failure_count_clone = failure_count.clone();
        
        let handle = tokio::spawn(async move {
            let _permit = semaphore_clone.acquire().await.unwrap();
            
            for op_id in 0..operations_per_thread {
                let result = env_clone
                    .execute_command(
                        "edit",
                        &json!({
                            "path": format!("concurrent_{}_{}.txt", thread_id, op_id),
                            "content": format!("Content from thread {} operation {}", thread_id, op_id)
                        }),
                        &context_clone,
                    )
                    .await;
                
                match result {
                    Ok(res) if res.success => {
                        success_count_clone.fetch_add(1, Ordering::Relaxed);
                    }
                    _ => {
                        failure_count_clone.fetch_add(1, Ordering::Relaxed);
                    }
                }
                
                // Small delay to avoid overwhelming the system
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        handle.await?;
    }
    
    let total_duration = start_time.elapsed();
    let total_operations = num_concurrent * operations_per_thread;
    let success_count = success_count.load(Ordering::Relaxed);
    let failure_count = failure_count.load(Ordering::Relaxed);
    
    let success_rate = success_count as f64 / total_operations as f64;
    let ops_per_second = total_operations as f64 / total_duration.as_secs_f64();
    
    // Performance assertions for concurrent execution
    assert!(success_rate > 0.90, "Concurrent success rate should be > 90%, got {:.2}", success_rate);
    assert!(ops_per_second > 5.0, "Should handle > 5 concurrent ops/sec, got {:.2}", ops_per_second);
    assert!(total_duration < Duration::from_secs(30), 
           "Concurrent test should complete < 30s, took {:?}", total_duration);
    
    println!("Concurrent execution performance: {:.2} ops/sec, success rate: {:.2}%", 
             ops_per_second, success_rate * 100.0);
    
    Ok(())
}

/// Memory usage test
#[tokio::test]
async fn test_memory_usage() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let context = env.create_context(SandboxLevel::WorkspaceWrite);
    
    // Get initial memory usage (approximate)
    let initial_memory = get_memory_usage_mb();
    
    // Perform memory-intensive operations
    let num_operations = 1000;
    
    for i in 0..num_operations {
        // Create files with increasing size
        let content_size = (i % 100 + 1) * 100; // 100 to 10,000 characters
        let content = "x".repeat(content_size);
        
        let _result = env.command_registry
            .execute_command(
                "edit",
                &json!({
                    "path": format!("memory_test_{}.txt", i),
                    "content": content
                }),
                &context,
            )
            .await?;
        
        // Check memory periodically
        if i % 100 == 0 {
            let current_memory = get_memory_usage_mb();
            let memory_growth = current_memory - initial_memory;
            
            // Memory growth should be reasonable
            assert!(memory_growth < 100.0, 
                   "Memory growth should be < 100MB, currently {:.2}MB", memory_growth);
        }
    }
    
    let final_memory = get_memory_usage_mb();
    let total_growth = final_memory - initial_memory;
    
    println!("Memory usage: initial {:.2}MB, final {:.2}MB, growth {:.2}MB", 
             initial_memory, final_memory, total_growth);
    
    // Memory growth should be bounded
    assert!(total_growth < 200.0, 
           "Total memory growth should be < 200MB, got {:.2}MB", total_growth);
    
    Ok(())
}

/// Load test simulation
#[tokio::test]
async fn test_load_simulation() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let config = LoadTestConfig {
        concurrent_users: 5, // Reduced for CI environments
        operations_per_user: 20,
        test_duration: Duration::from_secs(30),
        ramp_up_duration: Duration::from_secs(5),
        operation_delay: Duration::from_millis(200),
    };
    
    let start_time = Instant::now();
    let total_operations = Arc::new(AtomicUsize::new(0));
    let successful_operations = Arc::new(AtomicUsize::new(0));
    
    let mut handles = Vec::new();
    
    for user_id in 0..config.concurrent_users {
        let env_clone = env.command_registry.clone();
        let context = env.create_context(SandboxLevel::WorkspaceWrite);
        let config_clone = config.clone();
        let total_ops_clone = total_operations.clone();
        let success_ops_clone = successful_operations.clone();
        
        let handle = tokio::spawn(async move {
            // Stagger user start times (ramp-up)
            let ramp_delay = config_clone.ramp_up_duration * user_id as u32 / config_clone.concurrent_users as u32;
            tokio::time::sleep(ramp_delay).await;
            
            let user_start = Instant::now();
            let mut operations_performed = 0;
            
            while user_start.elapsed() < config_clone.test_duration && 
                  operations_performed < config_clone.operations_per_user {
                
                let operation_type = operations_performed % 3;
                let result = match operation_type {
                    0 => {
                        // Plan operation
                        env_clone.execute_command(
                            "plan",
                            &json!({"task": format!("Load test task user {} op {}", user_id, operations_performed)}),
                            &context,
                        ).await
                    }
                    1 => {
                        // Edit operation
                        env_clone.execute_command(
                            "edit",
                            &json!({
                                "path": format!("load_test_user_{}_op_{}.txt", user_id, operations_performed),
                                "content": format!("Load test content from user {}", user_id)
                            }),
                            &context,
                        ).await
                    }
                    _ => {
                        // Diff operation
                        env_clone.execute_command(
                            "diff",
                            &json!({
                                "path": format!("load_test_user_{}_op_{}.txt", user_id, operations_performed.saturating_sub(1))
                            }),
                            &context,
                        ).await
                    }
                };
                
                total_ops_clone.fetch_add(1, Ordering::Relaxed);
                
                if result.is_ok() && result.unwrap().success {
                    success_ops_clone.fetch_add(1, Ordering::Relaxed);
                }
                
                operations_performed += 1;
                
                // Delay between operations
                tokio::time::sleep(config_clone.operation_delay).await;
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all users to complete
    for handle in handles {
        handle.await?;
    }
    
    let total_duration = start_time.elapsed();
    let total_ops = total_operations.load(Ordering::Relaxed);
    let successful_ops = successful_operations.load(Ordering::Relaxed);
    
    let success_rate = successful_ops as f64 / total_ops as f64;
    let throughput = total_ops as f64 / total_duration.as_secs_f64();
    
    // Load test assertions
    assert!(success_rate > 0.85, "Load test success rate should be > 85%, got {:.2}", success_rate);
    assert!(throughput > 1.0, "Load test throughput should be > 1 ops/sec, got {:.2}", throughput);
    assert!(total_ops > 0, "Should have performed some operations");
    
    println!("Load test results: {} operations, {:.2}% success rate, {:.2} ops/sec", 
             total_ops, success_rate * 100.0, throughput);
    
    Ok(())
}

/// Provider performance test
#[tokio::test]
async fn test_provider_performance() -> Result<()> {
    let responses = vec![
        "Quick response 1".to_string(),
        "Quick response 2".to_string(),
        "Quick response 3".to_string(),
    ];
    
    let provider = ConfigurableMockProvider::new(responses);
    let num_requests = 100;
    
    let mut metrics = PerformanceMetrics::new();
    
    for i in 0..num_requests {
        let start_time = Instant::now();
        
        let request = fennec_core::provider::ProviderRequest {
            messages: vec![
                fennec_core::provider::ProviderMessage {
                    role: "user".to_string(),
                    content: format!("Performance test request {}", i),
                }
            ],
            model: "test-model".to_string(),
            temperature: None,
            max_tokens: None,
            stream: false,
        };
        
        let result = provider.complete(request).await;
        let duration = start_time.elapsed();
        let success = result.is_ok();
        
        metrics.add_measurement(duration, success);
    }
    
    // Provider performance assertions
    assert!(metrics.success_rate() == 1.0, "Provider should have 100% success rate");
    assert!(metrics.average_duration < Duration::from_millis(10), 
           "Provider avg response time should be < 10ms, got {:?}", metrics.average_duration);
    assert!(metrics.operations_per_second() > 100.0, 
           "Provider should handle > 100 ops/sec, got {:.2}", metrics.operations_per_second());
    
    println!("Provider performance: {:.2} ops/sec, avg: {:?}", 
             metrics.operations_per_second(), metrics.average_duration);
    
    Ok(())
}

/// Execution engine performance test
#[tokio::test]
async fn test_execution_engine_performance() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let context = env.create_context(SandboxLevel::WorkspaceWrite);
    
    let num_commands = 50;
    let mut execution_ids = Vec::new();
    
    let start_time = Instant::now();
    
    // Submit commands
    for i in 0..num_commands {
        let execution_id = env.execution_engine
            .submit_command(
                "edit".to_string(),
                json!({
                    "path": format!("perf_test_{}.txt", i),
                    "content": format!("Performance test content {}", i)
                }),
                context.clone(),
            )
            .await?;
        
        execution_ids.push(execution_id);
    }
    
    let submission_time = start_time.elapsed();
    
    // Wait for all executions to complete
    let mut completed_count = 0;
    let completion_start = Instant::now();
    let timeout = Duration::from_secs(30);
    
    while completed_count < num_commands && completion_start.elapsed() < timeout {
        completed_count = 0;
        
        for &execution_id in &execution_ids {
            if let Some(info) = env.execution_engine.get_execution_status(execution_id).await {
                if matches!(info.state, CommandState::Completed | CommandState::Failed { .. }) {
                    completed_count += 1;
                }
            }
        }
        
        if completed_count < num_commands {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    
    let total_time = start_time.elapsed();
    
    // Performance assertions
    assert_eq!(completed_count, num_commands, "All commands should complete");
    assert!(submission_time < Duration::from_secs(5), 
           "Command submission should be fast, took {:?}", submission_time);
    assert!(total_time < Duration::from_secs(20), 
           "All commands should complete within 20s, took {:?}", total_time);
    
    let throughput = num_commands as f64 / total_time.as_secs_f64();
    assert!(throughput > 2.0, "Should handle > 2 commands/sec, got {:.2}", throughput);
    
    println!("Execution engine performance: {:.2} commands/sec", throughput);
    
    Ok(())
}

/// Stress test with high concurrency
#[tokio::test]
async fn test_stress_high_concurrency() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let context = env.create_context(SandboxLevel::ReadOnly);
    
    let num_concurrent = 50;
    let semaphore = Arc::new(Semaphore::new(num_concurrent));
    let success_count = Arc::new(AtomicUsize::new(0));
    let total_count = Arc::new(AtomicUsize::new(0));
    
    let start_time = Instant::now();
    let mut handles = Vec::new();
    
    for i in 0..num_concurrent {
        let env_clone = env.command_registry.clone();
        let context_clone = context.clone();
        let semaphore_clone = semaphore.clone();
        let success_count_clone = success_count.clone();
        let total_count_clone = total_count.clone();
        
        let handle = tokio::spawn(async move {
            let _permit = semaphore_clone.acquire().await.unwrap();
            
            let result = env_clone
                .execute_command(
                    "plan",
                    &json!({"task": format!("Stress test task {}", i)}),
                    &context_clone,
                )
                .await;
            
            total_count_clone.fetch_add(1, Ordering::Relaxed);
            
            if result.is_ok() && result.unwrap().success {
                success_count_clone.fetch_add(1, Ordering::Relaxed);
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        handle.await?;
    }
    
    let duration = start_time.elapsed();
    let total = total_count.load(Ordering::Relaxed);
    let success = success_count.load(Ordering::Relaxed);
    
    let success_rate = success as f64 / total as f64;
    
    // Stress test assertions
    assert!(success_rate > 0.80, "Stress test success rate should be > 80%, got {:.2}", success_rate);
    assert!(duration < Duration::from_secs(10), 
           "Stress test should complete < 10s, took {:?}", duration);
    
    println!("Stress test: {} concurrent operations, {:.2}% success rate in {:?}", 
             total, success_rate * 100.0, duration);
    
    Ok(())
}

/// Benchmark critical paths
#[tokio::test]
async fn test_benchmark_critical_paths() -> Result<()> {
    let env = TestEnvironment::new().await?;
    
    // Benchmark different operations
    let benchmarks = vec![
        ("plan_simple", "plan", json!({"task": "Simple task"})),
        ("plan_complex", "plan", json!({"task": "Complex multi-step task with many requirements"})),
        ("edit_small", "edit", json!({"path": "small.txt", "content": "Small content"})),
        ("edit_large", "edit", json!({"path": "large.txt", "content": "Large content ".repeat(1000)})),
    ];
    
    for (benchmark_name, command, args) in benchmarks {
        let context = env.create_context(SandboxLevel::WorkspaceWrite);
        let iterations = 10;
        let mut durations = Vec::new();
        
        for _ in 0..iterations {
            let start_time = Instant::now();
            
            let _result = env.command_registry
                .execute_command(command, &args, &context)
                .await?;
            
            durations.push(start_time.elapsed());
        }
        
        let avg_duration = durations.iter().sum::<Duration>() / iterations as u32;
        let min_duration = durations.iter().min().unwrap();
        let max_duration = durations.iter().max().unwrap();
        
        println!("Benchmark {}: avg {:?}, min {:?}, max {:?}", 
                benchmark_name, avg_duration, min_duration, max_duration);
        
        // Benchmark assertions (adjust thresholds as needed)
        match benchmark_name {
            "plan_simple" => assert!(avg_duration < Duration::from_millis(200)),
            "plan_complex" => assert!(avg_duration < Duration::from_millis(500)),
            "edit_small" => assert!(avg_duration < Duration::from_millis(100)),
            "edit_large" => assert!(avg_duration < Duration::from_millis(300)),
            _ => {}
        }
    }
    
    Ok(())
}

/// Resource cleanup test
#[tokio::test]
async fn test_resource_cleanup() -> Result<()> {
    let env = TestEnvironment::new().await?;
    let context = env.create_context(SandboxLevel::WorkspaceWrite);
    
    let initial_files = env.list_workspace_files().await?;
    let initial_file_count = initial_files.len();
    
    // Create many files
    let num_files = 100;
    for i in 0..num_files {
        let _result = env.command_registry
            .execute_command(
                "edit",
                &json!({
                    "path": format!("cleanup_test_{}.txt", i),
                    "content": format!("Content {}", i)
                }),
                &context,
            )
            .await?;
    }
    
    let mid_files = env.list_workspace_files().await?;
    assert!(mid_files.len() >= initial_file_count + num_files);
    
    // In a real implementation, there would be a cleanup mechanism
    // For now, we just verify that the files were created
    
    println!("Resource cleanup test: created {} files", num_files);
    
    Ok(())
}

/// Helper function to get approximate memory usage
fn get_memory_usage_mb() -> f64 {
    // This is a simplified version - in a real implementation,
    // you would use platform-specific APIs or libraries like `psutil`
    
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        if let Ok(status) = fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<f64>() {
                            return kb / 1024.0; // Convert KB to MB
                        }
                    }
                }
            }
        }
    }
    
    // Fallback: return a reasonable estimate
    50.0 // MB
}

#[cfg(test)]
mod performance_integration_tests {
    use super::*;

    /// Comprehensive performance test suite
    #[tokio::test]
    async fn test_comprehensive_performance() -> Result<()> {
        let env = TestEnvironment::new().await?;
        
        // Test various performance aspects
        let test_scenarios = vec![
            ("single_user", 1, 50),
            ("few_users", 3, 30),
            ("many_users", 10, 20),
        ];
        
        for (scenario_name, users, ops_per_user) in test_scenarios {
            let start_time = Instant::now();
            let mut handles = Vec::new();
            
            for user_id in 0..users {
                let env_clone = env.command_registry.clone();
                let context = env.create_context(SandboxLevel::ReadOnly);
                
                let handle = tokio::spawn(async move {
                    for op_id in 0..ops_per_user {
                        let _result = env_clone
                            .execute_command(
                                "plan",
                                &json!({"task": format!("Scenario {} user {} op {}", scenario_name, user_id, op_id)}),
                                &context,
                            )
                            .await;
                    }
                });
                
                handles.push(handle);
            }
            
            for handle in handles {
                handle.await?;
            }
            
            let duration = start_time.elapsed();
            let total_ops = users * ops_per_user;
            let throughput = total_ops as f64 / duration.as_secs_f64();
            
            println!("Scenario {}: {} ops in {:?} ({:.2} ops/sec)", 
                    scenario_name, total_ops, duration, throughput);
            
            // Each scenario should maintain reasonable performance
            assert!(throughput > 1.0, "Scenario {} should maintain > 1 ops/sec", scenario_name);
        }
        
        Ok(())
    }

    /// Long-running stability test
    #[tokio::test]
    #[ignore = "long running test"]
    async fn test_long_running_stability() -> Result<()> {
        let env = TestEnvironment::new().await?;
        let context = env.create_context(SandboxLevel::ReadOnly);
        
        let test_duration = Duration::from_secs(300); // 5 minutes
        let start_time = Instant::now();
        let mut operation_count = 0;
        
        while start_time.elapsed() < test_duration {
            let result = env.command_registry
                .execute_command(
                    "plan",
                    &json!({"task": format!("Stability test operation {}", operation_count)}),
                    &context,
                )
                .await;
            
            assert!(result.is_ok(), "Operation {} failed: {:?}", operation_count, result);
            
            operation_count += 1;
            
            // Small delay to avoid overwhelming the system
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        println!("Stability test: {} operations over {:?}", 
                operation_count, start_time.elapsed());
        
        // Should maintain stability over long periods
        assert!(operation_count > 100, "Should complete many operations in long test");
        
        Ok(())
    }
}