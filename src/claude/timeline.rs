use chrono::{DateTime, Utc, Duration, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, BTreeMap, HashSet};
use crate::claude::{Conversation, MessageRole};

/// Time periods for filtering activity timeline
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimePeriod {
    LastDay,      // Past 24 hours
    LastTwoDay,   // Past 48 hours
    LastWeek,     // Past 7 days
    LastMonth,    // Past 30 days
    Custom { start: DateTime<Utc>, end: DateTime<Utc> },
}

impl TimePeriod {
    /// Get the start time for this time period
    pub fn start_time(&self) -> DateTime<Utc> {
        let now = Utc::now();
        match self {
            TimePeriod::LastDay => now - Duration::days(1),
            TimePeriod::LastTwoDay => now - Duration::days(2),
            TimePeriod::LastWeek => now - Duration::days(7),
            TimePeriod::LastMonth => now - Duration::days(30),
            TimePeriod::Custom { start, .. } => *start,
        }
    }

    /// Get the end time for this time period
    pub fn end_time(&self) -> DateTime<Utc> {
        match self {
            TimePeriod::Custom { end, .. } => *end,
            _ => Utc::now(),
        }
    }

    /// Check if a timestamp falls within this time period
    pub fn contains(&self, timestamp: DateTime<Utc>) -> bool {
        timestamp >= self.start_time() && timestamp <= self.end_time()
    }

    /// Get a human-readable label for this time period
    pub fn label(&self) -> &'static str {
        match self {
            TimePeriod::LastDay => "Past 24 hours",
            TimePeriod::LastTwoDay => "Past 48 hours", 
            TimePeriod::LastWeek => "Past week",
            TimePeriod::LastMonth => "Past month",
            TimePeriod::Custom { .. } => "Custom range",
        }
    }
}

/// Configuration for activity timeline generation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TimelineConfig {
    /// Time period to analyze
    pub period: TimePeriod,
    /// Depth of topical summaries
    pub summary_depth: SummaryDepth,
    /// Maximum number of conversations per project to analyze
    pub max_conversations_per_project: Option<usize>,
    /// Include empty projects in results
    pub include_empty_projects: bool,
}

impl Default for TimelineConfig {
    fn default() -> Self {
        Self {
            period: TimePeriod::LastTwoDay,
            summary_depth: SummaryDepth::Brief,
            max_conversations_per_project: Some(20),
            include_empty_projects: false,
        }
    }
}

/// Depth of topical summary generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SummaryDepth {
    Brief,        // 1-2 sentences
    Detailed,     // 1-2 paragraphs  
    Comprehensive, // Full analysis
}

/// Main activity timeline structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityTimeline {
    /// Configuration used to generate this timeline
    pub config: TimelineConfig,
    /// Generated timestamp
    pub generated_at: DateTime<Utc>,
    /// Activity grouped by project
    pub projects: BTreeMap<String, ProjectActivity>,
    /// Overall timeline statistics
    pub total_stats: TimelineStats,
    /// Temporal index for fast time-based lookups
    pub temporal_index: TemporalIndex,
}

/// Activity within a specific project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectActivity {
    /// Project path/name
    pub project_path: String,
    /// Conversations within the time period
    pub conversations: Vec<ConversationSummary>,
    /// Topical summary of project activity
    pub topical_summary: TopicalSummary,
    /// Project-specific statistics
    pub stats: ProjectStats,
    /// Most recent activity timestamp
    pub last_activity: Option<DateTime<Utc>>,
    /// Visual indicators for this project
    pub indicators: ActivityIndicators,
}

/// Summary of a conversation for timeline display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummary {
    /// Session ID
    pub session_id: String,
    /// Conversation title/summary
    pub title: Option<String>,
    /// Start time
    pub started_at: DateTime<Utc>,
    /// End time
    pub ended_at: Option<DateTime<Utc>>,
    /// Number of messages
    pub message_count: usize,
    /// Number of user messages
    pub user_message_count: usize,
    /// Number of assistant messages
    pub assistant_message_count: usize,
    /// Tool usage count
    pub tool_usage_count: usize,
    /// Main topics discussed
    pub topics: Vec<String>,
    /// Brief content summary
    pub content_summary: String,
}

/// Topical summary of project activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicalSummary {
    /// Key topics identified
    pub main_topics: Vec<Topic>,
    /// Overall project activity summary
    pub summary_text: String,
    /// Most frequently used tools
    pub frequent_tools: Vec<String>,
    /// Activity intensity (High, Medium, Low)
    pub intensity: ActivityIntensity,
}

/// A topic identified in project activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topic {
    /// Topic name/keyword
    pub name: String,
    /// Frequency of mention
    pub frequency: usize,
    /// Relevance score (0.0 to 1.0)
    pub relevance: f64,
    /// Sample context where this topic appears
    pub context_sample: Option<String>,
}

/// Activity intensity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityIntensity {
    Low,    // < 5 messages
    Medium, // 5-20 messages
    High,   // > 20 messages
}

/// Visual indicators for activity representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityIndicators {
    /// Progress bar representation (0.0 to 1.0)
    pub progress_bar: f64,
    /// Bar chart segments for visualization
    pub bar_segments: Vec<BarSegment>,
    /// Activity trend (increasing, stable, decreasing)
    pub trend: ActivityTrend,
    /// Sparkline data points for mini charts
    pub sparkline_data: Vec<u32>,
    /// Comparative ranking indicator
    pub ranking_indicator: RankingIndicator,
}

/// Individual bar segment for visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarSegment {
    /// Segment value (0.0 to 1.0)
    pub value: f64,
    /// Segment type (user_messages, assistant_messages, tools)
    pub segment_type: SegmentType,
    /// Color hint for rendering
    pub color_hint: String,
}

/// Types of bar segments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SegmentType {
    UserMessages,
    AssistantMessages,
    ToolUsage,
    Conversations,
}

/// Activity trend indicators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityTrend {
    Increasing,
    Stable,
    Decreasing,
    NoData,
}

/// Ranking indicator for comparative display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RankingIndicator {
    Top { rank: usize, total: usize },
    Middle { rank: usize, total: usize },
    Bottom { rank: usize, total: usize },
    Unranked,
}

/// Project-specific statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectStats {
    /// Total conversations in period
    pub conversation_count: usize,
    /// Total messages in period
    pub total_messages: usize,
    /// Messages by role
    pub messages_by_role: HashMap<MessageRole, usize>,
    /// Average conversation length
    pub avg_conversation_length: f64,
    /// Most active time of day (hour 0-23)
    pub peak_hour: Option<u8>,
    /// Tool usage frequency
    pub tool_usage: HashMap<String, usize>,
    /// Daily conversation frequency (conversations per day)
    pub conversation_frequency: f64,
    /// Message frequency (messages per day)
    pub message_frequency: f64,
    /// Activity score relative to other projects (0.0 to 1.0)
    pub activity_score: f64,
    /// Most frequently used tools (name, count) - sorted by usage
    pub top_tools: Vec<(String, usize)>,
    /// User to assistant message ratio
    pub user_assistant_ratio: f64,
    /// Average session duration in minutes
    pub avg_session_duration: Option<f64>,
    /// Activity rank compared to other projects (1-based)
    pub activity_rank: Option<usize>,
}

/// Overall timeline statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimelineStats {
    /// Total projects with activity
    pub active_projects: usize,
    /// Total conversations across all projects
    pub total_conversations: usize,
    /// Total messages across all projects
    pub total_messages: usize,
    /// Most active project
    pub most_active_project: Option<String>,
    /// Messages per day average
    pub messages_per_day: f64,
    /// Peak activity day
    pub peak_activity_day: Option<DateTime<Utc>>,
    /// Project activity ranking (project_name, activity_score)
    pub project_rankings: Vec<(String, f64)>,
    /// Total tool usage across all projects
    pub total_tool_usage: usize,
    /// Most popular tools across all projects
    pub global_top_tools: Vec<(String, usize)>,
    /// Average conversation length across all projects
    pub global_avg_conversation_length: f64,
    /// Overall user to assistant message ratio
    pub global_user_assistant_ratio: f64,
}

