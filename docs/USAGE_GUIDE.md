# Fennec Usage Guide

A comprehensive guide to using Fennec AI Assistant for development workflows.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Basic Commands](#basic-commands)
3. [Advanced Features](#advanced-features)
4. [Workflow Examples](#workflow-examples)
5. [Security and Sandbox](#security-and-sandbox)
6. [Memory and Context](#memory-and-context)
7. [Configuration](#configuration)
8. [Troubleshooting](#troubleshooting)

## Getting Started

### First Launch

After installation, start Fennec in your project directory:

```bash
cd /path/to/your/project
fennec
```

### Initial Configuration

On first run, Fennec will guide you through setup:

1. **API Key Setup**: Fennec will prompt for your OpenAI API key if not found
2. **Workspace Detection**: Automatically detects Git repositories and project structure
3. **AGENTS.md Setup**: Offers to create project-specific guidelines

### Understanding the Interface

Fennec's TUI consists of three main panels:

- **Chat Panel** (left): Conversation history and command input
- **Preview Panel** (right): File diffs, command previews, and results
- **Status Bar** (bottom): Current sandbox level, session info, and shortcuts

### Keyboard Shortcuts

- `Ctrl+C` - Exit Fennec
- `Tab` - Switch between panels
- `Enter` - Send message or confirm action
- `Esc` - Cancel current operation
- `Ctrl+L` - Clear chat history
- `Ctrl+S` - Save session summary

## Basic Commands

### Planning: `plan`

Generate structured implementation plans for tasks:

```
plan "Add user authentication to my web application"
```

**Features:**
- Breaks down complex tasks into manageable steps
- Considers existing codebase structure
- Provides time estimates and dependencies
- Suggests best practices and patterns

**Example Output:**
```
Implementation Plan: Add User Authentication

1. Database Schema
   - Create users table with email, password_hash, created_at
   - Add authentication_tokens table for sessions
   - Create database migration scripts

2. Backend Authentication
   - Implement password hashing with bcrypt
   - Create login/logout endpoints
   - Add JWT token generation and validation
   - Implement middleware for protected routes

3. Frontend Integration
   - Create login/register forms
   - Add authentication state management
   - Implement protected route handling
   - Add logout functionality

4. Security Considerations
   - Add rate limiting for login attempts
   - Implement password strength validation
   - Add CSRF protection
   - Configure secure session cookies
```

### File Editing: `edit`

Make precise file modifications with intelligent diff previews:

```
edit src/auth.rs "Add password validation function with strength requirements"
```

**Features:**
- Context-aware code modifications
- Syntax highlighting in previews
- Backup creation before changes
- Undo/redo capabilities

**Edit Types:**
- **Function additions**: Add new functions with proper signatures
- **Bug fixes**: Targeted corrections with context preservation
- **Refactoring**: Safe code restructuring with impact analysis
- **Feature additions**: New functionality with integration checks

### Command Execution: `run`

Execute shell commands safely within sandbox constraints:

```
run "cargo test --lib"
run "npm install lodash"
run "git status"
```

**Safety Features:**
- Sandbox policy enforcement
- Command risk assessment
- Approval workflows for dangerous operations
- Output capturing and analysis

### Diff Viewing: `diff`

Show file changes and comparisons:

```
diff src/main.rs
diff --staged
diff HEAD~1..HEAD
```

**Diff Types:**
- **File diffs**: Changes between versions
- **Git diffs**: Repository change analysis
- **Preview diffs**: Planned modifications
- **Comparison diffs**: Between different files

### Summarization: `summarize`

Create session summaries and memory updates:

```
summarize
summarize --depth detailed --type progress
summarize --output memory
```

**Summary Types:**
- **Session**: Complete conversation summary
- **Progress**: Development progress tracking
- **Brief**: Concise task completion notes

## Advanced Features

### Enhanced Commands

#### Advanced Planning

```
# Plan with specific complexity level
plan "Implement microservices architecture" --complexity high

# Plan with technology constraints
plan "Add real-time notifications" --tech "WebSocket, Redis"

# Plan with timeline
plan "Migrate to TypeScript" --timeline "2 weeks"
```

#### Sophisticated Editing

```
# Edit with specific style
edit src/utils.ts "Refactor to use functional programming patterns"

# Edit with test generation
edit src/calculator.js "Add error handling and generate unit tests"

# Edit with documentation
edit src/api.py "Add comprehensive docstrings and type hints"
```

#### Advanced Summarization

```
# Detailed technical summary
summarize --depth comprehensive --type technical

# Progress report for stakeholders
summarize --depth executive --type progress --output file

# Memory update with context preservation
summarize --type context --output memory --preserve-history
```

### Command Chaining

Chain multiple commands for complex workflows:

```
plan "Add logging system" && edit src/logger.rs "Implement structured logging" && run "cargo test logger" && summarize --type progress
```

### Macro Commands

Define reusable command sequences:

```toml
# In ~/.config/fennec/macros.toml
[macros.deploy]
description = "Full deployment workflow"
commands = [
    "run 'cargo test'",
    "run 'cargo build --release'",
    "run './scripts/deploy.sh'",
    "summarize --type deployment"
]
```

## Workflow Examples

### 1. Bug Fix Workflow

```bash
# Start Fennec in project directory
fennec --cd ~/projects/my-app

# Analyze the issue
plan "Fix memory leak in user session handling"

# Examine relevant files
diff src/session.rs
edit src/session.rs "Add proper cleanup in session destructor"

# Test the fix
run "cargo test session_tests"
run "cargo clippy -- -D warnings"

# Document the fix
summarize --type bugfix --output memory
```

### 2. Feature Development Workflow

```bash
# Start with planning
plan "Add dark mode toggle to settings page"

# Implement step by step
edit src/components/Settings.tsx "Add dark mode toggle component"
edit src/styles/themes.css "Add dark theme variables"
edit src/context/ThemeContext.tsx "Add theme state management"

# Test implementation
run "npm test -- --testNamePattern='dark mode'"
run "npm run build"

# Review changes
diff --staged
summarize --type feature --output progress
```

### 3. Refactoring Workflow

```bash
# Plan the refactoring
plan "Extract database operations into repository pattern"

# Execute refactoring in stages
edit src/models/user.rs "Extract UserRepository trait"
edit src/services/user_service.rs "Implement repository pattern"
edit src/controllers/user_controller.rs "Update to use repository"

# Verify refactoring
run "cargo test"
run "cargo clippy"

# Document changes
summarize --type refactoring --depth detailed
```

### 4. Code Review Workflow

```bash
# Start in read-only mode for safety
fennec --sandbox read-only

# Analyze pull request
diff feature-branch..main
plan "Review authentication security implementation"

# Check specific concerns
edit src/auth.rs "Analyze potential security vulnerabilities"
run "cargo audit"

# Generate review summary
summarize --type review --output file
```

## Security and Sandbox

### Sandbox Levels

#### Read-Only Mode
```bash
fennec --sandbox read-only
```

**Capabilities:**
- File reading and analysis
- Code review and exploration
- Planning and documentation
- No modifications or execution

**Use Cases:**
- Code exploration
- Security reviews
- Learning new codebases
- Safe experimentation

#### Workspace Write Mode (Default)
```bash
fennec --sandbox workspace-write
```

**Capabilities:**
- Read/write within project directory
- Limited shell command execution
- File modifications with approval
- Safe development operations

**Use Cases:**
- Regular development work
- Feature implementation
- Bug fixing
- Code refactoring

#### Full Access Mode
```bash
fennec --sandbox danger-full-access --ask-for-approval
```

**Capabilities:**
- Full system access
- Unrestricted shell execution
- Network operations
- System-wide modifications

**Use Cases:**
- System administration
- DevOps tasks
- Package management
- Advanced development workflows

### Approval Workflows

#### Manual Approval
```bash
fennec --ask-for-approval
```

All potentially dangerous operations require explicit user confirmation.

#### Auto-Approval for Low Risk
```bash
fennec --ask-for-approval --auto-approve-low-risk
```

Low-risk operations are automatically approved, high-risk operations require confirmation.

### Risk Assessment

Fennec automatically classifies operations by risk level:

- **Low Risk**: File reads, safe commands (`ls`, `cat`, `head`)
- **Medium Risk**: File writes, package installs, network requests
- **High Risk**: System modifications, sudo commands, destructive operations
- **Critical Risk**: Commands that could damage system or data

## Memory and Context

### AGENTS.md Integration

Fennec automatically loads and uses project guidelines from `AGENTS.md`:

```markdown
# Project Guidelines

## Coding Standards
- Use TypeScript for all new frontend code
- Follow the repository's ESLint configuration
- Write unit tests for all business logic functions

## Architecture Decisions
- Use Redux Toolkit for state management
- Implement error boundaries in React components
- Follow the established folder structure in src/

## Security Requirements
- Validate all user inputs
- Use HTTPS for all API calls
- Implement proper authentication checks
```

### Memory Files

Fennec maintains several memory files for project context:

#### `projectbrief.md`
High-level project description and goals:
```markdown
# Project Brief: E-commerce Platform

## Overview
Modern e-commerce platform built with React, Node.js, and PostgreSQL.

## Current Status
- User authentication: Complete
- Product catalog: In progress
- Payment integration: Planned

## Key Technologies
- Frontend: React 18, TypeScript, TailwindCSS
- Backend: Node.js, Express, TypeScript
- Database: PostgreSQL with Prisma ORM
```

#### `activeContext.md`
Current development context and focus:
```markdown
# Active Development Context

## Current Sprint: Payment Integration
- Implementing Stripe payment processing
- Adding order management system
- Creating payment confirmation workflows

## Recent Changes
- Added user authentication system
- Implemented product search functionality
- Set up CI/CD pipeline

## Next Priorities
1. Complete payment integration
2. Add order history page
3. Implement email notifications
```

#### `progress.md`
Development progress and session summaries:
```markdown
# Development Progress

## Session 2024-01-15
### Completed
- Fixed memory leak in session handling
- Added comprehensive error logging
- Updated user authentication tests

### Decisions
- Chose Redis for session storage
- Implemented JWT with refresh tokens
- Added rate limiting for login attempts

### Next Session
- Implement password reset functionality
- Add account verification workflow
- Set up monitoring dashboard
```

### Context Retrieval

Fennec intelligently retrieves relevant context for each task:

```
# Automatic context injection
plan "Add password reset feature"

# Fennec automatically includes:
# - Authentication system context from memory
# - Related security requirements from AGENTS.md
# - Recent authentication-related changes
# - Project architecture decisions
```

## Configuration

### Configuration File Location

Fennec looks for configuration in these locations (in order):
1. `--config` command line argument
2. `./fennec.toml` (project-specific)
3. `~/.config/fennec/config.toml` (user-specific)
4. Built-in defaults

### Example Configuration

```toml
# ~/.config/fennec/config.toml

[provider]
# Default LLM provider
default = "openai"
model = "gpt-4"
max_tokens = 4096
temperature = 0.1

[provider.openai]
api_key = "${OPENAI_API_KEY}"
base_url = "https://api.openai.com/v1"
timeout = 30

[security]
# Default sandbox level
default_sandbox = "workspace-write"
require_approval = false
auto_approve_low_risk = true

# Dangerous command patterns
dangerous_commands = [
    "rm -rf",
    "sudo rm",
    "format",
    "shutdown"
]

[memory]
# Memory file locations
project_brief = "projectbrief.md"
active_context = "activeContext.md"
progress_log = "progress.md"

# Memory retention settings
max_session_history = 100
auto_summarize_threshold = 50

[ui]
# TUI configuration
theme = "dark"
syntax_highlighting = true
show_line_numbers = true
wrap_long_lines = true

[audit]
# Audit logging configuration
enabled = true
log_file = "~/.local/share/fennec/audit.jsonl"
retention_days = 90
include_command_output = false
```

### Environment Variables

```bash
# Required
export OPENAI_API_KEY="sk-your-key-here"

# Optional
export FENNEC_CONFIG_PATH="~/.config/fennec/config.toml"
export FENNEC_LOG_LEVEL="info"
export FENNEC_AUDIT_LOG="~/.local/share/fennec/audit.jsonl"
```

## Troubleshooting

### Common Issues

#### API Key Not Found
```
Error: OpenAI API key not configured

Solution:
1. Set environment variable: export OPENAI_API_KEY="sk-your-key"
2. Or add to .env file: echo "OPENAI_API_KEY=sk-your-key" > .env
3. Or configure in config.toml: api_key = "sk-your-key"
```

#### Permission Denied
```
Error: Operation denied by sandbox policy

Solution:
1. Use higher sandbox level: fennec --sandbox danger-full-access
2. Enable approval prompts: fennec --ask-for-approval
3. Check file permissions in workspace
```

#### Command Not Found
```
Error: Command 'advanced-plan' not found

Solution:
1. Check available commands: help
2. Verify command syntax: plan --help
3. Update Fennec to latest version
```

#### Memory Files Not Loading
```
Warning: Failed to load AGENTS.md

Solution:
1. Create AGENTS.md in project root
2. Check file permissions
3. Verify file format (Markdown)
```

### Debug Mode

Enable verbose logging for troubleshooting:

```bash
fennec --verbose
# or
RUST_LOG=debug fennec
```

### Log Locations

- **Application logs**: `~/.local/share/fennec/fennec.log`
- **Audit logs**: `~/.local/share/fennec/audit.jsonl`
- **Session transcripts**: `~/.local/share/fennec/sessions/`
- **Memory files**: Project directory

### Getting Help

#### Built-in Help
```
help                    # Show all commands
plan --help            # Show command-specific help
status                  # Show current configuration and status
```

#### Community Resources
- **Documentation**: [docs/](../docs/)
- **Examples**: [examples/](../examples/)
- **Issues**: GitHub Issues
- **Discussions**: GitHub Discussions

### Performance Tuning

#### For Large Projects
```toml
[memory]
max_context_files = 50
index_update_interval = 300  # seconds
background_indexing = true

[provider]
request_timeout = 60
max_concurrent_requests = 3
```

#### For Slow Networks
```toml
[provider]
timeout = 120
retry_attempts = 5
retry_delay = 2000  # milliseconds
stream_timeout = 30
```

#### For Memory Constrained Systems
```toml
[memory]
max_session_history = 25
auto_cleanup_threshold = 100
compress_old_sessions = true

[ui]
buffer_size = 1000
lazy_rendering = true
```

This comprehensive usage guide covers all major aspects of using Fennec effectively. For specific use cases or advanced configurations, refer to the additional documentation in the `docs/` directory.