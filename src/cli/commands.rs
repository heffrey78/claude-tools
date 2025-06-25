use crate::claude::conversation::MessageRole as ConvMessageRole;
use crate::claude::{
    ActivityTimeline, AnalyticsEngine, ClaudeDirectory, ConversationExporter, ConversationParser,
    ExportConfig, SummaryDepth, TimePeriod, TimelineConfig,
};
use crate::cli::args::{
    Commands, ConversationExportFormat, ExportFormat, McpAction, MessageRole, OutputFormat,
    ServerSortField, ServerStatusFilter, TimelinePeriod,
};
use crate::config::AppConfig;
use crate::errors::Result;
use crate::mcp::{McpServer, ServerDiscovery, ServerStatus};
use crate::ui::{App, Event, EventHandler};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, path::PathBuf, time::Duration};

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
            export,
            output,
            include_metadata,
            include_tools,
            include_timestamps,
        } => execute_show(
            claude_dir,
            conversation_id,
            format,
            role,
            export,
            output,
            include_metadata,
            include_tools,
            include_timestamps,
            verbose,
        ),
        Commands::Search {
            query,
            regex,
            ignore_case,
            context,
            model,
            tool,
            role,
            after,
            before,
            min_messages,
            max_messages,
            min_duration,
            max_duration,
            limit,
        } => execute_search(
            claude_dir,
            query,
            regex,
            ignore_case,
            context,
            model,
            tool,
            role,
            after,
            before,
            min_messages,
            max_messages,
            min_duration,
            max_duration,
            limit,
            verbose,
        ),
        Commands::Stats {
            conversation_id,
            global,
            export,
            detailed,
        } => execute_stats(
            claude_dir,
            conversation_id,
            global,
            export,
            detailed,
            verbose,
        ),
        Commands::Timeline {
            period,
            detailed,
            format,
            export,
            output,
            max_conversations,
            include_empty,
        } => execute_timeline(
            claude_dir,
            period,
            detailed,
            format,
            export,
            output,
            max_conversations,
            include_empty,
            verbose,
        ),
        Commands::Interactive => execute_interactive(claude_dir, verbose),
        Commands::Mcp { action } => execute_mcp(action, verbose),
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
    export: Option<ConversationExportFormat>,
    output: Option<String>,
    include_metadata: bool,
    include_tools: bool,
    include_timestamps: bool,
    verbose: bool,
) -> Result<()> {
    if verbose {
        eprintln!("Showing conversation: {}", conversation_id);
    }

    let parser = ConversationParser::new(claude_dir);

    match parser.get_conversation(&conversation_id)? {
        Some(conversation) => {
            // Handle export functionality first
            if let Some(export_format) = export {
                return handle_conversation_export(
                    &conversation,
                    export_format,
                    output,
                    include_metadata,
                    include_tools,
                    include_timestamps,
                    verbose,
                );
            }

            // Handle regular display formats
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
    regex: bool,
    ignore_case: bool,
    _context: usize,
    model: Option<String>,
    tool: Option<String>,
    role: Option<MessageRole>,
    after: Option<String>,
    before: Option<String>,
    min_messages: Option<usize>,
    max_messages: Option<usize>,
    min_duration: Option<u32>,
    max_duration: Option<u32>,
    limit: usize,
    verbose: bool,
) -> Result<()> {
    use crate::claude::search::{
        BooleanQueryParser, DateRange, MessageRole as SearchRole, SearchEngine, SearchMode,
        SearchQuery,
    };
    use chrono::{DateTime, Utc};

    if verbose {
        eprintln!("🔍 Searching for: {}", query);
        if model.is_some() || tool.is_some() || after.is_some() || before.is_some() {
            eprintln!(
                "📊 Filters applied: model={:?}, tool={:?}, date_range={:?}-{:?}",
                model, tool, after, before
            );
        }
    }

    // Parse conversations and build search index
    let parser = ConversationParser::new(claude_dir);
    let conversations = parser.parse_all_conversations()?;

    let mut search_engine = SearchEngine::new();
    search_engine.build_index(conversations)?;

    // Build search query
    let mut search_query = SearchQuery::default();

    // Determine search mode and set query
    if regex {
        search_query.regex_pattern = Some(query.clone());
        search_query.search_mode = SearchMode::Regex;
    } else if query.contains("AND")
        || query.contains("OR")
        || query.contains("NOT")
        || query.contains('(')
    {
        // Try boolean search if it looks like boolean syntax
        match BooleanQueryParser::parse(&query) {
            Ok(boolean_query) => {
                search_query.boolean_query = Some(boolean_query);
                search_query.search_mode = SearchMode::Advanced;
            }
            Err(_) => {
                // Fall back to text search if boolean parsing fails
                search_query.text = Some(query.clone());
                search_query.search_mode = if ignore_case {
                    SearchMode::Text
                } else {
                    SearchMode::Text
                };
            }
        }
    } else {
        search_query.text = Some(query.clone());
        search_query.search_mode = SearchMode::Text;
    }

    // Apply filters
    if let Some(model_filter) = model {
        search_query.model_filter = Some(model_filter);
    }

    if let Some(tool_filter) = tool {
        search_query.tool_filter = Some(tool_filter);
    }

    if let Some(role_filter) = role {
        let search_role = match role_filter {
            MessageRole::User => SearchRole::User,
            MessageRole::Assistant => SearchRole::Assistant,
            MessageRole::System => SearchRole::System,
            MessageRole::Tool => SearchRole::Tool,
        };
        search_query.message_role_filter = Some(search_role);
    }

    // Parse date filters
    let mut date_range = DateRange {
        start: None,
        end: None,
    };
    if let Some(after_str) = after {
        if let Ok(date) = parse_date_string(&after_str) {
            date_range.start = Some(date);
        } else {
            eprintln!("⚠️  Warning: Could not parse 'after' date: {}", after_str);
        }
    }
    if let Some(before_str) = before {
        if let Ok(date) = parse_date_string(&before_str) {
            date_range.end = Some(date);
        } else {
            eprintln!("⚠️  Warning: Could not parse 'before' date: {}", before_str);
        }
    }
    if date_range.start.is_some() || date_range.end.is_some() {
        search_query.date_range = Some(date_range);
    }

    // Apply other filters
    search_query.min_messages = min_messages;
    search_query.max_messages = max_messages;
    search_query.min_duration_minutes = min_duration;
    search_query.max_duration_minutes = max_duration;
    search_query.max_results = Some(limit);

    // Execute search
    let results = search_engine.search(&search_query)?;

    if results.is_empty() {
        println!("❌ No conversations found matching the search criteria");
        return Ok(());
    }

    println!(
        "🔍 Found {} conversation(s) matching '{}'",
        results.len(),
        query
    );
    println!();

    for result in results {
        let conv = &result.conversation;
        println!(
            "📄 Session: {} (Score: {:.2})",
            conv.session_id, result.relevance_score
        );
        println!("   Project: {}", conv.project_path);
        if let Some(summary) = &conv.summary {
            println!("   Summary: {}", summary);
        }
        println!(
            "   Messages: {} | Matches: {}",
            conv.messages.len(),
            result.match_count
        );

        // Show highlighted matches
        if !result.match_highlights.is_empty() {
            println!("   🎯 Highlights:");
            for highlight in result.match_highlights.iter().take(3) {
                // Show up to 3 highlights
                if let Some(message) = conv.messages.get(highlight.message_index) {
                    let role_str = match message.role {
                        ConvMessageRole::User => "User",
                        ConvMessageRole::Assistant => "Assistant",
                        ConvMessageRole::System => "System",
                    };

                    let start = highlight.start.saturating_sub(30);
                    let end = (highlight.end + 30).min(message.content.len());
                    let snippet = &message.content[start..end];

                    println!(
                        "      {} {}...{}",
                        role_str,
                        if start > 0 { "..." } else { "" },
                        snippet.replace('\n', " ")
                    );
                }
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
    println!(
        "   Avg. messages per conversation: {:.1}",
        stats.average_messages_per_conversation
    );

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
    for (i, model) in analytics
        .model_analytics
        .top_models
        .iter()
        .take(5)
        .enumerate()
    {
        println!(
            "   {}. {} - {} uses ({:.1}%)",
            i + 1,
            model.model_name,
            model.usage_count,
            model.percentage
        );
    }

    println!();
    println!("🛠️  Top Tools:");
    for (i, tool) in analytics
        .tool_analytics
        .top_tools
        .iter()
        .take(5)
        .enumerate()
    {
        println!(
            "   {}. {} - {} uses ({:.1}%)",
            i + 1,
            tool.tool_name,
            tool.usage_count,
            tool.percentage
        );
    }

    println!();
    println!("📁 Top Projects:");
    for (i, project) in analytics
        .project_analytics
        .top_projects
        .iter()
        .take(5)
        .enumerate()
    {
        println!(
            "   {}. {} - {} conversations ({:.1}%)",
            i + 1,
            project.project_name,
            project.conversation_count,
            project.percentage
        );
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
        println!(
            "     {}:00 - {} conversations ({:.1}%)",
            peak.hour, peak.count, peak.percentage
        );
    }

    // Weekday usage
    let weekdays = [
        "Sunday",
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
    ];
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
        println!(
            "   Average conversation duration: {:.1} minutes",
            avg_duration
        );
    }
    println!(
        "   Average turns per conversation: {:.1}",
        quality.average_turns_per_conversation
    );
    println!("   Completion rate: {:.1}%", quality.completion_rate);

    let msg_dist = &quality.message_length_distribution;
    println!("   Message length stats:");
    println!("     Average: {:.0} characters", msg_dist.mean);
    println!("     Median: {} characters", msg_dist.median);
    println!(
        "     95th percentile: {} characters",
        msg_dist.percentile_95
    );

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
    verbose: bool,
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
        }
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
        }
    }

    Ok(())
}

/// Generate CSV export of analytics data
fn generate_csv_export(analytics: &crate::claude::ConversationAnalytics) -> Result<String> {
    let mut csv_content = String::new();

    // Basic stats section
    csv_content.push_str("Section,Metric,Value\n");
    csv_content.push_str(&format!(
        "Basic,Total Conversations,{}\n",
        analytics.basic_stats.total_conversations
    ));
    csv_content.push_str(&format!(
        "Basic,Total Messages,{}\n",
        analytics.basic_stats.total_messages
    ));
    csv_content.push_str(&format!(
        "Basic,User Messages,{}\n",
        analytics.basic_stats.total_user_messages
    ));
    csv_content.push_str(&format!(
        "Basic,Assistant Messages,{}\n",
        analytics.basic_stats.total_assistant_messages
    ));
    csv_content.push_str(&format!(
        "Basic,System Messages,{}\n",
        analytics.basic_stats.total_system_messages
    ));
    csv_content.push_str(&format!(
        "Basic,Tool Uses,{}\n",
        analytics.basic_stats.total_tool_uses
    ));
    csv_content.push_str(&format!(
        "Basic,Avg Messages per Conversation,{:.2}\n",
        analytics.basic_stats.average_messages_per_conversation
    ));

    // Model usage
    csv_content.push_str("\nModel,Usage Count,Percentage\n");
    for model in &analytics.model_analytics.top_models {
        csv_content.push_str(&format!(
            "{},{},{:.2}\n",
            model.model_name, model.usage_count, model.percentage
        ));
    }

    // Tool usage
    csv_content.push_str("\nTool,Usage Count,Percentage\n");
    for tool in &analytics.tool_analytics.top_tools {
        csv_content.push_str(&format!(
            "{},{},{:.2}\n",
            tool.tool_name, tool.usage_count, tool.percentage
        ));
    }

    // Project usage
    csv_content.push_str("\nProject,Conversations,Messages,Percentage\n");
    for project in &analytics.project_analytics.top_projects {
        csv_content.push_str(&format!(
            "{},{},{},{:.2}\n",
            project.project_name,
            project.conversation_count,
            project.message_count,
            project.percentage
        ));
    }

    Ok(csv_content)
}

fn execute_timeline(
    claude_dir: ClaudeDirectory,
    period: TimelinePeriod,
    detailed: bool,
    format: OutputFormat,
    export: Option<ExportFormat>,
    output: Option<String>,
    max_conversations: usize,
    include_empty: bool,
    verbose: bool,
) -> Result<()> {
    if verbose {
        eprintln!("Generating activity timeline for {:?} period...", period);
    }

    // Convert CLI period to internal period
    let timeline_period = match period {
        TimelinePeriod::Day => TimePeriod::LastDay,
        TimelinePeriod::TwoDay => TimePeriod::LastTwoDay,
        TimelinePeriod::Week => TimePeriod::LastWeek,
        TimelinePeriod::Month => TimePeriod::LastMonth,
    };

    // Create timeline configuration
    let config = TimelineConfig {
        period: timeline_period,
        summary_depth: if detailed {
            SummaryDepth::Detailed
        } else {
            SummaryDepth::Brief
        },
        max_conversations_per_project: Some(max_conversations),
        include_empty_projects: include_empty,
    };

    // Parse conversations and generate timeline
    let parser = ConversationParser::new(claude_dir);
    let conversations = parser.parse_all_conversations()?;

    if verbose {
        eprintln!(
            "Found {} conversations, generating timeline...",
            conversations.len()
        );
    }

    let timeline = ActivityTimeline::create_filtered_timeline(conversations, config);

    // Handle export first if requested
    if let Some(export_format) = export {
        return handle_timeline_export(&timeline, export_format, output, verbose);
    }

    // Display timeline based on format
    match format {
        OutputFormat::Human => display_timeline_human(&timeline, detailed),
        OutputFormat::Json => display_timeline_json(&timeline)?,
        OutputFormat::Markdown => display_timeline_markdown(&timeline, detailed),
        OutputFormat::Text => display_timeline_text(&timeline, detailed),
    }

    Ok(())
}

/// Display timeline in human-readable format
fn display_timeline_human(timeline: &ActivityTimeline, detailed: bool) {
    println!("📊 Activity Timeline - {}", timeline.config.period.label());
    println!(
        "   Generated: {}",
        timeline.generated_at.format("%Y-%m-%d %H:%M:%S")
    );
    println!();

    // Overall statistics
    let stats = &timeline.total_stats;
    println!("📈 Overview:");
    println!("   Active projects: {}", stats.active_projects);
    println!("   Total conversations: {}", stats.total_conversations);
    println!("   Total messages: {}", stats.total_messages);
    println!("   Messages per day: {:.1}", stats.messages_per_day);

    if let Some(most_active) = &stats.most_active_project {
        println!("   Most active project: {}", most_active);
    }
    println!();

    // Project breakdown
    let projects = timeline.projects_by_activity();
    if projects.is_empty() {
        println!("💤 No activity found in the selected time period");
        return;
    }

    println!("📁 Project Activity ({} projects):", projects.len());
    println!();

    for (i, project) in projects.iter().enumerate() {
        let rank = i + 1;
        println!("{}. 📁 {}", rank, project.project_path);
        println!(
            "   📊 {} conversations, {} messages",
            project.stats.conversation_count, project.stats.total_messages
        );

        if detailed {
            println!(
                "   💬 Avg messages/conversation: {:.1}",
                project.stats.avg_conversation_length
            );
            println!(
                "   📅 Conversations/day: {:.1}",
                project.stats.conversation_frequency
            );
            println!("   📈 Messages/day: {:.1}", project.stats.message_frequency);

            if let Some(peak_hour) = project.stats.peak_hour {
                println!("   🕐 Peak hour: {}:00", peak_hour);
            }

            // Show activity indicators using progress bar
            let progress_percentage = (project.indicators.progress_bar * 100.0) as u32;
            let bar_length = (project.indicators.progress_bar * 10.0) as usize;
            println!(
                "   📊 Activity: {} {}% ({} msgs)",
                "█".repeat(bar_length),
                progress_percentage,
                project.stats.total_messages
            );

            // Show top tools if any
            if !project.stats.top_tools.is_empty() {
                println!(
                    "   🛠️  Top tools: {}",
                    project
                        .stats
                        .top_tools
                        .iter()
                        .take(3)
                        .map(|(name, count)| format!("{} ({})", name, count))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }

            // Show topical summary
            if !project.topical_summary.summary_text.is_empty() {
                println!("   📝 Summary: {}", project.topical_summary.summary_text);
            }
        }

        println!();
    }

    // Show global tools summary
    if !stats.global_top_tools.is_empty() {
        println!("🛠️  Most Used Tools:");
        for (i, (tool, count)) in stats.global_top_tools.iter().take(5).enumerate() {
            println!("   {}. {} - {} uses", i + 1, tool, count);
        }
        println!();
    }
}

/// Display timeline in JSON format
fn display_timeline_json(timeline: &ActivityTimeline) -> Result<()> {
    let json = serde_json::to_string_pretty(timeline)?;
    println!("{}", json);
    Ok(())
}

/// Display timeline in markdown format
fn display_timeline_markdown(timeline: &ActivityTimeline, detailed: bool) {
    println!("# Activity Timeline - {}", timeline.config.period.label());
    println!();
    println!(
        "**Generated:** {}",
        timeline.generated_at.format("%Y-%m-%d %H:%M:%S")
    );
    println!();

    let stats = &timeline.total_stats;
    println!("## Overview");
    println!();
    println!("- **Active projects:** {}", stats.active_projects);
    println!("- **Total conversations:** {}", stats.total_conversations);
    println!("- **Total messages:** {}", stats.total_messages);
    println!("- **Messages per day:** {:.1}", stats.messages_per_day);

    if let Some(most_active) = &stats.most_active_project {
        println!("- **Most active project:** {}", most_active);
    }
    println!();

    let projects = timeline.projects_by_activity();
    if projects.is_empty() {
        println!("## No Activity");
        println!();
        println!("No activity found in the selected time period.");
        return;
    }

    println!("## Project Activity");
    println!();

    for (i, project) in projects.iter().enumerate() {
        let rank = i + 1;
        println!("### {}. {}", rank, project.project_path);
        println!();
        println!("- **Conversations:** {}", project.stats.conversation_count);
        println!("- **Messages:** {}", project.stats.total_messages);

        if detailed {
            println!(
                "- **Avg messages/conversation:** {:.1}",
                project.stats.avg_conversation_length
            );
            println!(
                "- **Activity frequency:** {:.1} conversations/day, {:.1} messages/day",
                project.stats.conversation_frequency, project.stats.message_frequency
            );

            if let Some(peak_hour) = project.stats.peak_hour {
                println!("- **Peak hour:** {}:00", peak_hour);
            }

            if !project.stats.top_tools.is_empty() {
                println!(
                    "- **Top tools:** {}",
                    project
                        .stats
                        .top_tools
                        .iter()
                        .take(3)
                        .map(|(name, count)| format!("{} ({})", name, count))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }

            if !project.topical_summary.summary_text.is_empty() {
                println!("- **Summary:** {}", project.topical_summary.summary_text);
            }
        }

        println!();
    }
}

/// Display timeline in plain text format
fn display_timeline_text(timeline: &ActivityTimeline, detailed: bool) {
    println!("Activity Timeline - {}", timeline.config.period.label());
    println!(
        "Generated: {}",
        timeline.generated_at.format("%Y-%m-%d %H:%M:%S")
    );
    println!();

    let stats = &timeline.total_stats;
    println!("OVERVIEW");
    println!("Active projects: {}", stats.active_projects);
    println!("Total conversations: {}", stats.total_conversations);
    println!("Total messages: {}", stats.total_messages);
    println!("Messages per day: {:.1}", stats.messages_per_day);

    if let Some(most_active) = &stats.most_active_project {
        println!("Most active project: {}", most_active);
    }
    println!();

    let projects = timeline.projects_by_activity();
    if projects.is_empty() {
        println!("NO ACTIVITY");
        println!("No activity found in the selected time period.");
        return;
    }

    println!("PROJECT ACTIVITY");
    println!();

    for (i, project) in projects.iter().enumerate() {
        let rank = i + 1;
        println!("{}. {}", rank, project.project_path);
        println!(
            "   {} conversations, {} messages",
            project.stats.conversation_count, project.stats.total_messages
        );

        if detailed {
            println!(
                "   Avg messages/conversation: {:.1}",
                project.stats.avg_conversation_length
            );
            println!(
                "   Activity: {:.1} conversations/day, {:.1} messages/day",
                project.stats.conversation_frequency, project.stats.message_frequency
            );

            if let Some(peak_hour) = project.stats.peak_hour {
                println!("   Peak hour: {}:00", peak_hour);
            }

            if !project.stats.top_tools.is_empty() {
                println!(
                    "   Top tools: {}",
                    project
                        .stats
                        .top_tools
                        .iter()
                        .take(3)
                        .map(|(name, count)| format!("{} ({})", name, count))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }

            if !project.topical_summary.summary_text.is_empty() {
                println!("   Summary: {}", project.topical_summary.summary_text);
            }
        }

        println!();
    }
}

/// Handle timeline export functionality
fn handle_timeline_export(
    timeline: &ActivityTimeline,
    export_format: ExportFormat,
    output: Option<String>,
    verbose: bool,
) -> Result<()> {
    let timestamp = timeline.generated_at.format("%Y%m%d_%H%M%S");

    let (filename, data) = match export_format {
        ExportFormat::Json => {
            let filename = output.unwrap_or_else(|| format!("timeline_{}.json", timestamp));
            let data = serde_json::to_string_pretty(timeline)?;
            (filename, data)
        }
        ExportFormat::Csv => {
            let filename = output.unwrap_or_else(|| format!("timeline_{}.csv", timestamp));
            let data = generate_timeline_csv_export(timeline)?;
            (filename, data)
        }
    };

    std::fs::write(&filename, &data)?;

    println!("📄 Timeline exported to: {}", filename);
    if verbose {
        println!("   Format: {:?}", export_format);
        println!("   Size: {} bytes", data.len());
        println!("   Projects: {}", timeline.projects.len());
    }

    Ok(())
}

/// Generate CSV export of timeline data
fn generate_timeline_csv_export(timeline: &ActivityTimeline) -> Result<String> {
    let mut csv_content = String::new();

    // Header
    csv_content.push_str("Project,Conversations,Messages,Avg_Messages_Per_Conv,Conv_Per_Day,Msg_Per_Day,Peak_Hour,Top_Tools,Summary\n");

    // Data rows
    for project in timeline.projects_by_activity() {
        let top_tools = project
            .stats
            .top_tools
            .iter()
            .take(3)
            .map(|(name, count)| format!("{}({})", name, count))
            .collect::<Vec<_>>()
            .join(";");

        let summary = project
            .topical_summary
            .summary_text
            .replace(',', ";")
            .replace('\n', " ");
        let peak_hour = project
            .stats
            .peak_hour
            .map(|h| h.to_string())
            .unwrap_or_else(|| "".to_string());

        csv_content.push_str(&format!(
            "{},{},{},{:.1},{:.1},{:.1},{},{},{}\n",
            project.project_path,
            project.stats.conversation_count,
            project.stats.total_messages,
            project.stats.avg_conversation_length,
            project.stats.conversation_frequency,
            project.stats.message_frequency,
            peak_hour,
            top_tools,
            summary
        ));
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
            Ok(Event::FileChanged(path)) => {
                // Handle file system change events
                app.handle_file_change(path);
            }
            Ok(Event::ConfigChanged) => {
                // Configuration change events - reload config
                match AppConfig::load_from_file(AppConfig::default_config_path().unwrap_or_default()) {
                    Ok(config) => {
                        if let Err(e) = app.update_config(config) {
                            eprintln!("Failed to update config: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to reload config: {}", e);
                    }
                }
            }
            Err(_) => {
                break;
            }
        }
    }
    Ok(())
}

/// Handle conversation export functionality
fn handle_conversation_export(
    conversation: &crate::claude::Conversation,
    export_format: ConversationExportFormat,
    output: Option<String>,
    include_metadata: bool,
    include_tools: bool,
    include_timestamps: bool,
    verbose: bool,
) -> Result<()> {
    // Convert CLI export format to internal format
    let internal_format = match export_format {
        ConversationExportFormat::Markdown => crate::claude::export::ExportFormat::Markdown,
        ConversationExportFormat::Html => crate::claude::export::ExportFormat::Html,
        ConversationExportFormat::Pdf => crate::claude::export::ExportFormat::Pdf,
        ConversationExportFormat::Json => crate::claude::export::ExportFormat::Json,
    };

    // Determine output path
    let output_path = match output {
        Some(path) => PathBuf::from(path),
        None => {
            let extension = match export_format {
                ConversationExportFormat::Markdown => "md",
                ConversationExportFormat::Html => "html",
                ConversationExportFormat::Pdf => "pdf",
                ConversationExportFormat::Json => "json",
            };
            PathBuf::from(format!(
                "conversation_{}.{}",
                &conversation.session_id[..8],
                extension
            ))
        }
    };

    // Create export configuration
    let config = ExportConfig {
        output_path: output_path.clone(),
        format: internal_format,
        include_metadata,
        include_tool_usage: include_tools,
        include_timestamps,
        template_path: None,
        title: Some(format!("Conversation: {}", conversation.session_id)),
    };

    if verbose {
        eprintln!("Exporting conversation to: {}", output_path.display());
    }

    // Create exporter and export
    let exporter = ConversationExporter::new(config);
    match exporter.export_conversation(conversation) {
        Ok(result) => {
            println!("📄 Conversation exported successfully!");
            println!("   File: {}", result.file_path.display());
            println!("   Size: {} bytes", result.file_size);
            println!("   Messages: {}", result.message_count);
            if verbose {
                println!("   Export time: {}ms", result.duration_ms);
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("❌ Export failed: {}", e);
            Err(e)
        }
    }
}

fn execute_mcp(action: McpAction, verbose: bool) -> Result<()> {
    match action {
        McpAction::List {
            detailed,
            status,
            format,
            sort,
        } => execute_mcp_list(detailed, status, format, sort, verbose),
        McpAction::Discover {
            health_check,
            verbose: discover_verbose,
            paths,
        } => execute_mcp_discover(health_check, discover_verbose || verbose, paths),
        McpAction::Add {
            name,
            command,
            args,
            env,
            global,
            project,
        } => crate::mcp::commands::execute_mcp_add(
            name, command, args, env, global, project, verbose,
        ),
        McpAction::Remove {
            name,
            global,
            project,
        } => crate::mcp::commands::execute_mcp_remove(name, global, project, verbose),
        McpAction::Update {
            name,
            command,
            args,
            env,
            global,
            project,
        } => crate::mcp::commands::execute_mcp_update(
            name, command, args, env, global, project, verbose,
        ),
    }
}

fn execute_mcp_list(
    detailed: bool,
    status_filter: Option<ServerStatusFilter>,
    format: OutputFormat,
    sort: ServerSortField,
    verbose: bool,
) -> Result<()> {
    // First show Claude Code servers from ~/.claude.json
    if verbose {
        eprintln!("Loading Claude Code MCP servers from ~/.claude.json...");
    }

    match crate::mcp::commands::list_claude_servers(verbose) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Warning: Could not load Claude Code servers: {}", e);
        }
    }

    println!();
    println!("{}", "─".repeat(60));
    println!();

    // Then discover and show other MCP servers
    if verbose {
        eprintln!("Discovering other MCP servers...");
    }

    // Discover servers
    let discovery = ServerDiscovery::new();
    let result = discovery.discover_servers()?;

    if verbose {
        eprintln!("{}", result.summary());
        if !result.errors.is_empty() {
            eprintln!("Warnings:");
            for error in &result.errors {
                eprintln!("  {}: {}", error.path.display(), error.message);
            }
        }
        eprintln!();
    }

    // Filter servers by status if specified
    let mut servers = result.servers;
    if let Some(filter) = status_filter {
        servers = filter_servers_by_status(servers, &filter);
    }

    // Sort servers
    sort_servers(&mut servers, &sort);

    // Display results
    match format {
        OutputFormat::Human => {
            display_servers_human(&servers, detailed);
        }
        OutputFormat::Json => {
            display_servers_json(&servers)?;
        }
        OutputFormat::Markdown | OutputFormat::Text => {
            display_servers_markdown(&servers, detailed);
        }
    }

    Ok(())
}

fn execute_mcp_discover(
    health_check: bool,
    verbose: bool,
    custom_paths: Vec<String>,
) -> Result<()> {
    if verbose {
        eprintln!("Starting MCP server discovery...");
    }

    // Create discovery instance
    let discovery = if custom_paths.is_empty() {
        ServerDiscovery::new()
    } else {
        let paths: Vec<std::path::PathBuf> = custom_paths
            .into_iter()
            .map(std::path::PathBuf::from)
            .collect();
        ServerDiscovery::with_paths(paths)
    }
    .with_health_checks(health_check);

    // Perform discovery
    let result = discovery.discover_servers()?;

    // Display results
    println!("🔍 {}", result.summary());

    if !result.servers.is_empty() {
        println!("\n📋 Discovered servers:");
        for server in &result.servers {
            let status_emoji = server.status.emoji();
            println!(
                "  {} {} - {}",
                status_emoji,
                server.name,
                server.transport.description()
            );
            if verbose {
                println!("     ID: {}", server.id);
                println!("     Config: {}", server.config_path.display());
                if let Some(version) = &server.version {
                    println!("     Version: {}", version);
                }
                if !server.capabilities.is_empty() {
                    println!(
                        "     Capabilities: {}",
                        server
                            .capabilities
                            .iter()
                            .map(|c| c.description())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
            }
        }
    }

    if !result.errors.is_empty() {
        println!("\n⚠️  Warnings ({}):", result.errors.len());
        for error in &result.errors {
            println!("  {} - {}", error.path.display(), error.message);
        }
    }

    if verbose {
        println!("\n📁 Scanned paths:");
        for path in &result.scanned_paths {
            let exists = if path.exists() { "✓" } else { "✗" };
            println!("  {} {}", exists, path.display());
        }
    }

    Ok(())
}

fn filter_servers_by_status(
    servers: Vec<McpServer>,
    filter: &ServerStatusFilter,
) -> Vec<McpServer> {
    servers
        .into_iter()
        .filter(|server| match filter {
            ServerStatusFilter::Running => matches!(server.status, ServerStatus::Running),
            ServerStatusFilter::Stopped => matches!(server.status, ServerStatus::Stopped),
            ServerStatusFilter::Error => matches!(server.status, ServerStatus::Error(_)),
            ServerStatusFilter::Unknown => matches!(server.status, ServerStatus::Unknown),
            ServerStatusFilter::Transitional => matches!(
                server.status,
                ServerStatus::Starting | ServerStatus::Stopping
            ),
        })
        .collect()
}

fn sort_servers(servers: &mut [McpServer], sort: &ServerSortField) {
    match sort {
        ServerSortField::Name => {
            servers.sort_by(|a, b| a.name.cmp(&b.name));
        }
        ServerSortField::Status => {
            servers.sort_by(|a, b| format!("{:?}", a.status).cmp(&format!("{:?}", b.status)));
        }
        ServerSortField::Version => {
            servers.sort_by(|a, b| {
                let a_version = a.version.as_deref().unwrap_or("");
                let b_version = b.version.as_deref().unwrap_or("");
                a_version.cmp(b_version)
            });
        }
        ServerSortField::LastCheck => {
            servers.sort_by(|a, b| a.last_health_check.cmp(&b.last_health_check));
        }
    }
}

fn display_servers_human(servers: &[McpServer], detailed: bool) {
    if servers.is_empty() {
        println!("No MCP servers found");
        println!();
        println!("💡 Tips:");
        println!("  • Install MCP servers in VS Code or Cursor");
        println!("  • Create MCP configurations in ~/.mcp/");
        println!("  • Use 'claude-tools mcp discover --verbose' to see scanned paths");
        return;
    }

    println!("🖥️  Found {} MCP server(s):", servers.len());
    println!();

    for server in servers {
        let status_emoji = server.status.emoji();
        println!("📄 {} {}", status_emoji, server.name);

        if detailed {
            for line in server.detailed_info() {
                println!("   {}", line);
            }
        } else {
            println!("   {}", server.transport.description());
            if let Some(version) = &server.version {
                println!("   Version: {}", version);
            }
            if let Some(description) = &server.description {
                println!("   {}", description);
            }
        }

        println!();
    }
}

fn display_servers_json(servers: &[McpServer]) -> Result<()> {
    let json = serde_json::to_string_pretty(servers)?;
    println!("{}", json);
    Ok(())
}

fn display_servers_markdown(servers: &[McpServer], detailed: bool) {
    println!("# MCP Servers");
    println!();

    if servers.is_empty() {
        println!("No MCP servers found.");
        return;
    }

    println!("Found {} MCP server(s):", servers.len());
    println!();

    for server in servers {
        let status_emoji = server.status.emoji();
        println!("## {} {}", status_emoji, server.name);

        if let Some(description) = &server.description {
            println!("{}", description);
            println!();
        }

        if detailed {
            println!("**Details:**");
            for line in server.detailed_info() {
                println!("- {}", line);
            }
        } else {
            println!("- **Transport:** {}", server.transport.description());
            if let Some(version) = &server.version {
                println!("- **Version:** {}", version);
            }
        }

        println!();
    }
}

/// Parse natural language date strings into DateTime<Utc>
fn parse_date_string(
    date_str: &str,
) -> std::result::Result<chrono::DateTime<chrono::Utc>, crate::errors::ClaudeToolsError> {
    use chrono::{Duration, NaiveDate, TimeZone, Utc};

    let date_str = date_str.trim().to_lowercase();

    // Handle relative dates
    if date_str.contains("ago") {
        let parts: Vec<&str> = date_str.split_whitespace().collect();
        if parts.len() >= 3 {
            if let Ok(amount) = parts[0].parse::<i64>() {
                let unit = parts[1];
                let now = Utc::now();

                let duration = match unit {
                    "second" | "seconds" | "sec" | "s" => Duration::seconds(amount),
                    "minute" | "minutes" | "min" | "m" => Duration::minutes(amount),
                    "hour" | "hours" | "hr" | "h" => Duration::hours(amount),
                    "day" | "days" | "d" => Duration::days(amount),
                    "week" | "weeks" | "w" => Duration::weeks(amount),
                    "month" | "months" => Duration::days(amount * 30), // Approximate
                    "year" | "years" | "y" => Duration::days(amount * 365), // Approximate
                    _ => {
                        return Err(crate::errors::ClaudeToolsError::General(anyhow::anyhow!(
                            "Unknown time unit: {}",
                            unit
                        )))
                    }
                };

                return Ok(now - duration);
            }
        }
    }

    // Handle relative keywords
    match date_str.as_str() {
        "now" | "today" => return Ok(Utc::now()),
        "yesterday" => return Ok(Utc::now() - Duration::days(1)),
        "last week" | "1 week ago" => return Ok(Utc::now() - Duration::weeks(1)),
        "last month" | "1 month ago" => return Ok(Utc::now() - Duration::days(30)),
        "last year" | "1 year ago" => return Ok(Utc::now() - Duration::days(365)),
        _ => {}
    }

    // Try ISO 8601 format (YYYY-MM-DD)
    if let Ok(naive_date) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
        if let Some(datetime) = naive_date.and_hms_opt(0, 0, 0) {
            return Ok(Utc.from_utc_datetime(&datetime));
        }
    }

    // Try date with time (YYYY-MM-DD HH:MM:SS)
    if let Ok(naive_datetime) =
        chrono::NaiveDateTime::parse_from_str(&date_str, "%Y-%m-%d %H:%M:%S")
    {
        return Ok(Utc.from_utc_datetime(&naive_datetime));
    }

    // Try other common formats
    let formats = [
        "%Y/%m/%d",
        "%m/%d/%Y",
        "%d/%m/%Y",
        "%Y-%m-%d %H:%M",
        "%Y/%m/%d %H:%M:%S",
        "%m/%d/%Y %H:%M:%S",
        "%d/%m/%Y %H:%M:%S",
    ];

    for format in &formats {
        if format.contains("%H") {
            if let Ok(naive_datetime) = chrono::NaiveDateTime::parse_from_str(&date_str, format) {
                return Ok(Utc.from_utc_datetime(&naive_datetime));
            }
        } else {
            if let Ok(naive_date) = NaiveDate::parse_from_str(&date_str, format) {
                if let Some(datetime) = naive_date.and_hms_opt(0, 0, 0) {
                    return Ok(Utc.from_utc_datetime(&datetime));
                }
            }
        }
    }

    Err(crate::errors::ClaudeToolsError::General(anyhow::anyhow!(
        "Could not parse date: {}",
        date_str
    )))
}
