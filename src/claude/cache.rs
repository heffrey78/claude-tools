use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::timeline::{ActivityTimeline, TimelineConfig, TimePeriod};
use crate::errors::ClaudeToolsError;

/// Cache metadata for timeline data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetadata {
    /// Version of the cache format
    pub version: u32,
    /// When the cache was created
    pub created_at: DateTime<Utc>,
    /// Hash of the conversation directory at cache time
    pub directory_hash: u64,
    /// Timeline configuration used
    pub config: TimelineConfig,
    /// Number of conversations processed
    pub conversation_count: usize,
    /// Cache file size in bytes
    pub file_size: u64,
}

/// Cached timeline data with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedTimeline {
    /// Cache metadata
    pub metadata: CacheMetadata,
    /// The actual timeline data
    pub timeline: ActivityTimeline,
}

/// Timeline cache manager
pub struct TimelineCache {
    /// Base cache directory
    cache_dir: PathBuf,
    /// Current cache format version
    version: u32,
}

impl TimelineCache {
    /// Current cache format version
    const CACHE_VERSION: u32 = 1;
    
    /// Cache directory name within Claude directory
    const CACHE_DIR_NAME: &'static str = "timeline_cache";
    
    /// Create a new timeline cache manager
    pub fn new(claude_dir: &Path) -> Result<Self, ClaudeToolsError> {
        let cache_dir = claude_dir.join(Self::CACHE_DIR_NAME);
        
        // Create cache directory if it doesn't exist
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir)?;
        }
        
        Ok(Self {
            cache_dir,
            version: Self::CACHE_VERSION,
        })
    }
    
    /// Generate cache file path for a timeline configuration
    fn cache_file_path(&self, config: &TimelineConfig) -> PathBuf {
        let config_hash = self.hash_config(config);
        self.cache_dir.join(format!("timeline_{:x}.json", config_hash))
    }
    
    /// Calculate hash of timeline configuration
    fn hash_config(&self, config: &TimelineConfig) -> u64 {
        let mut hasher = DefaultHasher::new();
        
        // Hash the period
        match config.period {
            TimePeriod::LastDay => "day".hash(&mut hasher),
            TimePeriod::LastTwoDay => "two_day".hash(&mut hasher),
            TimePeriod::LastWeek => "week".hash(&mut hasher),
            TimePeriod::LastMonth => "month".hash(&mut hasher),
            TimePeriod::Custom { start, end } => {
                "custom".hash(&mut hasher);
                start.timestamp().hash(&mut hasher);
                end.timestamp().hash(&mut hasher);
            }
        }
        
        // Hash other config parameters
        config.summary_depth.hash(&mut hasher);
        config.max_conversations_per_project.hash(&mut hasher);
        config.include_empty_projects.hash(&mut hasher);
        
        hasher.finish()
    }
    
    /// Calculate hash of conversation directory (based on modification times)
    pub fn hash_conversation_directory(&self, conversations_dir: &Path) -> Result<u64, ClaudeToolsError> {
        let mut hasher = DefaultHasher::new();
        
        if !conversations_dir.exists() {
            return Ok(0);
        }
        
        // Read directory entries and hash their modification times
        let entries = fs::read_dir(conversations_dir)?;
        
        let mut _file_hashes: Vec<u64> = Vec::new();
        
        for entry in entries {
            let entry = entry?;
            
            let path = entry.path();
            
            // Only process .jsonl files
            if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                let metadata = entry.metadata()?;
                
                if let Ok(modified) = metadata.modified() {
                    if let Ok(duration) = modified.duration_since(SystemTime::UNIX_EPOCH) {
                        path.to_string_lossy().hash(&mut hasher);
                        duration.as_secs().hash(&mut hasher);
                        metadata.len().hash(&mut hasher);
                    }
                }
            }
        }
        
        Ok(hasher.finish())
    }
    
    /// Check if cached timeline exists and is valid
    pub fn is_cache_valid(&self, config: &TimelineConfig, conversations_dir: &Path) -> Result<bool, ClaudeToolsError> {
        let cache_file = self.cache_file_path(config);
        
        if !cache_file.exists() {
            return Ok(false);
        }
        
        // Try to read and validate cache metadata
        match self.load_cache_metadata(&cache_file) {
            Ok(metadata) => {
                // Check version compatibility
                if metadata.version != self.version {
                    return Ok(false);
                }
                
                // Check if conversation directory has changed
                let current_hash = self.hash_conversation_directory(conversations_dir)?;
                if metadata.directory_hash != current_hash {
                    return Ok(false);
                }
                
                // Check config compatibility
                if !self.configs_compatible(&metadata.config, config) {
                    return Ok(false);
                }
                
                Ok(true)
            }
            Err(_) => Ok(false), // Cache file corrupted or unreadable
        }
    }
    
    /// Check if two timeline configs are compatible for caching
    fn configs_compatible(&self, cached: &TimelineConfig, requested: &TimelineConfig) -> bool {
        // For now, require exact match - could be relaxed for some parameters
        cached.period == requested.period &&
        cached.summary_depth == requested.summary_depth &&
        cached.max_conversations_per_project == requested.max_conversations_per_project &&
        cached.include_empty_projects == requested.include_empty_projects
    }
    
    /// Load cached timeline if valid, or try to filter from a longer period
    pub fn load_timeline(&self, config: &TimelineConfig, conversations_dir: &Path) -> Result<Option<ActivityTimeline>, ClaudeToolsError> {
        // First try exact match
        if self.is_cache_valid(config, conversations_dir)? {
            let cache_file = self.cache_file_path(config);
            let cached_data = self.load_cached_timeline(&cache_file)?;
            return Ok(Some(cached_data.timeline));
        }
        
        // Try to find a compatible base timeline with longer period that we can filter
        let longer_periods = self.get_longer_periods(&config.period);
        
        for longer_period in longer_periods {
            let base_config = TimelineConfig {
                period: longer_period,
                ..config.clone()
            };
            
            if self.is_cache_valid(&base_config, conversations_dir)? {
                let cache_file = self.cache_file_path(&base_config);
                let cached_data = self.load_cached_timeline(&cache_file)?;
                
                // Try to filter the base timeline to the requested period
                match cached_data.timeline.filter_to_period(config.period) {
                    Ok(filtered_timeline) => {
                        return Ok(Some(filtered_timeline));
                    }
                    Err(_) => {
                        // Filtering failed, continue to next longer period
                        continue;
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    /// Get time periods that are longer than the given period (for filtering)
    fn get_longer_periods(&self, period: &TimePeriod) -> Vec<TimePeriod> {
        match period {
            TimePeriod::LastDay => vec![TimePeriod::LastTwoDay, TimePeriod::LastWeek, TimePeriod::LastMonth],
            TimePeriod::LastTwoDay => vec![TimePeriod::LastWeek, TimePeriod::LastMonth],
            TimePeriod::LastWeek => vec![TimePeriod::LastMonth],
            TimePeriod::LastMonth => vec![], // Already the longest standard period
            TimePeriod::Custom { .. } => vec![], // Don't try to filter custom periods
        }
    }
    
    /// Save timeline to cache
    pub fn save_timeline(
        &self, 
        timeline: &ActivityTimeline, 
        conversations_dir: &Path,
        conversation_count: usize
    ) -> Result<(), ClaudeToolsError> {
        let cache_file = self.cache_file_path(&timeline.config);
        let directory_hash = self.hash_conversation_directory(conversations_dir)?;
        
        let metadata = CacheMetadata {
            version: self.version,
            created_at: Utc::now(),
            directory_hash,
            config: timeline.config.clone(),
            conversation_count,
            file_size: 0, // Will be updated after serialization
        };
        
        let cached_timeline = CachedTimeline {
            metadata,
            timeline: timeline.clone(),
        };
        
        let serialized = serde_json::to_string_pretty(&cached_timeline)?;
        fs::write(&cache_file, &serialized)?;
        
        // Update file size in metadata
        if let Ok(metadata) = fs::metadata(&cache_file) {
            let mut cached_timeline = cached_timeline;
            cached_timeline.metadata.file_size = metadata.len();
            
            let updated_serialized = serde_json::to_string_pretty(&cached_timeline)?;
            
            let _ = fs::write(&cache_file, updated_serialized);
        }
        
        Ok(())
    }
    
    /// Load cache metadata only
    fn load_cache_metadata(&self, cache_file: &Path) -> Result<CacheMetadata, ClaudeToolsError> {
        let content = fs::read_to_string(cache_file)?;
        let cached_data: CachedTimeline = serde_json::from_str(&content)?;
        
        Ok(cached_data.metadata)
    }
    
    /// Load full cached timeline
    fn load_cached_timeline(&self, cache_file: &Path) -> Result<CachedTimeline, ClaudeToolsError> {
        let content = fs::read_to_string(cache_file)?;
        Ok(serde_json::from_str(&content)?)
    }
    
    /// Clear all cached timelines
    pub fn clear_cache(&self) -> Result<usize, ClaudeToolsError> {
        if !self.cache_dir.exists() {
            return Ok(0);
        }
        
        let entries = fs::read_dir(&self.cache_dir)?;
        let mut cleared_count = 0;
        
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                fs::remove_file(&path)?;
                cleared_count += 1;
            }
        }
        
        Ok(cleared_count)
    }
    
    /// Get cache statistics
    pub fn cache_stats(&self) -> Result<CacheStats, ClaudeToolsError> {
        if !self.cache_dir.exists() {
            return Ok(CacheStats::default());
        }
        
        let entries = fs::read_dir(&self.cache_dir)?;
        let mut stats = CacheStats::default();
        
        for entry in entries {
            let entry = entry?;
            
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                stats.file_count += 1;
                
                if let Ok(metadata) = fs::metadata(&path) {
                    stats.total_size += metadata.len();
                    
                    if let Ok(cache_metadata) = self.load_cache_metadata(&path) {
                        if cache_metadata.created_at > stats.newest_cache {
                            stats.newest_cache = cache_metadata.created_at;
                        }
                        if stats.oldest_cache == DateTime::<Utc>::MIN_UTC || cache_metadata.created_at < stats.oldest_cache {
                            stats.oldest_cache = cache_metadata.created_at;
                        }
                    }
                }
            }
        }
        
        Ok(stats)
    }
}

/// Cache statistics
#[derive(Debug, Default)]
pub struct CacheStats {
    /// Number of cache files
    pub file_count: usize,
    /// Total size of cache files in bytes
    pub total_size: u64,
    /// Newest cache timestamp
    pub newest_cache: DateTime<Utc>,
    /// Oldest cache timestamp  
    pub oldest_cache: DateTime<Utc>,
}

impl CacheStats {
    /// Get human-readable cache size
    pub fn size_human_readable(&self) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
        let mut size = self.total_size as f64;
        let mut unit_index = 0;
        
        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }
        
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_cache_creation() {
        let temp_dir = tempdir().unwrap();
        let cache = TimelineCache::new(temp_dir.path()).unwrap();
        
        assert!(temp_dir.path().join("timeline_cache").exists());
    }
    
    #[test]
    fn test_config_hashing() {
        let temp_dir = tempdir().unwrap();
        let cache = TimelineCache::new(temp_dir.path()).unwrap();
        
        let config1 = TimelineConfig::default();
        let config2 = TimelineConfig::default();
        
        assert_eq!(cache.hash_config(&config1), cache.hash_config(&config2));
    }
    
    #[test]
    fn test_cache_file_paths() {
        let temp_dir = tempdir().unwrap();
        let cache = TimelineCache::new(temp_dir.path()).unwrap();
        
        let config = TimelineConfig::default();
        let path = cache.cache_file_path(&config);
        
        assert!(path.starts_with(temp_dir.path()));
        assert!(path.extension().unwrap() == "json");
    }
}