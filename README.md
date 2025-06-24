# Claude Tools

A comprehensive suite of tools for working with Claude, Claude Code, and MCP servers.

## Current Status: Phase 2 Complete! 🎉

✅ **Advanced Features Delivered** - Timeline Analytics, Export System, Performance Optimization

## Features

### 🖥️ Interactive Terminal UI
- **Vim-style Navigation**: j/k, g/G, /, ?, q keyboard shortcuts
- **Conversation Browser**: Scrollable list with summaries and metadata
- **Detail View**: Full conversation display with markdown rendering and syntax highlighting
- **Advanced Search**: TF-IDF ranking, regex support, visual highlighting, navigation (n/N)
- **Timeline Dashboard**: Activity timeline with project analytics (press 't')
- **Analytics Dashboard**: Comprehensive statistics and insights (press 'a')
- **Export Functionality**: Export conversations directly from UI (press 'e' in detail view)
- **Help System**: Built-in help overlay with all keyboard shortcuts

### 🔍 Advanced Search Features
- **Boolean Queries**: Complex logic with AND, OR, NOT operators and parentheses grouping
- **Natural Language Dates**: Parse "7 days ago", "last week", "yesterday" automatically
- **Multi-Criteria Filtering**: Filter by model, tool, role, date range, message count, duration
- **Regular Expressions**: Full regex pattern matching with validation and optimization
- **Visual Highlighting**: Match highlighting in conversation snippets
- **Relevance Scoring**: TF-IDF ranking with recency and conversation quality boosting
- **Performance**: <50ms search times with parallel processing and intelligent caching

### 📊 Timeline & Analytics Dashboard
- **Activity Timeline**: Visual timeline showing project-level activity patterns
- **Time Period Analysis**: 24h, 48h, 1 week, 1 month time windows
- **Project Ranking**: Intelligent ranking with activity indicators and trends
- **Tool Usage Tracking**: Top tools analysis across projects and time periods
- **Intelligent Caching**: Hash-based validation with instant period switching
- **Export Capabilities**: Export timeline and analytics to JSON or CSV
- **Interactive Navigation**: Navigate from timeline to project conversations
- **Performance Optimization**: <200ms timeline generation with smart filtering

### 📊 Advanced Analytics
- **Comprehensive Metrics**: 6 analytics categories covering all conversation aspects
- **Temporal Analysis**: Peak usage hours, weekday patterns, activity trends
- **Model & Tool Usage**: Track which models and tools you use most
- **Project Analytics**: Understand conversation distribution across projects
- **Quality Metrics**: Average duration, completion rates, message length stats
- **Interactive Dashboard**: Terminal UI analytics view with scrolling and navigation

### 💻 Command Line Interface
- **List Conversations**: View all conversations with optional detailed view
- **Show Conversations**: Display full conversation content in multiple formats
- **Timeline Commands**: Generate activity timelines with various time periods
- **Export Conversations**: Export to Markdown, HTML, JSON formats with full metadata
- **Search Content**: Find conversations by content with match highlighting
- **Statistics**: Comprehensive analytics with export capabilities (JSON/CSV)
- **MCP Management**: Discover and manage MCP servers

### 🚀 Quick Start
```bash
# Clone and build
git clone <repository-url>
cd claude_code
cargo build --release

# Install globally for easier usage
cargo install --path .

# Interactive terminal UI (recommended)
claude-tools interactive  # or: cargo run -- interactive

# Timeline analysis
claude-tools timeline --period day
claude-tools timeline --period week --detailed
claude-tools timeline --export json --output timeline.json

# Command line interface
claude-tools list --detailed
claude-tools search "(rust OR python) AND error"  # Boolean queries
claude-tools search "async.*function" --regex        # Regular expressions
claude-tools search "debug" --tool bash --after "7 days ago"  # Multi-criteria
claude-tools show <conversation-id> --format markdown
claude-tools show <id> --export html --output conversation.html
claude-tools stats --global --export csv
```

