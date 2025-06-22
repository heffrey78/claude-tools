use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "claude-tools",
    version,
    about = "A comprehensive suite of tools for working with Claude, Claude Code, and MCP servers",
    long_about = "Claude Tools provides utilities for browsing conversation history, managing MCP servers, and coordinating Claude Code workflows.

QUICK START:
    claude-tools interactive           # Start the interactive browser (recommended)
    claude-tools list --detailed       # List all conversations with details
    claude-tools search \"rust code\"    # Search for conversations containing 'rust code'
    claude-tools show <id>             # Show specific conversation content

EXAMPLES:
    # Interactive browsing with search and navigation
    claude-tools interactive
    
    # List recent conversations from last 7 days
    claude-tools list --since 7 --detailed
    
    # Search with regex patterns
    claude-tools search --regex \"error.*handling\"
    
    # Show conversation in markdown format
    claude-tools show abc123 --format markdown
    
    # Get statistics for your conversation history
    claude-tools stats --global

The interactive mode provides the best experience with vim-style navigation,
real-time search with highlighting, and comprehensive keyboard shortcuts.",
    after_help = "For more help within interactive mode, press '?' or 'h' for keyboard shortcuts."
)]
pub struct Cli {
    /// Path to Claude directory (defaults to ~/.claude/)
    #[arg(long, global = true, value_name = "DIR")]
    pub claude_dir: Option<PathBuf>,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List conversations and projects
    #[command(
        alias = "ls",
        long_about = "List conversations and projects with optional filtering and detailed view.

EXAMPLES:
    claude-tools list                      # List all conversations (summary)
    claude-tools list --detailed          # Show detailed conversation info
    claude-tools list --since 7           # Show conversations from last 7 days
    claude-tools list --project \"my-app\"   # Filter by project path
    claude-tools list --since 3 --detailed # Recent detailed conversations"
    )]
    List {
        /// Show only recent conversations (last N days)
        #[arg(long, value_name = "DAYS", help = "Show conversations from last N days (e.g., --since 7)")]
        since: Option<u32>,

        /// Filter by project path
        #[arg(long, value_name = "PATH", help = "Filter conversations by project path (partial match)")]
        project: Option<String>,

        /// Show detailed information including message counts and timestamps
        #[arg(short, long)]
        detailed: bool,
    },

    /// Show detailed conversation content
    #[command(
        alias = "cat",
        long_about = "Display the complete content of a specific conversation with formatting options.

EXAMPLES:
    claude-tools show abc123                    # Show conversation in human-readable format
    claude-tools show abc --format markdown    # Export conversation as markdown
    claude-tools show abc --format json        # Raw JSON output for scripting
    claude-tools show abc --role user          # Show only user messages
    claude-tools show abc --role assistant     # Show only assistant responses
    claude-tools show abc --export markdown --output conversation.md  # Export to file

The conversation ID can be a full ID or a unique prefix. Use 'list' command to find IDs."
    )]
    Show {
        /// Conversation ID or partial ID (use 'list' to find IDs)
        #[arg(help = "Conversation ID or unique prefix (e.g., 'abc123' or just 'abc')")]
        conversation_id: String,

        /// Output format: human (default), json, markdown, or text
        #[arg(long, value_enum, default_value = "human")]
        format: OutputFormat,

        /// Show only messages from this role (user, assistant, system, tool)
        #[arg(long, value_enum)]
        role: Option<MessageRole>,

        /// Export format (when saving to file)
        #[arg(long, value_enum)]
        export: Option<ConversationExportFormat>,

        /// Output file path (use with --export)
        #[arg(long, value_name = "FILE")]
        output: Option<String>,

        /// Include metadata in export
        #[arg(long)]
        include_metadata: bool,

        /// Include tool usage information
        #[arg(long)]
        include_tools: bool,

        /// Include timestamps
        #[arg(long)]
        include_timestamps: bool,
    },

    /// Search conversations
    #[command(
        alias = "find",
        long_about = "Search through conversation content with support for regular expressions and context.

EXAMPLES:
    claude-tools search \"rust code\"              # Simple text search
    claude-tools search --regex \"error.*handling\" # Regular expression search  
    claude-tools search --ignore-case \"ERROR\"     # Case-insensitive search
    claude-tools search \"function\" --context 2    # Show 2 lines of context
    claude-tools search \"async\" --regex -C 1      # Regex with context

TIP: Use the interactive mode (claude-tools interactive) for real-time search
     with visual highlighting and navigation between results."
    )]
    Search {
        /// Search query (supports text patterns or regex with --regex)
        #[arg(help = "Text to search for in conversation content")]
        query: String,

        /// Use regular expressions for pattern matching
        #[arg(short, long, help = "Enable regex pattern matching (e.g., 'error.*handling')")]
        regex: bool,

        /// Case insensitive search (ignore letter case)
        #[arg(short, long)]
        ignore_case: bool,

        /// Show context around matches (number of lines before/after)
        #[arg(short = 'C', long, default_value = "0")]
        context: usize,
    },

    /// Show conversation statistics
    #[command(
        alias = "info",
        long_about = "Display statistics about conversations, including message counts, models used, and activity patterns.

