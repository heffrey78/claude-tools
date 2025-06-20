use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "claude-tools",
    version,
    about = "A comprehensive suite of tools for working with Claude, Claude Code, and MCP servers",
    long_about = "Claude Tools provides utilities for browsing conversation history, managing MCP servers, and coordinating Claude Code workflows."
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
    #[command(alias = "ls")]
    List {
        /// Show only recent conversations (last N days)
        #[arg(long, value_name = "DAYS")]
        since: Option<u32>,

        /// Filter by project path
        #[arg(long, value_name = "PATH")]
        project: Option<String>,

        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },

    /// Show detailed conversation content
    #[command(alias = "cat")]
    Show {
        /// Conversation ID or partial ID
        conversation_id: String,

        /// Output format
        #[arg(long, value_enum, default_value = "human")]
        format: OutputFormat,

        /// Show only messages from this role
        #[arg(long, value_enum)]
        role: Option<MessageRole>,
    },

    /// Search conversations
    #[command(alias = "find")]
    Search {
        /// Search query
        query: String,

        /// Use regular expressions
        #[arg(short, long)]
        regex: bool,

        /// Case insensitive search
        #[arg(short, long)]
        ignore_case: bool,

        /// Show context around matches (lines)
        #[arg(short = 'C', long, default_value = "0")]
        context: usize,
    },

    /// Show conversation statistics
    #[command(alias = "info")]
    Stats {
        /// Show statistics for specific conversation
        conversation_id: Option<String>,

        /// Show global statistics
        #[arg(long)]
        global: bool,
    },

    /// Interactive browse mode
    #[command(alias = "browse")]
    Interactive,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum OutputFormat {
    /// Human-readable format with syntax highlighting
    Human,
    /// Raw JSON output
    Json,
    /// Markdown format
    Markdown,
    /// Plain text
    Text,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}