/// Temporal indexing for fast time-based queries
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TemporalIndex {
    /// Conversations indexed by day
    pub by_day: BTreeMap<String, Vec<String>>, // Date -> Session IDs
    /// Conversations indexed by hour
    pub by_hour: BTreeMap<u8, Vec<String>>, // Hour (0-23) -> Session IDs
    /// Activity heatmap data
    pub activity_heatmap: HashMap<String, usize>, // Date -> Message count
}

impl ActivityTimeline {
    /// Create a new empty activity timeline
    pub fn new(config: TimelineConfig) -> Self {
        Self {
            config,
            generated_at: Utc::now(),
            projects: BTreeMap::new(),
            total_stats: TimelineStats::default(),
            temporal_index: TemporalIndex::default(),
        }
    }

    /// Filter conversations by time period
    pub fn filter_conversations_by_time_period(
        conversations: &[Conversation],
        period: TimePeriod,
    ) -> Vec<&Conversation> {
        let start_time = period.start_time();
        let end_time = period.end_time();

        conversations
            .iter()
            .filter(|conv| {
                // Check if conversation has activity within the time period
                if let Some(last_updated) = conv.last_updated {
                    last_updated >= start_time && last_updated <= end_time
                } else if let Some(started_at) = conv.started_at {
                    started_at >= start_time && started_at <= end_time
                } else {
                    // If no timestamps, check message timestamps
                    conv.messages.iter().any(|msg| {
                        msg.timestamp >= start_time && msg.timestamp <= end_time
                    })
                }
            })
            .collect()
    }

    /// Group conversations by project path
    pub fn group_conversations_by_project(
        conversations: Vec<&Conversation>,
    ) -> HashMap<String, Vec<&Conversation>> {
        let mut project_groups: HashMap<String, Vec<&Conversation>> = HashMap::new();

        for conversation in conversations {
            let project_path = normalize_project_path(&conversation.project_path);
            project_groups
                .entry(project_path)
                .or_default()
                .push(conversation);
        }

        project_groups
    }

    /// Create timeline with advanced filtering options
    pub fn create_filtered_timeline(
        all_conversations: Vec<Conversation>,
        config: TimelineConfig,
    ) -> Self {
        let mut timeline = Self::new(config.clone());

        // Step 1: Filter by time period
        let filtered_convs = Self::filter_conversations_by_time_period(&all_conversations, config.period);
        
        // Step 2: Group by project
        let project_groups = Self::group_conversations_by_project(filtered_convs);

        // Step 3: Process each project group
        for (project_path, conversations) in project_groups {
            if conversations.is_empty() && !config.include_empty_projects {
                continue;
            }

            // Apply max conversations limit if specified
            let limited_conversations: Vec<_> = if let Some(max_count) = config.max_conversations_per_project {
                // Sort by most recent first, then take limit
                let mut sorted_convs = conversations;
                sorted_convs.sort_by(|a, b| {
                    let a_time = a.last_updated.unwrap_or_else(|| a.started_at.unwrap_or_else(Utc::now));
                    let b_time = b.last_updated.unwrap_or_else(|| b.started_at.unwrap_or_else(Utc::now));
                    b_time.cmp(&a_time) // Most recent first
                });
                sorted_convs.into_iter().take(max_count).cloned().collect()
            } else {
                conversations.into_iter().cloned().collect()
            };

            let project_activity = ProjectActivity::from_conversations(
                project_path.clone(),
                limited_conversations,
                config.summary_depth,
            );

            timeline.projects.insert(project_path, project_activity);
        }

        // Step 4: Generate statistics and build indexes
        timeline.generate_statistics();
        timeline.build_temporal_index();

        timeline
    }

    /// Generate activity timeline from conversations (legacy method - use create_filtered_timeline for new code)
    pub fn from_conversations(
        conversations: Vec<Conversation>,
        config: TimelineConfig,
    ) -> Self {
        // Use the new advanced filtering method
        Self::create_filtered_timeline(conversations, config)
    }

    /// Get projects sorted by activity level (most active first)
    pub fn projects_by_activity(&self) -> Vec<&ProjectActivity> {
        let mut projects: Vec<_> = self.projects.values().collect();
        projects.sort_by(|a, b| {
            b.stats.total_messages.cmp(&a.stats.total_messages)
                .then_with(|| b.stats.conversation_count.cmp(&a.stats.conversation_count))
        });
        projects
    }

    /// Get conversations from a specific project within the time period
    pub fn get_project_conversations(&self, project_path: &str) -> Option<&ProjectActivity> {
        self.projects.get(project_path)
    }

    /// Get activity for a specific day
    pub fn get_daily_activity(&self, date: &str) -> Vec<String> {
        self.temporal_index
            .by_day
            .get(date)
            .cloned()
            .unwrap_or_default()
    }

    /// Generate overall timeline statistics
    fn generate_statistics(&mut self) {
        let mut stats = TimelineStats::default();
        
        stats.active_projects = self.projects.len();
        
        let mut most_active_count = 0;
        let mut most_active_project = None;
        let mut total_conversations = 0;
        let mut total_messages = 0;
        let mut global_tool_counts: HashMap<String, usize> = HashMap::new();
        let mut total_user_messages = 0;
        let mut total_assistant_messages = 0;
        let mut all_conversation_lengths = Vec::new();

        // Calculate time period in days
        let days = match self.config.period {
            TimePeriod::LastDay => 1.0,
            TimePeriod::LastTwoDay => 2.0,
            TimePeriod::LastWeek => 7.0,
            TimePeriod::LastMonth => 30.0,
            TimePeriod::Custom { start, end } => {
                (end - start).num_days() as f64
            }
        };

        // First pass: collect basic statistics
        for (project_path, project) in &self.projects {
            total_conversations += project.stats.conversation_count;
            total_messages += project.stats.total_messages;

            if project.stats.total_messages > most_active_count {
                most_active_count = project.stats.total_messages;
                most_active_project = Some(project_path.clone());
            }

            // Aggregate tool usage
            for (tool_name, count) in &project.stats.tool_usage {
                *global_tool_counts.entry(tool_name.clone()).or_default() += count;
            }

            // Aggregate message counts by role
            total_user_messages += project.stats.messages_by_role.get(&MessageRole::User).unwrap_or(&0);
            total_assistant_messages += project.stats.messages_by_role.get(&MessageRole::Assistant).unwrap_or(&0);

            // Collect conversation lengths
            for conv in &project.conversations {
                all_conversation_lengths.push(conv.message_count);
            }
        }

        // Calculate global metrics
        stats.total_conversations = total_conversations;
        stats.total_messages = total_messages;
        stats.most_active_project = most_active_project;
        stats.total_tool_usage = global_tool_counts.values().sum();
        
        // Create global top tools list
        let mut global_tools: Vec<_> = global_tool_counts.into_iter().collect();
        global_tools.sort_by(|a, b| b.1.cmp(&a.1));
        stats.global_top_tools = global_tools.into_iter().take(10).collect();

        // Calculate global conversation length average
        stats.global_avg_conversation_length = if !all_conversation_lengths.is_empty() {
            all_conversation_lengths.iter().sum::<usize>() as f64 / all_conversation_lengths.len() as f64
        } else {
            0.0
        };

        // Calculate global user/assistant ratio
        stats.global_user_assistant_ratio = if total_assistant_messages > 0 {
            total_user_messages as f64 / total_assistant_messages as f64
        } else {
            total_user_messages as f64
        };

        stats.messages_per_day = if days > 0.0 {
            total_messages as f64 / days
        } else {
            0.0
        };

        // Second pass: calculate comparative metrics and visual indicators
        let max_messages = self.projects.values().map(|p| p.stats.total_messages).max().unwrap_or(1);
        let mut project_names_and_messages: Vec<_> = self.projects.iter()
            .map(|(name, project)| (name.clone(), project.stats.total_messages))
            .collect();
        project_names_and_messages.sort_by(|a, b| b.1.cmp(&a.1));

        // Create project rankings
        stats.project_rankings = project_names_and_messages
            .iter()
            .enumerate()
            .map(|(_rank, (name, messages))| {
                let activity_score = *messages as f64 / max_messages as f64;
                (name.clone(), activity_score)
            })
            .collect();

        // Collect all the updates to apply later
        let mut project_updates = Vec::new();
        for (rank, (project_name, _)) in project_names_and_messages.iter().enumerate() {
            if let Some(project) = self.projects.get(project_name) {
                let conversation_frequency = project.stats.conversation_count as f64 / days;
                let message_frequency = project.stats.total_messages as f64 / days;
                let activity_score = project.stats.total_messages as f64 / max_messages as f64;
                let activity_rank = Some(rank + 1);
                
                // Generate indicators with current project data
                let indicators = self.generate_activity_indicators_for_stats(
                    &project.stats, 
                    &project.conversations,
                    rank + 1, 
                    project_names_and_messages.len()
                );

                project_updates.push((
                    project_name.clone(),
                    conversation_frequency,
                    message_frequency,
                    activity_score,
                    activity_rank,
                    indicators,
                ));
            }
        }

        // Apply updates
        for (project_name, conv_freq, msg_freq, act_score, act_rank, indicators) in project_updates {
            if let Some(project) = self.projects.get_mut(&project_name) {
                project.stats.conversation_frequency = conv_freq;
                project.stats.message_frequency = msg_freq;
                project.stats.activity_score = act_score;
                project.stats.activity_rank = act_rank;
                project.indicators = indicators;
            }
        }

        self.total_stats = stats;
    }

