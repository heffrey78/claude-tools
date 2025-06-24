use crate::errors::ClaudeToolsError;
use crate::mcp::config::{McpConfig, ServerConfig};
use crate::mcp::server::{McpServer, ServerStatus};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Result of server discovery operation
#[derive(Debug, Clone)]
pub struct DiscoveryResult {
    /// Successfully discovered servers
    pub servers: Vec<McpServer>,
    /// Paths that were scanned
    pub scanned_paths: Vec<PathBuf>,
    /// Errors encountered during discovery
    pub errors: Vec<DiscoveryError>,
    /// Total scan duration
    pub scan_duration: std::time::Duration,
}

/// Error encountered during server discovery
#[derive(Debug, Clone)]
pub struct DiscoveryError {
    /// Path where the error occurred
    pub path: PathBuf,
    /// Error message
    pub message: String,
    /// Whether this error is critical (prevents discovery) or warning
    pub is_critical: bool,
}

/// Configuration file format found during discovery
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigFormat {
    /// Standard MCP JSON configuration
    McpJson,
    /// VS Code settings JSON
    VsCodeSettings,
    /// Local discovery markdown file
    LocalDiscoveryMd,
    /// Cursor settings JSON
    CursorSettings,
}

impl ConfigFormat {
    /// Returns the expected file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            ConfigFormat::McpJson | ConfigFormat::VsCodeSettings | ConfigFormat::CursorSettings => {
                "json"
            }
            ConfigFormat::LocalDiscoveryMd => "md",
        }
    }

    /// Returns a human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            ConfigFormat::McpJson => "MCP JSON Configuration",
            ConfigFormat::VsCodeSettings => "VS Code Settings",
            ConfigFormat::LocalDiscoveryMd => "Local Discovery Markdown",
            ConfigFormat::CursorSettings => "Cursor Settings",
        }
    }
}

/// MCP server discovery engine
#[derive(Clone)]
pub struct ServerDiscovery {
    /// Paths to scan for server configurations
    discovery_paths: Vec<PathBuf>,
    /// Whether to include user-level configurations
    include_user_config: bool,
    /// Whether to include workspace-level configurations
    include_workspace_config: bool,
    /// Whether to perform health checks during discovery
    perform_health_checks: bool,
}

impl ServerDiscovery {
    /// Creates a new server discovery instance with default paths
    pub fn new() -> Self {
        Self {
            discovery_paths: Self::default_discovery_paths(),
            include_user_config: true,
            include_workspace_config: true,
            perform_health_checks: false,
        }
    }

    /// Creates a discovery instance with custom paths
    pub fn with_paths(paths: Vec<PathBuf>) -> Self {
        Self {
            discovery_paths: paths,
            include_user_config: true,
            include_workspace_config: true,
            perform_health_checks: false,
        }
    }

    /// Sets whether to perform health checks during discovery
    pub fn with_health_checks(mut self, enabled: bool) -> Self {
        self.perform_health_checks = enabled;
        self
    }

    /// Returns the default discovery paths for the current platform
    fn default_discovery_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // User home directory paths
        if let Some(home_dir) = dirs::home_dir() {
            // Standard MCP discovery path
            paths.push(home_dir.join(".mcp"));

            // VS Code settings
            if cfg!(target_os = "windows") {
                paths.push(home_dir.join("AppData/Roaming/Code/User"));
            } else if cfg!(target_os = "macos") {
                paths.push(home_dir.join("Library/Application Support/Code/User"));
            } else {
                paths.push(home_dir.join(".config/Code/User"));
            }

            // Cursor settings
            if cfg!(target_os = "windows") {
                paths.push(home_dir.join("AppData/Roaming/Cursor/User"));
            } else if cfg!(target_os = "macos") {
                paths.push(home_dir.join("Library/Application Support/Cursor/User"));
            } else {
                paths.push(home_dir.join(".config/Cursor/User"));
            }
        }

        // Current working directory and common workspace locations
        if let Ok(current_dir) = std::env::current_dir() {
            paths.push(current_dir.join(".vscode"));
            paths.push(current_dir.join(".cursor"));
            paths.push(current_dir.clone()); // For .mcp.json files
        }

