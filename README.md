# Claude Tools

A comprehensive suite of tools for working with Claude, Claude Code, and MCP servers.

## Current Status: MVP Complete! 🎉

✅ **MVP Delivered** - Claude History Browser CLI with Terminal UI

## Features

### 🖥️ Interactive Terminal UI
- **Vim-style Navigation**: j/k, g/G, /, ?, q keyboard shortcuts
- **Conversation Browser**: Scrollable list with summaries and metadata
- **Detail View**: Full conversation display with markdown rendering and syntax highlighting
- **Advanced Search**: TF-IDF ranking, regex support, visual highlighting, navigation (n/N)
- **Analytics Dashboard**: Comprehensive statistics and insights (press 'a')
- **Export Functionality**: Export conversations directly from UI (press 'e' in detail view)
- **Help System**: Built-in help overlay with all keyboard shortcuts

### 🔍 Advanced Search Features
- **Multiple Search Modes**: Text, regex (`regex:`), and fuzzy search (`fuzzy:`)
- **Visual Highlighting**: Yellow background highlighting of search matches
- **Search Navigation**: Navigate between results with 'n' and 'N' keys
- **Performance**: <50ms search times with parallel processing and caching
- **Smart Ranking**: TF-IDF scoring with recency boost and length normalization
- **Filtering**: Date range and project filtering capabilities

### 📊 Analytics and Insights
- **Comprehensive Metrics**: 6 analytics categories covering all conversation aspects
- **Temporal Analysis**: Peak usage hours, weekday patterns, activity trends
- **Model & Tool Usage**: Track which models and tools you use most
- **Project Analytics**: Understand conversation distribution across projects
- **Quality Metrics**: Average duration, completion rates, message length stats
- **Export Capabilities**: Export analytics to JSON or CSV for external analysis
- **Interactive Dashboard**: Terminal UI analytics view with scrolling and navigation

### 📊 Command Line Interface
- **List Conversations**: View all conversations with optional detailed view
- **Show Conversations**: Display full conversation content in multiple formats
- **Export Conversations**: Export to Markdown, HTML, JSON formats with full metadata
- **Search Content**: Find conversations by content with match highlighting
- **Statistics**: Comprehensive analytics with export capabilities (JSON/CSV)

### 🚀 Quick Start
```bash
# Clone and build
git clone <repository-url>
cd claude_code
cargo build --release

# Interactive terminal UI (recommended)
cargo run -- interactive

# Or use command line interface
cargo run -- list --detailed
cargo run -- search "claude code"
cargo run -- show <conversation-id>
cargo run -- show <id> --export markdown --include-metadata --include-tools
cargo run -- stats --detailed
cargo run -- stats --export json  # Export analytics data
```

### 📈 Performance
- **127+ conversations** parsed efficiently from real ~/.claude/ data
- **17,976+ messages** processed with memory-efficient streaming
- **<2.4s startup** time for full directory analysis
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

### Phase 2: Enhanced Browsing (In Progress)
- ✅ Advanced search with TF-IDF ranking and regex support
- ✅ Visual search highlighting and navigation
- ✅ Markdown rendering with syntax highlighting
- ✅ Conversation statistics and analytics
- ✅ Export to multiple formats (markdown, HTML, JSON) with CLI and interactive UI
- 🔲 Conversation tagging and organization

### Phase 3: MCP Server Management (Planned)
- Server discovery and lifecycle management
- Configuration management
- Health monitoring and diagnostics
- Log aggregation and viewing

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
- **CLI**: clap for argument parsing
- **TUI**: ratatui for terminal interface with crossterm
- **Search**: Custom TF-IDF engine with rayon parallel processing
- **Rendering**: pulldown-cmark for markdown, syntect for syntax highlighting
- **JSON**: serde_json with streaming support
- **Caching**: LRU caching for performance optimization

## Project Structure

```
claude_code/
├── tasks/                  # Task management
│   ├── tasks-directive.md  # Task creation guidelines
│   ├── roadmap.md         # Project roadmap
│   ├── active/            # Current sprint tasks
│   ├── backlog/           # Future tasks
│   ├── completed/         # Archived tasks
│   └── templates/         # Task templates
├── src/                   # Source code (coming soon)
├── tests/                 # Test suites (coming soon)
└── docs/                  # Documentation (coming soon)
```

## Contributing

This project follows a structured task management approach. See [tasks/tasks-directive.md](tasks/tasks-directive.md) for guidelines on creating and managing tasks.

## License

[License TBD]

---

*This project is in early development. Star and watch for updates!*