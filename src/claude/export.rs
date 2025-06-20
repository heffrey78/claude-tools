use super::conversation::{Conversation, MessageRole};
use crate::errors::ClaudeToolsError;
use chrono::Utc;
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::PathBuf;

/// Export format types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    Markdown,
    Html,
    Pdf,
    Json,
}

/// Export configuration options
#[derive(Debug, Clone)]
pub struct ExportConfig {
    /// Output file path
    pub output_path: PathBuf,
    /// Export format
    pub format: ExportFormat,
    /// Include conversation metadata
    pub include_metadata: bool,
    /// Include tool usage information
    pub include_tool_usage: bool,
    /// Include timestamps
    pub include_timestamps: bool,
    /// Custom template path (optional)
    pub template_path: Option<PathBuf>,
    /// Title for the export
    pub title: Option<String>,
}

/// Main export engine
pub struct ConversationExporter {
    config: ExportConfig,
}

/// Export result with metadata
#[derive(Debug)]
pub struct ExportResult {
    /// Path to the exported file
    pub file_path: PathBuf,
    /// Size of the exported file in bytes
    pub file_size: u64,
    /// Number of conversations exported
    pub conversation_count: usize,
    /// Number of messages exported
    pub message_count: usize,
    /// Export duration in milliseconds
    pub duration_ms: u128,
}

impl ConversationExporter {
    /// Create a new exporter with configuration
    pub fn new(config: ExportConfig) -> Self {
        Self { config }
    }

    /// Export a single conversation
    pub fn export_conversation(&self, conversation: &Conversation) -> Result<ExportResult, ClaudeToolsError> {
        let start_time = std::time::Instant::now();
        
        let content = match self.config.format {
            ExportFormat::Markdown => self.generate_markdown(conversation)?,
            ExportFormat::Html => self.generate_html(conversation)?,
            ExportFormat::Pdf => self.generate_pdf(conversation)?,
            ExportFormat::Json => self.generate_json(conversation)?,
        };

        // Write to file
        fs::write(&self.config.output_path, content)?;
        
        // Get file metadata
        let metadata = fs::metadata(&self.config.output_path)?;
        let duration = start_time.elapsed();

        Ok(ExportResult {
            file_path: self.config.output_path.clone(),
            file_size: metadata.len(),
            conversation_count: 1,
            message_count: conversation.messages.len(),
            duration_ms: duration.as_millis(),
        })
    }

    /// Export multiple conversations as an archive
    pub fn export_conversations(&self, conversations: &[Conversation]) -> Result<ExportResult, ClaudeToolsError> {
        let start_time = std::time::Instant::now();
        
        match self.config.format {
            ExportFormat::Json => {
                // Export as JSON array
                let content = serde_json::to_string_pretty(conversations)?;
                fs::write(&self.config.output_path, content)?;
            },
            _ => {
                // Create ZIP archive with individual files
                return self.export_as_archive(conversations);
            }
        }

        let metadata = fs::metadata(&self.config.output_path)?;
        let duration = start_time.elapsed();
        let total_messages: usize = conversations.iter().map(|c| c.messages.len()).sum();

        Ok(ExportResult {
            file_path: self.config.output_path.clone(),
            file_size: metadata.len(),
            conversation_count: conversations.len(),
            message_count: total_messages,
            duration_ms: duration.as_millis(),
        })
    }

    /// Generate markdown content for a conversation
    fn generate_markdown(&self, conversation: &Conversation) -> Result<String, ClaudeToolsError> {
        let mut content = String::new();

        // Title and metadata
        if let Some(title) = &self.config.title {
            content.push_str(&format!("# {}\n\n", title));
        } else {
            content.push_str(&format!("# Conversation: {}\n\n", conversation.session_id));
        }

        if self.config.include_metadata {
            content.push_str("## Conversation Details\n\n");
            content.push_str(&format!("**Session ID:** `{}`\n", conversation.session_id));
            content.push_str(&format!("**Project:** `{}`\n", conversation.project_path));
            
            if let Some(summary) = &conversation.summary {
                content.push_str(&format!("**Summary:** {}\n", summary));
            }
            
            if let Some(started) = conversation.started_at {
                content.push_str(&format!("**Started:** {}\n", started.format("%Y-%m-%d %H:%M:%S UTC")));
            }
            
            if let Some(updated) = conversation.last_updated {
                content.push_str(&format!("**Last Updated:** {}\n", updated.format("%Y-%m-%d %H:%M:%S UTC")));
            }
            
            content.push_str(&format!("**Messages:** {}\n\n", conversation.messages.len()));
        }

        // Table of contents
        content.push_str("## Messages\n\n");

        // Messages
        for (index, message) in conversation.messages.iter().enumerate() {
            let role_icon = match message.role {
                MessageRole::User => "👤",
                MessageRole::Assistant => "🤖",
                MessageRole::System => "⚙️",
            };

            let role_name = match message.role {
                MessageRole::User => "User",
                MessageRole::Assistant => "Assistant", 
                MessageRole::System => "System",
            };

            content.push_str(&format!("### {} {} Message {}\n\n", role_icon, role_name, index + 1));

            if self.config.include_timestamps {
                content.push_str(&format!("**Timestamp:** {}\n", message.timestamp.format("%Y-%m-%d %H:%M:%S UTC")));
            }

            if let Some(model) = &message.model {
                content.push_str(&format!("**Model:** `{}`\n", model));
            }

            content.push_str("\n");
            content.push_str(&message.content);
            content.push_str("\n\n");

            // Tool usage
            if self.config.include_tool_usage && !message.tool_uses.is_empty() {
                content.push_str("**Tool Usage:**\n\n");
                for tool in &message.tool_uses {
                    content.push_str(&format!("- **{}** (ID: `{}`)\n", tool.name, tool.id));
                }
                content.push_str("\n");
            }

            content.push_str("---\n\n");
        }

        // Footer
        content.push_str(&format!("\n*Exported on {} UTC*\n", Utc::now().format("%Y-%m-%d %H:%M:%S")));

        Ok(content)
    }

