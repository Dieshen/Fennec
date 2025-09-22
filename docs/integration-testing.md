# Integration Testing Suite

This document describes the comprehensive integration testing framework for Fennec, covering end-to-end workflows, provider mocking, security testing, and performance validation.

## Overview

The integration test suite validates that all major components of Fennec work correctly together, ensuring the system meets its requirements for reliability, security, and performance.

## Test Categories

### 1. End-to-End Workflow Tests (`tests/integration/end_to_end_workflow.rs`)

These tests simulate complete user workflows from start to finish:

- **Planning Workflows**: Test the AI planning system with various task complexities
- **Plan → Edit → Run Cycles**: Validate the complete development workflow
- **Error Recovery**: Test system behavior when operations fail
- **Rollback Functionality**: Verify backup and restore mechanisms
- **Concurrent Workflows**: Test multiple simultaneous operations
- **Sandbox Level Validation**: Ensure security policies are enforced

#### Key Test Scenarios

```rust
// Simple planning workflow
test_planning_workflow()

// Complete development cycle
test_plan_edit_run_workflow()

// Error handling and recovery
test_workflow_error_recovery()

// Backup and rollback
test_workflow_with_rollback()
```

### 2. Provider Mocking System (`tests/integration/provider_mocking.rs`)

Comprehensive testing of AI provider integrations:

- **Mock Provider Implementation**: Configurable fake responses for testing
- **Error Injection**: Simulate provider failures and timeouts
- **Latency Simulation**: Test system behavior under slow network conditions
- **Response Validation**: Ensure responses are properly handled
- **Failover Testing**: Validate fallback mechanisms

#### Mock Provider Features

- **Configurable Responses**: Predefined response sequences
- **Error Simulation**: Controllable failure scenarios
- **Latency Control**: Configurable response delays
- **Concurrent Usage**: Thread-safe provider implementation

### 3. Filesystem Sandbox Security Tests (`tests/integration/sandbox_security.rs`)

Comprehensive security validation:

- **Path Traversal Prevention**: Block attempts to access files outside workspace
- **Sandbox Level Enforcement**: Validate read-only, workspace-write, and full-access modes
- **Symbolic Link Protection**: Prevent symlink-based attacks
- **Command Execution Restrictions**: Control shell command access
- **Network Access Control**: Manage external network connections
- **Approval Workflow Testing**: Validate user consent mechanisms

#### Security Test Categories

```rust
// Sandbox policy enforcement
test_readonly_sandbox_restrictions()
test_workspace_write_sandbox_restrictions()
test_full_access_sandbox_with_approval()

// Attack prevention
test_path_traversal_prevention()
test_symbolic_link_prevention()
test_dangerous_shell_command_detection()
```

### 4. Command Execution Testing (`tests/integration/command_execution.rs`)

Complete command lifecycle validation:

- **Preview → Approval → Execution**: Test the full command flow
- **State Management**: Verify command state transitions
- **Concurrent Execution**: Test multiple commands running simultaneously
- **Timeout Handling**: Validate command timeout mechanisms
- **Audit Logging**: Ensure all operations are properly logged
- **Backup Creation**: Test automatic backup mechanisms

#### Command Lifecycle Tests

```rust
// Basic execution flow
test_command_approval_workflow()
test_command_denial_workflow()

// Advanced scenarios
test_concurrent_command_execution()
test_command_execution_timeout()
test_command_backup_and_rollback()
```

### 5. TUI Integration Testing (`tests/integration/tui_integration.rs`)

Automated UI testing framework:

- **Keyboard Navigation**: Test all navigation paths
- **Input Handling**: Validate text input and command entry
- **Error Display**: Ensure errors are properly shown to users
- **State Management**: Test UI state transitions
- **Accessibility**: Verify keyboard-only operation
- **Responsiveness**: Test UI performance under load

#### TUI Test Harness

The `TuiTestHarness` provides:
- Simulated keyboard events
- Screen content verification
- Workflow automation
- Performance testing

### 6. Performance and Load Testing (`tests/integration/performance_load.rs`)

System performance validation:

- **Command Execution Performance**: Measure operation latency
- **Concurrent Load Testing**: Test system under multiple users
- **Memory Usage Monitoring**: Track resource consumption
- **Provider Performance**: Measure AI provider response times
- **Stress Testing**: Validate system stability under load
- **Benchmark Critical Paths**: Identify performance bottlenecks

#### Performance Metrics

```rust
pub struct PerformanceMetrics {
    pub operation_count: usize,
    pub total_duration: Duration,
    pub average_duration: Duration,
    pub success_rate: f64,
    pub operations_per_second: f64,
}
```

## Test Infrastructure

### Common Test Utilities (`tests/integration/common.rs`)

Shared infrastructure for all tests:

- **TestEnvironment**: Complete test setup with temporary workspace
- **ConfigurableMockProvider**: Advanced mock AI provider
- **Assertion Helpers**: Common validation functions
- **Test Data Generation**: Utilities for creating test scenarios

