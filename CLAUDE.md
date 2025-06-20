# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Claude Tools is a CLI suite for working with Claude, Claude Code, and MCP servers. Currently in MVP phase, focusing on a Claude History Browser that explores conversation history in `~/.claude/`.

## Common Development Commands

```bash
# Build and run
cargo build
cargo run -- --help

# Testing
cargo test                        # Run all tests
cargo test -- --nocapture        # See test output
cargo test test_name             # Run specific test

# Code quality
cargo fmt                        # Format code
cargo clippy                     # Lint code
cargo clippy -- -D warnings      # Strict linting

# Check before committing
cargo check && cargo fmt && cargo clippy && cargo test
```

## Architecture

### Module Structure
- `src/cli/` - Command-line interface using clap
  - `args.rs` - CLI argument definitions (list, show, search, stats, interactive)
  - `commands.rs` - Command implementations (currently placeholders)
- `src/claude/` - Claude-specific functionality
  - `directory.rs` - Claude directory detection and validation
- `src/errors.rs` - Custom error types using thiserror

### Task Management System
The project uses a sophisticated task-driven development approach:
- `tasks/active/` - Current work items (TASK-0002 through TASK-0007)
- `tasks/completed/` - Finished tasks
- `tasks/backlog/` - Future work
- Each task has Gherkin-style behavioral specifications

### Development Phases
1. **Phase 1 (Current)**: MVP History Browser
2. **Phase 2**: Enhanced browsing with analytics/exports
3. **Phase 3**: MCP server management
4. **Phase 4**: Claude Code SDK integration
5. **Phase 5**: Agentic task management

## Key Implementation Details

- **Error Handling**: Uses `anyhow` for application errors and `thiserror` for library errors
- **JSON Processing**: Designed for memory-efficient streaming of large (1GB+) conversation files
- **Testing**: Integration tests use `assert_cmd` for CLI testing
- **Platform Support**: Cross-platform directory handling via `dirs` crate

## Current Status

- **COMPLETED: TASK-0002** - Claude directory analysis and parsing fully implemented
- **COMPLETED: TASK-0004** - Terminal UI with keyboard navigation fully implemented
- All core commands (list, show, search, stats) are functional with real data
- Interactive terminal UI provides intuitive conversation browsing experience
- Supports parsing 127+ conversations from actual ~/.claude/ directory
- Memory-efficient JSONL parsing with error handling for corrupted files
- Full conversation metadata extraction and search capabilities

## Terminal UI Features

The interactive mode (`cargo run -- interactive`) provides:
- **Vim-style Navigation**: j/k, g/G, /, ?, q keyboard shortcuts
- **Conversation Browser**: Scrollable list with summaries and metadata
- **Detail View**: Full conversation display with message content
- **Search Interface**: Live search across all conversation content
- **Help System**: Built-in help overlay with all keyboard shortcuts