    /// Generate HTML content for a conversation
    fn generate_html(&self, conversation: &Conversation) -> Result<String, ClaudeToolsError> {
        let default_title = format!("Conversation: {}", conversation.session_id);
        let title = self.config.title.as_ref().unwrap_or(&default_title);

        let mut content = String::new();

        // HTML header with CSS
        content.push_str(&format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 800px;
            margin: 0 auto;
            padding: 20px;
            background-color: #fafafa;
        }}
        .header {{
            background: #fff;
            padding: 20px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            margin-bottom: 20px;
        }}
        .message {{
            background: #fff;
            margin: 15px 0;
            padding: 15px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }}
        .message.user {{
            border-left: 4px solid #007bff;
        }}
        .message.assistant {{
            border-left: 4px solid #28a745;
        }}
        .message.system {{
            border-left: 4px solid #ffc107;
        }}
        .message-header {{
            display: flex;
            align-items: center;
            margin-bottom: 10px;
            font-weight: bold;
        }}
        .role-icon {{
            margin-right: 8px;
            font-size: 1.2em;
        }}
        .timestamp {{
            margin-left: auto;
            font-size: 0.9em;
            color: #666;
            font-weight: normal;
        }}
        .model {{
            background: #e9ecef;
            padding: 2px 6px;
            border-radius: 3px;
            font-size: 0.8em;
            margin-left: 10px;
        }}
        .content {{
            white-space: pre-wrap;
            font-family: 'SF Mono', Monaco, Consolas, monospace;
        }}
        .tools {{
            margin-top: 10px;
            padding: 10px;
            background: #f8f9fa;
            border-radius: 4px;
        }}
        .tool {{
            font-size: 0.9em;
            color: #495057;
        }}
        .metadata {{
            background: #e3f2fd;
            padding: 15px;
            border-radius: 5px;
            margin-bottom: 20px;
        }}
        .metadata dt {{
            font-weight: bold;
            margin-top: 5px;
        }}
        .metadata dd {{
            margin: 0 0 5px 20px;
        }}
        pre {{
            background: #f8f9fa;
            padding: 10px;
            border-radius: 4px;
            overflow-x: auto;
        }}
        code {{
            background: #f8f9fa;
            padding: 2px 4px;
            border-radius: 3px;
            font-family: 'SF Mono', Monaco, Consolas, monospace;
        }}
        .footer {{
            text-align: center;
            color: #666;
            font-size: 0.9em;
            margin-top: 30px;
            padding: 20px;
        }}
    </style>
</head>
<body>
"#, title));

