# Fennec Memory Bank

This directory contains the memory bank for the Fennec AI Assistant project. The memory bank serves as a persistent knowledge base that helps maintain context across development sessions and provides quick reference to important project information.

## Directory Structure

```
memory_bank/
├── README.md                 # This file - overview and guidelines
├── project_overview.md       # High-level project summary and architecture
├── crate_structure.md        # Detailed crate organization and dependencies
├── development_workflow.md   # Common development tasks and procedures
├── security_model.md         # Security architecture and sandbox levels
├── memory_system.md          # Memory and context management approach
├── provider_architecture.md  # LLM provider abstraction and implementation
├── tui_architecture.md       # Terminal UI organization and components
└── recent_changes.md         # Recent commits and important changes
```

## Usage Guidelines

### When to Update
- At the start of new development sessions
- After major architectural changes
- When adding new features or crates
- After important bug fixes or refactoring
- When context understanding improves

### How to Use
1. **Start of Session**: Read relevant memory bank files to understand current state
2. **During Development**: Reference specific files for context and patterns
3. **End of Session**: Update memory bank with new insights and changes

### Memory Bank Principles
- **Accuracy**: Keep information current and factual
- **Conciseness**: Focus on essential knowledge and patterns
- **Accessibility**: Structure for quick reference and understanding
- **Context Preservation**: Maintain important decisions and rationale

## Key Project Information

**Project**: Fennec AI Assistant
**Type**: Terminal UI AI assistant for developers
**Language**: Rust (2021 edition)
**Architecture**: Multi-crate workspace with clear separation of concerns
**License**: MIT OR Apache-2.0
**Status**: MVP complete, production ready

## Quick Reference

- **Main Binary**: `fennec-cli`
- **Core Types**: `fennec-core`
- **UI Layer**: `fennec-tui`
- **Security**: `fennec-security` (sandbox and audit)
- **Memory**: `fennec-memory` (persistence and context)
- **Providers**: `fennec-provider` (LLM abstractions)
- **Commands**: `fennec-commands` (built-in commands)
- **Orchestration**: `fennec-orchestration` (session management)

## Development Commands

```bash
# Build and test
cargo build
cargo test
cargo clippy --all-targets --all-features

# Run Fennec
cargo run --bin fennec

# Smoke test
./scripts/smoke.sh
```

---

*Last updated: 2025-09-23*