EXAMPLES:
    claude-tools stats                    # Quick overview of conversation history
    claude-tools stats --global          # Detailed global statistics
    claude-tools stats abc123            # Statistics for specific conversation

Statistics include message counts by role, model usage, conversation length
distribution, and temporal activity patterns."
    )]
    Stats {
        /// Show statistics for specific conversation (optional)
        #[arg(help = "Conversation ID to analyze (omit for global stats)")]
        conversation_id: Option<String>,

        /// Show comprehensive global statistics and analytics
        #[arg(long)]
        global: bool,

        /// Export analytics data to file (csv, json)
        #[arg(long, value_enum)]
        export: Option<ExportFormat>,

        /// Show detailed analytics dashboard
        #[arg(long)]
        detailed: bool,
    },

    /// Interactive browse mode
    #[command(
        alias = "browse",
        long_about = "Start the interactive terminal browser for exploring conversations.

This is the recommended way to use claude-tools. The interactive mode provides:
• Vim-style keyboard navigation (j/k, g/G, /, ?)
• Real-time search with visual highlighting
• Conversation browsing with markdown rendering
• Advanced search with regex support (use 'regex:' prefix)
• Search result navigation with 'n' and 'N' keys
• Activity timeline dashboard (press 't' key)

KEYBOARD SHORTCUTS:
    j/↓, k/↑     Navigate up/down
    g, G         Go to first/last
    Enter        Open conversation
    /            Start search
    n, N         Next/previous search result
    t            Timeline dashboard
    ?, h         Show help overlay
    q, Esc       Quit/back
    r            Refresh"
    )]
    Interactive,

    /// Activity timeline dashboard
    #[command(
        alias = "activity",
        long_about = "Display an activity timeline showing conversation patterns and project breakdowns.

The timeline dashboard provides:
• Project-level activity summaries with ranking indicators
• Time period filtering (24h, 48h, 1 week, 1 month)
• Message frequency analysis and visual activity bars  
• Tool usage tracking and top tools by project
• Topic extraction and topical summaries
• Intelligent caching for instant performance

EXAMPLES:
    claude-tools timeline                    # Show default timeline (48h)
    claude-tools timeline --period day      # Last 24 hours
    claude-tools timeline --period week     # Last 7 days
    claude-tools timeline --detailed        # Comprehensive view
    claude-tools timeline --export json     # Export timeline data
    claude-tools timeline --format markdown # Timeline in markdown

TIP: Use the interactive mode (claude-tools interactive) and press 't' 
     for a full terminal UI with navigation and real-time filtering."
    )]
    Timeline {
        /// Time period to analyze
        #[arg(long, value_enum, default_value = "two-day")]
        period: TimelinePeriod,

        /// Show detailed project breakdowns and statistics  
        #[arg(short, long)]
        detailed: bool,

        /// Output format for timeline display
        #[arg(long, value_enum, default_value = "human")]
        format: OutputFormat,

        /// Export timeline data to file
        #[arg(long, value_enum)]
        export: Option<ExportFormat>,

        /// Output file path (use with --export)
        #[arg(long, value_name = "FILE")]
        output: Option<String>,

        /// Maximum conversations per project to analyze
        #[arg(long, default_value = "20")]
        max_conversations: usize,

        /// Include projects with no activity in the period
        #[arg(long)]
        include_empty: bool,
    },

    /// Manage MCP servers
    #[command(
        alias = "server",
        long_about = "Discover, list, and manage MCP (Model Context Protocol) servers.

EXAMPLES:
    claude-tools mcp list                      # List all discovered MCP servers
    claude-tools mcp list --detailed          # Show detailed server information
    claude-tools mcp list --status running    # Filter servers by status
    claude-tools mcp discover                  # Force re-discovery of servers
    claude-tools mcp discover --health-check  # Discover with health checks

