use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::env;

/// Main application configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Configuration format version for migration support
    pub version: String,
    /// Real-time update settings
    pub realtime: RealtimeConfig,
    /// Timeline dashboard settings
    pub timeline: TimelineConfig,
    /// User interface preferences
    pub ui: UiConfig,
}

/// Real-time update configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeConfig {
    /// Whether auto-refresh is enabled
    pub enabled: bool,
    /// Debounce duration in milliseconds
    pub debounce_ms: u64,
    /// Whether to watch conversation files
    pub watch_conversations: bool,
    /// Whether to watch MCP configuration files
    pub watch_mcp_configs: bool,
    /// Auto-refresh interval in seconds (for periodic full refresh)
    pub refresh_interval_seconds: u64,
}

/// Timeline dashboard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineConfig {
    /// Default time period for timeline view
    pub default_period: String, // Serialized TimePeriod
    /// Summary depth level
    pub summary_depth: String,
    /// Maximum conversations to analyze
    pub max_conversations: Option<usize>,
    /// Whether to enable timeline caching
    pub enable_caching: bool,
}

/// User interface configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// Default view when starting interactive mode
    pub default_view: String, // Serialized AppState
    /// UI theme preference
    pub theme: String,
    /// Whether to show status messages
    pub show_status_messages: bool,
    /// Status message display duration in milliseconds
    pub status_message_duration_ms: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            realtime: RealtimeConfig::default(),
            timeline: TimelineConfig::default(),
            ui: UiConfig::default(),
        }
    }
}

impl Default for RealtimeConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Default to disabled for non-disruptive behavior
            debounce_ms: 500,
            watch_conversations: true,
            watch_mcp_configs: true,
            refresh_interval_seconds: 30,
        }
    }
}

impl Default for TimelineConfig {
    fn default() -> Self {
        Self {
            default_period: "48h".to_string(),
            summary_depth: "detailed".to_string(),
            max_conversations: None,
            enable_caching: true,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            default_view: "ConversationList".to_string(),
            theme: "default".to_string(),
            show_status_messages: true,
            status_message_duration_ms: 3000,
        }
    }
}

impl AppConfig {
    /// Load configuration from file, or create default if file doesn't exist
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        
        if !path.exists() {
            // Create default configuration and save it
            let config = Self::default();
            config.save_to_file(path)?;
            return Ok(config);
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        
        let config: AppConfig = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
        
        config.validate()?;
        Ok(config)
    }

