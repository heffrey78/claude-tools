use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Claude Code's ~/.claude.json configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeConfig {
    /// Global MCP servers available to all projects
    #[serde(default)]
    pub mcp_servers: HashMap<String, ClaudeMcpServer>,

    /// Project-specific configurations
    #[serde(default)]
    pub projects: HashMap<String, ProjectConfig>,

    /// User ID
    pub user_id: Option<String>,

    /// OAuth account information
    pub oauth_account: Option<Value>,

    /// Subscription status
    pub has_available_subscription: Option<bool>,

    /// Onboarding state
    pub has_completed_onboarding: Option<bool>,

    /// Number of startups
    pub num_startups: Option<u32>,

    /// Tips history
    pub tips_history: Option<HashMap<String, u32>>,

    /// Other fields preserved as raw JSON
    #[serde(flatten)]
    pub other: HashMap<String, Value>,
}

/// MCP server configuration in Claude format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeMcpServer {
    /// Server type (usually "stdio")
    #[serde(rename = "type")]
    pub server_type: String,

    /// Command to execute
    pub command: String,

    /// Command arguments
    #[serde(default)]
    pub args: Vec<String>,

    /// Environment variables (may contain API keys)
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// Project-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfig {
    /// Project-specific MCP servers
    #[serde(default)]
    pub mcp_servers: HashMap<String, ClaudeMcpServer>,

    /// Enabled MCP JSON servers
    #[serde(default)]
    pub enabled_mcpjson_servers: Vec<String>,

    /// Disabled MCP JSON servers
    #[serde(default)]
    pub disabled_mcpjson_servers: Vec<String>,

    /// MCP context URIs
    #[serde(default)]
    pub mcp_context_uris: Vec<String>,

    /// Allowed tools
    #[serde(default)]
    pub allowed_tools: Vec<String>,

    /// Don't crawl directory flag
    #[serde(default)]
    pub dont_crawl_directory: bool,

    /// Trust dialog accepted
    #[serde(default)]
    pub has_trust_dialog_accepted: bool,

    /// Ignore patterns
    #[serde(default)]
    pub ignore_patterns: Vec<String>,

    /// Conversation history
    #[serde(default)]
    pub history: Vec<Value>,

    /// Session metadata
    pub last_session_id: Option<String>,
    pub last_duration: Option<i64>,
    pub last_api_duration: Option<i64>,
    pub last_cost: Option<f64>,
    pub last_total_input_tokens: Option<i64>,
    pub last_total_output_tokens: Option<i64>,

    /// Other project fields
    #[serde(flatten)]
    pub other: HashMap<String, Value>,
}