MCP servers provide tools, resources, and prompts that extend Claude's capabilities.
Use the interactive mode for a better server management experience."
    )]
    Mcp {
        #[command(subcommand)]
        action: McpAction,
    },
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum OutputFormat {
    /// Human-readable format with syntax highlighting and formatting (default)
    Human,
    /// Raw JSON output for scripting and data processing
    Json,
    /// Markdown format for documentation and export
    Markdown,
    /// Plain text without formatting for simple output
    Text,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum MessageRole {
    /// Messages sent by the user
    User,
    /// Messages from Claude (assistant responses)
    Assistant,
    /// System messages and prompts
    System,
    /// Tool execution results and calls
    Tool,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum ExportFormat {
    /// CSV format for spreadsheet analysis
    Csv,
    /// JSON format for programmatic analysis
    Json,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum TimelinePeriod {
    /// Last 24 hours of activity
    Day,
    /// Last 48 hours of activity (default)
    TwoDay,
    /// Last 7 days of activity
    Week,
    /// Last 30 days of activity
    Month,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum ConversationExportFormat {
    /// Markdown format for documentation and sharing
    Markdown,
    /// HTML format with styling for web viewing
    Html,
    /// PDF format for printing and archiving
    Pdf,
    /// JSON format for programmatic processing
    Json,
}

#[derive(Subcommand, Clone, Debug)]
pub enum McpAction {
    /// List discovered MCP servers
    #[command(
        alias = "ls",
        long_about = "List all discovered MCP servers with their status and capabilities.

EXAMPLES:
    claude-tools mcp list                   # List all servers (summary)
    claude-tools mcp list --detailed       # Show detailed server information
    claude-tools mcp list --status running # Show only running servers
    claude-tools mcp list --format json    # Output in JSON format"
    )]
    List {
        /// Show detailed server information
        #[arg(short, long)]
        detailed: bool,

        /// Filter servers by status
        #[arg(long, value_enum)]
        status: Option<ServerStatusFilter>,

        /// Output format
        #[arg(long, value_enum, default_value = "human")]
        format: OutputFormat,

        /// Sort servers by field
        #[arg(long, value_enum, default_value = "name")]
        sort: ServerSortField,
    },

    /// Discover MCP servers
    #[command(
        long_about = "Force re-discovery of MCP servers from all configured paths.

EXAMPLES:
    claude-tools mcp discover                # Basic discovery
    claude-tools mcp discover --health-check # Discovery with health checks
    claude-tools mcp discover --verbose      # Show discovery progress"
    )]
    Discover {
        /// Perform health checks during discovery
        #[arg(long)]
        health_check: bool,

        /// Show discovery progress and details
        #[arg(short, long)]
        verbose: bool,

        /// Custom discovery paths (overrides defaults)
        #[arg(long, value_name = "PATH")]
        paths: Vec<String>,
    },
    
    /// Add a new MCP server to Claude Code configuration
    #[command(
        long_about = "Add a new MCP server to ~/.claude.json configuration.

EXAMPLES:
    # Add a global MCP server
    claude-tools mcp add --global brave-search --command npx --args '@modelcontextprotocol/server-brave-search'
    
    # Add with environment variables
    claude-tools mcp add --global weather --command npx --args '@example/weather-server' --env API_KEY=secret
    
    # Add to current project
    claude-tools mcp add my-server --command ./server.js"
    )]
    Add {
        /// Server name
        name: String,
        
        /// Command to execute
        #[arg(long)]
        command: String,
        
        /// Command arguments
        #[arg(long, value_delimiter = ' ')]
        args: Vec<String>,
        
        /// Environment variables (KEY=VALUE format)
        #[arg(long, value_delimiter = ' ')]
        env: Vec<String>,
        
        /// Add as global server (available to all projects)
        #[arg(long)]
        global: bool,
        
        /// Project path (defaults to current directory)
        #[arg(long)]
        project: Option<String>,
    },
    
    /// Remove an MCP server from Claude Code configuration
    #[command(
        long_about = "Remove an MCP server from ~/.claude.json configuration.

EXAMPLES:
    claude-tools mcp remove brave-search --global  # Remove global server
    claude-tools mcp remove my-server               # Remove from current project"
    )]
    Remove {
        /// Server name
        name: String,
        
        /// Remove from global configuration
        #[arg(long)]
        global: bool,
        
        /// Project path (defaults to current directory)
        #[arg(long)]
        project: Option<String>,
    },
    
    /// Update MCP server configuration
    #[command(
        long_about = "Update an existing MCP server configuration.

EXAMPLES:
    claude-tools mcp update brave --env BRAVE_API_KEY=new-key
    claude-tools mcp update my-server --args '-v' '--debug'"
    )]
    Update {
        /// Server name
        name: String,
        
        /// Update command
        #[arg(long)]
        command: Option<String>,
        
        /// Update arguments
        #[arg(long, value_delimiter = ' ')]
        args: Option<Vec<String>>,
        
        /// Update environment variables (KEY=VALUE format)
        #[arg(long, value_delimiter = ' ')]
        env: Option<Vec<String>>,
        
        /// Update global server
        #[arg(long)]
        global: bool,
        
        /// Project path (defaults to current directory)
        #[arg(long)]
        project: Option<String>,
    },
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum ServerStatusFilter {
    /// Show only running servers
    Running,
    /// Show only stopped servers
    Stopped,
    /// Show only servers with errors
    Error,
    /// Show only servers with unknown status
    Unknown,
    /// Show servers in transitional states (starting/stopping)
    Transitional,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum ServerSortField {
    /// Sort by server name
    Name,
    /// Sort by server status
    Status,
    /// Sort by server version
    Version,
    /// Sort by last health check time
    LastCheck,
}
