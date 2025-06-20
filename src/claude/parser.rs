use super::conversation::{Conversation, ConversationEntry};
use super::directory::ClaudeDirectory;
use crate::errors::ClaudeToolsError;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Parser for Claude conversation files
pub struct ConversationParser {
    claude_dir: ClaudeDirectory,
}

impl ConversationParser {
    /// Create a new conversation parser
    pub fn new(claude_dir: ClaudeDirectory) -> Self {
        Self { claude_dir }
    }

    /// Parse all conversations in the Claude directory
    pub fn parse_all_conversations(&self) -> Result<Vec<Conversation>, ClaudeToolsError> {
        let mut conversations = Vec::new();
        let projects_dir = self.claude_dir.path.join("projects");

        if !projects_dir.exists() {
            return Ok(conversations);
        }

        // Iterate through all project directories
        for entry in fs::read_dir(&projects_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let project_conversations = self.parse_project_conversations(&path)?;
                conversations.extend(project_conversations);
            }
        }

        Ok(conversations)
    }

    /// Parse all conversations in a specific project directory
    pub fn parse_project_conversations(
        &self,
        project_dir: &Path,
    ) -> Result<Vec<Conversation>, ClaudeToolsError> {
        let mut conversations = Vec::new();
        let project_name = project_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Find all .jsonl files in the project directory
        for entry in fs::read_dir(project_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                match self.parse_conversation_file(&path, &project_name) {
                    Ok(conversation) => conversations.push(conversation),
                    Err(e) => {
                        eprintln!("Warning: Failed to parse {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(conversations)
    }

    /// Parse a single conversation file
    pub fn parse_conversation_file(
        &self,
        file_path: &Path,
        project_name: &str,
    ) -> Result<Conversation, ClaudeToolsError> {
        let session_id = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| ClaudeToolsError::Config("Invalid file name".to_string()))?
            .to_string();

        let file = fs::File::open(file_path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        // Parse each line as a separate JSON object
        for (line_num, line) in reader.lines().enumerate() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<ConversationEntry>(&line) {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to parse line {} in {}: {}",
                        line_num + 1,
                        file_path.display(),
                        e
                    );
                }
            }
        }

        Ok(Conversation::from_entries(
            session_id,
            project_name.to_string(),
            entries,
        ))
    }

    /// Get conversations for a specific project
    pub fn get_project_conversations(
        &self,
        project_path: &str,
    ) -> Result<Vec<Conversation>, ClaudeToolsError> {
        let normalized_path = project_path.replace('/', "-");
        let project_dir = self.claude_dir.path.join("projects").join(&normalized_path);

        if !project_dir.exists() {
            return Ok(Vec::new());
        }

        self.parse_project_conversations(&project_dir)
    }

    /// Get a specific conversation by ID
    pub fn get_conversation(
        &self,
        session_id: &str,
    ) -> Result<Option<Conversation>, ClaudeToolsError> {
        let projects_dir = self.claude_dir.path.join("projects");

        if !projects_dir.exists() {
            return Ok(None);
        }

        // Search through all project directories
        for entry in fs::read_dir(&projects_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let conversation_file = path.join(format!("{}.jsonl", session_id));
                if conversation_file.exists() {
                    let project_name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown");
                    return Ok(Some(
                        self.parse_conversation_file(&conversation_file, project_name)?,
                    ));
                }
            }
        }

        Ok(None)
    }

    /// Search conversations by content
    pub fn search_conversations(&self, query: &str) -> Result<Vec<Conversation>, ClaudeToolsError> {
        let all_conversations = self.parse_all_conversations()?;
        let query_lower = query.to_lowercase();

        Ok(all_conversations
            .into_iter()
            .filter(|conv| {
                // Search in summary
                if let Some(ref summary) = conv.summary {
                    if summary.to_lowercase().contains(&query_lower) {
                        return true;
                    }
                }

                // Search in messages
                conv.messages
                    .iter()
                    .any(|msg| msg.content.to_lowercase().contains(&query_lower))
            })
            .collect())
    }

    /// Get conversation statistics
    pub fn get_stats(&self) -> Result<ConversationStats, ClaudeToolsError> {
        let conversations = self.parse_all_conversations()?;

        let total_conversations = conversations.len();
        let total_messages: usize = conversations.iter().map(|c| c.messages.len()).sum();
        let total_user_messages: usize = conversations.iter().map(|c| c.user_message_count()).sum();
        let total_assistant_messages: usize = conversations
            .iter()
            .map(|c| c.assistant_message_count())
            .sum();

        let mut projects: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for conv in &conversations {
            *projects.entry(conv.project_path.clone()).or_insert(0) += 1;
        }

        Ok(ConversationStats {
            total_conversations,
            total_messages,
            total_user_messages,
            total_assistant_messages,
            projects,
        })
    }
}

/// Statistics about conversations
#[derive(Debug)]
pub struct ConversationStats {
    pub total_conversations: usize,
    pub total_messages: usize,
    pub total_user_messages: usize,
    pub total_assistant_messages: usize,
    pub projects: std::collections::HashMap<String, usize>,
}
