---
name: rust-best-practices
description: Enforce Rust best practices including ownership patterns, error handling, testing, documentation, and code quality standards. Use when working with Rust code or when the user requests Rust-specific guidance. Includes patterns for cargo tooling, performance optimization, and idiomatic Rust conventions.
---

# Rust Best Practices

You are assisting with a Rust project. Apply Rust-specific best practices and conventions throughout all interactions.

## Core Principles

- **Safety First**: Prioritize memory safety and thread safety. Leverage Rust's ownership system.
- **Idiomatic Code**: Follow Rust conventions and patterns from the official style guide.
- **Performance**: Write efficient code, but prioritize correctness and readability first.
- **Error Handling**: Use Result and Option types appropriately. Avoid unwrap() in production code.

## Code Quality Standards

### Ownership and Borrowing
- Prefer borrowing (&T, &mut T) over owned types when possible
- Use references to avoid unnecessary clones
- Apply the "borrow checker rules": one mutable reference OR multiple immutable references
- Document lifetime relationships when they're not obvious

### Error Handling
- Use Result<T, E> for recoverable errors
- Use Option<T> for optional values
- Prefer ? operator over unwrap() or expect() in library code
- Use expect() with descriptive messages only in application code where context is clear
- Create custom error types using thiserror or similar for complex error scenarios
- Implement proper error propagation chains

### Type Design
- Use newtype pattern for semantic type safety
- Prefer enums over booleans for state representation
- Make invalid states unrepresentable through type design
- Use zero-sized types (ZSTs) for compile-time guarantees
- Implement appropriate traits (Debug, Clone, PartialEq, etc.) for types

### Code Organization
- Keep modules focused and cohesive
- Use pub(crate) for internal APIs
- Expose minimal public surface area
- Group related functionality in submodules
- Follow the module hierarchy conventions

### Performance Patterns
- Avoid unnecessary allocations and clones
- Use iterators instead of loops when appropriate
- Prefer iterator chains over intermediate collections
- Use &str for string slices, String for owned strings
- Consider Vec capacity pre-allocation when size is known
- Use Cow<str> when you might need either borrowed or owned data

### Async Code (when applicable)
- Use async/await for I/O-bound operations
- Prefer async over blocking operations in async contexts
- Be mindful of Send + Sync bounds for async functions
- Avoid blocking operations in async code
- Use tokio::spawn for concurrent tasks

## Testing Practices

### Unit Tests
- Place unit tests in a tests module at the bottom of each file
- Use #[cfg(test)] attribute
- Test both success and failure cases
- Use descriptive test names that explain what is being tested
- Follow the Arrange-Act-Assert pattern

### Integration Tests
- Place integration tests in tests/ directory
- Each file in tests/ is compiled as a separate crate
- Use common/ directory for shared test utilities
- Test the public API as users would interact with it

### Test Organization
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptive_name() {
        // Arrange
        let input = setup_test_data();

        // Act
        let result = function_under_test(input);

        // Assert
        assert_eq!(result, expected_value);
    }
}
```

## Documentation Standards

### Code Documentation
- Write doc comments (///) for all public items
- Include examples in doc comments where helpful
- Document safety requirements for unsafe code
- Explain panic conditions
- Document error cases

### Example Doc Comment
```rust
/// Processes the input data and returns a result.
///
/// # Arguments
///
/// * `input` - The data to process
///
/// # Returns
///
/// Returns `Ok(processed_data)` on success, or an error if processing fails.
///
/// # Errors
///
/// Returns `Err` if:
/// - Input is empty
/// - Input contains invalid characters
///
/// # Examples
///
/// ```
/// let result = process_data("valid input");
/// assert!(result.is_ok());
/// ```
pub fn process_data(input: &str) -> Result<String, Error> {
    // Implementation
}
```

## Common Patterns

### Builder Pattern
Use the builder pattern for complex construction:
```rust
pub struct Config {
    // fields
}

impl Config {
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }
}
```

### Newtype Pattern
Use newtype pattern for type safety:
```rust
pub struct UserId(u64);
pub struct ProductId(u64);
```

### Error Handling Pattern
Create custom error types:
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}
```

## Cargo and Dependencies

### Cargo.toml Management
- Keep dependencies minimal and up-to-date
- Use workspace dependencies for multi-crate projects
- Specify version requirements appropriately
- Document why each dependency is needed
- Use features to make optional dependencies optional

### Version Specifications
- Use caret requirements (^1.0) for semantic versioning
- Lock versions in applications (Cargo.lock)
- Be more permissive in libraries
- Regularly audit dependencies with cargo-audit

## Code Review Checklist

When reviewing or writing Rust code, verify:
- [ ] No unwrap() or expect() without justification
- [ ] Error handling is appropriate and propagated correctly
- [ ] No unnecessary clones or allocations
- [ ] Ownership and borrowing are used idiomatically
- [ ] Public APIs are well-documented
- [ ] Tests cover both success and error cases
- [ ] No compiler warnings remain
- [ ] Clippy lints are addressed
- [ ] Code follows the Rust style guide (rustfmt)

## Tool Usage

### Before Writing Code
- Read existing code to understand patterns and conventions
- Use Grep to search for similar implementations
- Check existing error handling patterns in the codebase

### When Making Changes
- Run cargo check to verify compilation
- Run cargo clippy to catch common mistakes
- Run cargo fmt to ensure consistent formatting
- Run cargo test to verify all tests pass
- Update documentation as needed

### Commit Process
- Ensure all tests pass before committing
- Run cargo fmt before committing
- Address all clippy warnings
- Write clear commit messages describing the changes

## Task Management

Use the TodoWrite tool to track Rust-specific tasks:
- Break down complex implementations into smaller steps
- Track compilation, testing, and linting as separate tasks
- Mark tasks complete only when tests pass and lints are clean
- Include "Run cargo fmt" and "Run cargo clippy" as final tasks

## Performance Considerations

- Profile before optimizing
- Use cargo bench for benchmarking
- Consider using Cow for potentially owned data
- Avoid premature optimization
- Use release builds for performance testing

## Security Best Practices

- Validate all inputs at boundaries
- Use type system to enforce invariants
- Avoid unsafe code unless absolutely necessary
- Document all unsafe blocks with safety justification
- Use cargo-audit to check for vulnerable dependencies
- Be careful with user-controlled format strings

## When to Use Unsafe

Unsafe code should be:
- Justified with clear safety documentation
- Minimized to the smallest possible scope
- Wrapped in safe abstractions
- Thoroughly tested
- Reviewed carefully

## Common Anti-Patterns to Avoid

- Using unwrap() without clear justification
- Cloning unnecessarily
- Using String when &str would suffice
- Ignoring compiler warnings
- Over-using Rc/Arc when ownership transfer is better
- Using panic! for normal error conditions
- Exposing implementation details in public APIs

## Interaction Style

- Keep responses concise and technical
- Focus on code quality and correctness
- Provide file paths with line numbers when referencing code
- Use TodoWrite to plan and track tasks
- Run tests after making changes
- Verify changes with cargo check and cargo clippy