impl ClaudeConfig {
    /// Load configuration from ~/.claude.json
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;
        Self::load_from_path(&config_path)
    }

    /// Load configuration from a specific path
    pub fn load_from_path(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;

        let config: Self = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse {}", path.display()))?;

        Ok(config)
    }

    /// Save configuration to ~/.claude.json
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        self.save_to_path(&config_path)
    }

    /// Save configuration to a specific path
    pub fn save_to_path(&self, path: &Path) -> Result<()> {
        // Create backup first
        self.create_backup(path)?;

        // Serialize with pretty printing
        let json =
            serde_json::to_string_pretty(self).context("Failed to serialize configuration")?;

        // Write atomically by writing to temp file first
        let temp_path = path.with_extension("tmp");
        fs::write(&temp_path, &json).context("Failed to write temporary file")?;

        // Move temp file to final location
        fs::rename(&temp_path, path).context("Failed to save configuration")?;

        Ok(())
    }

    /// Create a backup of the configuration file
    fn create_backup(&self, path: &Path) -> Result<()> {
        if path.exists() {
            let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
            let backup_name = format!(".claude.json.backup.{}", timestamp);
            let backup_path = path.parent().unwrap_or(Path::new(".")).join(backup_name);

            fs::copy(path, &backup_path)
                .with_context(|| format!("Failed to create backup at {}", backup_path.display()))?;
        }
        Ok(())
    }

    /// Get the default configuration path
    fn config_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Failed to determine home directory")?;
        Ok(home.join(".claude.json"))
    }

    /// Get all MCP servers (global + project-specific)
    pub fn get_all_servers(&self, project_path: Option<&str>) -> HashMap<String, &ClaudeMcpServer> {
        let mut servers = HashMap::new();

        // Add global servers
        for (name, server) in &self.mcp_servers {
            servers.insert(format!("global:{}", name), server);
        }

        // Add project-specific servers if project path is provided
        if let Some(path) = project_path {
            if let Some(project) = self.projects.get(path) {
                for (name, server) in &project.mcp_servers {
                    servers.insert(format!("project:{}", name), server);
                }
            }
        }

        servers
    }

    /// Add a global MCP server
    pub fn add_global_server(&mut self, name: String, server: ClaudeMcpServer) {
        self.mcp_servers.insert(name, server);
    }

    /// Add a project-specific MCP server
    pub fn add_project_server(
        &mut self,
        project_path: &str,
        name: String,
        server: ClaudeMcpServer,
    ) {
        let project = self
            .projects
            .entry(project_path.to_string())
            .or_insert_with(|| ProjectConfig::default());
        project.mcp_servers.insert(name, server);
    }

    /// Remove a global MCP server
    pub fn remove_global_server(&mut self, name: &str) -> Option<ClaudeMcpServer> {
        self.mcp_servers.remove(name)
    }

    /// Remove a project-specific MCP server
    pub fn remove_project_server(
        &mut self,
        project_path: &str,
        name: &str,
    ) -> Option<ClaudeMcpServer> {
        self.projects
            .get_mut(project_path)
            .and_then(|project| project.mcp_servers.remove(name))
    }

    /// Update server environment variables (with masking for display)
    pub fn update_server_env(
        &mut self,
        server_name: &str,
        env_key: &str,
        env_value: &str,
        is_global: bool,
        project_path: Option<&str>,
    ) -> Result<()> {
        let server = if is_global {
            self.mcp_servers
                .get_mut(server_name)
                .context("Server not found in global configuration")?
        } else {
            let path = project_path.context("Project path required for project-specific server")?;
            self.projects
                .get_mut(path)
                .and_then(|p| p.mcp_servers.get_mut(server_name))
                .context("Server not found in project configuration")?
        };

        server
            .env
            .insert(env_key.to_string(), env_value.to_string());
        Ok(())
    }
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            mcp_servers: HashMap::new(),
            enabled_mcpjson_servers: Vec::new(),
            disabled_mcpjson_servers: Vec::new(),
            mcp_context_uris: Vec::new(),
            allowed_tools: Vec::new(),
            dont_crawl_directory: false,
            has_trust_dialog_accepted: false,
            ignore_patterns: Vec::new(),
            history: Vec::new(),
            last_session_id: None,
            last_duration: None,
            last_api_duration: None,
            last_cost: None,
            last_total_input_tokens: None,
            last_total_output_tokens: None,
            other: HashMap::new(),
        }
    }
}

/// Display helper for MCP servers with masked API keys
pub struct MaskedServerDisplay<'a> {
    pub name: &'a str,
    pub server: &'a ClaudeMcpServer,
}

impl<'a> MaskedServerDisplay<'a> {
    pub fn new(name: &'a str, server: &'a ClaudeMcpServer) -> Self {
        Self { name, server }
    }

    /// Display server configuration with masked sensitive data
    pub fn display(&self) -> String {
        let mut parts = vec![
            format!("Name: {}", self.name),
            format!("Type: {}", self.server.server_type),
            format!("Command: {}", self.server.command),
        ];

        if !self.server.args.is_empty() {
            parts.push(format!("Args: {}", self.server.args.join(" ")));
        }

        if !self.server.env.is_empty() {
            let masked_env: Vec<String> = self
                .server
                .env
                .iter()
                .map(|(k, v)| {
                    if k.to_lowercase().contains("key") || k.to_lowercase().contains("token") {
                        format!("{}=***", k)
                    } else {
                        format!("{}={}", k, v)
                    }
                })
                .collect();
            parts.push(format!("Environment: {}", masked_env.join(", ")));
        }

        parts.join("\n  ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_mcp_server_serialization() {
        let server = ClaudeMcpServer {
            server_type: "stdio".to_string(),
            command: "npx".to_string(),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-brave-search".to_string(),
            ],
            env: HashMap::from([("BRAVE_API_KEY".to_string(), "test-key".to_string())]),
        };

        let json = serde_json::to_string_pretty(&server).unwrap();
        assert!(json.contains("\"type\": \"stdio\""));
        assert!(json.contains("\"command\": \"npx\""));

        let deserialized: ClaudeMcpServer = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.server_type, "stdio");
        assert_eq!(deserialized.command, "npx");
        assert_eq!(deserialized.args.len(), 2);
    }

    #[test]
    fn test_masked_display() {
        let server = ClaudeMcpServer {
            server_type: "stdio".to_string(),
            command: "npx".to_string(),
            args: vec![],
            env: HashMap::from([
                ("BRAVE_API_KEY".to_string(), "secret-key".to_string()),
                ("DEBUG".to_string(), "true".to_string()),
            ]),
        };

        let display = MaskedServerDisplay::new("test-server", &server);
        let output = display.display();

        assert!(output.contains("BRAVE_API_KEY=***"));
        assert!(output.contains("DEBUG=true"));
        assert!(!output.contains("secret-key"));
    }
}
