use crate::claude::conversation::MessageRole as ConvMessageRole;
use crate::claude::{ClaudeDirectory, ConversationParser};
use crate::cli::args::{Commands, MessageRole, OutputFormat};
use crate::errors::Result;
use crate::ui::{App, Event, EventHandler};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};

pub fn execute_command(
    claude_dir: ClaudeDirectory,
    command: Commands,
    verbose: bool,
) -> Result<()> {
    match command {
        Commands::List {
            since,
            project,
            detailed,
        } => execute_list(claude_dir, since, project, detailed, verbose),
        Commands::Show {
            conversation_id,
            format,
            role,
        } => execute_show(claude_dir, conversation_id, format, role, verbose),
        Commands::Search {
            query,
            regex,
            ignore_case,
            context,
        } => execute_search(claude_dir, query, regex, ignore_case, context, verbose),
        Commands::Stats {
            conversation_id,
            global,
        } => execute_stats(claude_dir, conversation_id, global, verbose),
        Commands::Interactive => execute_interactive(claude_dir, verbose),
    }
}

fn execute_list(
    claude_dir: ClaudeDirectory,
    _since: Option<u32>,
    project: Option<String>,
    detailed: bool,
    verbose: bool,
) -> Result<()> {
    if verbose {
        eprintln!("Listing conversations in: {}", claude_dir.path.display());
    }

    let parser = ConversationParser::new(claude_dir.clone());

    let conversations = if let Some(project_path) = project {
        parser.get_project_conversations(&project_path)?
    } else {
        parser.parse_all_conversations()?
    };

    if conversations.is_empty() {
        println!("No conversations found");
        return Ok(());
    }

    println!("📁 Found {} conversation(s):", conversations.len());
    println!();

    for conv in conversations {
        if detailed {
            println!("📄 Session: {}", conv.session_id);
            println!("   Project: {}", conv.project_path);
            if let Some(summary) = &conv.summary {
                println!("   Summary: {}", summary);
            }
            println!(
                "   Messages: {} (User: {}, Assistant: {})",
                conv.messages.len(),
                conv.user_message_count(),
                conv.assistant_message_count()
            );
            if let Some(started) = conv.started_at {
                println!("   Started: {}", started.format("%Y-%m-%d %H:%M:%S"));
            }
            if let Some(duration) = conv.duration() {
                let minutes = duration.num_minutes();
                let seconds = duration.num_seconds() % 60;
                println!("   Duration: {}m {}s", minutes, seconds);
            }
            println!();
        } else {
            let summary = conv.summary.as_deref().unwrap_or("No summary");
            println!("📄 {} - {}", conv.session_id, summary);
        }
    }

    Ok(())
}

fn execute_show(
    claude_dir: ClaudeDirectory,
    conversation_id: String,
    format: OutputFormat,
    role: Option<MessageRole>,
    verbose: bool,
) -> Result<()> {
    if verbose {
        eprintln!("Showing conversation: {}", conversation_id);
    }

    let parser = ConversationParser::new(claude_dir);

    match parser.get_conversation(&conversation_id)? {
        Some(conversation) => {
            match format {
                OutputFormat::Human => {
                    println!("📄 Conversation: {}", conversation.session_id);
                    println!("📁 Project: {}", conversation.project_path);
                    if let Some(summary) = &conversation.summary {
                        println!("📝 Summary: {}", summary);
                    }
                    println!();

                    for msg in &conversation.messages {
                        let should_show = match &role {
                            Some(MessageRole::User) => msg.role == ConvMessageRole::User,
                            Some(MessageRole::Assistant) => msg.role == ConvMessageRole::Assistant,
                            Some(MessageRole::System) => msg.role == ConvMessageRole::System,
                            Some(MessageRole::Tool) => false, // Tool messages are shown as part of assistant messages
                            None => true,
                        };

                        if should_show {
                            let role_str = match msg.role {
                                ConvMessageRole::User => "👤 User",
                                ConvMessageRole::Assistant => "🤖 Assistant",
                                ConvMessageRole::System => "⚙️ System",
                            };

                            println!(
                                "{} [{}]",
                                role_str,
                                msg.timestamp.format("%Y-%m-%d %H:%M:%S")
                            );
                            if let Some(model) = &msg.model {
                                println!("   Model: {}", model);
                            }
                            println!("{}", msg.content);

                            if !msg.tool_uses.is_empty() {
                                println!("   🛠️ Tool uses:");
                                for tool in &msg.tool_uses {
                                    println!("      - {}: {}", tool.name, tool.id);
                                }
                            }
                            println!();
                        }
                    }
                }
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&conversation)?);
                }
                OutputFormat::Markdown | OutputFormat::Text => {
                    // For now, use human format for markdown and text
                    println!("📄 Conversation: {}", conversation.session_id);
                    println!("Project: {}", conversation.project_path);
                    if let Some(summary) = &conversation.summary {
                        println!("Summary: {}", summary);
                    }
                    for msg in &conversation.messages {
                        let role_str = match msg.role {
                            ConvMessageRole::User => "User",
                            ConvMessageRole::Assistant => "Assistant",
                            ConvMessageRole::System => "System",
                        };
                        println!(
                            "\n{} [{}]:\n{}",
                            role_str,
                            msg.timestamp.format("%Y-%m-%d %H:%M:%S"),
                            msg.content
                        );
                    }
                }
            }
        }
        None => {
            println!("❌ Conversation not found: {}", conversation_id);
        }
    }

    Ok(())
}