    /// Build temporal index for fast time-based queries
    fn build_temporal_index(&mut self) {
        let mut temporal_index = TemporalIndex::default();

        for project in self.projects.values() {
            for conv_summary in &project.conversations {
                let session_id = conv_summary.session_id.clone();
                let date = conv_summary.started_at.format("%Y-%m-%d").to_string();
                let hour = conv_summary.started_at.hour() as u8;

                // Index by day
                temporal_index
                    .by_day
                    .entry(date.clone())
                    .or_default()
                    .push(session_id.clone());

                // Index by hour
                temporal_index
                    .by_hour
                    .entry(hour)
                    .or_default()
                    .push(session_id);

                // Update activity heatmap
                *temporal_index
                    .activity_heatmap
                    .entry(date)
                    .or_default() += conv_summary.message_count;
            }
        }

        self.temporal_index = temporal_index;
    }

    /// Generate visual activity indicators for a project (using project stats and conversations)
    fn generate_activity_indicators_for_stats(
        &self, 
        stats: &ProjectStats, 
        conversations: &[ConversationSummary],
        rank: usize, 
        total_projects: usize
    ) -> ActivityIndicators {
        // Calculate progress bar value (activity score)
        let progress_bar = stats.activity_score;

        // Create bar segments for visualization
        let user_messages = stats.messages_by_role.get(&MessageRole::User).unwrap_or(&0);
        let assistant_messages = stats.messages_by_role.get(&MessageRole::Assistant).unwrap_or(&0);
        let total_msgs = stats.total_messages;
        let total_tools: usize = stats.tool_usage.values().sum();

        let mut bar_segments = Vec::new();
        
        if total_msgs > 0 {
            bar_segments.push(BarSegment {
                value: *user_messages as f64 / total_msgs as f64,
                segment_type: SegmentType::UserMessages,
                color_hint: "blue".to_string(),
            });
            
            bar_segments.push(BarSegment {
                value: *assistant_messages as f64 / total_msgs as f64,
                segment_type: SegmentType::AssistantMessages,
                color_hint: "green".to_string(),
            });
            
            if total_tools > 0 {
                bar_segments.push(BarSegment {
                    value: total_tools as f64 / total_msgs as f64,
                    segment_type: SegmentType::ToolUsage,
                    color_hint: "yellow".to_string(),
                });
            }
        }

        // Generate sparkline data (daily message counts)
        let sparkline_data = self.generate_sparkline_data_from_conversations(conversations);

        // Determine activity trend
        let trend = self.calculate_activity_trend(&sparkline_data);

        // Create ranking indicator
        let ranking_indicator = match rank {
            1..=3 if total_projects > 3 => RankingIndicator::Top { rank, total: total_projects },
            r if r > total_projects * 2 / 3 => RankingIndicator::Bottom { rank, total: total_projects },
            _ => RankingIndicator::Middle { rank, total: total_projects },
        };

        ActivityIndicators {
            progress_bar,
            bar_segments,
            trend,
            sparkline_data,
            ranking_indicator,
        }
    }

    /// Generate visual activity indicators for a project (legacy method)
    fn generate_activity_indicators(&self, project: &ProjectActivity, rank: usize, total_projects: usize) -> ActivityIndicators {
        // Calculate progress bar value (activity score)
        let progress_bar = project.stats.activity_score;

        // Create bar segments for visualization
        let user_messages = project.stats.messages_by_role.get(&MessageRole::User).unwrap_or(&0);
        let assistant_messages = project.stats.messages_by_role.get(&MessageRole::Assistant).unwrap_or(&0);
        let total_msgs = project.stats.total_messages;
        let total_tools: usize = project.stats.tool_usage.values().sum();

        let mut bar_segments = Vec::new();
        
        if total_msgs > 0 {
            bar_segments.push(BarSegment {
                value: *user_messages as f64 / total_msgs as f64,
                segment_type: SegmentType::UserMessages,
                color_hint: "blue".to_string(),
            });
            
            bar_segments.push(BarSegment {
                value: *assistant_messages as f64 / total_msgs as f64,
                segment_type: SegmentType::AssistantMessages,
                color_hint: "green".to_string(),
            });
            
            if total_tools > 0 {
                bar_segments.push(BarSegment {
                    value: total_tools as f64 / total_msgs as f64,
                    segment_type: SegmentType::ToolUsage,
                    color_hint: "yellow".to_string(),
                });
            }
        }

        // Generate sparkline data (daily message counts)
        let sparkline_data = self.generate_sparkline_data(project);

        // Determine activity trend
        let trend = self.calculate_activity_trend(&sparkline_data);

        // Create ranking indicator
        let ranking_indicator = match rank {
            1..=3 if total_projects > 3 => RankingIndicator::Top { rank, total: total_projects },
            r if r > total_projects * 2 / 3 => RankingIndicator::Bottom { rank, total: total_projects },
            _ => RankingIndicator::Middle { rank, total: total_projects },
        };

        ActivityIndicators {
            progress_bar,
            bar_segments,
            trend,
            sparkline_data,
            ranking_indicator,
        }
    }

    /// Generate sparkline data showing daily activity from conversations
    fn generate_sparkline_data_from_conversations(&self, conversations: &[ConversationSummary]) -> Vec<u32> {
        // Create daily message count data for the last 7 days
        let mut daily_counts = vec![0u32; 7];
        let now = Utc::now();

        for conv in conversations {
            let days_ago = (now - conv.started_at).num_days();
            if days_ago >= 0 && days_ago < 7 {
                let index = (6 - days_ago) as usize; // Reverse order (most recent first)
                daily_counts[index] += conv.message_count as u32;
            }
        }

        daily_counts
    }

    /// Generate sparkline data showing daily activity (legacy method)
    fn generate_sparkline_data(&self, project: &ProjectActivity) -> Vec<u32> {
        self.generate_sparkline_data_from_conversations(&project.conversations)
    }

    /// Calculate activity trend from sparkline data
    fn calculate_activity_trend(&self, sparkline_data: &[u32]) -> ActivityTrend {
        if sparkline_data.len() < 3 {
            return ActivityTrend::NoData;
        }

        let recent = sparkline_data.iter().rev().take(3).sum::<u32>() as f64;
        let earlier = sparkline_data.iter().take(3).sum::<u32>() as f64;

        if recent > earlier * 1.2 {
            ActivityTrend::Increasing
        } else if recent < earlier * 0.8 {
            ActivityTrend::Decreasing
        } else {
            ActivityTrend::Stable
        }
    }

    /// Efficiently filter large datasets using parallel processing
    pub fn filter_conversations_parallel(
        conversations: &[Conversation],
        period: TimePeriod,
        batch_size: usize,
    ) -> Vec<&Conversation> {
        if conversations.len() < batch_size {
            // For small datasets, use sequential filtering
            return Self::filter_conversations_by_time_period(conversations, period);
        }

        // Process in batches for memory efficiency
        let mut results = Vec::new();
        for chunk in conversations.chunks(batch_size) {
            let mut chunk_results = Self::filter_conversations_by_time_period(chunk, period);
            results.append(&mut chunk_results);
        }
        
        results
    }

