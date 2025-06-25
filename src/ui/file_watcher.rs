use crate::config::AppConfig;
use crate::ui::events::Event;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Classification of update scope for performance optimization
#[derive(Debug, Clone, PartialEq)]
pub enum UpdateScope {
    /// Single file changed, minimal update needed
    Minimal(PathBuf),
    /// Multiple files changed, incremental update
    Incremental(Vec<PathBuf>),
    /// Major changes requiring full refresh
    Full,
}

/// Update manager for coordinating file system changes
pub struct UpdateManager {
    /// Last time a full scan was performed
    last_full_scan: Instant,
    /// Files pending update processing
    pending_updates: HashMap<PathBuf, Instant>,
    /// Threshold for triggering full refresh vs incremental
    update_threshold: usize,
    /// Duration to batch updates before processing
    batch_duration: Duration,
}

/// File system watcher for real-time updates
pub struct FileWatcher {
    /// The notify watcher instance
    watcher: RecommendedWatcher,
    /// Event sender for file system events
    event_sender: mpsc::Sender<Event>,
    /// Debounce duration to prevent rapid-fire updates
    debounce_duration: Duration,
    /// Last update times per file for debouncing
    last_update: HashMap<PathBuf, Instant>,
    /// Whether file watching is enabled
    enabled: bool,
}

