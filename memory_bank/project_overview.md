# Fennec Project Overview

## Mission & Vision

Fennec is a powerful, secure, and extensible Terminal User Interface (TUI) AI assistant designed specifically for developers. It brings AI-powered development workflows directly to the terminal with enterprise-grade security, comprehensive audit trails, and intelligent memory management.

## Core Value Propositions

### 🖥️ Interactive Terminal Interface
- Clean, responsive TUI built with ratatui for optimal developer experience
- Multi-pane layout with chat, preview, and status panels
- Command palette with fuzzy search and keyboard shortcuts
- Real-time streaming responses with syntax highlighting

### 🤖 AI-Powered Development Commands
- **`plan`** - Generate structured implementation plans and task breakdowns
- **`edit`** - Make precise file edits with intelligent diff previews
- **`run`** - Execute shell commands safely within sandbox constraints
- **`diff`** - Show detailed file changes and git-style diffs
- **`summarize`** - Create session summaries and memory updates

### 🔒 Enterprise-Grade Security
- Three-tier sandbox model: `read-only`, `workspace-write`, `danger-full-access`
- Approval workflows with risk assessment and confirmation prompts
- Path traversal protection and command filtering
- Capability-based permissions system with fine-grained controls

### 🧠 Intelligent Memory System
- AGENTS.md integration for repository-specific guidelines
- Cline-style memory files (`projectbrief.md`, `activeContext.md`, `progress.md`)
- Session transcripts with full conversation history
- Git history awareness for contextual code understanding

## Technical Architecture

### Language & Ecosystem
- **Rust 2021 Edition** - Memory safety, performance, and modern language features
- **Async/Await** - Tokio runtime for concurrent operations
- **TUI Framework** - ratatui + crossterm for terminal interface
- **Workspace** - Multi-crate monorepo with clear separation of concerns

### Design Principles
- **Security by Design** - All operations validated through sandbox policies
- **Provider Agnostic** - Clean abstractions for multiple LLM backends
- **Extensible Commands** - Plugin-ready command system with capability declarations
- **Audit Everything** - Complete traceability of all system actions
- **Memory Aware** - Context-aware operations with persistent learning

## Current Status

**Release Status**: 🎯 MVP Complete, Production Ready

**Completed Milestones**:
- ✅ Milestone 0: Project scaffold and workspace setup
- ✅ Milestone 1: Core conversation loop and TUI
- ✅ Milestone 2: Editing and sandbox enforcement
- ✅ Milestone 3: Memory and summaries
- ✅ Milestone 4: Hardening and release readiness

## Key Differentiators

### vs. Other AI Assistants
- **Terminal-native** - Designed for developer workflow integration
- **Security-first** - Enterprise-grade sandbox and audit from day one
- **Memory persistence** - Context carries across sessions
- **Git-aware** - Understanding of repository context and history

### vs. Web-based Solutions
- **Offline-capable** - Core functionality works without constant connectivity
- **Performance** - Native Rust performance for large codebases
- **Privacy** - Local processing where possible, controlled external calls

## Target Users

### Primary
- **Software developers** working in terminal environments
- **DevOps engineers** managing infrastructure and deployments
- **Technical leads** reviewing code and planning architecture

### Secondary
- **Students** learning programming and development workflows
- **Open source maintainers** managing large codebases
- **Enterprise teams** requiring auditable AI assistance

## Integration Philosophy

### Inspiration Sources
- **Claude Code** - Interactive coding experience and diff workflows
- **Codex CLI** - Sandbox security and approval patterns
- **Cline Memory Bank** - Persistent memory and project context

### Compatibility Goals
- **Cline memory files** - Import and use existing project context
- **Standard tools** - Integrate with git, cargo, npm, etc.
- **Editor agnostic** - Work with any editor or IDE

---

*This overview provides the strategic context for all development decisions and architectural choices in the Fennec project.*