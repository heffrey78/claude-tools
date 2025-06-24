use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClaudeToolsError {
    #[error("Claude directory not found: {path}")]
    DirectoryNotFound { path: String },

    #[error("Invalid Claude directory: {path} - {reason}")]
    InvalidDirectory { path: String, reason: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("General error: {0}")]
    General(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, ClaudeToolsError>;
