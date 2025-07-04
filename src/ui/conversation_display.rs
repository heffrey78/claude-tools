use crate::claude::conversation::{ConversationMessage, MessageRole};
use crate::claude::search::{HighlightType, MatchHighlight};
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Parser, Tag, TagEnd};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};
use syntect::{
    easy::HighlightLines,
    highlighting::{Style as SyntectStyle, ThemeSet},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};
use textwrap::Options;

/// Renders Claude Code conversations with markdown support and syntax highlighting
pub struct ConversationRenderer {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    terminal_width: usize,
}

impl ConversationRenderer {
    /// Create a new conversation renderer
    pub fn new(terminal_width: usize) -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            terminal_width: terminal_width.saturating_sub(4), // Account for borders and padding
        }
    }

    /// Update terminal width for responsive rendering
    pub fn update_width(&mut self, width: usize) {
        self.terminal_width = width.saturating_sub(4);
    }

    /// Render a complete conversation message with markdown formatting
    pub fn render_message(&self, message: &ConversationMessage) -> Text<'_> {
        self.render_message_with_highlights(message, &[])
    }

    /// Render a complete conversation message with search highlights
    pub fn render_message_with_highlights(
        &self,
        message: &ConversationMessage,
        highlights: &[MatchHighlight],
    ) -> Text<'_> {
        let mut lines = Vec::new();

        // Add message header with speaker and timestamp
        let header = self.render_message_header(message);
        lines.push(header);

        // Add separator line
        lines.push(Line::from(""));

        // Render message content with markdown and highlights
        let content_lines =
            self.render_markdown_content_with_highlights(&message.content, highlights);
        lines.extend(content_lines);

        // Add tool uses if any
        if !message.tool_uses.is_empty() {
            lines.push(Line::from(""));
            lines.extend(self.render_tool_uses(&message.tool_uses));
        }

        // Add bottom separator
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "─".repeat(self.terminal_width),
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));

        Text::from(lines)
    }

    /// Render message header with speaker identification and timestamp
    fn render_message_header(&self, message: &ConversationMessage) -> Line<'_> {
        let (speaker_icon, speaker_name, speaker_style) = match message.role {
            MessageRole::User => (
                "👤",
                "User",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            MessageRole::Assistant => (
                "🤖",
                "Claude",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            MessageRole::System => (
                "⚙️",
                "System",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        };

        let timestamp = message.timestamp.format("%H:%M:%S").to_string();
        let model_info = message
            .model
            .as_deref()
            .unwrap_or("claude")
            .split('/')
            .last()
            .unwrap_or("unknown");

        Line::from(vec![
            Span::styled(format!("{} ", speaker_icon), speaker_style),
            Span::styled(speaker_name, speaker_style),
            Span::raw(" "),
            Span::styled(
                format!("[{}]", timestamp),
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw(" "),
            Span::styled(
                format!("({})", model_info),
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            ),
        ])
    }

    /// Render markdown content with search highlights
    fn render_markdown_content_with_highlights(
        &self,
        content: &str,
        highlights: &[MatchHighlight],
    ) -> Vec<Line<'_>> {
        // Simple approach: if there are highlights, render without markdown parsing
        // and apply highlights directly to the raw content
        if !highlights.is_empty() {
            return self.render_plain_text_with_highlights(content, highlights);
        }
        
        let parser = Parser::new(content);
        let mut lines = Vec::new();
        let mut current_line = Vec::new();
        let mut in_code_block = false;
        let mut code_lang: Option<String> = None;
        let mut code_content = String::new();
        let mut in_heading = false;
        let mut heading_level = 1;
        let mut in_emphasis = false;
        let mut in_strong = false;

        for event in parser {
            match event {
                Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang))) => {
                    in_code_block = true;
                    code_lang = Some(lang.to_string());
                    code_content.clear();

                    // Finish current line before code block
                    if !current_line.is_empty() {
                        lines.push(Line::from(current_line.clone()));
                        current_line.clear();
                    }
                }
                Event::End(TagEnd::CodeBlock) => {
                    if in_code_block {
                        let highlighted_lines =
                            self.highlight_code(&code_content, code_lang.as_deref());
                        lines.extend(highlighted_lines);
                        in_code_block = false;
                        code_lang = None;
                        code_content.clear();
                    }
                }
                Event::Start(Tag::Heading { level, .. }) => {
                    in_heading = true;
                    heading_level = match level {
                        HeadingLevel::H1 => 1,
                        HeadingLevel::H2 => 2,
                        HeadingLevel::H3 => 3,
                        HeadingLevel::H4 => 4,
                        HeadingLevel::H5 => 5,
                        HeadingLevel::H6 => 6,
                    };

                    if !current_line.is_empty() {
                        lines.push(Line::from(current_line.clone()));
                        current_line.clear();
                    }
                    lines.push(Line::from("")); // Add space before heading
                }
                Event::End(TagEnd::Heading(_)) => {
                    if in_heading {
                        // Add heading prefix and styling
                        let prefix = "#".repeat(heading_level);
                        let mut heading_line = vec![Span::styled(
                            format!("{} ", prefix),
                            Style::default()
                                .fg(Color::Blue)
                                .add_modifier(Modifier::BOLD),
                        )];

                        // Style the heading text
                        for span in current_line.drain(..) {
                            let styled_span = match span.style.fg {
                                Some(_) => span, // Keep existing styling
                                None => Span::styled(
                                    span.content,
                                    Style::default()
                                        .fg(Color::Blue)
                                        .add_modifier(Modifier::BOLD),
                                ),
                            };
                            heading_line.push(styled_span);
                        }

                        lines.push(Line::from(heading_line));
                        lines.push(Line::from("")); // Add space after heading
                        in_heading = false;
                    }
                }
                Event::Start(Tag::Strong) => {
                    in_strong = true;
                }
                Event::End(TagEnd::Strong) => {
                    in_strong = false;
                }
                Event::Start(Tag::Emphasis) => {
                    in_emphasis = true;
                }
                Event::End(TagEnd::Emphasis) => {
                    in_emphasis = false;
                }
                Event::Start(Tag::List(_)) => {
                    if !current_line.is_empty() {
                        lines.push(Line::from(current_line.clone()));
                        current_line.clear();
                    }
                }
                Event::Start(Tag::Item) => {
                    if !current_line.is_empty() {
                        lines.push(Line::from(current_line.clone()));
                        current_line.clear();
                    }
                    current_line.push(Span::styled("• ", Style::default().fg(Color::Yellow)));
                }
                Event::Text(text) => {
                    if in_code_block {
                        code_content.push_str(&text);
                    } else {
                        let wrapped_text = self.wrap_text(&text);
                        for (i, line_text) in wrapped_text.into_iter().enumerate() {
                            if i > 0 {
                                lines.push(Line::from(current_line.clone()));
                                current_line.clear();
                            }

                            let mut style = Style::default().fg(Color::White);
                            if in_strong {
                                style = style.add_modifier(Modifier::BOLD);
                            }
                            if in_emphasis {
                                style = style.add_modifier(Modifier::ITALIC);
                            }

                            // Apply search highlights - but only to the actual content, not duplicating it
                            let highlighted_spans =
                                self.apply_text_highlights(&line_text, highlights, style);
                            current_line.extend(highlighted_spans);
                        }
                    }
                }
                Event::Code(code) => {
                    let style = Style::default().fg(Color::Yellow).bg(Color::DarkGray);
                    current_line.push(Span::styled(format!("`{}`", code), style));
                }
                Event::SoftBreak => {
                    current_line.push(Span::raw(" "));
                }
                Event::HardBreak => {
                    lines.push(Line::from(current_line.clone()));
                    current_line.clear();
                }
                _ => {} // Handle other events as needed
            }
        }

        // Add any remaining content
        if !current_line.is_empty() {
            lines.push(Line::from(current_line));
        }

        lines
    }

    /// Highlight code with syntax highlighting
    fn highlight_code(&self, code: &str, language: Option<&str>) -> Vec<Line<'static>> {
        let lang = language.unwrap_or("text");
        let syntax = self
            .syntax_set
            .find_syntax_by_token(lang)
            .or_else(|| self.syntax_set.find_syntax_by_extension(lang))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        // Use a dark theme for better terminal visibility
        let theme = &self.theme_set.themes["base16-ocean.dark"];
        let mut highlighter = HighlightLines::new(syntax, theme);

        let mut lines = Vec::new();

        // Add code block header
        lines.push(Line::from(vec![
            Span::styled("```".to_string(), Style::default().fg(Color::DarkGray)),
            Span::styled(
                lang.to_string(),
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::ITALIC),
            ),
        ]));

        // Highlight each line
        for line in LinesWithEndings::from(code) {
            let highlighted = highlighter
                .highlight_line(line, &self.syntax_set)
                .unwrap_or_default();

            let mut spans = vec![
                Span::styled("  ".to_string(), Style::default()), // Indentation
            ];

            for (style, text) in highlighted {
                let color = syntect_style_to_ratatui_color(style);
                spans.push(Span::styled(text.to_owned(), Style::default().fg(color)));
            }

            lines.push(Line::from(spans));
        }

        // Add code block footer
        lines.push(Line::from(Span::styled(
            "```".to_string(),
            Style::default().fg(Color::DarkGray),
        )));

        lines
    }

    /// Render tool uses section  
    fn render_tool_uses(
        &self,
        tool_uses: &[crate::claude::conversation::ToolUse],
    ) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        lines.push(Line::from(Span::styled(
            format!("🛠️ Tool Uses ({})", tool_uses.len()),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )));

        for (i, tool_use) in tool_uses.iter().enumerate() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled(format!("{}. ", i + 1), Style::default().fg(Color::Magenta)),
                Span::styled(
                    tool_use.name.clone(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));

            // Show tool parameters (truncated if too long)
            let params_str = tool_use.input.to_string();
            let truncated = if params_str.len() > 100 {
                format!("{}...", &params_str[..97])
            } else {
                params_str
            };

            let wrapped_params = self.wrap_text(&truncated);
            for line_text in wrapped_params {
                lines.push(Line::from(vec![
                    Span::raw("   ".to_string()),
                    Span::styled(line_text, Style::default().fg(Color::DarkGray)),
                ]));
            }
        }

        lines
    }

    /// Wrap text to terminal width
    fn wrap_text(&self, text: &str) -> Vec<String> {
        let options = Options::new(self.terminal_width)
            .break_words(false)
            .word_separator(textwrap::WordSeparator::AsciiSpace);

        textwrap::wrap(text, &options)
            .into_iter()
            .map(|cow| cow.to_string())
            .collect()
    }

    /// Render plain text content with highlights (bypassing markdown parsing)
    fn render_plain_text_with_highlights(
        &self,
        content: &str,
        highlights: &[MatchHighlight],
    ) -> Vec<Line<'_>> {
        let wrapped_text = self.wrap_text(content);
        let mut lines = Vec::new();
        let mut content_offset = 0;
        
        for line_text in wrapped_text {
            let line_end = content_offset + line_text.len();
            
            // Find highlights that overlap with this line
            let line_highlights: Vec<MatchHighlight> = highlights
                .iter()
                .filter_map(|highlight| {
                    // Check if this highlight overlaps with the current line
                    if highlight.start < line_end && highlight.end > content_offset {
                        let line_start = highlight.start.saturating_sub(content_offset);
                        let line_end_pos = (highlight.end - content_offset).min(line_text.len());
                        
                        if line_start < line_text.len() && line_end_pos > line_start {
                            Some(MatchHighlight {
                                message_index: highlight.message_index,
                                start: line_start,
                                end: line_end_pos,
                                matched_text: highlight.matched_text.clone(),
                                highlight_type: highlight.highlight_type.clone(),
                            })
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect();

            let base_style = Style::default().fg(Color::White);
            let highlighted_spans = self.apply_text_highlights(&line_text, &line_highlights, base_style);
            lines.push(Line::from(highlighted_spans));
            
            content_offset += line_text.len();
        }
        
        lines
    }

    /// Apply search highlights to text spans
    fn apply_text_highlights(
        &self,
        text: &str,
        highlights: &[MatchHighlight],
        base_style: Style,
    ) -> Vec<Span<'static>> {
        if highlights.is_empty() {
            return vec![Span::styled(text.to_owned(), base_style)];
        }

        let mut spans = Vec::new();
        let mut last_end = 0;

        // Use exact positions from highlights instead of re-searching
        // This prevents duplicate highlighting when multiple MatchHighlight objects
        // refer to the same search term
        let mut text_highlights = Vec::new();
        let mut processed_positions = std::collections::HashSet::new();
        
        for highlight in highlights {
            let start = highlight.start;
            let end = highlight.end;
            
            // Only add if this exact position hasn't been processed yet
            // This prevents duplicate highlights for the same position
            if !processed_positions.contains(&start) && start < text.len() && end <= text.len() {
                text_highlights.push((start, end, &highlight.matched_text, &highlight.highlight_type));
                processed_positions.insert(start);
            }
        }

        // Sort highlights by position
        text_highlights.sort_by_key(|(start, _, _, _)| *start);

        // Merge overlapping highlights
        let mut merged_highlights = Vec::new();
        for (start, end, matched_text, highlight_type) in text_highlights {
            if let Some((_last_start, last_end, _, last_type)) = merged_highlights.last_mut() {
                if start <= *last_end {
                    // Overlapping highlights - merge them
                    *last_end = end.max(*last_end);
                    // Keep the more specific highlight type (InConversationSearch over GlobalSearch)
                    if *highlight_type == HighlightType::InConversationSearch {
                        *last_type = highlight_type;
                    }
                } else {
                    merged_highlights.push((start, end, matched_text, highlight_type));
                }
            } else {
                merged_highlights.push((start, end, matched_text, highlight_type));
            }
        }

        // Build spans with highlights
        for (start, end, _, highlight_type) in merged_highlights {
            // Add text before highlight
            if start > last_end {
                spans.push(Span::styled(text[last_end..start].to_owned(), base_style));
            }

            // Add highlighted text with different styles based on highlight type
            let highlight_style = match highlight_type {
                HighlightType::GlobalSearch => Style::default()
                    .bg(Color::Yellow)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
                HighlightType::InConversationSearch => Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            };
            
            spans.push(Span::styled(text[start..end].to_owned(), highlight_style));

            last_end = end;
        }

        // Add remaining text
        if last_end < text.len() {
            spans.push(Span::styled(text[last_end..].to_owned(), base_style));
        }

        if spans.is_empty() {
            vec![Span::styled(text.to_owned(), base_style)]
        } else {
            spans
        }
    }
}

/// Convert syntect style to ratatui color
fn syntect_style_to_ratatui_color(style: SyntectStyle) -> Color {
    Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude::conversation::ToolUse;
    use chrono::Utc;

    fn create_test_message() -> ConversationMessage {
        ConversationMessage {
            uuid: "test-uuid".to_string(),
            parent_uuid: None,
            role: MessageRole::Assistant,
            content: "# Hello\n\nThis is a **test** message with `code` and:\n\n```rust\nfn main() {\n    println!(\"Hello, world!\");\n}\n```".to_string(),
            timestamp: Utc::now(),
            model: Some("claude-3-5-sonnet".to_string()),
            tool_uses: vec![],
        }
    }

    #[test]
    fn test_conversation_renderer_creation() {
        let renderer = ConversationRenderer::new(80);
        assert_eq!(renderer.terminal_width, 76); // 80 - 4 for borders
    }

    #[test]
    fn test_message_rendering() {
        let renderer = ConversationRenderer::new(80);
        let message = create_test_message();
        let rendered = renderer.render_message(&message);

        // Should have header, content, and separator
        assert!(!rendered.lines.is_empty());
    }

    #[test]
    fn test_width_update() {
        let mut renderer = ConversationRenderer::new(80);
        renderer.update_width(120);
        assert_eq!(renderer.terminal_width, 116); // 120 - 4 for borders
    }

    #[test]
    fn test_progressive_search_highlighting() {
        let renderer = ConversationRenderer::new(80);
        let content = "init is analyzing your codebasecodebasecodebase";
        
        // Simulate the exact bug scenario: multiple matches for "codebase"
        let highlights = vec![
            MatchHighlight {
                message_index: 0,
                start: 26, // Position of first "codebase" in the text
                end: 34,
                matched_text: "codebase".to_string(),
                highlight_type: HighlightType::InConversationSearch,
            },
            MatchHighlight {
                message_index: 0,
                start: 34, // Position of second "codebase"
                end: 42,
                matched_text: "codebase".to_string(),
                highlight_type: HighlightType::InConversationSearch,
            },
            MatchHighlight {
                message_index: 0,
                start: 42, // Position of third "codebase"
                end: 50,
                matched_text: "codebase".to_string(),
                highlight_type: HighlightType::InConversationSearch,
            },
        ];

        let base_style = Style::default();
        let spans = renderer.apply_text_highlights(content, &highlights, base_style);
        
        // Reconstruct the full text from spans
        let reconstructed_text: String = spans.iter().map(|span| span.content.as_ref()).collect();
        
        // The bug would cause "codebase" to appear multiple times
        // This test ensures the text is reconstructed correctly
        assert_eq!(reconstructed_text, content);
        
        // The most important check: verify no span contains repeated text like "codebasecodebase"
        for span in &spans {
            assert!(!span.content.contains("codebasecodebase"), 
                "Span should not contain duplicated text: {}", span.content);
        }
        
        // Additional check: the text should contain exactly 3 occurrences of "codebase"
        let codebase_count = reconstructed_text.matches("codebase").count();
        assert_eq!(codebase_count, 3, "Should have exactly 3 occurrences of 'codebase'");
    }

    #[test]
    fn test_duplicate_highlights_fix() {
        use crate::claude::search::{MatchHighlight, HighlightType};
        use ratatui::style::Style;
        
        let renderer = ConversationRenderer::new(80);
        
        // Test text with repeated words that was causing the bug
        let text = "The codebase contains many files, and the codebase is well organized. This codebase is great.";
        
        // Create multiple highlights for the same word (simulating the bug scenario)
        let highlights = vec![
            MatchHighlight {
                message_index: 0,
                start: 4,  // First "codebase" - but these positions are relative to the entire message
                end: 12,   // The function will find occurrences in the text line
                matched_text: "codebase".to_string(),
                highlight_type: HighlightType::InConversationSearch,
            },
            MatchHighlight {
                message_index: 0,
                start: 43, // Second "codebase"
                end: 51,
                matched_text: "codebase".to_string(),
                highlight_type: HighlightType::InConversationSearch,
            },
            MatchHighlight {
                message_index: 0,
                start: 77, // Third "codebase"
                end: 85,
                matched_text: "codebase".to_string(),
                highlight_type: HighlightType::InConversationSearch,
            },
        ];
        
        // Apply highlights
        let spans = renderer.apply_text_highlights(text, &highlights, Style::default());
        
        // Reconstruct the text from spans
        let reconstructed_text = spans.iter()
            .map(|span| span.content.as_ref())
            .collect::<Vec<_>>()
            .join("");
        
        // Verify the text is properly reconstructed (no duplication)
        assert_eq!(reconstructed_text, text, "Text should be reconstructed without duplication");
        
        // Count the number of highlighted spans (should be reasonable, not excessive)
        let highlight_spans = spans.iter()
            .filter(|span| span.style.bg.is_some())
            .count();
        
        // Should have exactly 3 highlighted spans (one for each unique occurrence)
        assert_eq!(highlight_spans, 3, "Should have exactly 3 highlighted spans for 3 occurrences");
        
        // Verify no individual span contains repeated text
        for span in spans.iter() {
            if span.content.contains("codebase") {
                let codebase_count = span.content.matches("codebase").count();
                assert_eq!(codebase_count, 1, "Each span should contain 'codebase' at most once, found {} times in '{}'", codebase_count, span.content);
            }
        }
    }
}
