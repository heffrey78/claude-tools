use crate::claude::{ClaudeDirectory, Conversation, ConversationParser, SearchEngine, SearchQuery, SearchResult, SearchMode, MatchHighlight, AnalyticsEngine, ConversationAnalytics, ConversationExporter, ExportConfig, ExportFormat, ActivityTimeline, TimelineConfig, TimePeriod, SummaryDepth, ActivityTrend, RankingIndicator, MessageRole, TimelineCache};
use crate::errors::ClaudeToolsError;
use crate::mcp::{ServerDiscovery, McpServer, ServerStatus, DiscoveryResult};
use crate::ui::conversation_display::ConversationRenderer;
use crossterm::event::{KeyCode, KeyEvent};
use std::collections::HashSet;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

/// Application state
#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    /// Browsing conversation list
    ConversationList,
    /// Viewing a specific conversation
    ConversationDetail,
    /// Search mode
    Search,
    /// Analytics dashboard
    Analytics,
    /// Timeline dashboard
    Timeline,
    /// Export mode
    Export,
    /// MCP server dashboard
    McpServerDashboard,
    /// Help overlay
    Help,
    /// Quitting application
    Quitting,
}

/// Main application struct
pub struct App {
    /// Current application state
    pub state: AppState,
    /// Should quit the application
    pub should_quit: bool,
    /// Conversation parser
    parser: ConversationParser,
    /// List of all conversations
    conversations: Vec<Conversation>,
    /// Current conversation list state
    pub conversation_list_state: ListState,
    /// Currently selected conversation
    selected_conversation: Option<Conversation>,
    /// Search query
    search_query: String,
    /// Search results
    search_results: Vec<Conversation>,
    /// Current scroll position in conversation detail
    detail_scroll: usize,
    /// Status message
    status_message: Option<String>,
    /// Error message
    error_message: Option<String>,
    /// Conversation renderer for markdown and syntax highlighting
    conversation_renderer: ConversationRenderer,
    /// Advanced search engine
    search_engine: SearchEngine,
    /// Current search results from advanced search
    advanced_search_results: Vec<SearchResult>,
    /// Current search mode
    current_search_mode: SearchMode,
    /// Current search result index for navigation
    current_search_result_index: usize,
    /// Search navigation active
    search_navigation_active: bool,
    /// Analytics engine
    analytics_engine: Option<AnalyticsEngine>,
    /// Cached analytics data
    analytics_data: Option<ConversationAnalytics>,
    /// Analytics scroll position
    analytics_scroll: usize,
    /// Export format selection
    export_format_index: usize,
    /// Available export formats
    export_formats: Vec<ExportFormat>,
    /// MCP server discovery engine
    server_discovery: ServerDiscovery,
    /// Discovered MCP servers
    mcp_servers: Vec<McpServer>,
    /// MCP server list state
    pub mcp_server_list_state: ListState,
    /// Last MCP discovery result
    last_discovery_result: Option<DiscoveryResult>,
    /// MCP dashboard scroll position
    mcp_scroll: usize,
    /// Auto-refresh MCP servers
    mcp_auto_refresh: bool,
    /// Activity timeline data
    activity_timeline: Option<ActivityTimeline>,
    /// Timeline configuration
    timeline_config: TimelineConfig,
    /// Timeline scroll position
    timeline_scroll: usize,
    /// Selected timeline project index
    timeline_project_index: usize,
    /// Timeline projects list (for navigation)
    timeline_projects: Vec<String>,
    /// Expanded timeline projects (for showing/hiding details)
    timeline_expanded_projects: std::collections::HashSet<String>,
    /// Timeline cache manager
    timeline_cache: Option<TimelineCache>,
    /// Timeline loading state
    timeline_loading: bool,
}

impl App {
    /// Initialize timeline cache
    fn initialize_timeline_cache(claude_dir: &ClaudeDirectory) -> Option<TimelineCache> {
        match TimelineCache::new(&claude_dir.path) {
            Ok(cache) => Some(cache),
            Err(_) => None, // Graceful fallback if cache can't be initialized
        }
    }

    /// Create a new application
    pub fn new(claude_dir: ClaudeDirectory) -> Result<Self, ClaudeToolsError> {
        let timeline_cache = Self::initialize_timeline_cache(&claude_dir);
        let parser = ConversationParser::new(claude_dir);
        let conversations = parser.parse_all_conversations()?;

        let mut list_state = ListState::default();
        if !conversations.is_empty() {
            list_state.select(Some(0));
        }

        // Build search engine
        let search_engine = parser.build_search_engine()?;

        // Initialize MCP server discovery
        let server_discovery = ServerDiscovery::new();
        let mcp_server_list_state = ListState::default();

        Ok(Self {
            state: AppState::ConversationList,
            should_quit: false,
            parser,
            conversations,
            conversation_list_state: list_state,
            selected_conversation: None,
            search_query: String::new(),
            search_results: Vec::new(),
            detail_scroll: 0,
            status_message: None,
            error_message: None,
            conversation_renderer: ConversationRenderer::new(80), // Default width, will update on render
            search_engine,
            advanced_search_results: Vec::new(),
            current_search_mode: SearchMode::Text,
            current_search_result_index: 0,
            search_navigation_active: false,
            analytics_engine: None,
            analytics_data: None,
            analytics_scroll: 0,
            export_format_index: 0,
            export_formats: vec![
                ExportFormat::Markdown,
                ExportFormat::Html,
                ExportFormat::Json,
                ExportFormat::Pdf,
            ],
            server_discovery,
            mcp_servers: Vec::new(),
            mcp_server_list_state,
            last_discovery_result: None,
            mcp_scroll: 0,
            mcp_auto_refresh: true,
            activity_timeline: None,
            timeline_config: TimelineConfig::default(),
            timeline_scroll: 0,
            timeline_project_index: 0,
            timeline_projects: Vec::new(),
            timeline_expanded_projects: HashSet::new(),
            timeline_cache,
            timeline_loading: false,
        })
    }

    /// Handle key events
    pub fn handle_key_event(&mut self, key: KeyEvent) {
        match self.state {
            AppState::ConversationList => self.handle_list_key_event(key),
            AppState::ConversationDetail => self.handle_detail_key_event(key),
            AppState::Search => self.handle_search_key_event(key),
            AppState::Analytics => self.handle_analytics_key_event(key),
            AppState::Timeline => self.handle_timeline_key_event(key),
            AppState::Export => self.handle_export_key_event(key),
            AppState::McpServerDashboard => self.handle_mcp_key_event(key),
            AppState::Help => self.handle_help_key_event(key),
            AppState::Quitting => {}
        }
    }

