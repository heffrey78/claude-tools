use crate::claude::conversation::MessageRole as ConvMessageRole;
use crate::claude::{ClaudeDirectory, ConversationParser, AnalyticsEngine};
use crate::cli::args::{Commands, MessageRole, OutputFormat, ExportFormat};
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
            export,
            detailed,
        } => execute_stats(claude_dir, conversation_id, global, export, detailed, verbose),
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
    global: bool,
    export: Option<ExportFormat>,
    detailed: bool,
    verbose: bool,
) -> Result<()> {
    if verbose {
        if let Some(id) = &conversation_id {
            eprintln!("Getting stats for conversation: {}", id);
        } else {
            eprintln!("Generating comprehensive analytics...");
        }
    }

    let parser = ConversationParser::new(claude_dir);

    if let Some(id) = conversation_id {
        // Stats for specific conversation (existing implementation)
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
        // Global analytics with new engine
        let conversations = parser.parse_all_conversations()?;
        let mut analytics_engine = AnalyticsEngine::new(conversations);
        let analytics = analytics_engine.generate_analytics()?;

        // Handle export first if requested
        if let Some(export_format) = export {
            return handle_export(analytics, export_format, verbose);
        }

        // Display analytics based on detail level
        if detailed || global {
            display_detailed_analytics(analytics);
        } else {
            display_basic_analytics(analytics);
        }
    }

    Ok(())
}

/// Display basic analytics summary
fn display_basic_analytics(analytics: &crate::claude::ConversationAnalytics) {
    let stats = &analytics.basic_stats;
    
    println!("📊 Conversation Analytics Summary");
    println!();
    println!("📈 Basic Statistics:");
    println!("   Total conversations: {}", stats.total_conversations);
    println!("   Total messages: {}", stats.total_messages);
    println!("   User messages: {}", stats.total_user_messages);
    println!("   Assistant messages: {}", stats.total_assistant_messages);
    println!("   System messages: {}", stats.total_system_messages);
    println!("   Tool uses: {}", stats.total_tool_uses);
    println!("   Avg. messages per conversation: {:.1}", stats.average_messages_per_conversation);
    
    if let Some(date_range) = &stats.date_range.earliest {
        println!("   First conversation: {}", date_range.format("%Y-%m-%d"));
    }
    if let Some(date_range) = &stats.date_range.latest {
        println!("   Latest conversation: {}", date_range.format("%Y-%m-%d"));
    }
    if let Some(span) = stats.date_range.span_days {
        println!("   Activity span: {} days", span);
    }

    println!();
    println!("🏆 Top Models:");
    for (i, model) in analytics.model_analytics.top_models.iter().take(5).enumerate() {
        println!("   {}. {} - {} uses ({:.1}%)", 
            i + 1, model.model_name, model.usage_count, model.percentage);
    }

    println!();
    println!("🛠️  Top Tools:");
    for (i, tool) in analytics.tool_analytics.top_tools.iter().take(5).enumerate() {
        println!("   {}. {} - {} uses ({:.1}%)", 
            i + 1, tool.tool_name, tool.usage_count, tool.percentage);
    }

    println!();
    println!("📁 Top Projects:");
    for (i, project) in analytics.project_analytics.top_projects.iter().take(5).enumerate() {
        println!("   {}. {} - {} conversations ({:.1}%)", 
            i + 1, project.project_name, project.conversation_count, project.percentage);
    }

    println!();
    println!("💡 Use --detailed for comprehensive analytics or --export json/csv for data export");
}