impl FileWatcher {
    /// Create a new file watcher
    pub fn new(event_sender: mpsc::Sender<Event>) -> Result<Self, notify::Error> {
        let sender_clone = event_sender.clone();
        
        let watcher = RecommendedWatcher::new(
            move |res: Result<notify::Event, notify::Error>| {
                match res {
                    Ok(event) => {
                        // Filter for file modification events
                        match event.kind {
                            EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => {
                                for path in event.paths {
                                    // Only watch for specific file types
                                    if path.extension().and_then(|s| s.to_str()) == Some("jsonl") 
                                        || path.extension().and_then(|s| s.to_str()) == Some("json") {
                                        let _ = sender_clone.send(Event::FileChanged(path));
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        eprintln!("File watch error: {:?}", e);
                    }
                }
            },
            Config::default(),
        )?;

        Ok(Self {
            watcher,
            event_sender,
            debounce_duration: Duration::from_millis(500),
            last_update: HashMap::new(),
            enabled: false,
        })
    }

    /// Watch a directory for file changes
    pub fn watch_directory(&mut self, path: &Path) -> Result<(), notify::Error> {
        if self.enabled {
            self.watcher.watch(path, RecursiveMode::NonRecursive)?;
        }
        Ok(())
    }

    /// Stop watching a directory
    pub fn unwatch_directory(&mut self, path: &Path) -> Result<(), notify::Error> {
        self.watcher.unwatch(path)?;
        Ok(())
    }

    /// Enable or disable file watching
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if file watching is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Set the debounce duration for file change events
    pub fn set_debounce_duration(&mut self, duration: Duration) {
        self.debounce_duration = duration;
    }

    /// Check if a file change should be notified (debouncing logic)
    pub fn should_notify(&mut self, path: &PathBuf) -> bool {
        let now = Instant::now();
        if let Some(last) = self.last_update.get(path) {
            if now.duration_since(*last) < self.debounce_duration {
                return false;
            }
        }
        self.last_update.insert(path.clone(), now);
        true
    }

    /// Clear debounce history (useful for testing or reset)
    pub fn clear_debounce_history(&mut self) {
        self.last_update.clear();
    }

    /// Update configuration for file watching
    pub fn update_config(&mut self, config: &AppConfig) {
        self.enabled = config.realtime.enabled;
        self.debounce_duration = config.debounce_duration();
    }
}

impl UpdateManager {
    /// Create a new update manager
    pub fn new() -> Self {
        Self {
            last_full_scan: Instant::now(),
            pending_updates: HashMap::new(),
            update_threshold: 10, // Trigger full refresh after 10+ files
            batch_duration: Duration::from_millis(1000), // Batch updates for 1 second
        }
    }

    /// Add a file change to the pending updates
    pub fn add_file_change(&mut self, path: PathBuf) {
        self.pending_updates.insert(path, Instant::now());
    }

    /// Get the update scope based on pending changes
    pub fn get_update_scope(&mut self) -> Option<UpdateScope> {
        let now = Instant::now();
        
        // Remove old pending updates (beyond batch duration)
        self.pending_updates.retain(|_, timestamp| {
            now.duration_since(*timestamp) < self.batch_duration
        });

        let pending_count = self.pending_updates.len();
        
        if pending_count == 0 {
            return None;
        }

        // Check if we should do a full refresh
        if pending_count >= self.update_threshold {
            let scope = Some(UpdateScope::Full);
            self.pending_updates.clear();
            self.last_full_scan = now;
            return scope;
        }

        // Single file change
        if pending_count == 1 {
            let path = self.pending_updates.keys().next().unwrap().clone();
            self.pending_updates.clear();
            return Some(UpdateScope::Minimal(path));
        }

        // Multiple files, but under threshold
        let paths: Vec<PathBuf> = self.pending_updates.keys().cloned().collect();
        self.pending_updates.clear();
        Some(UpdateScope::Incremental(paths))
    }

    /// Check if we should process pending updates (batch timing)
    pub fn should_process_updates(&self) -> bool {
        if self.pending_updates.is_empty() {
            return false;
        }

        // Process if oldest pending update is beyond batch duration
        let now = Instant::now();
        self.pending_updates.values()
            .min()
            .map(|oldest| now.duration_since(*oldest) >= self.batch_duration)
            .unwrap_or(false)
    }

    /// Get statistics about update manager state
    pub fn get_stats(&self) -> UpdateStats {
        UpdateStats {
            pending_updates: self.pending_updates.len(),
            last_full_scan_ago: self.last_full_scan.elapsed(),
            update_threshold: self.update_threshold,
        }
    }

    /// Clear all pending updates
    pub fn clear_pending(&mut self) {
        self.pending_updates.clear();
    }
}

/// Statistics about the update manager state
#[derive(Debug, Clone)]
pub struct UpdateStats {
    pub pending_updates: usize,
    pub last_full_scan_ago: Duration,
    pub update_threshold: usize,
}

impl Default for UpdateManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_file_watcher_creation() {
        let (sender, _receiver) = mpsc::channel();
        let watcher = FileWatcher::new(sender);
        assert!(watcher.is_ok());
    }

    #[test]
    fn test_enable_disable() {
        let (sender, _receiver) = mpsc::channel();
        let mut watcher = FileWatcher::new(sender).unwrap();
        
        assert!(!watcher.is_enabled());
        watcher.set_enabled(true);
        assert!(watcher.is_enabled());
        watcher.set_enabled(false);
        assert!(!watcher.is_enabled());
    }

    #[test]
    fn test_debounce_logic() {
        let (sender, _receiver) = mpsc::channel();
        let mut watcher = FileWatcher::new(sender).unwrap();
        let path = PathBuf::from("/test/path.jsonl");

        // First call should return true
        assert!(watcher.should_notify(&path));
        
        // Immediate second call should return false (debounced)
        assert!(!watcher.should_notify(&path));
        
        // After clearing history, should return true again
        watcher.clear_debounce_history();
        assert!(watcher.should_notify(&path));
    }

    #[test]
    fn test_watch_directory() {
        let (sender, _receiver) = mpsc::channel();
        let mut watcher = FileWatcher::new(sender).unwrap();
        let temp_dir = TempDir::new().unwrap();
        
        // Should succeed when enabled
        watcher.set_enabled(true);
        assert!(watcher.watch_directory(temp_dir.path()).is_ok());
        
        // Should still work when disabled (no-op internally)
        watcher.set_enabled(false);
        assert!(watcher.watch_directory(temp_dir.path()).is_ok());
    }

    #[test]
    fn test_update_config() {
        let (sender, _receiver) = mpsc::channel();
        let mut watcher = FileWatcher::new(sender).unwrap();
        let mut config = AppConfig::default();
        
        // Test enabling via config
        config.realtime.enabled = true;
        config.realtime.debounce_ms = 1000;
        watcher.update_config(&config);
        
        assert!(watcher.is_enabled());
        assert_eq!(watcher.debounce_duration, Duration::from_millis(1000));
    }

    #[test]
    fn test_update_manager_single_file() {
        let mut manager = UpdateManager::new();
        let path = PathBuf::from("/test/file.jsonl");
        
        assert!(manager.get_update_scope().is_none());
        
        manager.add_file_change(path.clone());
        let scope = manager.get_update_scope().unwrap();
        
        assert_eq!(scope, UpdateScope::Minimal(path));
        assert!(manager.get_update_scope().is_none()); // Should be cleared
    }

    #[test]
    fn test_update_manager_multiple_files() {
        let mut manager = UpdateManager::new();
        let paths = vec![
            PathBuf::from("/test/file1.jsonl"),
            PathBuf::from("/test/file2.jsonl"),
            PathBuf::from("/test/file3.jsonl"),
        ];
        
        for path in &paths {
            manager.add_file_change(path.clone());
        }
        
        let scope = manager.get_update_scope().unwrap();
        match scope {
            UpdateScope::Incremental(returned_paths) => {
                assert_eq!(returned_paths.len(), 3);
                for path in &paths {
                    assert!(returned_paths.contains(path));
                }
            }
            _ => panic!("Expected Incremental scope"),
        }
    }

    #[test]
    fn test_update_manager_full_refresh() {
        let mut manager = UpdateManager::new();
        manager.update_threshold = 3; // Lower threshold for testing
        
        // Add files beyond threshold
        for i in 0..5 {
            let path = PathBuf::from(format!("/test/file{}.jsonl", i));
            manager.add_file_change(path);
        }
        
        let scope = manager.get_update_scope().unwrap();
        assert_eq!(scope, UpdateScope::Full);
    }

    #[test]
    fn test_update_manager_batching() {
        let mut manager = UpdateManager::new();
        manager.batch_duration = Duration::from_millis(100);
        
        let path = PathBuf::from("/test/file.jsonl");
        manager.add_file_change(path);
        
        // Should not process immediately
        assert!(!manager.should_process_updates());
        
        // Wait for batch duration to pass
        std::thread::sleep(Duration::from_millis(150));
        assert!(manager.should_process_updates());
    }

    #[test]
    fn test_update_stats() {
        let mut manager = UpdateManager::new();
        let path = PathBuf::from("/test/file.jsonl");
        
        manager.add_file_change(path);
        let stats = manager.get_stats();
        
        assert_eq!(stats.pending_updates, 1);
        assert_eq!(stats.update_threshold, 10);
        assert!(stats.last_full_scan_ago < Duration::from_secs(1));
    }
}