    /// Filter conversations with multiple criteria
    pub fn filter_conversations_advanced<'a>(
        conversations: &'a [Conversation],
        period: TimePeriod,
        min_messages: Option<usize>,
        project_filter: Option<&str>,
        has_tools: Option<bool>,
    ) -> Vec<&'a Conversation> {
        conversations
            .iter()
            .filter(|conv| {
                // Time period filter
                let in_time_period = if let Some(last_updated) = conv.last_updated {
                    period.contains(last_updated)
                } else if let Some(started_at) = conv.started_at {
                    period.contains(started_at)
                } else {
                    conv.messages.iter().any(|msg| period.contains(msg.timestamp))
                };

                if !in_time_period {
                    return false;
                }

                // Message count filter
                if let Some(min_msg_count) = min_messages {
                    if conv.messages.len() < min_msg_count {
                        return false;
                    }
                }

                // Project filter
                if let Some(project_pattern) = project_filter {
                    if !conv.project_path.contains(project_pattern) {
                        return false;
                    }
                }

                // Tool usage filter
                if let Some(requires_tools) = has_tools {
                    let has_tool_usage = conv.messages.iter().any(|msg| !msg.tool_uses.is_empty());
                    if requires_tools != has_tool_usage {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    /// Get timeline statistics for a specific time range
    pub fn get_time_range_stats(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> TimelineStats {
        let custom_period = TimePeriod::Custom { start, end };
        let mut stats = TimelineStats::default();

        for project in self.projects.values() {
            for conv_summary in &project.conversations {
                if custom_period.contains(conv_summary.started_at) {
                    stats.total_conversations += 1;
                    stats.total_messages += conv_summary.message_count;
                }
            }
        }

        // Calculate messages per day for the custom period
        let days = (end - start).num_days() as f64;
        stats.messages_per_day = if days > 0.0 {
            stats.total_messages as f64 / days
        } else {
            0.0
        };

        stats
    }

    /// Filter existing timeline to a shorter time period (more efficient than regenerating)
    /// This method can only filter to a shorter or equal time period than the current one
    pub fn filter_to_period(&self, new_period: TimePeriod) -> Result<Self, String> {
        // Check if the new period is within the current period bounds
        let current_start = self.config.period.start_time();
        let current_end = self.config.period.end_time();
        let new_start = new_period.start_time();
        let new_end = new_period.end_time();

        if new_start < current_start || new_end > current_end {
            return Err("Cannot filter to a period larger than the current timeline period".to_string());
        }

        // Create new timeline with filtered data
        let new_config = TimelineConfig {
            period: new_period,
            ..self.config.clone()
        };

        let mut filtered_timeline = Self::new(new_config);

        // Filter each project's data
        for (project_path, project_activity) in &self.projects {
            let filtered_conversations: Vec<_> = project_activity
                .conversations
                .iter()
                .filter(|conv| {
                    // Check if conversation activity falls within the new time period
                    new_period.contains(conv.started_at) ||
                    new_period.contains(conv.ended_at.unwrap_or(conv.started_at))
                })
                .cloned()
                .collect();

            if !filtered_conversations.is_empty() || self.config.include_empty_projects {
                // Create filtered project activity
                let mut filtered_project = ProjectActivity {
                    project_path: project_path.clone(),
                    conversations: filtered_conversations,
                    stats: ProjectStats::default(),
                    topical_summary: project_activity.topical_summary.clone(),
                    indicators: project_activity.indicators.clone(),
                    last_activity: project_activity.last_activity,
                };

                // Recalculate stats for the filtered conversations
                filtered_project.recalculate_stats(&new_period);

                filtered_timeline.projects.insert(project_path.clone(), filtered_project);
            }
        }

        // Update temporal index for filtered period
        filtered_timeline.update_temporal_index();

        // Generate statistics for the filtered timeline
        filtered_timeline.generate_statistics();

        Ok(filtered_timeline)
    }

    /// Update temporal index based on current conversations
    fn update_temporal_index(&mut self) {
        let mut temporal_index = TemporalIndex::default();

        for project_activity in self.projects.values() {
            for conv_summary in &project_activity.conversations {
                // Index by day
                let day_key = conv_summary.started_at.format("%Y-%m-%d").to_string();
                temporal_index
                    .by_day
                    .entry(day_key.clone())
                    .or_default()
                    .push(conv_summary.session_id.clone());

                // Index by hour
                let hour = conv_summary.started_at.hour() as u8;
                temporal_index
                    .by_hour
                    .entry(hour)
                    .or_default()
                    .push(conv_summary.session_id.clone());

                // Update activity heatmap
                *temporal_index.activity_heatmap.entry(day_key).or_default() += conv_summary.message_count;
            }
        }

        self.temporal_index = temporal_index;
    }
}

impl ProjectActivity {
    /// Create project activity from conversations
    pub fn from_conversations(
        project_path: String,
        conversations: Vec<Conversation>,
        summary_depth: SummaryDepth,
    ) -> Self {
        let conversation_summaries: Vec<_> = conversations
            .iter()
            .map(ConversationSummary::from_conversation)
            .collect();

        let stats = ProjectStats::from_conversations(&conversations);
        let topical_summary = TopicalSummary::from_conversations(&conversations, summary_depth);
        
        let last_activity = conversations
            .iter()
            .filter_map(|c| c.last_updated)
            .max();

        // Create visual indicators (placeholder - will be calculated in second pass)
        let indicators = ActivityIndicators {
            progress_bar: 0.0,
            bar_segments: vec![],
            trend: ActivityTrend::NoData,
            sparkline_data: vec![],
            ranking_indicator: RankingIndicator::Unranked,
        };

        Self {
            project_path,
            conversations: conversation_summaries,
            topical_summary,
            stats,
            last_activity,
            indicators,
        }
    }

    /// Recalculate project statistics for filtered conversations
    pub fn recalculate_stats(&mut self, time_period: &TimePeriod) {
        // Reset stats
        self.stats = ProjectStats::default();
        
        self.stats.conversation_count = self.conversations.len();
        
        let mut total_length = 0;
        let mut hour_counts: HashMap<u8, usize> = HashMap::new();
        let mut _tool_counts: HashMap<String, usize> = HashMap::new();

        for conv_summary in &self.conversations {
            total_length += conv_summary.message_count;
            self.stats.total_messages += conv_summary.message_count;

            // Count messages by role (approximate from conversation summary)
            *self.stats.messages_by_role.entry(MessageRole::User).or_default() += conv_summary.user_message_count;
            *self.stats.messages_by_role.entry(MessageRole::Assistant).or_default() += conv_summary.assistant_message_count;

            // Track hour activity
            let hour = conv_summary.started_at.hour() as u8;
            *hour_counts.entry(hour).or_default() += conv_summary.message_count;

            // Count tool usage
            self.stats.total_messages += conv_summary.tool_usage_count;
        }

        // Calculate derived statistics
        self.stats.avg_conversation_length = if self.conversations.is_empty() {
            0.0
        } else {
            total_length as f64 / self.conversations.len() as f64
        };

        // Find peak hour
        self.stats.peak_hour = hour_counts
            .iter()
            .max_by_key(|(_, &count)| count)
            .map(|(&hour, _)| hour);

        // Calculate frequencies based on the time period
        let period_days = match time_period {
            TimePeriod::LastDay => 1.0,
            TimePeriod::LastTwoDay => 2.0,
            TimePeriod::LastWeek => 7.0,
            TimePeriod::LastMonth => 30.0,
            TimePeriod::Custom { start, end } => (end.signed_duration_since(*start).num_days() as f64).max(1.0),
        };

        self.stats.conversation_frequency = self.conversations.len() as f64 / period_days;
        self.stats.message_frequency = self.stats.total_messages as f64 / period_days;

        // Calculate user to assistant ratio
        let user_count = *self.stats.messages_by_role.get(&MessageRole::User).unwrap_or(&0) as f64;
        let assistant_count = *self.stats.messages_by_role.get(&MessageRole::Assistant).unwrap_or(&0) as f64;
        
        self.stats.user_assistant_ratio = if assistant_count > 0.0 {
            user_count / assistant_count
        } else {
            0.0
        };
    }
}

impl ConversationSummary {
    /// Create conversation summary from full conversation
    pub fn from_conversation(conversation: &Conversation) -> Self {
        let topics = extract_topics_from_conversation(conversation);
        let content_summary = generate_content_summary(conversation);

        Self {
            session_id: conversation.session_id.clone(),
            title: conversation.summary.clone(),
            started_at: conversation.started_at.unwrap_or_else(Utc::now),
            ended_at: conversation.last_updated,
            message_count: conversation.messages.len(),
            user_message_count: conversation.user_message_count(),
            assistant_message_count: conversation.assistant_message_count(),
            tool_usage_count: conversation.messages.iter()
                .map(|m| m.tool_uses.len())
                .sum(),
            topics,
            content_summary,
        }
    }

}

impl ProjectStats {
    /// Generate project statistics from conversations
    pub fn from_conversations(conversations: &[Conversation]) -> Self {
        let mut stats = Self::default();
        
        stats.conversation_count = conversations.len();
        
        let mut total_length = 0;
        let mut hour_counts: HashMap<u8, usize> = HashMap::new();
        let mut tool_counts: HashMap<String, usize> = HashMap::new();

        for conversation in conversations {
            total_length += conversation.messages.len();
            stats.total_messages += conversation.messages.len();

            // Count messages by role
            for message in &conversation.messages {
                *stats.messages_by_role.entry(message.role.clone()).or_default() += 1;

                // Track hour activity
                let hour = message.timestamp.hour() as u8;
                *hour_counts.entry(hour).or_default() += 1;

                // Count tool usage
                for tool_use in &message.tool_uses {
                    *tool_counts.entry(tool_use.name.clone()).or_default() += 1;
                }
            }
        }

        // Calculate average conversation length
        stats.avg_conversation_length = if stats.conversation_count > 0 {
            total_length as f64 / stats.conversation_count as f64
        } else {
            0.0
        };

        // Find peak hour
        stats.peak_hour = hour_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(hour, _)| hour);

        stats.tool_usage = tool_counts.clone();

        // Calculate frequency metrics
        let period_days = 1.0; // Default to 1 day, will be updated in second pass
        stats.conversation_frequency = stats.conversation_count as f64 / period_days;
        stats.message_frequency = stats.total_messages as f64 / period_days;

        // Calculate user to assistant ratio
        let user_messages = stats.messages_by_role.get(&MessageRole::User).unwrap_or(&0);
        let assistant_messages = stats.messages_by_role.get(&MessageRole::Assistant).unwrap_or(&0);
        stats.user_assistant_ratio = if *assistant_messages > 0 {
            *user_messages as f64 / *assistant_messages as f64
        } else {
            *user_messages as f64
        };

        // Create sorted top tools list
        let mut tool_vec: Vec<_> = tool_counts.into_iter().collect();
        tool_vec.sort_by(|a, b| b.1.cmp(&a.1));
        stats.top_tools = tool_vec.into_iter().take(5).collect();

        // Calculate average session duration
        stats.avg_session_duration = calculate_avg_session_duration(conversations);

        // Placeholder values for metrics requiring cross-project comparison
        stats.activity_score = 0.0;
        stats.activity_rank = None;

        stats
    }
}

impl TopicalSummary {
    /// Generate topical summary from conversations
    pub fn from_conversations(
        conversations: &[Conversation],
        depth: SummaryDepth,
    ) -> Self {
        let main_topics = extract_topics_from_conversations(conversations);
        let summary_text = generate_project_summary(conversations, depth);
        let frequent_tools = extract_frequent_tools(conversations);
        
        let total_messages: usize = conversations.iter()
            .map(|c| c.messages.len())
            .sum();

        let intensity = match total_messages {
            0..=4 => ActivityIntensity::Low,
            5..=20 => ActivityIntensity::Medium,
            _ => ActivityIntensity::High,
        };

        Self {
            main_topics,
            summary_text,
            frequent_tools,
            intensity,
        }
    }
}

/// Advanced text analyzer for topic extraction and summarization
struct TextAnalyzer {
    // Common stop words to filter out
    stop_words: HashSet<String>,
    // Technical terms that are often relevant
    technical_terms: HashSet<String>,
}

impl TextAnalyzer {
    fn new() -> Self {
        let stop_words = [
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with",
            "by", "from", "as", "is", "was", "are", "were", "be", "been", "being", "have", "has",
            "had", "do", "does", "did", "will", "would", "could", "should", "may", "might", "must",
            "can", "this", "that", "these", "those", "i", "you", "he", "she", "it", "we", "they",
            "me", "him", "her", "us", "them", "my", "your", "his", "her", "its", "our", "their",
            "what", "which", "who", "when", "where", "why", "how", "all", "any", "some", "no",
            "not", "very", "so", "just", "now", "then", "here", "there", "up", "out", "if", "about",
            "into", "over", "after", "more", "most", "other", "such", "only", "own", "same", "too",
            "each", "much", "many", "most", "more", "get", "go", "make", "take", "come", "see",
            "know", "think", "look", "want", "give", "use", "find", "tell", "ask", "work", "seem",
            "feel", "try", "leave", "call"
        ].iter().map(|&s| s.to_string()).collect();

        let technical_terms = [
            "rust", "python", "javascript", "typescript", "java", "cpp", "csharp", "go", "kotlin",
            "swift", "ruby", "php", "html", "css", "sql", "nosql", "mongodb", "postgresql", "mysql",
            "redis", "elasticsearch", "docker", "kubernetes", "aws", "azure", "gcp", "terraform",
            "ansible", "jenkins", "gitlab", "github", "git", "svn", "mercurial", "linux", "ubuntu",
            "centos", "debian", "windows", "macos", "ios", "android", "react", "vue", "angular",
            "node", "express", "django", "flask", "spring", "laravel", "rails", "django", "fastapi",
            "api", "rest", "graphql", "microservices", "serverless", "lambda", "function", "service",
            "database", "cache", "queue", "message", "event", "stream", "batch", "pipeline", "etl",
            "ml", "ai", "neural", "model", "algorithm", "data", "analytics", "visualization"
        ].iter().map(|&s| s.to_string()).collect();

        Self { stop_words, technical_terms }
    }

    /// Extract key phrases from text using frequency and position analysis
    fn extract_key_phrases(&self, text: &str, max_phrases: usize) -> Vec<String> {
        let cleaned_text = self.clean_text(text);
        let words = self.tokenize(&cleaned_text);
        
        // Extract single word topics
        let mut word_scores = HashMap::new();
        let total_words = words.len() as f64;
        
        for (pos, word) in words.iter().enumerate() {
            if self.is_meaningful_word(word) {
                let position_weight = 1.0 - (pos as f64 / total_words * 0.3); // Earlier words are more important
                let technical_bonus = if self.technical_terms.contains(word) { 2.0 } else { 1.0 };
                let length_bonus = if word.len() > 6 { 1.5 } else { 1.0 };
                
                *word_scores.entry(word.clone()).or_default() += position_weight * technical_bonus * length_bonus;
            }
        }
        
        // Extract two-word phrases
        let mut phrase_scores = HashMap::new();
        for window in words.windows(2) {
            if window.len() == 2 && self.is_meaningful_word(&window[0]) && self.is_meaningful_word(&window[1]) {
                let phrase = format!("{} {}", window[0], window[1]);
                *phrase_scores.entry(phrase).or_default() += 1.5; // Phrases are often more meaningful
            }
        }
        
        // Combine and rank results
        let mut all_topics: Vec<(String, f64)> = word_scores.into_iter()
            .chain(phrase_scores.into_iter())
            .collect();
            
        all_topics.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        all_topics.into_iter()
            .take(max_phrases)
            .map(|(phrase, _)| phrase)
            .collect()
    }

    /// Extract topics using TF-IDF analysis across multiple documents
    fn extract_topics_with_tfidf(
        &self, 
        documents: &[String], 
        _contexts: &HashMap<String, String>,
        max_topics: usize
    ) -> Vec<Topic> {
        let mut term_frequencies = HashMap::new();
        let mut document_frequencies: HashMap<String, usize> = HashMap::new();
        let total_docs = documents.len() as f64;
        
        // Calculate term frequencies and document frequencies
        for doc in documents {
            let words = self.tokenize(&self.clean_text(doc));
            let mut doc_terms = HashSet::new();
            
            for word in &words {
                if self.is_meaningful_word(word) {
                    *term_frequencies.entry(word.clone()).or_default() += 1;
                    doc_terms.insert(word.clone());
                }
            }
            
            // Count document frequency (how many documents contain each term)
            for term in doc_terms {
                *document_frequencies.entry(term).or_default() += 1;
            }
        }
        
        // Calculate TF-IDF scores
        let mut tfidf_scores = HashMap::new();
        for (term, tf) in &term_frequencies {
            if let Some(&df) = document_frequencies.get(term) {
                let idf = (total_docs / df as f64).ln();
                let tfidf = *tf as f64 * idf;
                
                // Apply additional scoring factors
                let technical_bonus = if self.technical_terms.contains(term) { 2.0 } else { 1.0 };
                let length_bonus = if term.len() > 6 { 1.3 } else { 1.0 };
                
                tfidf_scores.insert(term.clone(), tfidf * technical_bonus * length_bonus);
            }
        }
        
        // Convert to Topic structures
        let mut topics: Vec<Topic> = tfidf_scores.into_iter()
            .map(|(term, score)| {
                let frequency = term_frequencies.get(&term).copied().unwrap_or(0);
                Topic {
                    name: term.clone(),
                    frequency,
                    relevance: score,
                    context_sample: self.find_context_sample(&term, documents),
                }
            })
            .collect();
            
        topics.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap());
        topics.truncate(max_topics);
        
        topics
    }

    /// Clean and normalize text for analysis
    fn clean_text(&self, text: &str) -> String {
        text.to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() || c.is_whitespace() { c } else { ' ' })
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<&str>>()
            .join(" ")
    }

    /// Tokenize text into meaningful words
    fn tokenize(&self, text: &str) -> Vec<String> {
        text.split_whitespace()
            .map(|s| s.to_string())
            .filter(|word| word.len() > 2)
            .collect()
    }

    /// Check if a word is meaningful for topic extraction
    fn is_meaningful_word(&self, word: &str) -> bool {
        word.len() > 2 && 
        !self.stop_words.contains(word) &&
        !word.chars().all(|c| c.is_numeric()) &&
        word.chars().any(|c| c.is_alphabetic())
    }

    /// Find a context sample for a given term
    fn find_context_sample(&self, term: &str, documents: &[String]) -> Option<String> {
        for doc in documents {
            if doc.to_lowercase().contains(term) {
                // Find the sentence containing the term
                let sentences: Vec<&str> = doc.split(&['.', '!', '?']).collect();
                for sentence in sentences {
                    if sentence.to_lowercase().contains(term) {
                        let trimmed = sentence.trim();
                        if trimmed.len() > 10 && trimmed.len() < 200 {
                            return Some(trimmed.to_string());
                        }
                    }
                }
            }
        }
        None
    }
}

// Advanced text analysis for topic extraction
fn extract_topics_from_conversation(conversation: &Conversation) -> Vec<String> {
    let analyzer = TextAnalyzer::new();
    
    // Collect all relevant text content
    let mut text_content = Vec::new();
    
    // Include summary if available
    if let Some(summary) = &conversation.summary {
        text_content.push(summary.clone());
    }
    
    // Include user messages (these often contain the main topics/questions)
    for message in &conversation.messages {
        if message.role == MessageRole::User && !message.content.trim().is_empty() {
            text_content.push(message.content.clone());
        }
    }
    
    // Include first assistant message (often contains topic acknowledgment)
    if let Some(first_assistant) = conversation.messages
        .iter()
        .find(|m| m.role == MessageRole::Assistant) {
        text_content.push(first_assistant.content.clone());
    }
    
    // Extract topics using text analysis
    if text_content.is_empty() {
        return vec!["General discussion".to_string()];
    }
    
    let combined_text = text_content.join(" ");
    analyzer.extract_key_phrases(&combined_text, 3)
}

fn extract_topics_from_conversations(conversations: &[Conversation]) -> Vec<Topic> {
    let analyzer = TextAnalyzer::new();
    
    // Collect all text content from conversations
    let mut all_content = Vec::new();
    let mut conversation_contexts = HashMap::new();
    
    for conversation in conversations {
        let mut conv_content = Vec::new();
        
        // Prioritize user messages and summaries for topic extraction
        if let Some(summary) = &conversation.summary {
            conv_content.push(summary.clone());
        }
        
        for message in &conversation.messages {
            if message.role == MessageRole::User {
                conv_content.push(message.content.clone());
            }
        }
        
        let combined = conv_content.join(" ");
        if !combined.trim().is_empty() {
            all_content.push(combined.clone());
            conversation_contexts.insert(combined.clone(), conversation.session_id.clone());
        }
    }
    
    if all_content.is_empty() {
        return Vec::new();
    }
    
    // Use TF-IDF-like analysis for topic extraction
    analyzer.extract_topics_with_tfidf(&all_content, &conversation_contexts, 10)
}

/// Normalize project path for consistent grouping
fn normalize_project_path(path: &str) -> String {
    // Remove trailing slashes and normalize separators
    let normalized = path.trim_end_matches('/').replace('\\', "/");
    
    // Handle empty paths
    if normalized.is_empty() {
        return "unknown".to_string();
    }
    
    // Extract meaningful project name from path
    // For paths like "/Users/name/projects/my-project", extract "my-project"
    // For paths like "projects/subfolder/my-project", extract "my-project"
    if let Some(last_segment) = normalized.split('/').last() {
        if !last_segment.is_empty() && last_segment != "." && last_segment != ".." {
            return last_segment.to_string();
        }
    }
    
    // Fallback to full normalized path
    normalized
}

fn generate_content_summary(conversation: &Conversation) -> String {
    if let Some(summary) = &conversation.summary {
        summary.clone()
    } else if !conversation.messages.is_empty() {
        let first_user_msg = conversation.messages
            .iter()
            .find(|m| m.role == MessageRole::User)
            .map(|m| &m.content)
            .unwrap_or(&conversation.messages[0].content);
        
        // Take first 100 characters
        if first_user_msg.len() > 100 {
            format!("{}...", &first_user_msg[..97])
        } else {
            first_user_msg.clone()
        }
    } else {
        "Empty conversation".to_string()
    }
}

fn generate_project_summary(conversations: &[Conversation], depth: SummaryDepth) -> String {
    if conversations.is_empty() {
        return "No activity in this time period".to_string();
    }

    let _analyzer = TextAnalyzer::new();
    let total_messages: usize = conversations.iter().map(|c| c.messages.len()).sum();
    let total_conversations = conversations.len();
    
    // Extract key topics for context
    let topics = extract_topics_from_conversations(conversations);
    let main_topics: Vec<String> = topics.into_iter().take(3).map(|t| t.name).collect();
    
    // Analyze tool usage patterns
    let tools_used: HashSet<_> = conversations
        .iter()
        .flat_map(|c| &c.messages)
        .flat_map(|m| &m.tool_uses)
        .map(|t| &t.name)
        .collect();
    
    // Calculate activity patterns
    let avg_messages_per_conv = total_messages as f64 / total_conversations as f64;
    let user_messages: usize = conversations
        .iter()
        .flat_map(|c| &c.messages)
        .filter(|m| m.role == MessageRole::User)
        .count();
    
    match depth {
        SummaryDepth::Brief => {
            let topic_summary = if main_topics.is_empty() {
                "general development work"
            } else {
                &main_topics[0]
            };
            
            format!(
                "{} conversations with {} messages focusing on {}.",
                total_conversations, total_messages, topic_summary
            )
        }
        SummaryDepth::Detailed => {
            let topics_text = if main_topics.is_empty() {
                "various development topics".to_string()
            } else {
                main_topics.join(", ")
            };
            
            let tools_text = if tools_used.is_empty() {
                "standard development tools".to_string()
            } else {
                tools_used.into_iter().take(3).cloned().collect::<Vec<_>>().join(", ")
            };
            
            format!(
                "{} conversations generated {} messages (avg {:.1} per conversation). \
                Key topics: {}. Primary tools: {}. \
                Activity shows {} interaction pattern.",
                total_conversations, 
                total_messages,
                avg_messages_per_conv,
                topics_text,
                tools_text,
                if avg_messages_per_conv > 10.0 { "intensive" } else { "focused" }
            )
        }
        SummaryDepth::Comprehensive => {
            let topics_text = if main_topics.is_empty() {
                "No specific topics identified - conversations appear to cover general development work.".to_string()
            } else {
                format!("Primary focus areas include: {}.", main_topics.join(", "))
            };
            
            let tools_analysis = if tools_used.is_empty() {
                "No specific tools were used in these conversations.".to_string()
            } else {
                format!(
                    "Tool usage includes {} different tools, with {} being most frequently used.",
                    tools_used.len(),
                    tools_used.into_iter().take(3).cloned().collect::<Vec<_>>().join(", ")
                )
            };
            
            let interaction_analysis = format!(
                "Conversation patterns show {} interactions with an average of {:.1} messages per conversation. \
                User initiated {} queries, suggesting {} engagement level.",
                if total_conversations > 5 { "frequent" } else { "moderate" },
                avg_messages_per_conv,
                user_messages,
                if user_messages as f64 / total_messages as f64 > 0.4 { "high" } else { "standard" }
            );
            
            format!(
                "Comprehensive analysis of {} conversations with {} total messages. {} {} {}",
                total_conversations, total_messages, topics_text, tools_analysis, interaction_analysis
            )
        }
    }
}

/// Calculate average session duration in minutes
fn calculate_avg_session_duration(conversations: &[Conversation]) -> Option<f64> {
    let durations: Vec<f64> = conversations
        .iter()
        .filter_map(|conv| {
            if let (Some(start), Some(end)) = (conv.started_at, conv.last_updated) {
                let duration = end.signed_duration_since(start);
                Some(duration.num_minutes() as f64)
            } else {
                None
            }
        })
        .collect();

    if durations.is_empty() {
        None
    } else {
        Some(durations.iter().sum::<f64>() / durations.len() as f64)
    }
}

fn extract_frequent_tools(conversations: &[Conversation]) -> Vec<String> {
    let mut tool_counts: HashMap<String, usize> = HashMap::new();
    
    for conversation in conversations {
        for message in &conversation.messages {
            for tool_use in &message.tool_uses {
                *tool_counts.entry(tool_use.name.clone()).or_default() += 1;
            }
        }
    }
    
    let mut tools: Vec<_> = tool_counts.into_iter().collect();
    tools.sort_by(|a, b| b.1.cmp(&a.1));
    tools.into_iter().take(5).map(|(name, _)| name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude::conversation::ToolUse;

    #[test]
    fn test_time_period_contains() {
        let now = Utc::now();
        let yesterday = now - Duration::days(1);
        let two_days_ago = now - Duration::days(2);
        
        let period = TimePeriod::LastDay;
        assert!(period.contains(yesterday + Duration::hours(1)));
        assert!(!period.contains(two_days_ago));
    }

    #[test]
    fn test_time_period_labels() {
        assert_eq!(TimePeriod::LastDay.label(), "Past 24 hours");
        assert_eq!(TimePeriod::LastTwoDay.label(), "Past 48 hours");
        assert_eq!(TimePeriod::LastWeek.label(), "Past week");
    }

    #[test]
    fn test_activity_timeline_creation() {
        let config = TimelineConfig::default();
        let timeline = ActivityTimeline::new(config.clone());
        
        assert_eq!(timeline.config.period, config.period);
        assert!(timeline.projects.is_empty());
        assert_eq!(timeline.total_stats.active_projects, 0);
    }

    #[test]
    fn test_activity_intensity() {
        assert_eq!(
            match 3 {
                0..=4 => ActivityIntensity::Low,
                5..=20 => ActivityIntensity::Medium,
                _ => ActivityIntensity::High,
            },
            ActivityIntensity::Low
        );

        assert_eq!(
            match 15 {
                0..=4 => ActivityIntensity::Low,
                5..=20 => ActivityIntensity::Medium,
                _ => ActivityIntensity::High,
            },
            ActivityIntensity::Medium
        );

        assert_eq!(
            match 25 {
                0..=4 => ActivityIntensity::Low,
                5..=20 => ActivityIntensity::Medium,
                _ => ActivityIntensity::High,
            },
            ActivityIntensity::High
        );
    }

    #[test]
    fn test_normalize_project_path() {
        assert_eq!(normalize_project_path("/users/john/projects/my-app/"), "my-app");
        assert_eq!(normalize_project_path("C:\\projects\\my-app"), "my-app");
        assert_eq!(normalize_project_path("my-project"), "my-project");
        assert_eq!(normalize_project_path(""), "unknown");
        assert_eq!(normalize_project_path("/"), "unknown");
    }

    #[test]
    fn test_conversation_filtering_by_time_period() {
        let now = Utc::now();
        let conversations = vec![
            create_test_conversation("conv1", now - Duration::hours(1)), // Within 24h
            create_test_conversation("conv2", now - Duration::hours(36)), // Within 48h but not 24h  
            create_test_conversation("conv3", now - Duration::days(8)),  // Outside week
        ];

        // Test 24h filtering
        let filtered_24h = ActivityTimeline::filter_conversations_by_time_period(
            &conversations, 
            TimePeriod::LastDay
        );
        assert_eq!(filtered_24h.len(), 1);
        assert_eq!(filtered_24h[0].session_id, "conv1");

        // Test 48h filtering  
        let filtered_48h = ActivityTimeline::filter_conversations_by_time_period(
            &conversations,
            TimePeriod::LastTwoDay
        );
        assert_eq!(filtered_48h.len(), 2);

        // Test week filtering
        let filtered_week = ActivityTimeline::filter_conversations_by_time_period(
            &conversations,
            TimePeriod::LastWeek
        );
        assert_eq!(filtered_week.len(), 2);
    }

    #[test]
    fn test_conversation_grouping_by_project() {
        let conversations = vec![
            create_test_conversation_with_project("conv1", "/projects/app1"),
            create_test_conversation_with_project("conv2", "/projects/app1"),
            create_test_conversation_with_project("conv3", "/projects/app2"),
        ];

        let conv_refs: Vec<&Conversation> = conversations.iter().collect();
        let grouped = ActivityTimeline::group_conversations_by_project(conv_refs);

        assert_eq!(grouped.len(), 2);
        assert!(grouped.contains_key("app1"));
        assert!(grouped.contains_key("app2"));
        assert_eq!(grouped["app1"].len(), 2);
        assert_eq!(grouped["app2"].len(), 1);
    }

    #[test]
    fn test_advanced_filtering() {
        let now = Utc::now();
        let conversations = vec![
            create_test_conversation_with_tools("conv1", now - Duration::hours(1), 5, true),
            create_test_conversation_with_tools("conv2", now - Duration::hours(2), 2, false),
            create_test_conversation_with_tools("conv3", now - Duration::days(3), 10, true),
        ];

        // Filter for conversations with tools and minimum 3 messages
        let filtered = ActivityTimeline::filter_conversations_advanced(
            &conversations,
            TimePeriod::LastDay,
            Some(3),
            None,
            Some(true),
        );

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].session_id, "conv1");
    }

    #[test]
    fn test_batch_filtering() {
        let now = Utc::now();
        let conversations: Vec<_> = (0..50)
            .map(|i| create_test_conversation(&format!("conv{}", i), now - Duration::hours(i % 48)))
            .collect();

        let filtered_batch = ActivityTimeline::filter_conversations_parallel(
            &conversations,
            TimePeriod::LastTwoDay,
            10, // batch size
        );

        let filtered_normal = ActivityTimeline::filter_conversations_by_time_period(
            &conversations,
            TimePeriod::LastTwoDay,
        );

        // Should produce same results
        assert_eq!(filtered_batch.len(), filtered_normal.len());
    }

    #[test]
    fn test_timeline_creation_with_filtering() {
        let now = Utc::now();
        let conversations = vec![
            create_test_conversation_with_project("conv1", "/projects/app1"),
            create_test_conversation_with_project("conv2", "/projects/app1"),
            create_test_conversation_with_project("conv3", "/projects/app2"),
            create_test_conversation("old_conv", now - Duration::days(10)), // Outside time range
        ];

        let config = TimelineConfig {
            period: TimePeriod::LastWeek,
            summary_depth: SummaryDepth::Brief,
            max_conversations_per_project: Some(5),
            include_empty_projects: false,
        };

        let timeline = ActivityTimeline::create_filtered_timeline(conversations, config);

        // Should have 2 projects (app1, app2) and exclude old conversation
        assert_eq!(timeline.projects.len(), 2);
        assert!(timeline.projects.contains_key("app1"));
        assert!(timeline.projects.contains_key("app2"));
        
        // Check statistics
        assert_eq!(timeline.total_stats.active_projects, 2);
        assert_eq!(timeline.total_stats.total_conversations, 3);
    }

    // Helper functions for creating test data
    fn create_test_conversation(session_id: &str, timestamp: DateTime<Utc>) -> Conversation {
        Conversation {
            session_id: session_id.to_string(),
            project_path: "/default/project".to_string(),
            summary: Some(format!("Test conversation {}", session_id)),
            messages: vec![create_test_message(timestamp)],
            started_at: Some(timestamp),
            last_updated: Some(timestamp),
        }
    }

    fn create_test_conversation_with_project(session_id: &str, project_path: &str) -> Conversation {
        let now = Utc::now();
        Conversation {
            session_id: session_id.to_string(),
            project_path: project_path.to_string(),
            summary: Some(format!("Test conversation {}", session_id)),
            messages: vec![create_test_message(now)],
            started_at: Some(now),
            last_updated: Some(now),
        }
    }

    fn create_test_conversation_with_tools(
        session_id: &str, 
        timestamp: DateTime<Utc>, 
        message_count: usize,
        has_tools: bool
    ) -> Conversation {
        let mut messages = Vec::new();
        for i in 0..message_count {
            let mut msg = create_test_message(timestamp + Duration::minutes(i as i64));
            if has_tools && i == 0 {
                msg.tool_uses = vec![ToolUse {
                    id: "tool1".to_string(),
                    name: "test_tool".to_string(),
                    input: serde_json::json!({"param": "value"}),
                }];
            }
            messages.push(msg);
        }

        Conversation {
            session_id: session_id.to_string(),
            project_path: "/test/project".to_string(),
            summary: Some(format!("Test conversation {}", session_id)),
            messages,
            started_at: Some(timestamp),
            last_updated: Some(timestamp),
        }
    }

    fn create_test_message(timestamp: DateTime<Utc>) -> crate::claude::ConversationMessage {
        use crate::claude::ConversationMessage;
        
        ConversationMessage {
            uuid: format!("msg-{}", timestamp.timestamp()),
            parent_uuid: None,
            role: MessageRole::User,
            content: "Test message content".to_string(),
            timestamp,
            model: None,
            tool_uses: vec![],
        }
    }

    #[test]
    fn test_text_analyzer_basic_functionality() {
        let analyzer = TextAnalyzer::new();
        
        // Test meaningful word detection
        assert!(analyzer.is_meaningful_word("programming"));
        assert!(analyzer.is_meaningful_word("rust"));
        assert!(!analyzer.is_meaningful_word("the"));
        assert!(!analyzer.is_meaningful_word("123"));
        assert!(!analyzer.is_meaningful_word("a"));
        
        // Test text cleaning
        let cleaned = analyzer.clean_text("Hello, World! This is a test.");
        assert_eq!(cleaned, "hello world this is a test");
    }

    #[test]
    fn test_key_phrase_extraction() {
        let analyzer = TextAnalyzer::new();
        let text = "I need help with Rust programming. Specifically working with async functions and error handling in Rust applications.";
        
        let phrases = analyzer.extract_key_phrases(text, 5);
        
        // Should extract meaningful technical terms
        assert!(!phrases.is_empty());
        // Rust should be highly ranked due to technical term bonus
        assert!(phrases.iter().any(|p| p.contains("rust")));
    }

    #[test]
    fn test_tfidf_topic_extraction() {
        let analyzer = TextAnalyzer::new();
        let documents = vec![
            "Working on Rust programming with async functions".to_string(),
            "Debugging Python code with error handling".to_string(),
            "Rust async programming best practices".to_string(),
        ];
        let contexts = HashMap::new();
        
        let topics = analyzer.extract_topics_with_tfidf(&documents, &contexts, 5);
        
        assert!(!topics.is_empty());
        // Should identify "rust" and "async" as key topics
        let topic_names: Vec<_> = topics.iter().map(|t| &t.name).collect();
        assert!(topic_names.iter().any(|&name| name.contains("rust")));
    }

    #[test]
    fn test_topical_summary_generation() {
        let now = Utc::now();
        let conversations = vec![
            create_test_conversation_with_content(
                "conv1", 
                "Help me debug a Rust async function",
                "I'm having trouble with async/await in Rust. The function keeps hanging.",
                now
            ),
            create_test_conversation_with_content(
                "conv2",
                "Python error handling",
                "What's the best way to handle exceptions in Python web applications?",
                now
            ),
        ];

        // Test brief summary
        let brief_summary = generate_project_summary(&conversations, SummaryDepth::Brief);
        assert!(brief_summary.contains("2 conversations"));
        assert!(brief_summary.len() < 100); // Should be brief

        // Test detailed summary
        let detailed_summary = generate_project_summary(&conversations, SummaryDepth::Detailed);
        assert!(detailed_summary.contains("2 conversations"));
        assert!(detailed_summary.len() > brief_summary.len());

        // Test comprehensive summary
        let comprehensive_summary = generate_project_summary(&conversations, SummaryDepth::Comprehensive);
        assert!(comprehensive_summary.contains("Comprehensive analysis"));
        assert!(comprehensive_summary.len() > detailed_summary.len());
    }

    #[test]
    fn test_project_activity_topic_extraction() {
        let now = Utc::now();
        let conversations = vec![
            create_test_conversation_with_content(
                "conv1",
                "Rust async programming help",
                "I need assistance with async Rust programming patterns",
                now
            ),
        ];

        let project_activity = ProjectActivity::from_conversations(
            "test-project".to_string(),
            conversations,
            SummaryDepth::Detailed,
        );

        // Should have extracted meaningful topics
        assert!(!project_activity.topical_summary.main_topics.is_empty());
        assert!(!project_activity.topical_summary.summary_text.is_empty());
        
        // Activity intensity should be calculated
        assert!(matches!(
            project_activity.topical_summary.intensity,
            ActivityIntensity::Low | ActivityIntensity::Medium | ActivityIntensity::High
        ));
    }

    #[test]
    fn test_performance_large_conversation_set() {
        let now = Utc::now();
        let start_time = std::time::Instant::now();
        
        // Create a moderately large set of conversations for performance testing
        let conversations: Vec<_> = (0..20)
            .map(|i| create_test_conversation_with_content(
                &format!("conv{}", i),
                &format!("Topic {} discussion about programming", i),
                &format!("This is conversation {} about various programming topics including Rust, Python, and JavaScript development", i),
                now
            ))
            .collect();

        // Generate comprehensive summaries
        let _summary = generate_project_summary(&conversations, SummaryDepth::Comprehensive);
        let _topics = extract_topics_from_conversations(&conversations);
        
        let elapsed = start_time.elapsed();
        
        // Should complete within reasonable time (much less than 2s for this small test)
        assert!(elapsed.as_millis() < 1000, "Summary generation took too long: {:?}", elapsed);
    }

    #[test]
    fn test_empty_conversations_handling() {
        let empty_conversations = vec![];
        
        let summary = generate_project_summary(&empty_conversations, SummaryDepth::Brief);
        assert_eq!(summary, "No activity in this time period");
        
        let topics = extract_topics_from_conversations(&empty_conversations);
        assert!(topics.is_empty());
    }

    // Helper function for creating test conversations with specific content
    fn create_test_conversation_with_content(
        session_id: &str,
        summary: &str,
        user_message: &str,
        timestamp: DateTime<Utc>,
    ) -> Conversation {
        use crate::claude::ConversationMessage;
        
        let messages = vec![
            ConversationMessage {
                uuid: format!("{}-user", session_id),
                parent_uuid: None,
                role: MessageRole::User,
                content: user_message.to_string(),
                timestamp,
                model: None,
                tool_uses: vec![],
            },
            ConversationMessage {
                uuid: format!("{}-assistant", session_id),
                parent_uuid: Some(format!("{}-user", session_id)),
                role: MessageRole::Assistant,
                content: format!("I can help you with {}. Let me provide some guidance.", summary.to_lowercase()),
                timestamp: timestamp + Duration::minutes(1),
                model: Some("claude-3".to_string()),
                tool_uses: vec![],
            },
        ];

        Conversation {
            session_id: session_id.to_string(),
            project_path: "/test/project".to_string(),
            summary: Some(summary.to_string()),
            messages,
            started_at: Some(timestamp),
            last_updated: Some(timestamp + Duration::minutes(2)),
        }
    }
}