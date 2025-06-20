use crate::claude::{ClaudeDirectory, Conversation, ConversationParser, SearchEngine, SearchQuery, SearchResult, SearchMode, MatchHighlight};
use crate::errors::ClaudeToolsError;
use crate::ui::conversation_display::ConversationRenderer;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
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
}

impl App {
    /// Create a new application
    pub fn new(claude_dir: ClaudeDirectory) -> Result<Self, ClaudeToolsError> {
        let parser = ConversationParser::new(claude_dir);
        let conversations = parser.parse_all_conversations()?;

        let mut list_state = ListState::default();
        if !conversations.is_empty() {
            list_state.select(Some(0));
        }

        // Build search engine
        let search_engine = parser.build_search_engine()?;

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
        })
    }

    /// Handle key events
    pub fn handle_key_event(&mut self, key: KeyEvent) {
        match self.state {
            AppState::ConversationList => self.handle_list_key_event(key),
            AppState::ConversationDetail => self.handle_detail_key_event(key),
            AppState::Search => self.handle_search_key_event(key),
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

    /// Handle key events in help mode
    fn handle_help_key_event(&mut self, _key: KeyEvent) {
        self.state = AppState::ConversationList;
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
            AppState::Help => {
                self.render_conversation_list(frame, chunks[0]);
                self.render_help_overlay(frame, frame.area());
            }
            AppState::Quitting => {}
        }

        self.render_status_bar(frame, chunks[1]);

        if self.state == AppState::Search {
            self.render_search_input(frame, chunks[1]);
        }
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
        let help_text = vec![
            Line::from("🏠 Conversation List Navigation:"),
            Line::from("  j/↓    - Move down"),
            Line::from("  k/↑    - Move up"),
            Line::from("  g      - Go to first"),
            Line::from("  G      - Go to last"),
            Line::from("  Enter  - Open conversation"),
            Line::from("  /      - Search"),
            Line::from("  r      - Refresh"),
            Line::from(""),
            Line::from("📄 Conversation Detail Navigation:"),
            Line::from("  j/↓    - Scroll down"),
            Line::from("  k/↑    - Scroll up"),
            Line::from("  g      - Go to top"),
            Line::from("  G      - Go to bottom"),
            Line::from("  PgDn   - Page down"),
            Line::from("  PgUp   - Page up"),
            Line::from("  q/Esc  - Back to list"),
            Line::from(""),
            Line::from("🔍 Search Mode:"),
            Line::from("  Type   - Enter search query"),
            Line::from("  Enter  - Execute search"),
            Line::from("  Esc    - Cancel search"),
            Line::from("  n      - Next search result"),
            Line::from("  N      - Previous search result"),
            Line::from("  regex: - Use regex search"),
            Line::from("  fuzzy: - Use fuzzy search"),
            Line::from(""),
            Line::from("Press any key to close help"),
        ];

        let block = Block::default()
            .title("Help - Keyboard Shortcuts")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Yellow));

        let paragraph = Paragraph::new(help_text)
            .block(block)
            .wrap(Wrap { trim: true });

        let popup_area = centered_rect(80, 80, area);
        frame.render_widget(Clear, popup_area);
        frame.render_widget(paragraph, popup_area);
    }

    /// Render status bar
    fn render_status_bar(&mut self, frame: &mut Frame, area: Rect) {
        let mut status_text = match self.state {
            AppState::ConversationList => "Press ? for help, / to search, q to quit".to_string(),
            AppState::ConversationDetail => "Press q to go back, j/k to scroll".to_string(),
            AppState::Search => format!("Search: {}_", self.search_query),
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
