use crate::mcp::server::{AuthConfig, ServerCapability, ServerTransport};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Configuration for a single MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server name
    pub name: String,
    /// Server type/transport
    #[serde(rename = "type")]
    pub transport_type: String,
    /// Command to run (for stdio transport)
    pub command: Option<String>,
    /// Command arguments
    pub args: Option<Vec<String>>,
    /// Environment variables
    pub env: Option<HashMap<String, String>>,
    /// URL (for http/websocket transport)
    pub url: Option<String>,
    /// HTTP headers (for http transport)
    pub headers: Option<HashMap<String, String>>,
    /// Server description
    pub description: Option<String>,
    /// Server version
    pub version: Option<String>,
    /// Capabilities
    pub capabilities: Option<Vec<String>>,
    /// Health check configuration
    pub health_check: Option<HealthCheckConfig>,
    /// Authentication configuration
    pub auth: Option<AuthConfig>,
    /// Additional metadata
    pub metadata: Option<HashMap<String, String>>,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Health check URL or endpoint
    pub url: Option<String>,
    /// Health check interval in seconds
    pub interval: Option<u64>,
    /// Health check timeout in seconds
    pub timeout: Option<u64>,
    /// Expected response status code
    pub expected_status: Option<u16>,
    /// Expected response body pattern
    pub expected_body: Option<String>,
}

/// Root MCP configuration containing multiple servers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    /// MCP configuration version
    pub version: Option<String>,
    /// Server configurations
    pub servers: HashMap<String, ServerConfig>,
    /// Global settings
    pub settings: Option<GlobalSettings>,
}

/// Global MCP settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalSettings {
    /// Default health check interval
    pub default_health_check_interval: Option<u64>,
    /// Auto-discovery enabled
    pub auto_discovery: Option<bool>,
    /// Discovery paths
    pub discovery_paths: Option<Vec<PathBuf>>,
    /// Logging configuration
    pub logging: Option<LoggingConfig>,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: Option<String>,
    /// Log file path
    pub file: Option<PathBuf>,
    /// Log rotation settings
    pub rotation: Option<LogRotationConfig>,
}

/// Log rotation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRotationConfig {
    /// Maximum log file size in MB
    pub max_size_mb: Option<u64>,
    /// Maximum number of log files to keep
    pub max_files: Option<u32>,
}

impl ServerConfig {
    /// Converts the configuration to a ServerTransport
    pub fn to_transport(&self) -> Result<ServerTransport, String> {
        match self.transport_type.as_str() {
            "stdio" => {
                let command = self
                    .command
                    .as_ref()
                    .ok_or("stdio transport requires 'command' field")?;

                Ok(ServerTransport::Stdio {
                    command: command.clone(),
                    args: self.args.clone().unwrap_or_default(),
                    env: self.env.clone().unwrap_or_default(),
                })
            }
            "http" | "https" => {
                let url = self
                    .url
                    .as_ref()
                    .ok_or("http transport requires 'url' field")?;

                Ok(ServerTransport::Http {
                    url: url.clone(),
                    headers: self.headers.clone().unwrap_or_default(),
                })
            }
            "websocket" | "ws" | "wss" => {
                let url = self
                    .url
                    .as_ref()
                    .ok_or("websocket transport requires 'url' field")?;

                Ok(ServerTransport::WebSocket { url: url.clone() })
            }
            _ => Err(format!(
                "Unsupported transport type: {}",
                self.transport_type
            )),
        }
    }

    /// Converts capability strings to ServerCapability enum
    pub fn to_capabilities(&self) -> Vec<ServerCapability> {
        self.capabilities
            .as_ref()
            .map(|caps| {
                caps.iter()
                    .map(|cap| match cap.as_str() {
                        "resources" => ServerCapability::Resources,
                        "tools" => ServerCapability::Tools,
                        "prompts" => ServerCapability::Prompts,
                        custom => ServerCapability::Custom(custom.to_string()),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Validates the server configuration
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.name.is_empty() {
            errors.push("Server name cannot be empty".to_string());
        }

        match self.transport_type.as_str() {
            "stdio" => {
                if self.command.is_none() {
                    errors.push("stdio transport requires 'command' field".to_string());
                }
            }
            "http" | "https" | "websocket" | "ws" | "wss" => {
                if self.url.is_none() {
                    errors.push(format!(
                        "{} transport requires 'url' field",
                        self.transport_type
                    ));
                }
            }
            _ => {
                errors.push(format!(
                    "Unsupported transport type: {}",
                    self.transport_type
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl McpConfig {
    /// Creates a new empty MCP configuration
    pub fn new() -> Self {
        Self {
            version: Some("1.0".to_string()),
            servers: HashMap::new(),
            settings: None,
        }
    }

    /// Validates the entire configuration
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut all_errors = Vec::new();

        for (server_id, server_config) in &self.servers {
            if let Err(errors) = server_config.validate() {
                for error in errors {
                    all_errors.push(format!("Server '{}': {}", server_id, error));
                }
            }
        }

        if all_errors.is_empty() {
            Ok(())
        } else {
            Err(all_errors)
        }
    }

    /// Adds a server configuration
    pub fn add_server(&mut self, id: String, config: ServerConfig) {
        self.servers.insert(id, config);
    }

    /// Removes a server configuration
    pub fn remove_server(&mut self, id: &str) -> Option<ServerConfig> {
        self.servers.remove(id)
    }

    /// Gets a server configuration by ID
    pub fn get_server(&self, id: &str) -> Option<&ServerConfig> {
        self.servers.get(id)
    }

    /// Returns all server IDs
    pub fn server_ids(&self) -> Vec<String> {
        self.servers.keys().cloned().collect()
    }
}

impl Default for McpConfig {
    fn default() -> Self {
        Self::new()
    }
}