    /// Handle key events in conversation list mode
    fn handle_list_key_event(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
                self.state = AppState::Quitting;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.next_conversation();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.previous_conversation();
            }
            KeyCode::Char('g') => {
                self.first_conversation();
            }
            KeyCode::Char('G') => {
                self.last_conversation();
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.open_selected_conversation();
            }
            KeyCode::Char('/') => {
                self.start_search();
            }
            KeyCode::Char('n') => {
                if self.search_navigation_active {
                    self.next_search_result();
                }
            }
            KeyCode::Char('N') => {
                if self.search_navigation_active {
                    self.previous_search_result();
                }
            }
            KeyCode::Char('?') | KeyCode::Char('h') => {
                self.state = AppState::Help;
            }
            KeyCode::Char('r') => {
                self.refresh_conversations();
            }
            KeyCode::Char('a') => {
                self.start_analytics();
            }
            KeyCode::Char('t') => {
                self.start_timeline();
            }
            KeyCode::Char('m') => {
                self.start_mcp_dashboard();
            }
            _ => {}
        }
    }

    /// Handle key events in conversation detail mode
    fn handle_detail_key_event(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Backspace => {
                self.state = AppState::ConversationList;
                self.detail_scroll = 0;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(conversation) = &self.selected_conversation {
                    if self.detail_scroll < conversation.messages.len().saturating_sub(1) {
                        self.detail_scroll += 1;
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.detail_scroll > 0 {
                    self.detail_scroll -= 1;
                }
            }
            KeyCode::Char('g') => {
                self.detail_scroll = 0;
            }
            KeyCode::Char('G') => {
                if let Some(conversation) = &self.selected_conversation {
                    self.detail_scroll = conversation.messages.len().saturating_sub(1);
                }
            }
            KeyCode::PageDown => {
                if let Some(conversation) = &self.selected_conversation {
                    let max_scroll = conversation.messages.len().saturating_sub(1);
                    self.detail_scroll = (self.detail_scroll + 10).min(max_scroll);
                }
            }
            KeyCode::PageUp => {
                self.detail_scroll = self.detail_scroll.saturating_sub(10);
            }
            KeyCode::Char('e') => {
                self.start_export();
            }
            _ => {}
        }
    }

    /// Handle key events in search mode
    fn handle_search_key_event(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                self.execute_search();
                self.state = AppState::ConversationList;
            }
            KeyCode::Esc => {
                self.search_query.clear();
                self.state = AppState::ConversationList;
            }
            KeyCode::Backspace => {
                self.search_query.pop();
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
            }
            _ => {}
        }
    }

    /// Handle key events in analytics mode
    fn handle_analytics_key_event(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.state = AppState::ConversationList;
                self.analytics_scroll = 0;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.analytics_scroll = self.analytics_scroll.saturating_add(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.analytics_scroll = self.analytics_scroll.saturating_sub(1);
            }
            KeyCode::Char('g') => {
                self.analytics_scroll = 0;
            }
            KeyCode::Char('G') => {
                self.analytics_scroll = 100; // Max scroll, will be clamped by render
            }
            KeyCode::PageDown => {
                self.analytics_scroll = self.analytics_scroll.saturating_add(10);
            }
            KeyCode::PageUp => {
                self.analytics_scroll = self.analytics_scroll.saturating_sub(10);
            }
            KeyCode::Char('e') => {
                self.export_analytics();
            }
            KeyCode::Char('r') => {
                self.refresh_analytics();
            }
            _ => {}
        }
    }

    /// Handle key events in timeline mode
    fn handle_timeline_key_event(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.state = AppState::ConversationList;
                self.timeline_scroll = 0;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.timeline_project_index < self.timeline_projects.len().saturating_sub(1) {
                    self.timeline_project_index += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.timeline_project_index = self.timeline_project_index.saturating_sub(1);
            }
            KeyCode::Char('g') => {
                self.timeline_project_index = 0;
                self.timeline_scroll = 0;
            }
            KeyCode::Char('G') => {
                self.timeline_project_index = self.timeline_projects.len().saturating_sub(1);
            }
            KeyCode::Enter | KeyCode::Tab => {
                self.toggle_timeline_project();
            }
            KeyCode::PageDown => {
                self.timeline_scroll = self.timeline_scroll.saturating_add(10);
            }
            KeyCode::PageUp => {
                self.timeline_scroll = self.timeline_scroll.saturating_sub(10);
            }
            KeyCode::Char('r') => {
                self.refresh_timeline();
            }
            KeyCode::Char('1') => {
                self.update_timeline_config(TimelineConfig {
                    period: TimePeriod::LastDay,
                    ..self.timeline_config.clone()
                });
            }
            KeyCode::Char('2') => {
                self.update_timeline_config(TimelineConfig {
                    period: TimePeriod::LastTwoDay,
                    ..self.timeline_config.clone()
                });
            }
            KeyCode::Char('7') => {
                self.update_timeline_config(TimelineConfig {
                    period: TimePeriod::LastWeek,
                    ..self.timeline_config.clone()
                });
            }
            KeyCode::Char('3') => {
                self.update_timeline_config(TimelineConfig {
                    period: TimePeriod::LastMonth,
                    ..self.timeline_config.clone()
                });
            }
            KeyCode::Char('b') => {
                self.update_timeline_config(TimelineConfig {
                    summary_depth: SummaryDepth::Brief,
                    ..self.timeline_config.clone()
                });
            }
            KeyCode::Char('d') => {
                self.update_timeline_config(TimelineConfig {
                    summary_depth: SummaryDepth::Detailed,
                    ..self.timeline_config.clone()
                });
            }
            KeyCode::Char('c') => {
                self.update_timeline_config(TimelineConfig {
                    summary_depth: SummaryDepth::Comprehensive,
                    ..self.timeline_config.clone()
                });
            }
            KeyCode::Char('C') => {
                // Clear cache
                self.clear_timeline_cache();
            }
            _ => {}
        }
    }

    /// Handle key events in export mode
    fn handle_export_key_event(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.state = AppState::ConversationDetail;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.export_format_index < self.export_formats.len() - 1 {
                    self.export_format_index += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.export_format_index > 0 {
                    self.export_format_index -= 1;
                }
            }
            KeyCode::Enter => {
                // Check if format is available
                let format = &self.export_formats[self.export_format_index];
                let available = !matches!(format, ExportFormat::Pdf);
                
                if available {
                    self.execute_export();
                } else {
                    self.error_message = Some("PDF export requires external tools. Use HTML export and convert manually.".to_string());
                    self.state = AppState::ConversationDetail;
                }
            }
            _ => {}
        }
    }

    /// Handle key events in help mode
    fn handle_help_key_event(&mut self, _key: KeyEvent) {
        self.state = AppState::ConversationList;
    }

    /// Handle key events in MCP server dashboard mode
    fn handle_mcp_key_event(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.state = AppState::ConversationList;
                self.mcp_scroll = 0;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.next_mcp_server();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.previous_mcp_server();
            }
            KeyCode::Char('g') => {
                self.first_mcp_server();
            }
            KeyCode::Char('G') => {
                self.last_mcp_server();
            }
            KeyCode::Char('r') => {
                self.refresh_mcp_servers();
            }
            KeyCode::Char('d') => {
                self.discover_mcp_servers_with_health_check();
            }
            KeyCode::Char('?') | KeyCode::Char('h') => {
                self.state = AppState::Help;
            }
            KeyCode::PageDown => {
                self.mcp_scroll = self.mcp_scroll.saturating_add(10);
            }
            KeyCode::PageUp => {
                self.mcp_scroll = self.mcp_scroll.saturating_sub(10);
            }
            _ => {}
        }
    }

    /// Move to next conversation
    fn next_conversation(&mut self) {
        let conversations = self.get_current_conversation_list();
        let i = match self.conversation_list_state.selected() {
            Some(i) => {
                if i >= conversations.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.conversation_list_state.select(Some(i));
    }

    /// Move to previous conversation
    fn previous_conversation(&mut self) {
        let conversations = self.get_current_conversation_list();
        let i = match self.conversation_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    conversations.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.conversation_list_state.select(Some(i));
    }

    /// Move to first conversation
    fn first_conversation(&mut self) {
        self.conversation_list_state.select(Some(0));
    }

    /// Move to last conversation
    fn last_conversation(&mut self) {
        let conversations = self.get_current_conversation_list();
        if !conversations.is_empty() {
            self.conversation_list_state
                .select(Some(conversations.len() - 1));
        }
    }

    /// Move to next MCP server
    fn next_mcp_server(&mut self) {
        let i = match self.mcp_server_list_state.selected() {
            Some(i) => {
                if i >= self.mcp_servers.len().saturating_sub(1) {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.mcp_server_list_state.select(Some(i));
    }

    /// Move to previous MCP server
    fn previous_mcp_server(&mut self) {
        let i = match self.mcp_server_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.mcp_servers.len().saturating_sub(1)
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.mcp_server_list_state.select(Some(i));
    }

    /// Move to first MCP server
    fn first_mcp_server(&mut self) {
        if !self.mcp_servers.is_empty() {
            self.mcp_server_list_state.select(Some(0));
        }
    }

    /// Move to last MCP server
    fn last_mcp_server(&mut self) {
        if !self.mcp_servers.is_empty() {
            self.mcp_server_list_state.select(Some(self.mcp_servers.len() - 1));
        }
    }

    /// Open the currently selected conversation
    fn open_selected_conversation(&mut self) {
        if let Some(i) = self.conversation_list_state.selected() {
            let conversations = self.get_current_conversation_list();
            if let Some(conversation) = conversations.get(i) {
                self.selected_conversation = Some(conversation.clone());
                self.state = AppState::ConversationDetail;
                self.detail_scroll = 0;
            }
        }
    }

    /// Start search mode
    fn start_search(&mut self) {
        self.state = AppState::Search;
        self.search_query.clear();
        self.search_navigation_active = false;
    }

    /// Navigate to next search result
    fn next_search_result(&mut self) {
        if !self.advanced_search_results.is_empty() {
            self.current_search_result_index = 
                (self.current_search_result_index + 1) % self.advanced_search_results.len();
            
            // Update conversation list selection to match search result
            if let Some(search_result) = self.advanced_search_results.get(self.current_search_result_index) {
                if let Some(conv_index) = self.search_results.iter().position(|c| c.session_id == search_result.conversation.session_id) {
                    self.conversation_list_state.select(Some(conv_index));
                }
            }
            
            self.status_message = Some(format!(
                "Search result {}/{}", 
                self.current_search_result_index + 1, 
                self.advanced_search_results.len()
            ));
        }
    }

    /// Navigate to previous search result
    fn previous_search_result(&mut self) {
        if !self.advanced_search_results.is_empty() {
            self.current_search_result_index = if self.current_search_result_index == 0 {
                self.advanced_search_results.len() - 1
            } else {
                self.current_search_result_index - 1
            };
            
            // Update conversation list selection to match search result
            if let Some(search_result) = self.advanced_search_results.get(self.current_search_result_index) {
                if let Some(conv_index) = self.search_results.iter().position(|c| c.session_id == search_result.conversation.session_id) {
                    self.conversation_list_state.select(Some(conv_index));
                }
            }
            
            self.status_message = Some(format!(
                "Search result {}/{}", 
                self.current_search_result_index + 1, 
                self.advanced_search_results.len()
            ));
        }
    }

    /// Execute search with enhanced search engine
    fn execute_search(&mut self) {
        if self.search_query.is_empty() {
            self.search_results.clear();
            self.advanced_search_results.clear();
            self.status_message = Some("Search cleared".to_string());
        } else {
            // Determine search mode based on query pattern
            let search_mode = if self.search_query.starts_with("regex:") {
                SearchMode::Regex
            } else if self.search_query.starts_with("fuzzy:") {
                SearchMode::Fuzzy
            } else {
                SearchMode::Text
            };

            // Create search query
            let query = match search_mode {
                SearchMode::Regex => {
                    let pattern = self.search_query.strip_prefix("regex:").unwrap_or(&self.search_query);
                    SearchQuery::regex(pattern)
                }
                SearchMode::Text | SearchMode::Fuzzy => {
                    SearchQuery::text(&self.search_query)
                }
                SearchMode::Advanced => {
                    SearchQuery::text(&self.search_query)
                }
            };

            // Execute advanced search
            match self.search_engine.search(&query) {
                Ok(results) => {
                    // Convert SearchResult to Conversation for compatibility
                    self.search_results = results
                        .iter()
                        .map(|result| result.conversation.clone())
                        .collect();
                    
                    self.advanced_search_results = results;
                    self.current_search_mode = search_mode;
                    
                    let total_matches: usize = self.advanced_search_results
                        .iter()
                        .map(|r| r.match_count)
                        .sum();
                    
                    self.status_message = Some(format!(
                        "Found {} conversation(s) with {} total matches",
                        self.search_results.len(),
                        total_matches
                    ));
                    
                    if !self.search_results.is_empty() {
                        self.conversation_list_state.select(Some(0));
                        self.search_navigation_active = true;
                        self.current_search_result_index = 0;
                    } else {
                        self.search_navigation_active = false;
                    }
                }
                Err(e) => {
                    self.error_message = Some(format!("Search error: {}", e));
                }
            }
        }
    }

    /// Refresh conversations from directory
    fn refresh_conversations(&mut self) {
        match self.parser.parse_all_conversations() {
            Ok(conversations) => {
                self.conversations = conversations;
                self.status_message = Some(format!(
                    "Refreshed {} conversation(s)",
                    self.conversations.len()
                ));
                if !self.conversations.is_empty()
                    && self.conversation_list_state.selected().is_none()
                {
                    self.conversation_list_state.select(Some(0));
                }
            }
            Err(e) => {
                self.error_message = Some(format!("Refresh error: {}", e));
            }
        }
    }

    /// Get the current conversation list (either all or search results)
    fn get_current_conversation_list(&self) -> &[Conversation] {
        if self.search_results.is_empty() {
            &self.conversations
        } else {
            &self.search_results
        }
    }

    /// Start MCP server dashboard
    fn start_mcp_dashboard(&mut self) {
        self.state = AppState::McpServerDashboard;
        self.mcp_scroll = 0;
        
        // Perform initial discovery if no servers are loaded
        if self.mcp_servers.is_empty() {
            self.refresh_mcp_servers();
        }
    }

    /// Refresh MCP servers
    fn refresh_mcp_servers(&mut self) {
        match self.server_discovery.discover_servers() {
            Ok(result) => {
                self.mcp_servers = result.servers.clone();
                self.last_discovery_result = Some(result.clone());
                
                if !self.mcp_servers.is_empty() && self.mcp_server_list_state.selected().is_none() {
                    self.mcp_server_list_state.select(Some(0));
                }
                
                self.status_message = Some(format!(
                    "Found {} MCP server(s) in {:.2}s", 
                    result.server_count(),
                    result.scan_duration.as_secs_f64()
                ));
            }
            Err(e) => {
                self.error_message = Some(format!("MCP discovery error: {}", e));
            }
        }
    }

    /// Discover MCP servers with health checks
    fn discover_mcp_servers_with_health_check(&mut self) {
        let discovery = self.server_discovery.clone().with_health_checks(true);
        match discovery.discover_servers() {
            Ok(result) => {
                self.mcp_servers = result.servers.clone();
                self.last_discovery_result = Some(result.clone());
                
                if !self.mcp_servers.is_empty() && self.mcp_server_list_state.selected().is_none() {
                    self.mcp_server_list_state.select(Some(0));
                }
                
                self.status_message = Some(format!(
                    "Discovery with health checks: {} server(s) in {:.2}s", 
                    result.server_count(),
                    result.scan_duration.as_secs_f64()
                ));
            }
            Err(e) => {
                self.error_message = Some(format!("MCP health check error: {}", e));
            }
        }
    }

    /// Render the application
    pub fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(frame.area());

        match self.state {
            AppState::ConversationList | AppState::Search => {
                self.render_conversation_list(frame, chunks[0]);
            }
            AppState::ConversationDetail => {
                self.render_conversation_detail(frame, chunks[0]);
            }
            AppState::Analytics => {
                self.render_analytics_dashboard(frame, chunks[0]);
            }
            AppState::Timeline => {
                self.render_timeline_dashboard(frame, chunks[0]);
            }
            AppState::Export => {
                self.render_conversation_detail(frame, chunks[0]);
                self.render_export_overlay(frame, frame.area());
            }
            AppState::McpServerDashboard => {
                self.render_mcp_server_dashboard(frame, chunks[0]);
            }
            AppState::Help => {
                match self.state {
                    AppState::McpServerDashboard => self.render_mcp_server_dashboard(frame, chunks[0]),
                    _ => self.render_conversation_list(frame, chunks[0]),
                }
                self.render_help_overlay(frame, frame.area());
            }
            AppState::Quitting => {}
        }

        self.render_status_bar(frame, chunks[1]);

        if self.state == AppState::Search {
            self.render_search_input(frame, chunks[1]);
        }
    }

    /// Get context-sensitive help content based on current application state
    fn get_context_sensitive_help(&self) -> Vec<Line<'static>> {
        let mut help_text = Vec::new();
        
        // Header with application info
        help_text.push(Line::from(vec![
            Span::styled(
                "🚀 Claude Tools ".to_string(), 
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            ),
            Span::styled(
                "- Interactive Conversation Browser".to_string(),
                Style::default().fg(Color::White)
            ),
        ]));
        help_text.push(Line::from(""));
        
        // Context-specific help based on current state
        match self.state {
            AppState::ConversationList => {
                help_text.push(Line::from(vec![
                    Span::styled("📋 ".to_string(), Style::default().fg(Color::Blue)),
                    Span::styled("CONVERSATION LIST MODE".to_string(), Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)),
                ]));
                help_text.push(Line::from(""));
                
                help_text.extend(vec![
                    Line::from("📍 Navigation:"),
                    Line::from("  j / ↓      Move down in list"),
                    Line::from("  k / ↑      Move up in list"),
                    Line::from("  g          Jump to first conversation"),
                    Line::from("  G          Jump to last conversation"),
                    Line::from("  Enter      Open selected conversation"),
                    Line::from(""),
                    Line::from("🔍 Search & Filter:"),
                    Line::from("  /          Start search mode"),
                    Line::from("  n          Next search result (when searching)"),
                    Line::from("  N          Previous search result (when searching)"),
                    Line::from(""),
                    Line::from("🔧 Actions:"),
                    Line::from("  r          Refresh conversation list"),
                    Line::from("  a          Analytics dashboard"),
                    Line::from("  m          MCP server dashboard"),
                    Line::from("  q / Esc    Quit application"),
                ]);
                
                if self.search_navigation_active {
                    help_text.push(Line::from(""));
                    help_text.push(Line::from(vec![
                        Span::styled("🎯 ".to_string(), Style::default().fg(Color::Yellow)),
                        Span::styled("Search Active".to_string(), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                        Span::styled(" - Use 'n' and 'N' to navigate results".to_string(), Style::default().fg(Color::Yellow)),
                    ]));
                }
            },
            
            AppState::ConversationDetail => {
                help_text.push(Line::from(vec![
                    Span::styled("💬 ".to_string(), Style::default().fg(Color::Green)),
                    Span::styled("CONVERSATION DETAIL MODE".to_string(), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                ]));
                help_text.push(Line::from(""));
                
                help_text.extend(vec![
                    Line::from("📍 Navigation:"),
                    Line::from("  j / ↓      Scroll down through messages"),
                    Line::from("  k / ↑      Scroll up through messages"),
                    Line::from("  g          Jump to conversation start"),
                    Line::from("  G          Jump to conversation end"),
                    Line::from("  PgDn       Page down (fast scroll)"),
                    Line::from("  PgUp       Page up (fast scroll)"),
                    Line::from(""),
                    Line::from("🔧 Actions:"),
                    Line::from("  q / Esc    Return to conversation list"),
                    Line::from("  e          Export conversation to file"),
                    Line::from("  /          Search within conversation"),
                ]);
                
                if let Some(conversation) = &self.selected_conversation {
                    help_text.push(Line::from(""));
                    help_text.push(Line::from(vec![
                        Span::styled("📊 ".to_string(), Style::default().fg(Color::Cyan)),
                        Span::styled("Current: ".to_string(), Style::default().fg(Color::Cyan)),
                        Span::styled(
                            format!("{} ({} messages)", 
                                conversation.session_id.chars().take(8).collect::<String>(),
                                conversation.messages.len()
                            ),
                            Style::default().fg(Color::White)
                        ),
                    ]));
                }
            },
            
            AppState::Search => {
                help_text.push(Line::from(vec![
                    Span::styled("🔍 ".to_string(), Style::default().fg(Color::Yellow)),
                    Span::styled("SEARCH MODE".to_string(), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                ]));
                help_text.push(Line::from(""));
                
                help_text.extend(vec![
                    Line::from("⌨️  Basic Usage:"),
                    Line::from("  Type       Enter your search query"),
                    Line::from("  Enter      Execute search"),
                    Line::from("  Esc        Cancel and return to list"),
                    Line::from("  Backspace  Delete characters"),
                    Line::from(""),
                    Line::from("🎯 Advanced Search:"),
                    Line::from("  regex:pattern    Use regular expressions"),
                    Line::from("  fuzzy:text       Fuzzy/approximate matching"),
                    Line::from("  plain text       Standard text search"),
                    Line::from(""),
                    Line::from("📝 Examples:"),
                    Line::from("  error handling   Find conversations about error handling"),
                    Line::from("  regex:async.*fn  Find async functions (regex)"),
                    Line::from("  fuzzy:classs     Find 'class' with typos"),
                ]);
                
                if !self.search_query.is_empty() {
                    help_text.push(Line::from(""));
                    help_text.push(Line::from(vec![
                        Span::styled("💡 Current Query: ".to_string(), Style::default().fg(Color::Blue)),
                        Span::styled(self.search_query.clone(), Style::default().fg(Color::White).add_modifier(Modifier::ITALIC)),
                    ]));
                }
            },
            
            AppState::Analytics => {
                help_text.push(Line::from(vec![
                    Span::styled("📊 ".to_string(), Style::default().fg(Color::Blue)),
                    Span::styled("ANALYTICS DASHBOARD MODE".to_string(), Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)),
                ]));
                help_text.push(Line::from(""));
                
                help_text.extend(vec![
                    Line::from("📍 Navigation:"),
                    Line::from("  j / ↓      Scroll down through analytics"),
                    Line::from("  k / ↑      Scroll up through analytics"),
                    Line::from("  g          Jump to top of analytics"),
                    Line::from("  G          Jump to bottom of analytics"),
                    Line::from("  PgDn       Page down (fast scroll)"),
                    Line::from("  PgUp       Page up (fast scroll)"),
                    Line::from(""),
                    Line::from("🔧 Actions:"),
                    Line::from("  e          Export analytics to JSON file"),
                    Line::from("  r          Refresh analytics data"),
                    Line::from("  q / Esc    Return to conversation list"),
                ]);
            },

            AppState::Timeline => {
                help_text.push(Line::from(vec![
                    Span::styled("🕐 ".to_string(), Style::default().fg(Color::Magenta)),
                    Span::styled("ACTIVITY TIMELINE MODE".to_string(), Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
                ]));
                help_text.push(Line::from(""));
                
                help_text.extend(vec![
                    Line::from("📍 Navigation:"),
                    Line::from("  j / ↓      Move down project list"),
                    Line::from("  k / ↑      Move up project list"),
                    Line::from("  g          Jump to first project"),
                    Line::from("  G          Jump to last project"),
                    Line::from("  Enter/Tab  Expand/collapse project details"),
                    Line::from("  PgDn       Page down (fast scroll)"),
                    Line::from("  PgUp       Page up (fast scroll)"),
                    Line::from(""),
                    Line::from("⏰ Time Period Filters:"),
                    Line::from("  1          Past 24 hours"),
                    Line::from("  2          Past 48 hours"),
                    Line::from("  7          Past week"),
                    Line::from("  3          Past month"),
                    Line::from(""),
                    Line::from("📄 Summary Depth:"),
                    Line::from("  b          Brief summaries"),
                    Line::from("  d          Detailed summaries"),
                    Line::from("  c          Comprehensive summaries"),
                    Line::from(""),
                    Line::from("🔧 Actions:"),
                    Line::from("  r          Refresh timeline data"),
                    Line::from("  C          Clear timeline cache"),
                    Line::from("  q / Esc    Return to conversation list"),
                ]);
                
                if let Some(ref timeline) = self.activity_timeline {
                    help_text.push(Line::from(""));
                    help_text.push(Line::from(vec![
                        Span::styled("📊 Current: ".to_string(), Style::default().fg(Color::Cyan)),
                        Span::styled(
                            format!("{} - {} projects, {} conversations", 
                                timeline.config.period.label(),
                                timeline.projects.len(),
                                timeline.total_stats.total_conversations
                            ),
                            Style::default().fg(Color::White)
                        ),
                    ]));
                }
            },

            AppState::McpServerDashboard => {
                help_text.push(Line::from(vec![
                    Span::styled("🖥️  ".to_string(), Style::default().fg(Color::Cyan)),
                    Span::styled("MCP SERVER DASHBOARD MODE".to_string(), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                ]));
                help_text.push(Line::from(""));
                
                help_text.extend(vec![
                    Line::from("📍 Navigation:"),
                    Line::from("  j / ↓      Move down server list"),
                    Line::from("  k / ↑      Move up server list"),
                    Line::from("  g          Jump to first server"),
                    Line::from("  G          Jump to last server"),
                    Line::from("  PgDn       Page down (fast scroll)"),
                    Line::from("  PgUp       Page up (fast scroll)"),
                    Line::from(""),
                    Line::from("🔧 Actions:"),
                    Line::from("  r          Refresh server discovery"),
                    Line::from("  d          Discovery with health checks"),
                    Line::from("  ? / h      Show this help"),
                    Line::from("  q / Esc    Return to conversation list"),
                    Line::from(""),
                    Line::from("📊 Server Status:"),
                    Line::from("  🟢 Running - Server is active and responding"),
                    Line::from("  🔴 Stopped - Server is not running"),
                    Line::from("  🟡 Starting/Stopping - Server in transition"),
                    Line::from("  ⚠️  Error - Server has encountered issues"),
                    Line::from("  ❓ Unknown - Status could not be determined"),
                    Line::from(""),
                    Line::from("💡 MCP servers provide tools, resources, and prompts"),
                    Line::from("   that extend Claude's capabilities through the"),
                    Line::from("   Model Context Protocol (MCP)."),
                ]);
            },

            AppState::Export => {
                help_text.push(Line::from(vec![
                    Span::styled("📤 ".to_string(), Style::default().fg(Color::Magenta)),
                    Span::styled("EXPORT MODE".to_string(), Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
                ]));
                help_text.push(Line::from(""));
                
                help_text.extend(vec![
                    Line::from("📍 Format Selection:"),
                    Line::from("  j / ↓      Move down format list"),
                    Line::from("  k / ↑      Move up format list"),
                    Line::from(""),
                    Line::from("🔧 Actions:"),
                    Line::from("  Enter      Export conversation in selected format"),
                    Line::from("  q / Esc    Cancel export and return"),
                    Line::from(""),
                    Line::from("📋 Available Formats:"),
                    Line::from("  Markdown   - .md file with formatting"),
                    Line::from("  HTML       - .html file with styling"),
                    Line::from("  JSON       - .json file for processing"),
                    Line::from("  PDF        - .pdf file (external tool required)"),
                    Line::from(""),
                    Line::from("📝 Export includes metadata, timestamps, and tool usage"),
                ]);
            },
            
            AppState::Help => {
                // This shouldn't happen as we're in help mode, but just in case
                help_text.push(Line::from("Help is already displayed!"));
            },
            
            AppState::Quitting => {
                help_text.push(Line::from("Application is closing..."));
            },
        }
        
        // Universal shortcuts (always available)
        help_text.push(Line::from(""));
        help_text.push(Line::from(vec![
            Span::styled("⚡ ".to_string(), Style::default().fg(Color::Magenta)),
            Span::styled("UNIVERSAL".to_string(), Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
        ]));
        help_text.push(Line::from("  ? / h      Show this help (any mode)"));
        
        // Performance & features info
        help_text.push(Line::from(""));
        help_text.push(Line::from(vec![
            Span::styled("✨ Features: ".to_string(), Style::default().fg(Color::Green)),
            Span::styled("TF-IDF search, syntax highlighting, regex support".to_string(), Style::default().fg(Color::DarkGray)),
        ]));
        
        // Footer
        help_text.push(Line::from(""));
        help_text.push(Line::from(vec![
            Span::styled("Press any key to close help".to_string(), Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
        ]));
        
        help_text
    }

    /// Render conversation list
    fn render_conversation_list(&mut self, frame: &mut Frame, area: Rect) {
        let conversations = self.get_current_conversation_list();

        let items: Vec<ListItem> = conversations
            .iter()
            .map(|conv| {
                let summary = conv.summary.as_deref().unwrap_or("No summary");
                let project = &conv.project_path;
                let message_count = conv.messages.len();

                let content = format!(
                    "📄 {} ({})\n   📁 {}\n   💬 {} messages",
                    summary, conv.session_id, project, message_count
                );

                ListItem::new(content).style(Style::default().fg(Color::White))
            })
            .collect();

        let title = if self.search_results.is_empty() {
            format!("Conversations ({})", self.conversations.len())
        } else {
            format!(
                "Search Results ({}) - Query: '{}'",
                self.search_results.len(),
                self.search_query
            )
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .style(Style::default().fg(Color::White)),
            )
            .style(Style::default().fg(Color::White))
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::REVERSED)
                    .fg(Color::Yellow),
            );

        frame.render_stateful_widget(list, area, &mut self.conversation_list_state);
    }

    /// Render conversation detail with enhanced markdown formatting
    fn render_conversation_detail(&mut self, frame: &mut Frame, area: Rect) {
        if let Some(conversation) = &self.selected_conversation {
            // Update renderer width for responsive layout
            self.conversation_renderer.update_width(area.width as usize);

            let title = format!(
                "Conversation: {} (Message {}/{})",
                conversation.session_id,
                self.detail_scroll + 1,
                conversation.messages.len()
            );

            // Get visible messages based on scroll position
            let visible_messages = self.get_visible_messages(conversation, area.height as usize);
            
            // Render all visible messages with enhanced formatting and search highlights
            let mut all_lines = Vec::new();
            for (idx, message) in visible_messages.iter().enumerate() {
                let msg_idx = self.detail_scroll + idx;
                
                // Get highlights for this message
                let msg_highlights: Vec<MatchHighlight> = self.advanced_search_results.iter()
                    .flat_map(|result| &result.match_highlights)
                    .filter(|highlight| highlight.message_index == msg_idx)
                    .cloned()
                    .collect();
                
                let rendered_message = if msg_highlights.is_empty() {
                    self.conversation_renderer.render_message(message)
                } else {
                    self.conversation_renderer.render_message_with_highlights(message, &msg_highlights)
                };
                
                all_lines.extend(rendered_message.lines);
            }

            let paragraph = Paragraph::new(all_lines)
                .block(
                    Block::default()
                        .title(title)
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::White)),
                )
                .wrap(Wrap { trim: false });

            frame.render_widget(paragraph, area);
        }
    }

    /// Get messages that should be visible based on scroll position and screen height
    fn get_visible_messages<'a>(&self, conversation: &'a Conversation, screen_height: usize) -> &'a [crate::claude::conversation::ConversationMessage] {
        let start_idx = self.detail_scroll;
        let max_messages = (screen_height / 10).max(1); // Estimate ~10 lines per message
        let end_idx = (start_idx + max_messages).min(conversation.messages.len());
        
        &conversation.messages[start_idx..end_idx]
    }

    /// Render help overlay
    fn render_help_overlay(&mut self, frame: &mut Frame, area: Rect) {
        let help_text = self.get_context_sensitive_help();

        let title = match self.state {
            AppState::ConversationList => "Help - Conversation List",
            AppState::ConversationDetail => "Help - Conversation Detail",
            AppState::Search => "Help - Search Mode",
            AppState::Analytics => "Help - Analytics Dashboard",
            AppState::Timeline => "Help - Activity Timeline",
            AppState::Export => "Help - Export Mode",
            AppState::McpServerDashboard => "Help - MCP Server Dashboard",
            AppState::Help => "Help - Interactive Guide",
            AppState::Quitting => "Help",
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Yellow));

        let paragraph = Paragraph::new(help_text)
            .block(block)
            .wrap(Wrap { trim: true });

        let popup_area = centered_rect(80, 80, area);
        frame.render_widget(Clear, popup_area);
        frame.render_widget(paragraph, popup_area);
    }

    /// Render export overlay
    fn render_export_overlay(&mut self, frame: &mut Frame, area: Rect) {
        let mut export_text = Vec::new();
        
        // Header
        export_text.push(Line::from(vec![
            Span::styled("📤 ", Style::default().fg(Color::Magenta)),
            Span::styled("Export Conversation", 
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
        ]));
        export_text.push(Line::from(""));
        
        if let Some(ref conversation) = self.selected_conversation {
            export_text.push(Line::from(vec![
                Span::styled("Conversation: ", Style::default().fg(Color::White)),
                Span::styled(
                    format!("{} ({} messages)", 
                        conversation.session_id.chars().take(12).collect::<String>(),
                        conversation.messages.len()
                    ),
                    Style::default().fg(Color::Cyan)
                ),
            ]));
            export_text.push(Line::from(""));
        }
        
        export_text.push(Line::from("Select export format:"));
        export_text.push(Line::from(""));
        
        // Format options
        for (i, format) in self.export_formats.iter().enumerate() {
            let (name, description, available) = match format {
                ExportFormat::Markdown => ("Markdown", "Human-readable text with formatting", true),
                ExportFormat::Html => ("HTML", "Web page with styling and syntax highlighting", true),
                ExportFormat::Json => ("JSON", "Structured data for programmatic processing", true),
                ExportFormat::Pdf => ("PDF", "Print-ready document (external tool required)", false),
            };
            
            let style = if i == self.export_format_index {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::REVERSED)
            } else if available {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            
            let prefix = if i == self.export_format_index { "► " } else { "  " };
            let status = if available { "" } else { " (Not Available)" };
            
            export_text.push(Line::from(vec![
                Span::styled(format!("{}{}", prefix, name), style),
                Span::styled(status, Style::default().fg(Color::Red)),
            ]));
            export_text.push(Line::from(vec![
                Span::styled(format!("    {}", description), Style::default().fg(Color::DarkGray)),
            ]));
            export_text.push(Line::from(""));
        }
        
        // Instructions
        export_text.push(Line::from(""));
        export_text.push(Line::from(vec![
            Span::styled("Controls: ", Style::default().fg(Color::Blue)),
            Span::styled("j/k = navigate, Enter = export, q = cancel", Style::default().fg(Color::DarkGray)),
        ]));
        
        let block = Block::default()
            .title("Export Conversation")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Magenta));

        let paragraph = Paragraph::new(export_text)
            .block(block)
            .wrap(Wrap { trim: true });

        let popup_area = centered_rect(60, 70, area);
        frame.render_widget(Clear, popup_area);
        frame.render_widget(paragraph, popup_area);
    }

    /// Render status bar
    fn render_status_bar(&mut self, frame: &mut Frame, area: Rect) {
        let mut status_text = match self.state {
            AppState::ConversationList => "Press ? for help, / to search, a for analytics, t for timeline, m for MCP servers, q to quit".to_string(),
            AppState::ConversationDetail => "Press q to go back, j/k to scroll, e to export".to_string(),
            AppState::Search => format!("Search: {}_", self.search_query),
            AppState::Analytics => "Press j/k to scroll, e to export, r to refresh, q to go back".to_string(),
            AppState::Timeline => {
                let cache_status = if self.timeline_cache.is_some() { " • Cache enabled" } else { "" };
                format!("j/k: navigate, Enter/Tab: expand, 1/2/7/3: time periods, C: clear cache, q: back{}", cache_status)
            },
            AppState::Export => "Select format with j/k, Enter to export, q to cancel".to_string(),
            AppState::McpServerDashboard => "Press j/k to navigate, r to refresh, d for health checks, ? for help, q to go back".to_string(),
            AppState::Help => "Press any key to close help".to_string(),
            AppState::Quitting => "Goodbye!".to_string(),
        };

        if let Some(ref msg) = self.status_message {
            status_text = msg.clone();
        }

        if let Some(ref msg) = self.error_message {
            status_text = format!("Error: {}", msg);
        }

        let status = Paragraph::new(status_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default().fg(Color::White)),
            )
            .style(Style::default().fg(if self.error_message.is_some() {
                Color::Red
            } else {
                Color::White
            }));

        frame.render_widget(status, area);

        // Clear messages after displaying
        self.status_message = None;
        self.error_message = None;
    }

    /// Render search input
    fn render_search_input(&mut self, frame: &mut Frame, area: Rect) {
        let search_area = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width - 2,
            height: 1,
        };

        let search_text = format!("Search: {}_", self.search_query);
        let search_paragraph =
            Paragraph::new(search_text).style(Style::default().fg(Color::Yellow));

        frame.render_widget(search_paragraph, search_area);
    }

    /// Start analytics mode
    fn start_analytics(&mut self) {
        self.state = AppState::Analytics;
        self.analytics_scroll = 0;
        
        // Generate analytics if not cached
        if self.analytics_data.is_none() {
            if let Err(e) = self.generate_analytics() {
                self.error_message = Some(format!("Analytics error: {}", e));
                self.state = AppState::ConversationList;
            }
        }
    }

    /// Generate analytics data
    fn generate_analytics(&mut self) -> Result<(), ClaudeToolsError> {
        if self.analytics_engine.is_none() {
            self.analytics_engine = Some(AnalyticsEngine::new(self.conversations.clone()));
        }
        
        if let Some(ref mut engine) = self.analytics_engine {
            let analytics = engine.generate_analytics()?;
            self.analytics_data = Some(analytics.clone());
            self.status_message = Some("Analytics generated successfully".to_string());
        }
        
        Ok(())
    }

    /// Refresh analytics data
    fn refresh_analytics(&mut self) {
        self.analytics_data = None;
        self.analytics_engine = None;
        if let Err(e) = self.generate_analytics() {
            self.error_message = Some(format!("Analytics refresh error: {}", e));
        } else {
            self.status_message = Some("Analytics refreshed".to_string());
        }
    }

    /// Export analytics data
    fn export_analytics(&mut self) {
        if let Some(ref analytics) = self.analytics_data {
            let timestamp = analytics.generated_at.format("%Y%m%d_%H%M%S");
            let filename = format!("claude_analytics_{}.json", timestamp);
            
            match serde_json::to_string_pretty(analytics) {
                Ok(json_data) => {
                    match std::fs::write(&filename, json_data) {
                        Ok(_) => {
                            self.status_message = Some(format!("Analytics exported to {}", filename));
                        }
                        Err(e) => {
                            self.error_message = Some(format!("Export error: {}", e));
                        }
                    }
                }
                Err(e) => {
                    self.error_message = Some(format!("JSON error: {}", e));
                }
            }
        } else {
            self.error_message = Some("No analytics data to export".to_string());
        }
    }

    /// Start timeline mode
    fn start_timeline(&mut self) {
        self.state = AppState::Timeline;
        self.timeline_scroll = 0;
        self.timeline_project_index = 0;
        
        // Generate timeline if not cached
        if self.activity_timeline.is_none() {
            if let Err(e) = self.generate_timeline() {
                self.error_message = Some(format!("Timeline error: {}", e));
                self.state = AppState::ConversationList;
            }
        }
    }

    /// Generate timeline data with caching
    fn generate_timeline(&mut self) -> Result<(), ClaudeToolsError> {
        // Try to load from cache first
        if let Some(ref cache) = self.timeline_cache {
            let conversations_dir = self.parser.projects_dir();
            
            match cache.load_timeline(&self.timeline_config, &conversations_dir) {
                Ok(Some(cached_timeline)) => {
                    // Use cached timeline
                    self.timeline_projects = cached_timeline.projects.keys().cloned().collect();
                    self.timeline_projects.sort();
                    self.activity_timeline = Some(cached_timeline);
                    self.status_message = Some("Timeline loaded from cache".to_string());
                    self.timeline_loading = false;
                    return Ok(());
                }
                Ok(None) => {
                    // Cache miss or invalid - need to regenerate
                    self.status_message = Some("Generating timeline...".to_string());
                    self.timeline_loading = true;
                }
                Err(e) => {
                    // Cache error - proceed with generation
                    self.error_message = Some(format!("Cache error: {}", e));
                    self.timeline_loading = true;
                }
            }
        } else {
            self.timeline_loading = true;
        }
        
        // Generate new timeline
        let timeline = ActivityTimeline::create_filtered_timeline(
            self.conversations.clone(),
            self.timeline_config.clone(),
        );
        
        // Save to cache
        if let Some(ref cache) = self.timeline_cache {
            let conversations_dir = self.parser.projects_dir();
            if let Err(e) = cache.save_timeline(&timeline, &conversations_dir, self.conversations.len()) {
                // Don't fail if cache save fails - just log it
                self.error_message = Some(format!("Cache save failed: {}", e));
            }
        }
        
        // Extract project names for navigation
        self.timeline_projects = timeline.projects.keys().cloned().collect();
        self.timeline_projects.sort();
        
        self.activity_timeline = Some(timeline);
        self.status_message = Some("Timeline generated successfully".to_string());
        self.timeline_loading = false;
        
        Ok(())
    }

    /// Refresh timeline data (force regeneration, bypass cache)
    fn refresh_timeline(&mut self) {
        self.activity_timeline = None;
        self.timeline_projects.clear();
        self.timeline_expanded_projects.clear();
        self.timeline_project_index = 0;
        
        // Generate new timeline without checking cache
        self.status_message = Some("Regenerating timeline...".to_string());
        self.timeline_loading = true;
        
        let timeline = ActivityTimeline::create_filtered_timeline(
            self.conversations.clone(),
            self.timeline_config.clone(),
        );
        
        // Save to cache
        if let Some(ref cache) = self.timeline_cache {
            let conversations_dir = self.parser.projects_dir();
            if let Err(e) = cache.save_timeline(&timeline, &conversations_dir, self.conversations.len()) {
                self.error_message = Some(format!("Cache save failed: {}", e));
            }
        }
        
        // Extract project names for navigation
        self.timeline_projects = timeline.projects.keys().cloned().collect();
        self.timeline_projects.sort();
        
        self.activity_timeline = Some(timeline);
        self.status_message = Some("Timeline refreshed successfully".to_string());
        self.timeline_loading = false;
    }

    /// Update timeline configuration and regenerate
    fn update_timeline_config(&mut self, new_config: TimelineConfig) {
        self.timeline_config = new_config;
        self.refresh_timeline();
    }

    /// Clear timeline cache
    fn clear_timeline_cache(&mut self) {
        if let Some(ref cache) = self.timeline_cache {
            match cache.clear_cache() {
                Ok(count) => {
                    self.status_message = Some(format!("Cleared {} cache file(s)", count));
                }
                Err(e) => {
                    self.error_message = Some(format!("Cache clear error: {}", e));
                }
            }
        } else {
            self.status_message = Some("No cache available".to_string());
        }
    }

    /// Toggle project expansion in timeline
    fn toggle_timeline_project(&mut self) {
        if let Some(project) = self.timeline_projects.get(self.timeline_project_index) {
            if self.timeline_expanded_projects.contains(project) {
                self.timeline_expanded_projects.remove(project);
            } else {
                self.timeline_expanded_projects.insert(project.clone());
            }
        }
    }

    /// Create visual activity bar representation
    fn create_activity_bar(activity_score: f64) -> String {
        let bar_length = 10;
        let filled_length = (activity_score * bar_length as f64) as usize;
        let filled_char = "█";
        let empty_char = "░";
        
        let filled_part = filled_char.repeat(filled_length);
        let empty_part = empty_char.repeat(bar_length - filled_length);
        
        format!("[{}{}]", filled_part, empty_part)
    }

    /// Start export mode
    fn start_export(&mut self) {
        if self.selected_conversation.is_some() {
            self.state = AppState::Export;
            self.export_format_index = 0;
        } else {
            self.error_message = Some("No conversation selected for export".to_string());
        }
    }

    /// Execute export with selected format
    fn execute_export(&mut self) {
        if let Some(ref conversation) = self.selected_conversation {
            let format = &self.export_formats[self.export_format_index];
            
            // Generate filename
            let extension = match format {
                ExportFormat::Markdown => "md",
                ExportFormat::Html => "html",
                ExportFormat::Json => "json",
                ExportFormat::Pdf => "pdf",
            };
            let filename = format!("conversation_{}.{}", &conversation.session_id[..8], extension);
            
            // Create export config
            let config = ExportConfig {
                output_path: std::path::PathBuf::from(&filename),
                format: format.clone(),
                include_metadata: true,
                include_tool_usage: true,
                include_timestamps: true,
                template_path: None,
                title: Some(format!("Conversation: {}", conversation.session_id)),
            };

            // Create exporter and export
            let exporter = ConversationExporter::new(config);
            match exporter.export_conversation(conversation) {
                Ok(result) => {
                    self.status_message = Some(format!(
                        "Exported to {} ({} bytes, {} messages)", 
                        result.file_path.display(),
                        result.file_size,
                        result.message_count
                    ));
                    self.state = AppState::ConversationDetail;
                }
                Err(e) => {
                    self.error_message = Some(format!("Export failed: {}", e));
                    self.state = AppState::ConversationDetail;
                }
            }
        }
    }

    /// Render analytics dashboard
    fn render_analytics_dashboard(&mut self, frame: &mut Frame, area: Rect) {
        if let Some(ref analytics) = self.analytics_data {
            let mut content = Vec::new();
            
            // Header
            content.push(Line::from(vec![
                Span::styled("📊 ", Style::default().fg(Color::Blue)),
                Span::styled("Conversation Analytics Dashboard", 
                    Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" (Generated: {})", 
                    analytics.generated_at.format("%Y-%m-%d %H:%M:%S")),
                    Style::default().fg(Color::DarkGray)),
            ]));
            content.push(Line::from(""));
            
            // Basic Statistics Section
            content.push(Line::from(vec![
                Span::styled("📈 Basic Statistics", 
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            ]));
            let stats = &analytics.basic_stats;
            content.push(Line::from(format!("   Total conversations: {}", stats.total_conversations)));
            content.push(Line::from(format!("   Total messages: {}", stats.total_messages)));
            content.push(Line::from(format!("   User messages: {}", stats.total_user_messages)));
            content.push(Line::from(format!("   Assistant messages: {}", stats.total_assistant_messages)));
            content.push(Line::from(format!("   System messages: {}", stats.total_system_messages)));
            content.push(Line::from(format!("   Tool uses: {}", stats.total_tool_uses)));
            content.push(Line::from(format!("   Avg. messages per conversation: {:.1}", stats.average_messages_per_conversation)));
            
            if let Some(earliest) = &stats.date_range.earliest {
                content.push(Line::from(format!("   First conversation: {}", earliest.format("%Y-%m-%d"))));
            }
            if let Some(latest) = &stats.date_range.latest {
                content.push(Line::from(format!("   Latest conversation: {}", latest.format("%Y-%m-%d"))));
            }
            if let Some(span) = stats.date_range.span_days {
                content.push(Line::from(format!("   Activity span: {} days", span)));
            }
            content.push(Line::from(""));
            
            // Top Models Section
            content.push(Line::from(vec![
                Span::styled("🤖 Top Models", 
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ]));
            for (i, model) in analytics.model_analytics.top_models.iter().take(5).enumerate() {
                content.push(Line::from(format!("   {}. {} - {} uses ({:.1}%)", 
                    i + 1, model.model_name, model.usage_count, model.percentage)));
            }
            content.push(Line::from(""));
            
            // Top Tools Section
            content.push(Line::from(vec![
                Span::styled("🛠️  Top Tools", 
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]));
            for (i, tool) in analytics.tool_analytics.top_tools.iter().take(5).enumerate() {
                content.push(Line::from(format!("   {}. {} - {} uses ({:.1}%)", 
                    i + 1, tool.tool_name, tool.usage_count, tool.percentage)));
            }
            content.push(Line::from(""));
            
            // Top Projects Section
            content.push(Line::from(vec![
                Span::styled("📁 Top Projects", 
                    Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
            ]));
            for (i, project) in analytics.project_analytics.top_projects.iter().take(5).enumerate() {
                content.push(Line::from(format!("   {}. {} - {} conversations ({:.1}%)", 
                    i + 1, project.project_name, project.conversation_count, project.percentage)));
            }
            content.push(Line::from(""));
            
            // Temporal Analysis Section
            content.push(Line::from(vec![
                Span::styled("🕒 Peak Usage Hours", 
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            ]));
            for peak in analytics.temporal_analysis.peak_usage_hours.iter().take(3) {
                content.push(Line::from(format!("   {}:00 - {} conversations ({:.1}%)", 
                    peak.hour, peak.count, peak.percentage)));
            }
            content.push(Line::from(""));
            
            // Quality Metrics Section
            content.push(Line::from(vec![
                Span::styled("📊 Quality Metrics", 
                    Style::default().fg(Color::LightBlue).add_modifier(Modifier::BOLD)),
            ]));
            let quality = &analytics.quality_metrics;
            if let Some(avg_duration) = quality.average_conversation_duration {
                content.push(Line::from(format!("   Average conversation duration: {:.1} minutes", avg_duration)));
            }
            content.push(Line::from(format!("   Average turns per conversation: {:.1}", quality.average_turns_per_conversation)));
            content.push(Line::from(format!("   Completion rate: {:.1}%", quality.completion_rate)));
            
            let msg_dist = &quality.message_length_distribution;
            content.push(Line::from(format!("   Avg. message length: {:.0} characters", msg_dist.mean)));
            content.push(Line::from(format!("   Median message length: {} characters", msg_dist.median)));
            content.push(Line::from(""));
            
            // Conversation Length Distribution
            content.push(Line::from(vec![
                Span::styled("📈 Conversation Length Distribution", 
                    Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD)),
            ]));
            let conv_dist = &analytics.basic_stats.conversation_length_distribution;
            content.push(Line::from(format!("   Shortest: {} messages", conv_dist.min)));
            content.push(Line::from(format!("   Longest: {} messages", conv_dist.max)));
            content.push(Line::from(format!("   Average: {:.1} messages", conv_dist.mean)));
            content.push(Line::from(format!("   Median: {} messages", conv_dist.median)));
            content.push(Line::from(""));
            
            // Controls
            content.push(Line::from(vec![
                Span::styled("⌨️  Controls: ", Style::default().fg(Color::DarkGray)),
                Span::styled("j/k=scroll, g/G=top/bottom, e=export, r=refresh, q=back", 
                    Style::default().fg(Color::DarkGray)),
            ]));
            
            // Apply scrolling by slicing content
            let max_lines = area.height.saturating_sub(2) as usize; // Account for borders
            let visible_content = if self.analytics_scroll >= content.len() {
                Vec::new()
            } else {
                let end_idx = (self.analytics_scroll + max_lines).min(content.len());
                content[self.analytics_scroll..end_idx].to_vec()
            };
            
            let paragraph = Paragraph::new(visible_content)
                .block(
                    Block::default()
                        .title("Analytics Dashboard")
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::White)),
                )
                .wrap(Wrap { trim: false });
            
            frame.render_widget(paragraph, area);
        } else {
            // Show loading or error state
            let error_text = vec![
                Line::from(vec![
                    Span::styled("⚠️ ", Style::default().fg(Color::Yellow)),
                    Span::styled("No analytics data available", 
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(""),
                Line::from("Press 'r' to generate analytics data"),
                Line::from("Press 'q' to return to conversation list"),
            ];
            
            let paragraph = Paragraph::new(error_text)
                .block(
                    Block::default()
                        .title("Analytics Dashboard")
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::White)),
                )
                .wrap(Wrap { trim: false });
            
            frame.render_widget(paragraph, area);
        }
    }

    /// Render timeline dashboard
    fn render_timeline_dashboard(&mut self, frame: &mut Frame, area: Rect) {
        // Show loading state if timeline is being generated
        if self.timeline_loading {
            let loading_content = vec![
                Line::from(vec![
                    Span::styled("🕐 ", Style::default().fg(Color::Magenta)),
                    Span::styled("Activity Timeline Dashboard", 
                        Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("⏳ ", Style::default().fg(Color::Yellow)),
                    Span::styled("Generating timeline data...", 
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(""),
                Line::from("This may take a moment for large conversation datasets."),
                Line::from("Timeline will be cached for faster future access."),
                Line::from(""),
                Line::from("Press 'q' to cancel and return to conversation list."),
            ];
            
            let paragraph = Paragraph::new(loading_content)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title("Timeline Dashboard")
                    .border_style(Style::default().fg(Color::Magenta)))
                .wrap(Wrap { trim: true });
            
            frame.render_widget(paragraph, area);
            return;
        }
        
        if let Some(ref timeline) = self.activity_timeline {
            let mut content = Vec::new();
            
            // Header with configuration info
            content.push(Line::from(vec![
                Span::styled("🕐 ", Style::default().fg(Color::Magenta)),
                Span::styled("Activity Timeline Dashboard", 
                    Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" ({})", timeline.config.period.label()),
                    Style::default().fg(Color::DarkGray)),
            ]));
            content.push(Line::from(""));
            
            // Timeline stats with cache info
            content.push(Line::from(vec![
                Span::styled("📊 Timeline Statistics", 
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" (Generated: {})", 
                    timeline.generated_at.format("%H:%M:%S")),
                    Style::default().fg(Color::DarkGray)),
            ]));
            content.push(Line::from(format!("   Total projects: {}", timeline.projects.len())));
            content.push(Line::from(format!("   Total conversations: {}", timeline.total_stats.total_conversations)));
            content.push(Line::from(format!("   Total messages: {}", timeline.total_stats.total_messages)));
            if let Some(peak_day) = timeline.total_stats.peak_activity_day {
                content.push(Line::from(format!("   Peak activity day: {}", peak_day.format("%Y-%m-%d"))));
            }
            
            // Cache status indicator
            if self.timeline_cache.is_some() {
                content.push(Line::from(vec![
                    Span::styled("   💾 Cache: ", Style::default().fg(Color::Blue)),
                    Span::styled("Enabled", Style::default().fg(Color::Green)),
                    Span::styled(" (press C to clear)", Style::default().fg(Color::DarkGray)),
                ]));
            }
            content.push(Line::from(""));
            
            // Project listing with navigation indicators
            content.push(Line::from(vec![
                Span::styled("📁 Projects", 
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(" (use j/k to navigate, Enter/Tab to expand)", 
                    Style::default().fg(Color::DarkGray)),
            ]));
            content.push(Line::from(""));
            
            // Display projects
            for (index, project_name) in self.timeline_projects.iter().enumerate() {
                let is_selected = index == self.timeline_project_index;
                let is_expanded = self.timeline_expanded_projects.contains(project_name);
                
                if let Some(project_activity) = timeline.projects.get(project_name) {
                    // Project header
                    let prefix = if is_selected { "► " } else { "  " };
                    let expand_indicator = if is_expanded { "▼" } else { "▶" };
                    
                    let project_line = Line::from(vec![
                        Span::styled(prefix, 
                            if is_selected { Style::default().fg(Color::Yellow) } else { Style::default() }),
                        Span::styled(format!("{} ", expand_indicator), 
                            Style::default().fg(Color::Blue)),
                        Span::styled(project_name.clone(), 
                            if is_selected { 
                                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD) 
                            } else { 
                                Style::default().fg(Color::White) 
                            }),
                        Span::styled(format!(" ({} conversations)", project_activity.conversations.len()),
                            Style::default().fg(Color::DarkGray)),
                    ]);
                    content.push(project_line);
                    
                    // Show expanded content
                    if is_expanded {
                        // Activity metrics with visual indicators
                        let activity_bar = Self::create_activity_bar(project_activity.stats.activity_score);
                        let trend_icon = match project_activity.indicators.trend {
                            ActivityTrend::Increasing => "📈",
                            ActivityTrend::Stable => "📊", 
                            ActivityTrend::Decreasing => "📉",
                            ActivityTrend::NoData => "❓",
                        };
                        
                        content.push(Line::from(vec![
                            Span::styled("    📊 Activity: ", Style::default().fg(Color::Cyan)),
                            Span::styled(format!("{} messages ", project_activity.stats.total_messages), Style::default().fg(Color::White)),
                            Span::styled(activity_bar, Style::default().fg(Color::Green)),
                            Span::styled(format!(" {}%", (project_activity.stats.activity_score * 100.0) as u32), Style::default().fg(Color::Green)),
                            Span::styled(format!(" {}", trend_icon), Style::default()),
                        ]));

                        // Ranking and frequency metrics
                        if let Some(rank) = project_activity.stats.activity_rank {
                            let rank_style = match &project_activity.indicators.ranking_indicator {
                                RankingIndicator::Top { .. } => Style::default().fg(Color::Yellow),
                                RankingIndicator::Middle { .. } => Style::default().fg(Color::Cyan),
                                RankingIndicator::Bottom { .. } => Style::default().fg(Color::DarkGray),
                                RankingIndicator::Unranked => Style::default().fg(Color::DarkGray),
                            };
                            content.push(Line::from(vec![
                                Span::styled("    🏆 Rank: ", Style::default().fg(Color::Yellow)),
                                Span::styled(format!("#{} ", rank), rank_style.add_modifier(Modifier::BOLD)),
                                Span::styled(format!("({:.1} msg/day, {:.1} conv/day)", 
                                    project_activity.stats.message_frequency,
                                    project_activity.stats.conversation_frequency), 
                                    Style::default().fg(Color::DarkGray)),
                            ]));
                        }

                        // Message breakdown with visual segments  
                        let user_msgs = project_activity.stats.messages_by_role.get(&MessageRole::User).unwrap_or(&0);
                        let assistant_msgs = project_activity.stats.messages_by_role.get(&MessageRole::Assistant).unwrap_or(&0);
                        content.push(Line::from(vec![
                            Span::styled("    💬 Messages: ", Style::default().fg(Color::Blue)),
                            Span::styled(format!("{}👤 ", user_msgs), Style::default().fg(Color::Blue)),
                            Span::styled(format!("{}🤖 ", assistant_msgs), Style::default().fg(Color::Green)),
                            Span::styled(format!("(ratio {:.1}:1)", project_activity.stats.user_assistant_ratio), Style::default().fg(Color::DarkGray)),
                        ]));

                        // Tool usage
                        if !project_activity.stats.top_tools.is_empty() {
                            let top_tool_names: Vec<String> = project_activity.stats.top_tools.iter()
                                .take(3)
                                .map(|(name, count)| format!("{}({})", name, count))
                                .collect();
                            content.push(Line::from(vec![
                                Span::styled("    🔧 Top tools: ", Style::default().fg(Color::Yellow)),
                                Span::styled(top_tool_names.join(", "), Style::default().fg(Color::White)),
                            ]));
                        }

                        // Session duration
                        if let Some(duration) = project_activity.stats.avg_session_duration {
                            content.push(Line::from(vec![
                                Span::styled("    ⏱️  Avg session: ", Style::default().fg(Color::Magenta)),
                                Span::styled(format!("{:.0} minutes", duration), Style::default().fg(Color::White)),
                            ]));
                        }
                        if let Some(last_activity) = project_activity.last_activity {
                            content.push(Line::from(format!("    🕒 Last activity: {}", 
                                last_activity.format("%Y-%m-%d %H:%M"))));
                        }
                        
                        // Show topical summary
                        content.push(Line::from(format!("    📝 {}", project_activity.topical_summary.summary_text)));
                        
                        // Show key topics
                        if !project_activity.topical_summary.main_topics.is_empty() {
                            let topic_names: Vec<String> = project_activity.topical_summary.main_topics.iter()
                                .take(3)
                                .map(|t| t.name.clone())
                                .collect();
                            content.push(Line::from(format!("    🔖 Key topics: {}", 
                                topic_names.join(", "))));
                        }
                        
                        // Recent conversations
                        content.push(Line::from("    📋 Recent conversations:"));
                        for conv in project_activity.conversations.iter().take(3) {
                            let session_short = conv.session_id.chars().take(8).collect::<String>();
                            content.push(Line::from(format!("      • {} ({} messages) - {}", 
                                session_short, 
                                conv.message_count,
                                conv.started_at.format("%m/%d %H:%M"))));
                        }
                        
                        if project_activity.conversations.len() > 3 {
                            content.push(Line::from(format!("      ... and {} more", 
                                project_activity.conversations.len() - 3)));
                        }
                        content.push(Line::from(""));
                    }
                }
            }
            
            // Controls footer
            content.push(Line::from(""));
            content.push(Line::from(vec![
                Span::styled("⌨️  Controls: ", Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)),
            ]));
            content.push(Line::from("   j/k: Navigate projects  Enter/Tab: Expand/collapse  r: Refresh"));
            content.push(Line::from("   1: 24h  2: 48h  7: Week  3: Month  b/d/c: Brief/Detailed/Comprehensive"));
            content.push(Line::from("   C: Clear cache  q/Esc: Return to conversation list"));
            
            // Create scrollable paragraph
            let paragraph = Paragraph::new(content)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title("Timeline Dashboard")
                    .border_style(Style::default().fg(Color::Magenta)))
                .wrap(Wrap { trim: true })
                .scroll((self.timeline_scroll as u16, 0));
            
            frame.render_widget(paragraph, area);
        } else {
            // Show loading or empty state
            let empty_content = vec![
                Line::from(vec![
                    Span::styled("🕐 ", Style::default().fg(Color::Magenta)),
                    Span::styled("Activity Timeline Dashboard", 
                        Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("⏳ ", Style::default().fg(Color::Yellow)),
                    Span::styled("Generating timeline data...", 
                        Style::default().fg(Color::Yellow)),
                ]),
                Line::from(""),
                Line::from("Press 'r' to refresh or 'q' to return to conversation list."),
            ];
            
            let paragraph = Paragraph::new(empty_content)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title("Timeline Dashboard")
                    .border_style(Style::default().fg(Color::Magenta)))
                .wrap(Wrap { trim: true });
            
            frame.render_widget(paragraph, area);
        }
    }

    /// Render MCP server dashboard
    fn render_mcp_server_dashboard(&mut self, frame: &mut Frame, area: Rect) {
        if self.mcp_servers.is_empty() {
            // Show empty state with discovery tips
            let empty_content = vec![
                Line::from(vec![
                    Span::styled("🖥️  ", Style::default().fg(Color::Cyan)),
                    Span::styled("MCP Server Dashboard", 
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("🔍 ", Style::default().fg(Color::Yellow)),
                    Span::styled("No MCP servers found", 
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(""),
                Line::from("💡 Tips to get started:"),
                Line::from("  • Install MCP servers in VS Code or Cursor"),
                Line::from("  • Create MCP configurations in ~/.mcp/"),
                Line::from("  • Press 'r' to refresh server discovery"),
                Line::from("  • Press 'd' for discovery with health checks"),
                Line::from(""),
                Line::from("📁 Discovery scans these locations:"),
                Line::from("  • ~/.mcp/ (MCP configuration files)"),
                Line::from("  • VS Code User settings"),
                Line::from("  • Cursor User settings"),
                Line::from("  • .vscode/ and .cursor/ in current directory"),
                Line::from(""),
                Line::from(vec![
                    Span::styled("⌨️  Press 'r' to scan for servers or '?' for help", 
                        Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
                ]),
            ];
            
            let paragraph = Paragraph::new(empty_content)
                .block(
                    Block::default()
                        .title("MCP Server Dashboard")
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::White)),
                )
                .wrap(Wrap { trim: false });
            
            frame.render_widget(paragraph, area);
            return;
        }

        // Split the area into server list and details
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Render server list on the left
        self.render_mcp_server_list(frame, chunks[0]);
        
        // Render server details on the right
        self.render_mcp_server_details(frame, chunks[1]);
    }

    /// Render MCP server list
    fn render_mcp_server_list(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.mcp_servers
            .iter()
            .map(|server| {
                let status_emoji = server.status.emoji();

                let transport_desc = server.transport.description();
                let capabilities_count = server.capabilities.len();
                
                let content = format!(
                    "{} {}\n   🚀 {}\n   🛠️  {} capabilities",
                    status_emoji, 
                    server.name,
                    transport_desc,
                    capabilities_count
                );

                ListItem::new(content)
                    .style(Style::default().fg(Color::White))
            })
            .collect();

        let discovery_info = if let Some(ref result) = self.last_discovery_result {
            format!(
                "MCP Servers ({}) - Scanned {} paths in {:.2}s", 
                result.server_count(),
                result.scanned_paths.len(),
                result.scan_duration.as_secs_f64()
            )
        } else {
            format!("MCP Servers ({})", self.mcp_servers.len())
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .title(discovery_info)
                    .borders(Borders::ALL)
                    .style(Style::default().fg(Color::White)),
            )
            .style(Style::default().fg(Color::White))
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::REVERSED)
                    .fg(Color::Cyan),
            );

        frame.render_stateful_widget(list, area, &mut self.mcp_server_list_state);
    }

    /// Render MCP server details
    fn render_mcp_server_details(&mut self, frame: &mut Frame, area: Rect) {
        if let Some(selected_index) = self.mcp_server_list_state.selected() {
            if let Some(server) = self.mcp_servers.get(selected_index) {
                let mut details = Vec::new();

                // Header with server name and status
                details.push(Line::from(vec![
                    Span::styled(
                        format!("{} {}", server.status.emoji(), server.name),
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                    ),
                ]));
                details.push(Line::from(""));

                // Status information
                details.push(Line::from(vec![
                    Span::styled("Status: ", Style::default().fg(Color::White)),
                    Span::styled(
                        server.status.description(),
                        Style::default().fg(match server.status {
                            ServerStatus::Running => Color::Green,
                            ServerStatus::Stopped => Color::Red,
                            ServerStatus::Starting | ServerStatus::Stopping => Color::Yellow,
                            ServerStatus::Error(_) => Color::Red,
                            ServerStatus::Unknown => Color::DarkGray,
                        })
                    ),
                ]));

                // Basic information
                details.push(Line::from(vec![
                    Span::styled("ID: ", Style::default().fg(Color::White)),
                    Span::styled(server.id.clone(), Style::default().fg(Color::DarkGray)),
                ]));

                details.push(Line::from(vec![
                    Span::styled("Transport: ", Style::default().fg(Color::White)),
                    Span::styled(server.transport.description(), Style::default().fg(Color::White)),
                ]));

                if let Some(ref version) = server.version {
                    details.push(Line::from(vec![
                        Span::styled("Version: ", Style::default().fg(Color::White)),
                        Span::styled(version.clone(), Style::default().fg(Color::White)),
                    ]));
                }

                details.push(Line::from(vec![
                    Span::styled("Config: ", Style::default().fg(Color::White)),
                    Span::styled(server.config_path.display().to_string(), Style::default().fg(Color::DarkGray)),
                ]));

                if let Some(ref description) = server.description {
                    details.push(Line::from(""));
                    details.push(Line::from(vec![
                        Span::styled("Description:", Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)),
                    ]));
                    details.push(Line::from(description.clone()));
                }

                // Capabilities
                if !server.capabilities.is_empty() {
                    details.push(Line::from(""));
                    details.push(Line::from(vec![
                        Span::styled("Capabilities:", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    ]));
                    for capability in &server.capabilities {
                        details.push(Line::from(format!(
                            "  🛠️  {} - {}",
                            capability.name(),
                            capability.description()
                        )));
                    }
                }

                // Health check information
                if let Some(ref last_check) = server.last_health_check {
                    details.push(Line::from(""));
                    details.push(Line::from(vec![
                        Span::styled("Last Health Check: ", Style::default().fg(Color::White)),
                        Span::styled(
                            last_check.format("%Y-%m-%d %H:%M:%S").to_string(),
                            Style::default().fg(Color::DarkGray)
                        ),
                    ]));
                }

                // Error details if present
                if let ServerStatus::Error(ref error) = server.status {
                    details.push(Line::from(""));
                    details.push(Line::from(vec![
                        Span::styled("Error Details:", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                    ]));
                    details.push(Line::from(error.clone()));
                }

                // Discovery warnings
                if let Some(ref result) = self.last_discovery_result {
                    if !result.errors.is_empty() {
                        details.push(Line::from(""));
                        details.push(Line::from(vec![
                            Span::styled("Discovery Warnings:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                        ]));
                        for error in result.errors.iter().take(3) {
                            details.push(Line::from(format!(
                                "  ⚠️  {}: {}",
                                error.path.display(),
                                error.message
                            )));
                        }
                        if result.errors.len() > 3 {
                            details.push(Line::from(format!(
                                "  ... and {} more warnings",
                                result.errors.len() - 3
                            )));
                        }
                    }
                }

                // Instructions
                details.push(Line::from(""));
                details.push(Line::from(vec![
                    Span::styled("⌨️  j/k=navigate, r=refresh, d=health check, ?=help", 
                        Style::default().fg(Color::DarkGray)),
                ]));

                let paragraph = Paragraph::new(details)
                    .block(
                        Block::default()
                            .title("Server Details")
                            .borders(Borders::ALL)
                            .style(Style::default().fg(Color::White)),
                    )
                    .wrap(Wrap { trim: false });

                frame.render_widget(paragraph, area);
                return;
            }
        }

        // No server selected
        let no_selection = vec![
            Line::from(vec![
                Span::styled("📋 ", Style::default().fg(Color::DarkGray)),
                Span::styled("Server Details", Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(""),
            Line::from("Select a server from the list to view details"),
        ];

        let paragraph = Paragraph::new(no_selection)
            .block(
                Block::default()
                    .title("Server Details")
                    .borders(Borders::ALL)
                    .style(Style::default().fg(Color::White)),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }
}

/// Helper function to create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