        // Header section
        content.push_str(&format!(r#"
    <div class="header">
        <h1>{}</h1>
"#, title));

        if self.config.include_metadata {
            content.push_str(r#"        <div class="metadata">
            <dl>"#);
            
            content.push_str(&format!(r#"
                <dt>Session ID:</dt>
                <dd><code>{}</code></dd>
                <dt>Project:</dt>
                <dd><code>{}</code></dd>"#, 
                conversation.session_id, conversation.project_path));

            if let Some(summary) = &conversation.summary {
                content.push_str(&format!(r#"
                <dt>Summary:</dt>
                <dd>{}</dd>"#, html_escape(summary)));
            }

            if let Some(started) = conversation.started_at {
                content.push_str(&format!(r#"
                <dt>Started:</dt>
                <dd>{}</dd>"#, started.format("%Y-%m-%d %H:%M:%S UTC")));
            }

            content.push_str(&format!(r#"
                <dt>Messages:</dt>
                <dd>{}</dd>
            </dl>
        </div>"#, conversation.messages.len()));
        }

        content.push_str("    </div>\n");

        // Messages
        for message in &conversation.messages {
            let (role_class, role_icon, role_name) = match message.role {
                MessageRole::User => ("user", "👤", "User"),
                MessageRole::Assistant => ("assistant", "🤖", "Assistant"),
                MessageRole::System => ("system", "⚙️", "System"),
            };

            content.push_str(&format!(r#"
    <div class="message {}">
        <div class="message-header">
            <span class="role-icon">{}</span>
            <span>{}</span>"#, role_class, role_icon, role_name));

            if let Some(model) = &message.model {
                content.push_str(&format!(r#"
            <span class="model">{}</span>"#, html_escape(model)));
            }

            if self.config.include_timestamps {
                content.push_str(&format!(r#"
            <span class="timestamp">{}</span>"#, message.timestamp.format("%Y-%m-%d %H:%M:%S UTC")));
            }

            content.push_str(r#"
        </div>
        <div class="content">"#);
            
            content.push_str(&html_escape(&message.content));
            content.push_str("</div>");

            // Tool usage
            if self.config.include_tool_usage && !message.tool_uses.is_empty() {
                content.push_str(r#"
        <div class="tools">
            <strong>Tools Used:</strong><br>"#);
                for tool in &message.tool_uses {
                    content.push_str(&format!(r#"
            <div class="tool">• {} (ID: {})</div>"#, 
                html_escape(&tool.name), html_escape(&tool.id)));
                }
                content.push_str("\n        </div>");
            }

            content.push_str("\n    </div>");
        }

        // Footer
        content.push_str(&format!(r#"
    <div class="footer">
        <p>Exported on {} UTC</p>
    </div>
</body>
</html>"#, Utc::now().format("%Y-%m-%d %H:%M:%S")));

        Ok(content)
    }

    /// Generate PDF content (placeholder - will use external tool)
    fn generate_pdf(&self, _conversation: &Conversation) -> Result<String, ClaudeToolsError> {
        // For now, generate HTML and note that PDF conversion needs external tool
        // In a real implementation, we'd use wkhtmltopdf or similar
        Err(ClaudeToolsError::Config("PDF export not yet implemented - use HTML export and convert externally".to_string()))
    }

    /// Generate JSON content for a conversation
    fn generate_json(&self, conversation: &Conversation) -> Result<String, ClaudeToolsError> {
        Ok(serde_json::to_string_pretty(conversation)?)
    }

    /// Export multiple conversations as ZIP archive
    fn export_as_archive(&self, _conversations: &[Conversation]) -> Result<ExportResult, ClaudeToolsError> {
        // Placeholder for ZIP archive functionality
        // Would need zip crate dependency
        Err(ClaudeToolsError::Config("ZIP archive export not yet implemented".to_string()))
    }
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            output_path: PathBuf::from("conversation_export.md"),
            format: ExportFormat::Markdown,
            include_metadata: true,
            include_tool_usage: true,
            include_timestamps: true,
            template_path: None,
            title: None,
        }
    }
}

/// HTML escape utility function
fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_conversation() -> Conversation {
        Conversation {
            session_id: "test-123".to_string(),
            project_path: "/test/project".to_string(),
            summary: Some("Test conversation".to_string()),
            messages: vec![
                ConversationMessage {
                    role: MessageRole::User,
                    content: "Hello, how are you?".to_string(),
                    timestamp: Utc::now(),
                    model: None,
                    tool_uses: vec![],
                },
                ConversationMessage {
                    role: MessageRole::Assistant,
                    content: "I'm doing well, thank you!".to_string(),
                    timestamp: Utc::now(),
                    model: Some("claude-3".to_string()),
                    tool_uses: vec![],
                },
            ],
            started_at: Some(Utc::now()),
            last_updated: Some(Utc::now()),
        }
    }

    #[test]
    fn test_markdown_generation() {
        let conversation = create_test_conversation();
        let config = ExportConfig::default();
        let exporter = ConversationExporter::new(config);
        
        let result = exporter.generate_markdown(&conversation);
        assert!(result.is_ok());
        
        let content = result.unwrap();
        assert!(content.contains("# Conversation: test-123"));
        assert!(content.contains("Hello, how are you?"));
        assert!(content.contains("I'm doing well, thank you!"));
    }

    #[test]
    fn test_html_generation() {
        let conversation = create_test_conversation();
        let config = ExportConfig::default();
        let exporter = ConversationExporter::new(config);
        
        let result = exporter.generate_html(&conversation);
        assert!(result.is_ok());
        
        let content = result.unwrap();
        assert!(content.contains("<!DOCTYPE html"));
        assert!(content.contains("Hello, how are you?"));
        assert!(content.contains("I'm doing well, thank you!"));
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("<script>alert('xss')</script>"), "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;");
        assert_eq!(html_escape("AT&T"), "AT&amp;T");
    }
}