### 📈 Performance
- **127+ conversations** parsed efficiently from real ~/.claude/ data
- **17,976+ messages** processed with memory-efficient streaming
- **<200ms timeline generation** with intelligent caching and filtering
- **<50ms search times** with parallel processing and TF-IDF ranking
- **Instant period switching** via smart filtering instead of regeneration
- **Cross-platform** support (Linux, macOS, Windows)

## Development Roadmap

### Phase 1: MVP - History Browser ✅ Complete
- ✅ CLI foundation and argument parsing
- ✅ ~/.claude/ directory analysis and parsing  
- ✅ Streaming JSON reader for large files
- ✅ Terminal UI with keyboard navigation
- ✅ Human-readable conversation display
- ✅ Search and filtering functionality
- ✅ Help system and documentation

### Phase 2: Enhanced Browsing ✅ Complete
- ✅ Advanced search with boolean queries, regex, and multi-criteria filtering
- ✅ Visual search highlighting and navigation
- ✅ Markdown rendering with syntax highlighting
- ✅ Conversation statistics and analytics
- ✅ Export to multiple formats (markdown, HTML, JSON) with CLI and interactive UI
- ✅ Activity timeline dashboard with intelligent caching
- ✅ Timeline CLI integration with export functionality
- ✅ Performance optimization with smart filtering

### Phase 3: MCP Server Management (In Progress)
- ✅ Server discovery framework
- ✅ Configuration management structure
- ✅ Basic CLI integration for server listing
- 🔲 Health monitoring and diagnostics
- 🔲 Advanced server management features
- 🔲 Log aggregation and viewing

### Phase 4: Claude Code SDK Integration (Vision)
- Multi-instance coordination
- Workflow orchestration
- Session templates and presets
- Cross-session context sharing

### Phase 5: Agentic Task Management (Vision)
- Formal task pipeline system
- Automated execution with human oversight
- Dependency management
- Progress tracking and reporting

## Technical Stack

- **Language**: Rust (for performance and memory safety)
- **CLI**: clap for argument parsing with comprehensive help
- **TUI**: ratatui for terminal interface with crossterm
- **Search**: Custom TF-IDF engine with rayon parallel processing
- **Analytics**: Timeline analysis with temporal indexing and caching
- **Rendering**: pulldown-cmark for markdown, syntect for syntax highlighting
- **Export**: Multi-format export system (Markdown, HTML, JSON, CSV)
- **JSON**: serde_json with streaming support for large files
- **Caching**: Intelligent hash-based caching with LRU eviction
- **Performance**: Smart filtering and hierarchical cache lookups

## Project Structure

```
claude_code/
├── src/                       # Source code
│   ├── cli/                   # Command-line interface
│   │   ├── args.rs           # CLI argument definitions
│   │   └── commands.rs       # Command implementations
│   ├── claude/               # Claude-specific functionality
│   │   ├── analytics.rs      # Conversation analytics
│   │   ├── cache.rs          # Timeline caching system
│   │   ├── conversation.rs   # Data structures and parsing
│   │   ├── directory.rs      # Directory detection
│   │   ├── export.rs         # Export functionality
│   │   ├── search.rs         # Search engine
│   │   └── timeline.rs       # Activity timeline
│   ├── mcp/                  # MCP server management
│   │   ├── discovery.rs      # Server discovery
│   │   └── server.rs         # Server management
│   ├── ui/                   # Terminal user interface
│   │   ├── app.rs            # Main application state
│   │   └── conversation_display.rs # Rendering
│   ├── errors.rs             # Error handling
│   ├── lib.rs                # Library exports
│   └── main.rs               # Application entry point
├── tasks/                    # Task management
│   ├── active/               # Current work items
│   ├── completed/            # Finished tasks
│   └── backlog/              # Future tasks
├── tests/                    # Integration tests
├── CLAUDE.md                 # Development guidance
└── README.md                 # Project documentation
```

## Contributing

This project follows a structured task management approach. See [tasks/tasks-directive.md](tasks/tasks-directive.md) for guidelines on creating and managing tasks.

## License

[License TBD]

---

*This project is in early development. Star and watch for updates!*