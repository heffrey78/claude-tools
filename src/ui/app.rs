use crate::claude::{ClaudeDirectory, Conversation, ConversationParser};
use crate::errors::ClaudeToolsError;
use crossterm::event::{KeyCode, KeyEvent};
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
    }

    /// Execute search
    fn execute_search(&mut self) {
        if self.search_query.is_empty() {
            self.search_results.clear();
            self.status_message = Some("Search cleared".to_string());
        } else {
            match self.parser.search_conversations(&self.search_query) {
                Ok(results) => {
                    self.search_results = results;
                    self.status_message =
                        Some(format!("Found {} result(s)", self.search_results.len()));
                    self.conversation_list_state.select(Some(0));
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

    /// Render conversation detail
    fn render_conversation_detail(&mut self, frame: &mut Frame, area: Rect) {
        if let Some(conversation) = &self.selected_conversation {
            let available_height = area.height.saturating_sub(3); // Account for borders and title
            let mut lines = Vec::new();

            for (i, msg) in conversation.messages.iter().enumerate() {
                if i < self.detail_scroll {
                    continue;
                }

                let role_str = match msg.role {
                    crate::claude::conversation::MessageRole::User => "👤 User",
                    crate::claude::conversation::MessageRole::Assistant => "🤖 Assistant",
                    crate::claude::conversation::MessageRole::System => "⚙️ System",
                };

                let timestamp = msg.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
                let header = format!("{} [{}]", role_str, timestamp);

                lines.push(Line::from(vec![Span::styled(
                    header,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )]));

                // Add message content, wrapping long lines
                let content_lines = msg.content.lines().take(5); // Limit to 5 lines per message for scrolling
                for content_line in content_lines {
                    if content_line.trim().is_empty() {
                        lines.push(Line::from(""));
                    } else {
                        let truncated = if content_line.len()
                            > (area.width as usize).saturating_sub(4)
                        {
                            format!(
                                "{}...",
                                &content_line[..((area.width as usize).saturating_sub(7)).max(1)]
                            )
                        } else {
                            content_line.to_string()
                        };
                        lines.push(Line::from(vec![Span::styled(
                            truncated,
                            Style::default().fg(Color::White),
                        )]));
                    }
                }

                // Show tool uses if any
                if !msg.tool_uses.is_empty() {
                    lines.push(Line::from(vec![Span::styled(
                        format!("🛠️ {} tool use(s)", msg.tool_uses.len()),
                        Style::default().fg(Color::Yellow),
                    )]));
                }

                lines.push(Line::from("")); // Empty line between messages

                // Stop if we've filled the available height
                if lines.len() >= available_height as usize {
                    break;
                }
            }

            let title = format!(
                "Conversation: {} (Message {}/{})",
                conversation.session_id,
                self.detail_scroll + 1,
                conversation.messages.len()
            );

            let paragraph = Paragraph::new(lines)
                .block(
                    Block::default()
                        .title(title)
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::White)),
                )
                .wrap(Wrap { trim: true });

            frame.render_widget(paragraph, area);
        }
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
