use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Represents the current status of an MCP server
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServerStatus {
    /// Server is running and responding to health checks
    Running,
    /// Server is stopped
    Stopped,
    /// Server is starting up
    Starting,
    /// Server is stopping
    Stopping,
    /// Server has encountered an error
    Error(String),
    /// Server status is unknown
    Unknown,
}

impl ServerStatus {
    /// Returns true if the server is in a healthy running state
    pub fn is_healthy(&self) -> bool {
        matches!(self, ServerStatus::Running)
    }

    /// Returns true if the server is in a transitional state
    pub fn is_transitional(&self) -> bool {
        matches!(self, ServerStatus::Starting | ServerStatus::Stopping)
    }

    /// Returns an emoji representation of the status for UI display
    pub fn emoji(&self) -> &'static str {
        match self {
            ServerStatus::Running => "🟢",
            ServerStatus::Stopped => "🔴",
            ServerStatus::Starting => "🟡",
            ServerStatus::Stopping => "🟡",
            ServerStatus::Error(_) => "❌",
            ServerStatus::Unknown => "❓",
        }
    }

    /// Returns a color name for terminal UI
    pub fn color(&self) -> &'static str {
        match self {
            ServerStatus::Running => "green",
            ServerStatus::Stopped => "red",
            ServerStatus::Starting | ServerStatus::Stopping => "yellow",
            ServerStatus::Error(_) => "red",
            ServerStatus::Unknown => "gray",
        }
    }

    /// Returns a human-readable description of the status
    pub fn description(&self) -> String {
        match self {
            ServerStatus::Running => "Running".to_string(),
            ServerStatus::Stopped => "Stopped".to_string(),
            ServerStatus::Starting => "Starting".to_string(),
            ServerStatus::Stopping => "Stopping".to_string(),
            ServerStatus::Error(msg) => format!("Error: {}", msg),
            ServerStatus::Unknown => "Unknown".to_string(),
        }
    }
}

/// Represents a capability that an MCP server provides
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServerCapability {
    /// Server provides resources (file-like data)
    Resources,
    /// Server provides tools that can be called
    Tools,
    /// Server provides prompts/templates
    Prompts,
    /// Server supports custom capabilities
    Custom(String),
}

impl ServerCapability {
    /// Returns the name of the capability
    pub fn name(&self) -> &str {
        match self {
            ServerCapability::Resources => "Resources",
            ServerCapability::Tools => "Tools",
            ServerCapability::Prompts => "Prompts",
            ServerCapability::Custom(name) => name,
        }
    }

    /// Returns a human-readable description of the capability
    pub fn description(&self) -> &str {
        match self {
            ServerCapability::Resources => "Provides file-like resources",
            ServerCapability::Tools => "Provides callable tools",
            ServerCapability::Prompts => "Provides prompt templates",
            ServerCapability::Custom(name) => name,
        }
    }
}

/// Transport mechanism for communicating with the MCP server
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServerTransport {
    /// Standard I/O communication
    Stdio {
        command: String,
        args: Vec<String>,
        env: HashMap<String, String>,
    },
    /// HTTP/HTTPS communication
    Http {
        url: String,
        headers: HashMap<String, String>,
    },
    /// WebSocket communication
    WebSocket { url: String },
}

impl ServerTransport {
    /// Returns a human-readable description of the transport
    pub fn description(&self) -> String {
        match self {
            ServerTransport::Stdio { command, .. } => format!("Command: {}", command),
            ServerTransport::Http { url, .. } => format!("HTTP: {}", url),
            ServerTransport::WebSocket { url } => format!("WebSocket: {}", url),
        }
    }
}

/// Represents a complete MCP server with all its metadata and status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    /// Unique identifier for the server
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Server version
    pub version: Option<String>,
    /// Description of what the server does
    pub description: Option<String>,
    /// Current status of the server
    pub status: ServerStatus,
    /// Transport mechanism for communication
    pub transport: ServerTransport,
    /// Capabilities provided by the server
    pub capabilities: Vec<ServerCapability>,
    /// Path to the configuration file that defines this server
    pub config_path: PathBuf,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
    /// When the server was last health checked
    pub last_health_check: Option<DateTime<Utc>>,
    /// Server installation path (if applicable)
    pub installation_path: Option<PathBuf>,
    /// Health check endpoint URL (if applicable)
    pub health_check_url: Option<String>,
    /// Authentication configuration
    pub auth_config: Option<AuthConfig>,
}

/// Authentication configuration for MCP servers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Authentication type (oauth, apikey, basic, etc.)
    pub auth_type: String,
    /// OAuth configuration
    pub oauth: Option<OAuthConfig>,
    /// API key configuration
    pub api_key: Option<ApiKeyConfig>,
    /// Additional authentication parameters
    pub parameters: HashMap<String, String>,
}

/// OAuth configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    /// Client ID
    pub client_id: String,
    /// Authorization URL
    pub auth_url: String,
    /// Token URL
    pub token_url: String,
    /// Scopes
    pub scopes: Vec<String>,
}

/// API Key configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    /// Header name for the API key
    pub header_name: String,
    /// Key name for reference
    pub key_name: String,
}

impl McpServer {
    /// Creates a new MCP server instance
    pub fn new(id: String, name: String, transport: ServerTransport, config_path: PathBuf) -> Self {
        Self {
            id,
            name,
            version: None,
            description: None,
            status: ServerStatus::Unknown,
            transport,
            capabilities: Vec::new(),
            config_path,
            metadata: HashMap::new(),
            last_health_check: None,
            installation_path: None,
            health_check_url: None,
            auth_config: None,
        }
    }

    /// Returns a short identifier for display (first 8 characters of ID)
    pub fn short_id(&self) -> String {
        self.id.chars().take(8).collect()
    }

    /// Returns whether the server supports a specific capability
    pub fn has_capability(&self, capability: &ServerCapability) -> bool {
        self.capabilities.contains(capability)
    }

    /// Returns a summary line for the server suitable for list display
    pub fn summary(&self) -> String {
        let version = self.version.as_deref().unwrap_or("unknown");
        let status_emoji = self.status.emoji();
        format!(
            "{} {} - {} ({})",
            status_emoji,
            self.name,
            self.transport.description(),
            version
        )
    }

    /// Updates the server's health status
    pub fn update_health_status(&mut self, status: ServerStatus) {
        self.status = status;
        self.last_health_check = Some(Utc::now());
    }

    /// Returns whether the server configuration appears to be valid
    pub fn is_config_valid(&self) -> bool {
        !self.name.is_empty() && self.config_path.exists()
    }

    /// Returns a detailed description of the server for display
    pub fn detailed_info(&self) -> Vec<String> {
        let mut info = Vec::new();

        info.push(format!("ID: {}", self.id));
        info.push(format!("Name: {}", self.name));

        if let Some(version) = &self.version {
            info.push(format!("Version: {}", version));
        }

        if let Some(description) = &self.description {
            info.push(format!("Description: {}", description));
        }

        info.push(format!("Status: {:?}", self.status));
        info.push(format!("Transport: {}", self.transport.description()));
        info.push(format!("Config: {}", self.config_path.display()));

        if !self.capabilities.is_empty() {
            info.push("Capabilities:".to_string());
            for cap in &self.capabilities {
                info.push(format!("  - {}", cap.description()));
            }
        }

        if let Some(last_check) = self.last_health_check {
            info.push(format!(
                "Last Health Check: {}",
                last_check.format("%Y-%m-%d %H:%M:%S UTC")
            ));
        }

        info
    }
}

impl std::fmt::Display for McpServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}
