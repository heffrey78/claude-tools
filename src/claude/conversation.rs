use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a single conversation session from Claude Code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    /// Session ID (usually a UUID)
    pub session_id: String,
    /// Project path where the conversation took place
    pub project_path: String,
    /// Summary of the conversation (if available)
    pub summary: Option<String>,
    /// All messages in the conversation
    pub messages: Vec<ConversationMessage>,
    /// Timestamp of the first message
    pub started_at: Option<DateTime<Utc>>,
    /// Timestamp of the last message
    pub last_updated: Option<DateTime<Utc>>,
}

/// Types of entries in a conversation JSONL file
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ConversationEntry {
    Summary {
        summary: String,
        #[serde(rename = "leafUuid")]
        leaf_uuid: String,
    },
    User {
        #[serde(rename = "parentUuid")]
        parent_uuid: Option<String>,
        #[serde(rename = "sessionId")]
        session_id: String,
        message: Message,
        uuid: String,
        timestamp: DateTime<Utc>,
        cwd: Option<String>,
        version: Option<String>,
        #[serde(rename = "isSidechain")]
        is_sidechain: Option<bool>,
        #[serde(rename = "userType")]
        user_type: Option<String>,
        #[serde(rename = "isMeta")]
        is_meta: Option<bool>,
    },
    Assistant {
        #[serde(rename = "parentUuid")]
        parent_uuid: Option<String>,
        #[serde(rename = "sessionId")]
        session_id: String,
        message: AssistantMessage,
        uuid: String,
        timestamp: DateTime<Utc>,
        cwd: Option<String>,
        version: Option<String>,
        #[serde(rename = "requestId")]
        request_id: Option<String>,
    },
}

/// A message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: MessageContent,
}

/// Assistant-specific message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub id: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub role: String,
    pub model: Option<String>,
    pub content: Vec<AssistantContent>,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: Option<Usage>,
}

/// Content can be either a string or an array of content blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

/// A content block in a message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text { text: String },
    Image { source: ImageSource },
}

/// Assistant content blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AssistantContent {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

/// Image source information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub media_type: Option<String>,
    pub data: Option<String>,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub cache_creation_input_tokens: Option<u32>,
    pub cache_read_input_tokens: Option<u32>,
    pub output_tokens: u32,
    pub service_tier: Option<String>,
}

/// Processed message for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub uuid: String,
    pub parent_uuid: Option<String>,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub model: Option<String>,
    pub tool_uses: Vec<ToolUse>,
}

/// Role of the message sender
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// Tool use information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUse {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
}

impl Conversation {
    /// Create a new conversation from a vector of entries
    pub fn from_entries(
        session_id: String,
        project_path: String,
        entries: Vec<ConversationEntry>,
    ) -> Self {
        let mut messages = Vec::new();
        let mut summary = None;
        let mut started_at = None;
        let mut last_updated = None;

        for entry in entries {
            match entry {
                ConversationEntry::Summary { summary: s, .. } => {
                    summary = Some(s);
                }
                ConversationEntry::User {
                    uuid,
                    parent_uuid,
                    message,
                    timestamp,
                    ..
                } => {
                    if started_at.is_none() {
                        started_at = Some(timestamp);
                    }
                    last_updated = Some(timestamp);

                    let content = match message.content {
                        MessageContent::Text(text) => text,
                        MessageContent::Blocks(blocks) => blocks
                            .into_iter()
                            .filter_map(|block| match block {
                                ContentBlock::Text { text } => Some(text),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join("\n"),
                    };

                    messages.push(ConversationMessage {
                        uuid,
                        parent_uuid,
                        role: MessageRole::User,
                        content,
                        timestamp,
                        model: None,
                        tool_uses: vec![],
                    });
                }
                ConversationEntry::Assistant {
                    uuid,
                    parent_uuid,
                    message,
                    timestamp,
                    ..
                } => {
                    last_updated = Some(timestamp);

                    let mut content = String::new();
                    let mut tool_uses = Vec::new();

                    for block in message.content {
                        match block {
                            AssistantContent::Text { text } => {
                                if !content.is_empty() {
                                    content.push('\n');
                                }
                                content.push_str(&text);
                            }
                            AssistantContent::ToolUse { id, name, input } => {
                                tool_uses.push(ToolUse { id, name, input });
                            }
                        }
                    }

                    messages.push(ConversationMessage {
                        uuid,
                        parent_uuid,
                        role: MessageRole::Assistant,
                        content,
                        timestamp,
                        model: message.model,
                        tool_uses,
                    });
                }
            }
        }

        Conversation {
            session_id,
            project_path,
            summary,
            messages,
            started_at,
            last_updated,
        }
    }

    /// Get the duration of the conversation
    pub fn duration(&self) -> Option<chrono::Duration> {
        match (self.started_at, self.last_updated) {
            (Some(start), Some(end)) => Some(end - start),
            _ => None,
        }
    }

    /// Get the number of user messages
    pub fn user_message_count(&self) -> usize {
        self.messages
            .iter()
            .filter(|m| m.role == MessageRole::User)
            .count()
    }

    /// Get the number of assistant messages
    pub fn assistant_message_count(&self) -> usize {
        self.messages
            .iter()
            .filter(|m| m.role == MessageRole::Assistant)
            .count()
    }
}