### Test Fixtures (`tests/integration/fixtures.rs`)

Comprehensive test data:

- **Code Samples**: Multi-language code examples
- **Configuration Files**: Sample config files (TOML, JSON, YAML)
- **Test Tasks**: Predefined planning tasks of varying complexity
- **Project Templates**: Complete project structures
- **Mock Responses**: AI provider response samples
- **Test Scenarios**: Complete workflow definitions

## Running Tests

### Basic Integration Tests

```bash
# Run all integration tests
cargo test --test integration_test

# Run specific test suites
cargo test --test integration_test end_to_end_workflow
cargo test --test integration_test provider_mocking
cargo test --test integration_test sandbox_security
cargo test --test integration_test command_execution
cargo test --test integration_test tui_integration
cargo test --test integration_test performance_load
```

### Long-Running Tests

```bash
# Run comprehensive test suite (includes ignored tests)
cargo test --test integration_test test_full_integration_suite -- --ignored

# Run stability tests
cargo test --test integration_test -- --ignored "long running"
```

### Performance Benchmarks

```bash
# Run performance benchmarks
cargo bench --bench performance_benchmarks

# Generate HTML reports
cargo bench --bench performance_benchmarks -- --output-format html
```

## CI/CD Integration

The integration tests are automatically run in GitHub Actions:

### Test Matrix

- **Rust Versions**: stable, beta
- **Test Suites**: end-to-end, provider-mocking, sandbox-security, command-execution, tui-integration, performance
- **Platforms**: Ubuntu (with plans for Windows/macOS)

### Automated Workflows

1. **Pull Request Validation**: Run core integration tests
2. **Daily Comprehensive Testing**: Full test suite with performance regression detection
3. **Security Auditing**: Dependency vulnerability scanning
4. **Code Coverage**: Generate and upload coverage reports

### Performance Tracking

- Benchmark results are tracked over time
- Performance regressions are automatically detected
- Memory usage is monitored with Valgrind

## Test Configuration

### Environment Variables

```bash
# Enable verbose logging during tests
RUST_LOG=debug cargo test

# Set custom test timeouts
FENNEC_TEST_TIMEOUT=30 cargo test

# Use specific provider configurations
FENNEC_TEST_PROVIDER=mock cargo test
```

### Test-Specific Configuration

Tests use isolated temporary directories and mock configurations to ensure repeatability and isolation.

## Best Practices

### Writing Integration Tests

1. **Use TestEnvironment**: Always start with the provided test environment
2. **Clean Up Resources**: Tests automatically clean up temporary files
3. **Test Error Conditions**: Include negative test cases
4. **Validate Security**: Ensure security policies are tested
5. **Measure Performance**: Include timing assertions where appropriate

### Test Organization

1. **Group Related Tests**: Use modules to organize test functions
2. **Descriptive Names**: Test names should clearly indicate what is being tested
3. **Documentation**: Include docstrings explaining test purpose
4. **Assertions**: Use the provided assertion helpers for consistent error messages

### Debugging Tests

1. **Verbose Output**: Use `--verbose` flag for detailed test output
2. **Isolated Execution**: Run specific tests to isolate issues
3. **Log Analysis**: Check audit logs for detailed operation traces
4. **Environment Inspection**: Tests preserve temporary directories on failure

## Extending the Test Suite

### Adding New Test Categories

1. Create new module in `tests/integration/`
2. Implement test functions with appropriate annotations
3. Add to `tests/integration/mod.rs`
4. Update CI configuration to include new tests

### Creating Test Scenarios

1. Define scenario in `tests/integration/fixtures.rs`
2. Use `TestScenario` structure for complex workflows
3. Include expected outcomes and validation steps
4. Add to scenario collections for automated execution

### Performance Testing

1. Use `PerformanceMetrics` for consistent measurement
2. Include baseline expectations in assertions
3. Add to benchmark suite for tracking over time
4. Document performance requirements and assumptions

## Troubleshooting

### Common Issues

1. **Test Timeouts**: Increase timeout values for slow operations
2. **File Permission Errors**: Ensure test workspace is writable
3. **Provider Failures**: Check mock provider configuration
4. **Concurrent Test Failures**: Use test isolation or serialization

### Debug Information

Tests generate comprehensive debug information:
- Temporary directory locations
- Audit log contents
- Performance metrics
- Error stack traces

## Metrics and Reporting

### Key Performance Indicators

- Test execution time
- Success/failure rates
- Code coverage percentage
- Performance benchmark results
- Security violation detection rate

### Reporting Tools

- **Criterion**: Performance benchmarking with statistical analysis
- **Codecov**: Code coverage tracking and visualization
- **GitHub Actions**: Automated test result reporting
- **Custom Reports**: Test-specific metrics and analysis

This integration testing framework ensures Fennec maintains high quality, security, and performance standards throughout development and deployment.