/// Display detailed analytics dashboard
fn display_detailed_analytics(analytics: &crate::claude::ConversationAnalytics) {
    display_basic_analytics(analytics);
    
    println!();
    println!("🕒 Temporal Analysis:");
    
    // Peak usage hours
    println!("   Peak usage hours:");
    for peak in &analytics.temporal_analysis.peak_usage_hours {
        println!("     {}:00 - {} conversations ({:.1}%)", 
            peak.hour, peak.count, peak.percentage);
    }
    
    // Weekday usage
    let weekdays = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
    println!("   Usage by day of week:");
    for (day_num, count) in &analytics.temporal_analysis.usage_by_weekday {
        if let Some(day_name) = weekdays.get(*day_num as usize) {
            println!("     {} - {} conversations", day_name, count);
        }
    }

    println!();
    println!("📊 Quality Metrics:");
    let quality = &analytics.quality_metrics;
    if let Some(avg_duration) = quality.average_conversation_duration {
        println!("   Average conversation duration: {:.1} minutes", avg_duration);
    }
    println!("   Average turns per conversation: {:.1}", quality.average_turns_per_conversation);
    println!("   Completion rate: {:.1}%", quality.completion_rate);
    
    let msg_dist = &quality.message_length_distribution;
    println!("   Message length stats:");
    println!("     Average: {:.0} characters", msg_dist.mean);
    println!("     Median: {} characters", msg_dist.median);
    println!("     95th percentile: {} characters", msg_dist.percentile_95);

    println!();
    println!("📈 Conversation Length Distribution:");
    let conv_dist = &analytics.basic_stats.conversation_length_distribution;
    println!("   Shortest: {} messages", conv_dist.min);
    println!("   Longest: {} messages", conv_dist.max);
    println!("   Average: {:.1} messages", conv_dist.mean);
    println!("   Median: {} messages", conv_dist.median);
    println!("   95th percentile: {} messages", conv_dist.percentile_95);
}

/// Handle analytics export
fn handle_export(
    analytics: &crate::claude::ConversationAnalytics, 
    format: ExportFormat, 
    verbose: bool
) -> Result<()> {
    let timestamp = analytics.generated_at.format("%Y%m%d_%H%M%S");
    
    match format {
        ExportFormat::Json => {
            let filename = format!("claude_analytics_{}.json", timestamp);
            let json_data = serde_json::to_string_pretty(analytics)?;
            let data_size = json_data.len();
            std::fs::write(&filename, json_data)?;
            println!("📄 Analytics exported to: {}", filename);
            if verbose {
                println!("   Format: JSON");
                println!("   Size: {} bytes", data_size);
            }
        },
        ExportFormat::Csv => {
            let filename = format!("claude_analytics_{}.csv", timestamp);
            let csv_data = generate_csv_export(analytics)?;
            let row_count = csv_data.lines().count();
            std::fs::write(&filename, csv_data)?;
            println!("📄 Analytics exported to: {}", filename);
            if verbose {
                println!("   Format: CSV");
                println!("   Rows: {}", row_count);
            }
        },
    }
    
    Ok(())
}

/// Generate CSV export of analytics data
fn generate_csv_export(analytics: &crate::claude::ConversationAnalytics) -> Result<String> {
    let mut csv_content = String::new();
    
    // Basic stats section
    csv_content.push_str("Section,Metric,Value\n");
    csv_content.push_str(&format!("Basic,Total Conversations,{}\n", analytics.basic_stats.total_conversations));
    csv_content.push_str(&format!("Basic,Total Messages,{}\n", analytics.basic_stats.total_messages));
    csv_content.push_str(&format!("Basic,User Messages,{}\n", analytics.basic_stats.total_user_messages));
    csv_content.push_str(&format!("Basic,Assistant Messages,{}\n", analytics.basic_stats.total_assistant_messages));
    csv_content.push_str(&format!("Basic,System Messages,{}\n", analytics.basic_stats.total_system_messages));
    csv_content.push_str(&format!("Basic,Tool Uses,{}\n", analytics.basic_stats.total_tool_uses));
    csv_content.push_str(&format!("Basic,Avg Messages per Conversation,{:.2}\n", analytics.basic_stats.average_messages_per_conversation));
    
    // Model usage
    csv_content.push_str("\nModel,Usage Count,Percentage\n");
    for model in &analytics.model_analytics.top_models {
        csv_content.push_str(&format!("{},{},{:.2}\n", model.model_name, model.usage_count, model.percentage));
    }
    
    // Tool usage
    csv_content.push_str("\nTool,Usage Count,Percentage\n");
    for tool in &analytics.tool_analytics.top_tools {
        csv_content.push_str(&format!("{},{},{:.2}\n", tool.tool_name, tool.usage_count, tool.percentage));
    }
    
    // Project usage
    csv_content.push_str("\nProject,Conversations,Messages,Percentage\n");
    for project in &analytics.project_analytics.top_projects {
        csv_content.push_str(&format!("{},{},{},{:.2}\n", 
            project.project_name, project.conversation_count, project.message_count, project.percentage));
    }
    
    Ok(csv_content)
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