    /// Save configuration to file with atomic write
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
        }

        // Create backup if file exists
        if path.exists() {
            let backup_path = path.with_extension("json.backup");
            fs::copy(path, &backup_path)
                .with_context(|| format!("Failed to create backup: {}", backup_path.display()))?;
        }

        // Write to temporary file first, then rename (atomic operation)
        let temp_path = path.with_extension("json.tmp");
        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize configuration")?;
        
        fs::write(&temp_path, content)
            .with_context(|| format!("Failed to write temp config file: {}", temp_path.display()))?;
        
        fs::rename(&temp_path, path)
            .with_context(|| format!("Failed to rename temp file to: {}", path.display()))?;

        Ok(())
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<()> {
        // Validate version
        if self.version.is_empty() {
            return Err(anyhow::anyhow!("Configuration version cannot be empty"));
        }

        // Validate realtime config
        if self.realtime.debounce_ms < 100 || self.realtime.debounce_ms > 5000 {
            return Err(anyhow::anyhow!(
                "Debounce duration must be between 100ms and 5000ms, got: {}ms",
                self.realtime.debounce_ms
            ));
        }

        if self.realtime.refresh_interval_seconds < 1 || self.realtime.refresh_interval_seconds > 3600 {
            return Err(anyhow::anyhow!(
                "Refresh interval must be between 1 and 3600 seconds, got: {}s",
                self.realtime.refresh_interval_seconds
            ));
        }

        // Validate timeline config
        let valid_periods = ["24h", "48h", "1week", "1month"];
        if !valid_periods.contains(&self.timeline.default_period.as_str()) {
            return Err(anyhow::anyhow!(
                "Invalid default period: {}. Must be one of: {:?}",
                self.timeline.default_period,
                valid_periods
            ));
        }

        let valid_depths = ["brief", "detailed", "comprehensive"];
        if !valid_depths.contains(&self.timeline.summary_depth.as_str()) {
            return Err(anyhow::anyhow!(
                "Invalid summary depth: {}. Must be one of: {:?}",
                self.timeline.summary_depth,
                valid_depths
            ));
        }

        // Validate UI config
        let valid_views = ["ConversationList", "Timeline", "Analytics"];
        if !valid_views.contains(&self.ui.default_view.as_str()) {
            return Err(anyhow::anyhow!(
                "Invalid default view: {}. Must be one of: {:?}",
                self.ui.default_view,
                valid_views
            ));
        }

        if self.ui.status_message_duration_ms < 500 || self.ui.status_message_duration_ms > 10000 {
            return Err(anyhow::anyhow!(
                "Status message duration must be between 500ms and 10000ms, got: {}ms",
                self.ui.status_message_duration_ms
            ));
        }

        Ok(())
    }

    /// Get the default configuration file path
    pub fn default_config_path() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
        Ok(home.join(".claude").join("claude-tools-config.json"))
    }

    /// Get the project-specific configuration file path
    pub fn project_config_path() -> PathBuf {
        PathBuf::from(".claude-tools.json")
    }

    /// Load configuration with hierarchical precedence:
    /// CLI args > project config > user config > defaults
    pub fn load_hierarchical(
        project_config_override: Option<&Path>,
        user_config_override: Option<&Path>,
    ) -> Result<Self> {
        // Start with default configuration
        let mut config = Self::default();

        // Load user-level configuration
        let user_config_path = if let Some(path) = user_config_override {
            path.to_path_buf()
        } else {
            Self::default_config_path()?
        };

        if user_config_path.exists() {
            if let Ok(user_config) = Self::load_from_file(&user_config_path) {
                config = Self::merge_configs(config, user_config);
            }
        }

        // Load project-level configuration (if we're in a project directory)
        let project_config_path = if let Some(path) = project_config_override {
            Some(path.to_path_buf())
        } else {
            // Look for project config in current directory and parent directories
            Self::find_project_config()
        };

        if let Some(project_path) = project_config_path {
            if project_path.exists() {
                if let Ok(project_config) = Self::load_from_file(&project_path) {
                    config = Self::merge_configs(config, project_config);
                }
            }
        }

        config.validate()?;
        Ok(config)
    }

    /// Find project configuration by walking up directory tree
    fn find_project_config() -> Option<PathBuf> {
        let mut current_dir = env::current_dir().ok()?;
        
        loop {
            let config_path = current_dir.join(".claude-tools.json");
            if config_path.exists() {
                return Some(config_path);
            }
            
            // Move up one directory
            if !current_dir.pop() {
                break;
            }
        }
        
        None
    }

    /// Merge two configurations, with the second taking precedence
    fn merge_configs(base: Self, override_config: Self) -> Self {
        Self {
            version: if override_config.version != Self::default().version {
                override_config.version
            } else {
                base.version
            },
            realtime: Self::merge_realtime_config(base.realtime, override_config.realtime),
            timeline: Self::merge_timeline_config(base.timeline, override_config.timeline),
            ui: Self::merge_ui_config(base.ui, override_config.ui),
        }
    }

    /// Merge realtime configurations
    fn merge_realtime_config(base: RealtimeConfig, override_config: RealtimeConfig) -> RealtimeConfig {
        let default = RealtimeConfig::default();
        RealtimeConfig {
            enabled: if override_config.enabled != default.enabled {
                override_config.enabled
            } else {
                base.enabled
            },
            debounce_ms: if override_config.debounce_ms != default.debounce_ms {
                override_config.debounce_ms
            } else {
                base.debounce_ms
            },
            watch_conversations: if override_config.watch_conversations != default.watch_conversations {
                override_config.watch_conversations
            } else {
                base.watch_conversations
            },
            watch_mcp_configs: if override_config.watch_mcp_configs != default.watch_mcp_configs {
                override_config.watch_mcp_configs
            } else {
                base.watch_mcp_configs
            },
            refresh_interval_seconds: if override_config.refresh_interval_seconds != default.refresh_interval_seconds {
                override_config.refresh_interval_seconds
            } else {
                base.refresh_interval_seconds
            },
        }
    }

    /// Merge timeline configurations
    fn merge_timeline_config(base: TimelineConfig, override_config: TimelineConfig) -> TimelineConfig {
        let default = TimelineConfig::default();
        TimelineConfig {
            default_period: if override_config.default_period != default.default_period {
                override_config.default_period
            } else {
                base.default_period
            },
            summary_depth: if override_config.summary_depth != default.summary_depth {
                override_config.summary_depth
            } else {
                base.summary_depth
            },
            max_conversations: override_config.max_conversations.or(base.max_conversations),
            enable_caching: if override_config.enable_caching != default.enable_caching {
                override_config.enable_caching
            } else {
                base.enable_caching
            },
        }
    }

    /// Merge UI configurations
    fn merge_ui_config(base: UiConfig, override_config: UiConfig) -> UiConfig {
        let default = UiConfig::default();
        UiConfig {
            default_view: if override_config.default_view != default.default_view {
                override_config.default_view
            } else {
                base.default_view
            },
            theme: if override_config.theme != default.theme {
                override_config.theme
            } else {
                base.theme
            },
            show_status_messages: if override_config.show_status_messages != default.show_status_messages {
                override_config.show_status_messages
            } else {
                base.show_status_messages
            },
            status_message_duration_ms: if override_config.status_message_duration_ms != default.status_message_duration_ms {
                override_config.status_message_duration_ms
            } else {
                base.status_message_duration_ms
            },
        }
    }

    /// Migrate configuration from older versions
    pub fn migrate(mut self) -> Result<Self> {
        let current_version = "1.0";
        
        if self.version == current_version {
            return Ok(self); // No migration needed
        }
        
        // Migration logic for future versions
        match self.version.as_str() {
            "0.9" => {
                // Example migration: Add new fields with defaults
                // self.new_field = default_value;
                self.version = "1.0".to_string();
            }
            _ => {
                // Unknown version - reset to defaults but preserve user preferences where possible
                let defaults = Self::default();
                self.version = defaults.version;
                // Could preserve specific user settings here
            }
        }
        
        Ok(self)
    }

    /// Convert realtime debounce to Duration
    pub fn debounce_duration(&self) -> Duration {
        Duration::from_millis(self.realtime.debounce_ms)
    }

    /// Convert refresh interval to Duration
    pub fn refresh_interval(&self) -> Duration {
        Duration::from_secs(self.realtime.refresh_interval_seconds)
    }

    /// Convert status message duration to Duration
    pub fn status_message_duration(&self) -> Duration {
        Duration::from_millis(self.ui.status_message_duration_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.version, "1.0");
        assert!(!config.realtime.enabled); // Should default to disabled
        assert!(config.timeline.enable_caching);
        assert_eq!(config.ui.theme, "default");
    }

    #[test]
    fn test_config_validation() {
        let config = AppConfig::default();
        assert!(config.validate().is_ok());

        // Test invalid debounce duration
        let mut invalid_config = config.clone();
        invalid_config.realtime.debounce_ms = 50; // Too low
        assert!(invalid_config.validate().is_err());

        // Test invalid timeline period
        let mut invalid_config = config.clone();
        invalid_config.timeline.default_period = "invalid".to_string();
        assert!(invalid_config.validate().is_err());

        // Test invalid UI view
        let mut invalid_config = config.clone();
        invalid_config.ui.default_view = "InvalidView".to_string();
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test-config.json");

        let original_config = AppConfig::default();
        
        // Save config
        original_config.save_to_file(&config_path).unwrap();
        assert!(config_path.exists());

        // Load config
        let loaded_config = AppConfig::load_from_file(&config_path).unwrap();
        
        // Compare (using debug format since we don't have PartialEq)
        assert_eq!(format!("{:?}", original_config), format!("{:?}", loaded_config));
    }

    #[test]
    fn test_load_nonexistent_creates_default() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("nonexistent-config.json");

        assert!(!config_path.exists());
        
        let config = AppConfig::load_from_file(&config_path).unwrap();
        
        // Should create the file with default config
        assert!(config_path.exists());
        assert_eq!(config.version, "1.0");
    }

    #[test]
    fn test_backup_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test-config.json");
        let backup_path = config_path.with_extension("json.backup");

        // Create initial config
        let config1 = AppConfig::default();
        config1.save_to_file(&config_path).unwrap();
        assert!(config_path.exists());
        assert!(!backup_path.exists());

        // Save again (should create backup)
        let mut config2 = AppConfig::default();
        config2.version = "2.0".to_string();
        config2.save_to_file(&config_path).unwrap();
        
        assert!(config_path.exists());
        assert!(backup_path.exists());
    }

    #[test]
    fn test_duration_conversions() {
        let config = AppConfig::default();
        
        assert_eq!(config.debounce_duration(), Duration::from_millis(500));
        assert_eq!(config.refresh_interval(), Duration::from_secs(30));
        assert_eq!(config.status_message_duration(), Duration::from_millis(3000));
    }

    #[test]
    fn test_hierarchical_config_loading() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create user config
        let user_config_path = temp_dir.path().join("user-config.json");
        let mut user_config = AppConfig::default();
        user_config.realtime.enabled = true;
        user_config.ui.theme = "dark".to_string();
        user_config.save_to_file(&user_config_path).unwrap();
        
        // Create project config  
        let project_config_path = temp_dir.path().join("project-config.json");
        let mut project_config = AppConfig::default();
        project_config.realtime.debounce_ms = 1000;
        project_config.timeline.default_period = "1week".to_string();
        project_config.save_to_file(&project_config_path).unwrap();
        
        // Load hierarchical config
        let merged_config = AppConfig::load_hierarchical(
            Some(&project_config_path),
            Some(&user_config_path),
        ).unwrap();
        
        // Should have user config settings where not overridden
        assert!(merged_config.realtime.enabled); // From user config
        assert_eq!(merged_config.ui.theme, "dark"); // From user config
        
        // Should have project config overrides
        assert_eq!(merged_config.realtime.debounce_ms, 1000); // From project config
        assert_eq!(merged_config.timeline.default_period, "1week"); // From project config
        
        // Should have defaults for unspecified values
        assert!(merged_config.timeline.enable_caching); // Default value
    }

    #[test]
    fn test_config_merge_precedence() {
        let base_config = AppConfig {
            version: "1.0".to_string(),
            realtime: RealtimeConfig {
                enabled: false,
                debounce_ms: 500,
                watch_conversations: true,
                watch_mcp_configs: true,
                refresh_interval_seconds: 30,
            },
            timeline: TimelineConfig {
                default_period: "48h".to_string(),
                summary_depth: "detailed".to_string(),
                max_conversations: None,
                enable_caching: true,
            },
            ui: UiConfig {
                default_view: "ConversationList".to_string(),
                theme: "default".to_string(),
                show_status_messages: true,
                status_message_duration_ms: 3000,
            },
        };

        let override_config = AppConfig {
            version: "1.0".to_string(),
            realtime: RealtimeConfig {
                enabled: true, // Override
                debounce_ms: 500, // Same as default, so base should be kept
                watch_conversations: false, // Override
                watch_mcp_configs: true, // Same as default, so base should be kept
                refresh_interval_seconds: 60, // Override
            },
            timeline: TimelineConfig {
                default_period: "1week".to_string(), // Override
                summary_depth: "detailed".to_string(), // Same as default, so base should be kept
                max_conversations: Some(100), // Override (was None)
                enable_caching: true, // Same as default, so base should be kept
            },
            ui: UiConfig {
                default_view: "Timeline".to_string(), // Override
                theme: "default".to_string(), // Same as default, so base should be kept
                show_status_messages: false, // Override
                status_message_duration_ms: 3000, // Same as default, so base should be kept
            },
        };

        let merged = AppConfig::merge_configs(base_config.clone(), override_config);

        // Check that overrides took precedence
        assert!(merged.realtime.enabled); // Override
        assert!(!merged.realtime.watch_conversations); // Override
        assert_eq!(merged.realtime.refresh_interval_seconds, 60); // Override
        assert_eq!(merged.timeline.default_period, "1week"); // Override
        assert_eq!(merged.timeline.max_conversations, Some(100)); // Override
        assert_eq!(merged.ui.default_view, "Timeline"); // Override
        assert!(!merged.ui.show_status_messages); // Override

        // Check that base values were kept when override matched default
        assert_eq!(merged.realtime.debounce_ms, base_config.realtime.debounce_ms);
        assert_eq!(merged.realtime.watch_mcp_configs, base_config.realtime.watch_mcp_configs);
        assert_eq!(merged.timeline.summary_depth, base_config.timeline.summary_depth);
        assert_eq!(merged.timeline.enable_caching, base_config.timeline.enable_caching);
        assert_eq!(merged.ui.theme, base_config.ui.theme);
        assert_eq!(merged.ui.status_message_duration_ms, base_config.ui.status_message_duration_ms);
    }

    #[test]
    fn test_config_migration() {
        let mut old_config = AppConfig::default();
        old_config.version = "0.9".to_string();
        
        let migrated = old_config.migrate().unwrap();
        assert_eq!(migrated.version, "1.0");
    }

    #[test]
    fn test_config_migration_unknown_version() {
        let mut unknown_config = AppConfig::default();
        unknown_config.version = "999.0".to_string();
        
        let migrated = unknown_config.migrate().unwrap();
        assert_eq!(migrated.version, "1.0"); // Should reset to current version
    }

    #[test]
    fn test_project_config_path() {
        let path = AppConfig::project_config_path();
        assert_eq!(path, PathBuf::from(".claude-tools.json"));
    }

    #[test]
    fn test_find_project_config() {
        // This test is somewhat limited since we can't easily control the directory structure
        // In a real project with a .claude-tools.json file, this would find it
        let result = AppConfig::find_project_config();
        // Just verify it returns an Option<PathBuf> without erroring
        match result {
            Some(path) => {
                assert!(path.exists());
                assert_eq!(path.file_name().unwrap(), ".claude-tools.json");
            }
            None => {
                // No project config found, which is fine for tests
            }
        }
    }
}