/// Integration test library for Fennec
/// 
/// This module provides comprehensive integration testing capabilities for the Fennec
/// AI assistant system, including end-to-end workflows, provider mocking, sandbox security,
/// command execution, TUI integration, and performance testing.

// Import all integration test modules
pub mod integration;

// Re-export commonly used testing utilities
pub use integration::common::{
    TestEnvironment, TestConfig, ConfigurableMockProvider, 
    PerformanceMetrics, assertions
};

pub use integration::fixtures::{
    code_samples, config_samples, test_tasks, project_templates,
    mock_responses, test_scenarios, test_data_gen
};

/// Integration test runner for executing test suites
pub struct IntegrationTestRunner {
    pub test_env: TestEnvironment,
}

impl IntegrationTestRunner {
    /// Create a new integration test runner
    pub async fn new() -> anyhow::Result<Self> {
        let test_env = TestEnvironment::new().await?;
        Ok(Self { test_env })
    }

    /// Run a specific test scenario
    pub async fn run_scenario(&self, scenario: integration::fixtures::test_scenarios::TestScenario) -> anyhow::Result<()> {
        println!("Running scenario: {}", scenario.name);
        println!("Description: {}", scenario.description);

        for (step_idx, step) in scenario.steps.iter().enumerate() {
            println!("  Step {}: {} command", step_idx + 1, step.command);
            
            let context = self.test_env.create_context(fennec_security::SandboxLevel::WorkspaceWrite);
            
            let result = self.test_env.command_registry
                .execute_command(&step.command, &step.args, &context)
                .await;

            if step.expected_success {
                match result {
                    Ok(res) => {
                        if !res.success {
                            return Err(anyhow::anyhow!(
                                "Step {} expected success but command failed: {:?}", 
                                step_idx + 1, res.error
                            ));
                        }
                    }
                    Err(e) => {
                        return Err(anyhow::anyhow!(
                            "Step {} expected success but got error: {}", 
                            step_idx + 1, e
                        ));
                    }
                }
            } else {
                // For steps expected to fail, we don't error out if they fail
                match result {
                    Ok(res) if res.success => {
                        println!("    Warning: Step {} expected to fail but succeeded", step_idx + 1);
                    }
                    _ => {
                        println!("    Step {} failed as expected", step_idx + 1);
                    }
                }
            }

            if step.delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(step.delay_ms)).await;
            }
        }

        println!("Scenario completed successfully: {}", scenario.name);
        Ok(())
    }

    /// Run all predefined test scenarios
    pub async fn run_all_scenarios(&self) -> anyhow::Result<()> {
        let scenarios = integration::fixtures::test_scenarios::get_all_scenarios();
        
        for scenario in scenarios {
            self.run_scenario(scenario).await?;
        }

        Ok(())
    }

    /// Run performance benchmarks
    pub async fn run_performance_benchmarks(&self) -> anyhow::Result<integration::common::PerformanceMetrics> {
        use std::time::{Duration, Instant};
        
        let mut metrics = integration::common::PerformanceMetrics::new();
        let context = self.test_env.create_context(fennec_security::SandboxLevel::ReadOnly);
        
        // Run a series of plan commands to benchmark performance
        for i in 0..50 {
            let start_time = Instant::now();
            
            let result = self.test_env.command_registry
                .execute_command(
                    "plan",
                    &serde_json::json!({"task": format!("Benchmark task {}", i)}),
                    &context,
                )
                .await;
            
            let duration = start_time.elapsed();
            let success = result.is_ok() && result.unwrap().success;
            
            metrics.add_measurement(duration, success);
        }

        println!("Performance Benchmark Results:");
        println!("  Operations: {}", metrics.operation_count);
        println!("  Success Rate: {:.2}%", metrics.success_rate() * 100.0);
        println!("  Average Duration: {:?}", metrics.average_duration);
        println!("  Min Duration: {:?}", metrics.min_duration);
        println!("  Max Duration: {:?}", metrics.max_duration);
        println!("  Operations/sec: {:.2}", metrics.operations_per_second());

        Ok(metrics)
    }
}

/// Utility functions for integration testing
pub mod utils {
    use super::*;
    
    /// Setup test environment with sample project
    pub async fn setup_sample_project(env: &TestEnvironment, project_type: &str) -> anyhow::Result<()> {
        let files = match project_type {
            "rust" => integration::fixtures::project_templates::get_rust_project_structure(),
            "node" => integration::fixtures::project_templates::get_node_project_structure(),
            "python" => integration::fixtures::project_templates::get_python_project_structure(),
            _ => return Err(anyhow::anyhow!("Unknown project type: {}", project_type)),
        };

        for (path, content) in files {
            env.write_test_file(&path, &content).await?;
        }

        Ok(())
    }

    /// Verify test environment is working correctly
    pub async fn verify_test_environment(env: &TestEnvironment) -> anyhow::Result<()> {
        // Check workspace exists
        if !env.config.workspace_path.exists() {
            return Err(anyhow::anyhow!("Workspace path does not exist"));
        }

        // Test basic file operations
        env.write_test_file("test_verify.txt", "verification test").await?;
        let content = env.read_test_file("test_verify.txt").await?;
        if content.trim() != "verification test" {
            return Err(anyhow::anyhow!("File operations not working correctly"));
        }

        // Test command registry
        let commands = env.command_registry.list_commands().await;
        if commands.is_empty() {
            return Err(anyhow::anyhow!("No commands available in registry"));
        }

        println!("Test environment verification passed");
        Ok(())
    }

    /// Clean up test artifacts
    pub async fn cleanup_test_artifacts(env: &TestEnvironment) -> anyhow::Result<()> {
        // Remove all test files
        let files = env.list_workspace_files().await?;
        
        for file in files {
            let file_path = env.config.workspace_path.join(&file);
            if file_path.exists() {
                tokio::fs::remove_file(file_path).await?;
            }
        }

        println!("Test artifacts cleaned up");
        Ok(())
    }
}

#[cfg(test)]
mod integration_test_runner_tests {
    use super::*;

    #[tokio::test]
    async fn test_integration_test_runner_creation() -> anyhow::Result<()> {
        let _runner = IntegrationTestRunner::new().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_environment_verification() -> anyhow::Result<()> {
        let env = TestEnvironment::new().await?;
        utils::verify_test_environment(&env).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_sample_project_setup() -> anyhow::Result<()> {
        let env = TestEnvironment::new().await?;
        
        utils::setup_sample_project(&env, "rust").await?;
        
        // Verify files were created
        integration::common::assertions::assert_file_exists(&env, "Cargo.toml").await;
        integration::common::assertions::assert_file_exists(&env, "src/main.rs").await;
        
        Ok(())
    }

    #[tokio::test]
    async fn test_performance_benchmark() -> anyhow::Result<()> {
        let runner = IntegrationTestRunner::new().await?;
        let metrics = runner.run_performance_benchmarks().await?;
        
        assert!(metrics.operation_count > 0);
        assert!(metrics.success_rate() > 0.0);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_hello_world_scenario() -> anyhow::Result<()> {
        let runner = IntegrationTestRunner::new().await?;
        let scenario = integration::fixtures::test_scenarios::get_hello_world_scenario();
        
        runner.run_scenario(scenario).await?;
        
        Ok(())
    }
}