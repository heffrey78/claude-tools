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

KEYBOARD SHORTCUTS:
    j/↓, k/↑     Navigate up/down
    g, G         Go to first/last
    Enter        Open conversation
    /            Start search
    n, N         Next/previous search result
    ?, h         Show help overlay
    q, Esc       Quit/back
    r            Refresh"
    )]
    Interactive,
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