fn execute_search(
    claude_dir: ClaudeDirectory,
    query: String,
    _regex: bool,
    _ignore_case: bool,
    _context: usize,
    verbose: bool,
) -> Result<()> {
    if verbose {
        eprintln!("Searching for: {}", query);
    }

    let parser = ConversationParser::new(claude_dir);
    let results = parser.search_conversations(&query)?;

    if results.is_empty() {
        println!("No conversations found matching: {}", query);
        return Ok(());
    }

    println!(
        "🔍 Found {} conversation(s) matching '{}'",
        results.len(),
        query
    );
    println!();

    for conv in results {
        println!("📄 Session: {}", conv.session_id);
        println!("   Project: {}", conv.project_path);
        if let Some(summary) = &conv.summary {
            println!("   Summary: {}", summary);
        }

        // Show matching messages
        let query_lower = query.to_lowercase();
        for msg in &conv.messages {
            if msg.content.to_lowercase().contains(&query_lower) {
                let role_str = match msg.role {
                    ConvMessageRole::User => "User",
                    ConvMessageRole::Assistant => "Assistant",
                    ConvMessageRole::System => "System",
                };

                println!("   Match in {} message:", role_str);
                // Show a snippet of the matching content
                let snippet = if msg.content.len() > 200 {
                    format!("{}...", &msg.content[..200])
                } else {
                    msg.content.clone()
                };
                println!("   {}", snippet.replace('\n', " "));
            }
        }
        println!();
    }

    Ok(())
}

fn execute_stats(
    claude_dir: ClaudeDirectory,
    conversation_id: Option<String>,
    _global: bool,
    verbose: bool,
) -> Result<()> {
    if verbose {
        if let Some(id) = &conversation_id {
            eprintln!("Getting stats for conversation: {}", id);
        } else {
            eprintln!("Getting global statistics");
        }
    }

    let parser = ConversationParser::new(claude_dir);

    if let Some(id) = conversation_id {
        // Stats for specific conversation
        match parser.get_conversation(&id)? {
            Some(conv) => {
                println!("📊 Conversation Statistics");
                println!("   Session ID: {}", conv.session_id);
                println!("   Project: {}", conv.project_path);
                println!("   Total messages: {}", conv.messages.len());
                println!("   User messages: {}", conv.user_message_count());
                println!("   Assistant messages: {}", conv.assistant_message_count());

                if let Some(duration) = conv.duration() {
                    let minutes = duration.num_minutes();
                    println!("   Duration: {} minutes", minutes);
                }

                // Count tool uses
                let tool_uses: usize = conv.messages.iter().map(|m| m.tool_uses.len()).sum();
                if tool_uses > 0 {
                    println!("   Tool uses: {}", tool_uses);
                }
            }
            None => {
                println!("❌ Conversation not found: {}", id);
            }
        }
    } else {
        // Global stats
        let stats = parser.get_stats()?;

        println!("📊 Global Statistics");
        println!("   Total conversations: {}", stats.total_conversations);
        println!("   Total messages: {}", stats.total_messages);
        println!("   User messages: {}", stats.total_user_messages);
        println!("   Assistant messages: {}", stats.total_assistant_messages);
        println!();
        println!("📁 Projects:");

        let mut projects: Vec<_> = stats.projects.iter().collect();
        projects.sort_by(|a, b| b.1.cmp(a.1)); // Sort by count descending

        for (project, count) in projects {
            println!("   {} - {} conversation(s)", project, count);
        }
    }

    Ok(())
}

fn execute_interactive(claude_dir: ClaudeDirectory, verbose: bool) -> Result<()> {
    if verbose {
        eprintln!("Starting interactive mode");
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and event handler
    let mut app = App::new(claude_dir)?;
    let events = EventHandler::new(Duration::from_millis(250));

    // Main event loop
    let result = run_app(&mut terminal, &mut app, &events);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("Terminal UI error: {}", err);
    }

    Ok(())
}

/// Run the terminal application
fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    events: &EventHandler,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| app.render(f))?;

        if app.should_quit {
            break;
        }

        match events.next() {
            Ok(Event::Key(key)) => {
                app.handle_key_event(key);
            }
            Ok(Event::Resize(_, _)) => {
                // Terminal resized, will be handled on next draw
            }
            Ok(Event::Tick) => {
                // Periodic tick for animations or updates
            }
            Ok(Event::Mouse(_)) => {
                // Mouse events (not used for now)
            }
            Err(_) => {
                break;
            }
        }
    }
    Ok(())
}