        paths
    }

    /// Discovers all MCP servers from configured paths
    pub fn discover_servers(&self) -> Result<DiscoveryResult, ClaudeToolsError> {
        let start_time = std::time::Instant::now();
        let mut servers = Vec::new();
        let mut errors = Vec::new();
        let mut scanned_paths = Vec::new();

        for path in &self.discovery_paths {
            scanned_paths.push(path.clone());

            match self.discover_servers_in_path(path) {
                Ok(mut path_servers) => {
                    servers.append(&mut path_servers);
                }
                Err(error) => {
                    errors.push(DiscoveryError {
                        path: path.clone(),
                        message: error.to_string(),
                        is_critical: false,
                    });
                }
            }
        }

        // Perform health checks if enabled
        if self.perform_health_checks {
            for server in &mut servers {
                match self.check_server_health(server) {
                    Ok(status) => {
                        server.update_health_status(status);
                    }
                    Err(e) => {
                        server.update_health_status(ServerStatus::Error(e.to_string()));
                    }
                }
            }
        }

        let scan_duration = start_time.elapsed();

        Ok(DiscoveryResult {
            servers,
            scanned_paths,
            errors,
            scan_duration,
        })
    }

    /// Discovers servers in a specific path
    fn discover_servers_in_path(&self, path: &Path) -> Result<Vec<McpServer>, ClaudeToolsError> {
        let mut servers = Vec::new();

        if !path.exists() {
            return Ok(servers);
        }

        // Handle different discovery strategies based on path type
        if path.is_dir() {
            // Directory-based discovery
            servers.extend(self.discover_from_directory(path)?);
        } else if path.is_file() {
            // Single file discovery
            if let Some(server) = self.discover_from_file(path)? {
                servers.push(server);
            }
        }

        Ok(servers)
    }

    /// Discovers servers from a directory
    fn discover_from_directory(&self, dir: &Path) -> Result<Vec<McpServer>, ClaudeToolsError> {
        let mut servers = Vec::new();

        // Look for specific configuration files
        let config_files = [
            ("mcp.json", ConfigFormat::McpJson),
            ("settings.json", ConfigFormat::VsCodeSettings),
            (".mcp.json", ConfigFormat::McpJson),
        ];

        for (filename, format) in &config_files {
            let config_path = dir.join(filename);
            if config_path.exists() {
                match self.parse_config_file(&config_path, format.clone()) {
                    Ok(mut file_servers) => {
                        servers.append(&mut file_servers);
                    }
                    Err(e) => {
                        // Log error but continue discovery
                        eprintln!("Warning: Failed to parse {}: {}", config_path.display(), e);
                    }
                }
            }
        }

        // Also scan for individual JSON files in the directory
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(extension) = path.extension() {
                    if extension == "json" {
                        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        // Skip files we've already processed
                        if !["mcp.json", "settings.json", ".mcp.json"].contains(&filename) {
                            match self.parse_config_file(&path, ConfigFormat::McpJson) {
                                Ok(mut file_servers) => {
                                    servers.append(&mut file_servers);
                                }
                                Err(e) => {
                                    eprintln!(
                                        "Warning: Failed to parse individual JSON file {}: {}",
                                        path.display(),
                                        e
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        // For .mcp directories, look for markdown files (local discovery spec)
        if dir.file_name().and_then(|n| n.to_str()) == Some(".mcp") {
            match fs::read_dir(dir) {
                Ok(entries) => {
                    for entry in entries.flatten() {
                        if let Some(extension) = entry.path().extension() {
                            if extension == "md" {
                                match self.parse_local_discovery_file(&entry.path()) {
                                    Ok(server) => {
                                        servers.push(server);
                                    }
                                    Err(e) => {
                                        eprintln!(
                                            "Warning: Failed to parse {}: {}",
                                            entry.path().display(),
                                            e
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to read directory {}: {}", dir.display(), e);
                }
            }
        }

        Ok(servers)
    }

    /// Discovers a server from a single file
    fn discover_from_file(&self, file_path: &Path) -> Result<Option<McpServer>, ClaudeToolsError> {
        let extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

        match extension {
            "json" => {
                let format = self.detect_json_format(file_path)?;
                let mut servers = self.parse_config_file(file_path, format)?;
                Ok(servers.pop()) // Return the first server if any
            }
            "md" => {
                let server = self.parse_local_discovery_file(file_path)?;
                Ok(Some(server))
            }
            _ => Ok(None),
        }
    }

    /// Detects the JSON configuration format
    fn detect_json_format(&self, file_path: &Path) -> Result<ConfigFormat, ClaudeToolsError> {
        let filename = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        match filename {
            "mcp.json" | ".mcp.json" => Ok(ConfigFormat::McpJson),
            "settings.json" => {
                // Check if it's in a VS Code or Cursor directory
                if let Some(parent) = file_path.parent() {
                    let parent_name = parent.file_name().and_then(|n| n.to_str()).unwrap_or("");

                    if parent_name.contains("Code") {
                        Ok(ConfigFormat::VsCodeSettings)
                    } else if parent_name.contains("Cursor") {
                        Ok(ConfigFormat::CursorSettings)
                    } else {
                        Ok(ConfigFormat::VsCodeSettings) // Default assumption
                    }
                } else {
                    Ok(ConfigFormat::VsCodeSettings)
                }
            }
            _ => Ok(ConfigFormat::McpJson), // Default assumption
        }
    }

    /// Parses a configuration file and returns discovered servers
    fn parse_config_file(
        &self,
        file_path: &Path,
        format: ConfigFormat,
    ) -> Result<Vec<McpServer>, ClaudeToolsError> {
        let content = fs::read_to_string(file_path)?;

        match format {
            ConfigFormat::McpJson => {
                // Try to parse as full MCP config first
                if let Ok(config) = serde_json::from_str::<McpConfig>(&content) {
                    return self.servers_from_mcp_config(config, file_path.to_path_buf());
                }

                // If that fails, try to parse as individual server config
                if let Ok(server_config) = serde_json::from_str::<ServerConfig>(&content) {
                    let server_id = file_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();

                    match self.server_from_config(server_id, server_config, file_path.to_path_buf())
                    {
                        Ok(server) => return Ok(vec![server]),
                        Err(e) => {
                            return Err(ClaudeToolsError::Config(format!(
                                "Failed to parse individual server config: {}",
                                e
                            )))
                        }
                    }
                }

                Err(ClaudeToolsError::Config(
                    "Invalid MCP JSON format".to_string(),
                ))
            }
            ConfigFormat::VsCodeSettings | ConfigFormat::CursorSettings => {
                self.parse_vscode_settings(&content, file_path.to_path_buf())
            }
            ConfigFormat::LocalDiscoveryMd => {
                // This should not happen for JSON files
                Err(ClaudeToolsError::Config(
                    "Invalid format for JSON file".to_string(),
                ))
            }
        }
    }

    /// Converts an MCP configuration to server instances
    fn servers_from_mcp_config(
        &self,
        config: McpConfig,
        config_path: PathBuf,
    ) -> Result<Vec<McpServer>, ClaudeToolsError> {
        let mut servers = Vec::new();

        for (server_id, server_config) in config.servers {
            match self.server_from_config(server_id, server_config, config_path.clone()) {
                Ok(server) => servers.push(server),
                Err(e) => {
                    eprintln!("Warning: Failed to create server from config: {}", e);
                }
            }
        }

        Ok(servers)
    }

    /// Creates a server instance from a server configuration
    fn server_from_config(
        &self,
        id: String,
        config: ServerConfig,
        config_path: PathBuf,
    ) -> Result<McpServer, ClaudeToolsError> {
        let transport = config
            .to_transport()
            .map_err(|e| ClaudeToolsError::Config(e))?;

        let mut server = McpServer::new(id, config.name.clone(), transport, config_path);

        server.version = config.version.clone();
        server.description = config.description.clone();
        server.capabilities = config.to_capabilities();
        server.auth_config = config.auth;

        if let Some(metadata) = config.metadata {
            server.metadata = metadata;
        }

        if let Some(health_config) = config.health_check {
            server.health_check_url = health_config.url;
        }

        Ok(server)
    }

    /// Parses VS Code or Cursor settings file
    fn parse_vscode_settings(
        &self,
        content: &str,
        config_path: PathBuf,
    ) -> Result<Vec<McpServer>, ClaudeToolsError> {
        let settings: serde_json::Value = serde_json::from_str(content)?;
        let mut servers = Vec::new();

        // Look for MCP configuration in VS Code settings
        if let Some(mcp_config) = settings.get("mcp").or_else(|| settings.get("chat.mcp")) {
            if let Some(servers_config) = mcp_config.get("servers") {
                if let Some(servers_obj) = servers_config.as_object() {
                    for (server_id, server_value) in servers_obj {
                        match serde_json::from_value::<ServerConfig>(server_value.clone()) {
                            Ok(server_config) => {
                                match self.server_from_config(
                                    server_id.clone(),
                                    server_config,
                                    config_path.clone(),
                                ) {
                                    Ok(server) => servers.push(server),
                                    Err(e) => {
                                        eprintln!(
                                            "Warning: Failed to parse server '{}': {}",
                                            server_id, e
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!(
                                    "Warning: Failed to deserialize server '{}': {}",
                                    server_id, e
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(servers)
    }

    /// Parses a local discovery markdown file
    fn parse_local_discovery_file(&self, file_path: &Path) -> Result<McpServer, ClaudeToolsError> {
        let _content = fs::read_to_string(file_path)?;

        // For now, create a basic server entry for markdown files
        // In a full implementation, we would parse the markdown content
        // and extract server information using an LLM or structured parsing

        let file_stem = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let server_id = file_stem.to_string();
        let server_name = file_stem.replace("-", " ").replace("_", " ");

        // Create a placeholder transport (would be parsed from markdown in full implementation)
        let transport = crate::mcp::server::ServerTransport::Stdio {
            command: "echo".to_string(),
            args: vec!["not implemented".to_string()],
            env: HashMap::new(),
        };

        let mut server = McpServer::new(server_id, server_name, transport, file_path.to_path_buf());
        server.description =
            Some("Local discovery server (parsing not yet implemented)".to_string());
        server.status = ServerStatus::Unknown;

        Ok(server)
    }

    /// Performs a basic health check on a server
    fn check_server_health(&self, server: &McpServer) -> Result<ServerStatus, ClaudeToolsError> {
        match &server.transport {
            crate::mcp::server::ServerTransport::Stdio { command, .. } => {
                // Check if the command exists and is executable
                match std::process::Command::new("which").arg(command).output() {
                    Ok(output) => {
                        if output.status.success() {
                            // Command exists, try a quick test to see if it's the right server
                            match std::process::Command::new(command).arg("--help").output() {
                                Ok(_) => Ok(ServerStatus::Stopped), // Command exists but not running
                                Err(_) => {
                                    Ok(ServerStatus::Error("Command not executable".to_string()))
                                }
                            }
                        } else {
                            Ok(ServerStatus::Error(format!(
                                "Command '{}' not found",
                                command
                            )))
                        }
                    }
                    Err(_) => {
                        // Fallback: just check if the command exists in PATH
                        Ok(ServerStatus::Unknown)
                    }
                }
            }
            crate::mcp::server::ServerTransport::Http { url, .. } => {
                // For HTTP servers, we could perform a basic connectivity check
                // For now, just mark as unknown since we don't have HTTP client
                Ok(ServerStatus::Unknown)
            }
            crate::mcp::server::ServerTransport::WebSocket { .. } => {
                // For WebSocket servers, we could attempt a connection
                // For now, just mark as unknown
                Ok(ServerStatus::Unknown)
            }
        }
    }
}

impl Default for ServerDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

impl DiscoveryResult {
    /// Returns the number of successfully discovered servers
    pub fn server_count(&self) -> usize {
        self.servers.len()
    }

    /// Returns the number of errors encountered
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// Returns servers filtered by status
    pub fn servers_with_status(&self, status: &ServerStatus) -> Vec<&McpServer> {
        self.servers
            .iter()
            .filter(|server| {
                std::mem::discriminant(&server.status) == std::mem::discriminant(status)
            })
            .collect()
    }

    /// Returns a summary of the discovery operation
    pub fn summary(&self) -> String {
        format!(
            "Discovered {} servers from {} paths in {:.2}s ({} errors)",
            self.server_count(),
            self.scanned_paths.len(),
            self.scan_duration.as_secs_f64(),
            self.error_count()
        )
    }
}
