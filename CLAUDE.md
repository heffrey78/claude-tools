# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Claude Tools is a comprehensive CLI suite for working with Claude, Claude Code, and MCP servers. The project has evolved beyond MVP to include advanced features like activity timeline analysis, conversation analytics, export functionality, and intelligent caching systems.

## Common Development Commands

```bash
# Build and run
cargo build
cargo run -- --help

# Install globally for direct usage
cargo install --path .           # Install claude-tools binary globally
claude-tools --help              # Use installed binary directly

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

# Timeline CLI Examples
cargo run -- timeline --period day        # Via cargo
claude-tools timeline --period week       # Direct usage (after install)
claude-tools timeline --detailed --export json --output timeline.json
```

## Architecture

### Module Structure
- `src/cli/` - Command-line interface using clap
  - `args.rs` - CLI argument definitions (list, show, search, stats, interactive, timeline, mcp)
  - `commands.rs` - Command implementations with full functionality
- `src/claude/` - Claude-specific functionality
  - `directory.rs` - Claude directory detection and validation
  - `conversation.rs` - Conversation parsing and data structures
  - `analytics.rs` - Conversation analytics and statistics
  - `timeline.rs` - Activity timeline generation and analysis
  - `cache.rs` - Intelligent timeline caching system
  - `export.rs` - Conversation export functionality (Markdown, HTML, JSON)
  - `search.rs` - Advanced search engine with boolean queries, regex, and filtering
- `src/ui/` - Terminal user interface
  - `app.rs` - Main application state and event handling
  - `conversation_display.rs` - Conversation rendering with syntax highlighting
- `src/mcp/` - MCP server management
  - `discovery.rs` - MCP server discovery and monitoring
  - `server.rs` - Server configuration and status management
- `src/errors.rs` - Custom error types using thiserror

### Task Management System
The project uses a sophisticated task-driven development approach:
- `tasks/active/` - Current work items
- `tasks/completed/` - Finished tasks (TASK-0002 through TASK-0008)
- `tasks/backlog/` - Future work
- Each task has Gherkin-style behavioral specifications

### Development Phases
1. **Phase 1 (COMPLETED)**: MVP History Browser
2. **Phase 2 (COMPLETED)**: Enhanced browsing with analytics/exports
3. **Phase 3 (IN PROGRESS)**: MCP server management
4. **Phase 4**: Claude Code SDK integration
5. **Phase 5**: Agentic task management

## Key Implementation Details

- **Error Handling**: Uses `anyhow` for application errors and `thiserror` for library errors
- **JSON Processing**: Memory-efficient streaming of large (1GB+) conversation files with graceful error handling
- **Caching System**: Intelligent timeline caching with hash-based validation and hierarchical filtering
- **Performance**: Smart filtering instead of regeneration for timeline period switching
- **Export Formats**: Comprehensive export support (Markdown, HTML, JSON, CSV) with proper formatting
- **Testing**: Integration tests use `assert_cmd` for CLI testing
- **Platform Support**: Cross-platform directory handling via `dirs` crate
- **Terminal UI**: Built with `ratatui` for rich interactive experiences

## Current Status

### Completed Features
- **COMPLETED: TASK-0002** - Claude directory analysis and parsing fully implemented
- **COMPLETED: TASK-0004** - Terminal UI with keyboard navigation fully implemented
- **COMPLETED: TASK-0005** - Advanced conversation analytics and export functionality
- **COMPLETED: TASK-0006** - CLI integration and navigation features
- **COMPLETED: TASK-0008** - Activity timeline dashboard with intelligent caching
- **COMPLETED: TASK-0012** - Advanced search with boolean queries, regex, and multi-criteria filtering

### Core Functionality
- All CLI commands (list, show, search, stats, timeline, interactive) fully functional
- Interactive terminal UI with comprehensive conversation browsing
- Activity timeline dashboard with project-level analytics
- Advanced search engine with boolean queries, regex, natural language dates, and multi-criteria filtering
- Conversation export system (Markdown, HTML, JSON formats)
- Intelligent caching system for optimal timeline performance
- MCP server discovery and management framework

### Data Processing
- Supports parsing 127+ conversations from actual ~/.claude/ directory
- Memory-efficient JSONL parsing with silent error handling for corrupted files
- Full conversation metadata extraction and search capabilities
- Timeline analysis with tool usage tracking and topic extraction
- Robust UI rendering with screen corruption prevention during refresh operations

## Command Line Interface

### Interactive Mode
The interactive mode (`claude-tools interactive` or `cargo run -- interactive`) provides:
- **Vim-style Navigation**: j/k, g/G, /, ?, q keyboard shortcuts
- **Conversation Browser**: Scrollable list with summaries and metadata
- **Detail View**: Full conversation display with message content
- **Search Interface**: Live search across all conversation content with highlighting
- **Timeline Dashboard**: Press 't' for activity timeline with project navigation
- **Analytics Dashboard**: Press 'a' for conversation analytics and statistics
- **Help System**: Built-in help overlay with all keyboard shortcuts
- **Refresh Operations**: Press 'r' for clean data refresh without UI corruption
- **Smart Navigation**: Timeline navigation preserves context and returns properly on escape

### CLI Commands
```bash
# Core commands
claude-tools list --detailed                    # List conversations with details
claude-tools show <id> --format markdown        # Show conversation content
claude-tools search "(rust OR python) AND error" # Boolean search with operators
claude-tools search "async.*function" --regex       # Regular expression search
claude-tools search "debug" --tool bash --after "7 days ago" # Multi-criteria filtering
claude-tools stats --global                     # Global conversation statistics

# Timeline analysis
claude-tools timeline --period day              # Last 24 hours activity
claude-tools timeline --period week --detailed  # Detailed weekly timeline
claude-tools timeline --export json --output timeline.json

# Export functionality
claude-tools show <id> --export html --output conversation.html
claude-tools stats --export csv --output analytics.csv

# MCP server management
claude-tools mcp list --detailed                # List discovered MCP servers
claude-tools mcp discover --health-check        # Rediscover servers
```

### Timeline Features
- **Time Periods**: 24h, 48h, 1 week, 1 month analysis
- **Project Analytics**: Message counts, tool usage, peak activity times
- **Intelligent Caching**: Hash-based validation with instant period switching
- **Export Options**: JSON and CSV export for external analysis
- **Visual Indicators**: Activity bars, ranking indicators, trend analysis

### Advanced Search Features
- **Boolean Queries**: Support for AND, OR, NOT operators with parentheses grouping
- **Natural Language Dates**: Parse relative dates like "7 days ago", "last week", "yesterday"
- **Multi-Criteria Filtering**: Combine model, tool, role, date range, message count, and duration filters
- **Regular Expressions**: Full regex pattern matching with syntax validation
- **Relevance Scoring**: TF-IDF based ranking with recency and message length boosting
- **Result Highlighting**: Visual highlighting of matched terms in conversation snippets