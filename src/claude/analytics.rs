use super::conversation::{Conversation, MessageRole};
use crate::errors::ClaudeToolsError;
use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// Comprehensive analytics engine for Claude Code conversations
pub struct AnalyticsEngine {
    conversations: Vec<Conversation>,
    cached_analytics: Option<ConversationAnalytics>,
}

/// Complete analytics data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationAnalytics {
    /// Basic conversation statistics
    pub basic_stats: BasicStats,
    /// Temporal usage patterns
    pub temporal_analysis: TemporalAnalysis,
    /// Model usage analytics
    pub model_analytics: ModelAnalytics,
    /// Tool usage statistics
    pub tool_analytics: ToolAnalytics,
    /// Project-based analytics
    pub project_analytics: ProjectAnalytics,
    /// Conversation quality metrics
    pub quality_metrics: QualityMetrics,
    /// Generation timestamp
    pub generated_at: DateTime<Utc>,
}

/// Basic conversation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicStats {
    pub total_conversations: usize,
    pub total_messages: usize,
    pub total_user_messages: usize,
    pub total_assistant_messages: usize,
    pub total_system_messages: usize,
    pub total_tool_uses: usize,
    pub average_messages_per_conversation: f64,
    pub conversation_length_distribution: LengthDistribution,
    pub date_range: DateRange,
}

/// Temporal usage pattern analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalAnalysis {
    /// Conversations per day over time
    pub conversations_per_day: BTreeMap<String, usize>, // Date string -> count
    /// Messages per day over time
    pub messages_per_day: BTreeMap<String, usize>,
    /// Usage by hour of day (0-23)
    pub usage_by_hour: HashMap<u8, usize>,
    /// Usage by day of week (0-6, Sunday = 0)
    pub usage_by_weekday: HashMap<u8, usize>,
    /// Most active periods
    pub peak_usage_hours: Vec<PeakUsage>,
    /// Activity trends
    pub activity_trends: ActivityTrends,
}

/// Model usage analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelAnalytics {
    /// Model usage frequency
    pub model_usage_count: HashMap<String, usize>,
    /// Average conversation length per model
    pub avg_conversation_length_per_model: HashMap<String, f64>,
    /// Model usage over time
    pub model_usage_over_time: HashMap<String, BTreeMap<String, usize>>,
    /// Most popular models
    pub top_models: Vec<ModelUsage>,
}

/// Tool usage analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolAnalytics {
    /// Tool usage frequency
    pub tool_usage_count: HashMap<String, usize>,
    /// Tool success rates (if determinable)
    pub tool_success_rates: HashMap<String, f64>,
    /// Average tools per conversation
    pub average_tools_per_conversation: f64,
    /// Tool usage trends over time
    pub tool_usage_over_time: HashMap<String, BTreeMap<String, usize>>,
    /// Most popular tools
    pub top_tools: Vec<ToolUsage>,
}

/// Project-based analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAnalytics {
    /// Conversations per project
    pub conversations_per_project: HashMap<String, usize>,
    /// Messages per project
    pub messages_per_project: HashMap<String, usize>,
    /// Average conversation length per project
    pub avg_conversation_length_per_project: HashMap<String, f64>,
    /// Project activity over time
    pub project_activity_over_time: HashMap<String, BTreeMap<String, usize>>,
    /// Most active projects
    pub top_projects: Vec<ProjectUsage>,
}

/// Conversation quality metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    /// Average conversation duration (minutes)
    pub average_conversation_duration: Option<f64>,
    /// Turn-taking patterns (user-assistant exchanges)
    pub average_turns_per_conversation: f64,
    /// Message length distribution
    pub message_length_distribution: LengthDistribution,
    /// Completion rates (conversations with proper endings)
    pub completion_rate: f64,
    /// Response time patterns (if determinable)
    pub response_patterns: ResponsePatterns,
}

/// Supporting data structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LengthDistribution {
    pub min: usize,
    pub max: usize,
    pub mean: f64,
    pub median: usize,
    pub percentile_25: usize,
    pub percentile_75: usize,
    pub percentile_90: usize,
    pub percentile_95: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub earliest: Option<DateTime<Utc>>,
    pub latest: Option<DateTime<Utc>>,
    pub span_days: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeakUsage {
    pub hour: u8,
    pub count: usize,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityTrends {
    pub weekly_growth_rate: f64,
    pub monthly_growth_rate: f64,
    pub most_active_month: String,
    pub most_active_day: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsage {
    pub model_name: String,
    pub usage_count: usize,
    pub percentage: f64,
    pub avg_conversation_length: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsage {
    pub tool_name: String,
    pub usage_count: usize,
    pub percentage: f64,
    pub success_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectUsage {
    pub project_name: String,
    pub conversation_count: usize,
    pub message_count: usize,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponsePatterns {
    pub average_response_time_minutes: Option<f64>,
    pub response_time_distribution: Option<LengthDistribution>,
}

impl AnalyticsEngine {
    /// Create a new analytics engine
    pub fn new(conversations: Vec<Conversation>) -> Self {
        Self {
            conversations,
            cached_analytics: None,
        }
    }

    /// Generate comprehensive analytics
    pub fn generate_analytics(&mut self) -> Result<&ConversationAnalytics, ClaudeToolsError> {
        if self.cached_analytics.is_none() {
            self.cached_analytics = Some(self.compute_analytics()?);
        }
        Ok(self.cached_analytics.as_ref().unwrap())
    }

    /// Force regeneration of analytics (clears cache)
    pub fn regenerate_analytics(&mut self) -> Result<&ConversationAnalytics, ClaudeToolsError> {
        self.cached_analytics = None;
        self.generate_analytics()
    }

    /// Compute all analytics from scratch
    fn compute_analytics(&self) -> Result<ConversationAnalytics, ClaudeToolsError> {
        Ok(ConversationAnalytics {
            basic_stats: self.compute_basic_stats(),
            temporal_analysis: self.compute_temporal_analysis(),
            model_analytics: self.compute_model_analytics(),
            tool_analytics: self.compute_tool_analytics(),
            project_analytics: self.compute_project_analytics(),
            quality_metrics: self.compute_quality_metrics(),
            generated_at: Utc::now(),
        })
    }

    /// Compute basic conversation statistics
    fn compute_basic_stats(&self) -> BasicStats {
        let total_conversations = self.conversations.len();
        let total_messages: usize = self.conversations.iter().map(|c| c.messages.len()).sum();
        let total_user_messages: usize = self
            .conversations
            .iter()
            .map(|c| c.user_message_count())
            .sum();
        let total_assistant_messages: usize = self
            .conversations
            .iter()
            .map(|c| c.assistant_message_count())
            .sum();
        let total_system_messages: usize = self
            .conversations
            .iter()
            .flat_map(|c| &c.messages)
            .filter(|m| m.role == MessageRole::System)
            .count();
        let total_tool_uses: usize = self
            .conversations
            .iter()
            .flat_map(|c| &c.messages)
            .map(|m| m.tool_uses.len())
            .sum();

        let average_messages_per_conversation = if total_conversations > 0 {
            total_messages as f64 / total_conversations as f64
        } else {
            0.0
        };

        let message_counts: Vec<usize> = self
            .conversations
            .iter()
            .map(|c| c.messages.len())
            .collect();
        let conversation_length_distribution = Self::calculate_length_distribution(&message_counts);

        let date_range = self.calculate_date_range();

        BasicStats {
            total_conversations,
            total_messages,
            total_user_messages,
            total_assistant_messages,
            total_system_messages,
            total_tool_uses,
            average_messages_per_conversation,
            conversation_length_distribution,
            date_range,
        }
    }

    /// Compute temporal usage patterns
    fn compute_temporal_analysis(&self) -> TemporalAnalysis {
        let mut conversations_per_day = BTreeMap::new();
        let mut messages_per_day = BTreeMap::new();
        let mut usage_by_hour = HashMap::new();
        let mut usage_by_weekday = HashMap::new();

        for conversation in &self.conversations {
            if let Some(started_at) = conversation.started_at {
                // Daily aggregations
                let date_key = started_at.format("%Y-%m-%d").to_string();
                *conversations_per_day.entry(date_key.clone()).or_insert(0) += 1;
                *messages_per_day.entry(date_key).or_insert(0) += conversation.messages.len();

                // Hourly aggregation
                let hour = started_at.hour() as u8;
                *usage_by_hour.entry(hour).or_insert(0) += 1;

                // Weekday aggregation (Sunday = 0)
                let weekday = started_at.weekday().num_days_from_sunday() as u8;
                *usage_by_weekday.entry(weekday).or_insert(0) += 1;
            }
        }

        // Calculate peak usage hours
        let mut peak_usage_hours: Vec<_> = usage_by_hour
            .iter()
            .map(|(&hour, &count)| PeakUsage {
                hour,
                count,
                percentage: if self.conversations.len() > 0 {
                    count as f64 / self.conversations.len() as f64 * 100.0
                } else {
                    0.0
                },
            })
            .collect();
        peak_usage_hours.sort_by(|a, b| b.count.cmp(&a.count));
        peak_usage_hours.truncate(5); // Top 5 hours

        let activity_trends = self.calculate_activity_trends(&conversations_per_day);

        TemporalAnalysis {
            conversations_per_day,
            messages_per_day,
            usage_by_hour,
            usage_by_weekday,
            peak_usage_hours,
            activity_trends,
        }
    }

    /// Compute model usage analytics
    fn compute_model_analytics(&self) -> ModelAnalytics {
        let mut model_usage_count = HashMap::new();
        let mut model_message_counts = HashMap::new();
        let mut model_conversation_counts = HashMap::new();
        let mut model_usage_over_time = HashMap::new();

        for conversation in &self.conversations {
            let mut conversation_models = HashMap::new();

            for message in &conversation.messages {
                if let Some(ref model) = message.model {
                    *model_usage_count.entry(model.clone()).or_insert(0) += 1;
                    *model_message_counts.entry(model.clone()).or_insert(0) += 1;
                    conversation_models.insert(model.clone(), true);

                    // Track usage over time
                    if let Some(started_at) = conversation.started_at {
                        let date_key = started_at.format("%Y-%m-%d").to_string();
                        *model_usage_over_time
                            .entry(model.clone())
                            .or_insert_with(BTreeMap::new)
                            .entry(date_key)
                            .or_insert(0) += 1;
                    }
                }
            }

            // Count unique models per conversation
            for model in conversation_models.keys() {
                *model_conversation_counts.entry(model.clone()).or_insert(0) += 1;
            }
        }

        // Calculate average conversation length per model
        let mut avg_conversation_length_per_model = HashMap::new();
        for (model, conversation_count) in &model_conversation_counts {
            if let Some(&message_count) = model_message_counts.get(model) {
                avg_conversation_length_per_model.insert(
                    model.clone(),
                    message_count as f64 / *conversation_count as f64,
                );
            }
        }

        // Calculate top models
        let total_model_usage: usize = model_usage_count.values().sum();
        let mut top_models: Vec<_> = model_usage_count
            .iter()
            .map(|(model, &count)| ModelUsage {
                model_name: model.clone(),
                usage_count: count,
                percentage: if total_model_usage > 0 {
                    count as f64 / total_model_usage as f64 * 100.0
                } else {
                    0.0
                },
                avg_conversation_length: avg_conversation_length_per_model
                    .get(model)
                    .copied()
                    .unwrap_or(0.0),
            })
            .collect();
        top_models.sort_by(|a, b| b.usage_count.cmp(&a.usage_count));

        ModelAnalytics {
            model_usage_count,
            avg_conversation_length_per_model,
            model_usage_over_time,
            top_models,
        }
    }

    /// Compute tool usage analytics
    fn compute_tool_analytics(&self) -> ToolAnalytics {
        let mut tool_usage_count = HashMap::new();
        let mut tool_usage_over_time = HashMap::new();
        let mut _total_conversations_with_tools = 0;
        let mut total_tool_uses = 0;

        for conversation in &self.conversations {
            let mut conversation_has_tools = false;

            for message in &conversation.messages {
                for tool_use in &message.tool_uses {
                    *tool_usage_count.entry(tool_use.name.clone()).or_insert(0) += 1;
                    total_tool_uses += 1;
                    conversation_has_tools = true;

                    // Track usage over time
                    if let Some(started_at) = conversation.started_at {
                        let date_key = started_at.format("%Y-%m-%d").to_string();
                        *tool_usage_over_time
                            .entry(tool_use.name.clone())
                            .or_insert_with(BTreeMap::new)
                            .entry(date_key)
                            .or_insert(0) += 1;
                    }
                }
            }

            if conversation_has_tools {
                _total_conversations_with_tools += 1;
            }
        }

        let average_tools_per_conversation = if self.conversations.len() > 0 {
            total_tool_uses as f64 / self.conversations.len() as f64
        } else {
            0.0
        };

        // For now, assume 100% success rate (we don't have failure data)
        let tool_success_rates: HashMap<String, f64> = tool_usage_count
            .keys()
            .map(|tool| (tool.clone(), 100.0))
            .collect();

        // Calculate top tools
        let mut top_tools: Vec<_> = tool_usage_count
            .iter()
            .map(|(tool, &count)| ToolUsage {
                tool_name: tool.clone(),
                usage_count: count,
                percentage: if total_tool_uses > 0 {
                    count as f64 / total_tool_uses as f64 * 100.0
                } else {
                    0.0
                },
                success_rate: 100.0, // Default success rate
            })
            .collect();
        top_tools.sort_by(|a, b| b.usage_count.cmp(&a.usage_count));

        ToolAnalytics {
            tool_usage_count,
            tool_success_rates,
            average_tools_per_conversation,
            tool_usage_over_time,
            top_tools,
        }
    }

    /// Compute project-based analytics
    fn compute_project_analytics(&self) -> ProjectAnalytics {
        let mut conversations_per_project = HashMap::new();
        let mut messages_per_project = HashMap::new();
        let mut project_activity_over_time = HashMap::new();

        for conversation in &self.conversations {
            let project = &conversation.project_path;
            *conversations_per_project
                .entry(project.clone())
                .or_insert(0) += 1;
            *messages_per_project.entry(project.clone()).or_insert(0) +=
                conversation.messages.len();

            // Track activity over time
            if let Some(started_at) = conversation.started_at {
                let date_key = started_at.format("%Y-%m-%d").to_string();
                *project_activity_over_time
                    .entry(project.clone())
                    .or_insert_with(BTreeMap::new)
                    .entry(date_key)
                    .or_insert(0) += 1;
            }
        }

        // Calculate average conversation length per project
        let mut avg_conversation_length_per_project = HashMap::new();
        for (project, &conversation_count) in &conversations_per_project {
            if let Some(&message_count) = messages_per_project.get(project) {
                avg_conversation_length_per_project.insert(
                    project.clone(),
                    message_count as f64 / conversation_count as f64,
                );
            }
        }

        // Calculate top projects
        let total_conversations = self.conversations.len();
        let mut top_projects: Vec<_> = conversations_per_project
            .iter()
            .map(|(project, &conv_count)| ProjectUsage {
                project_name: project.clone(),
                conversation_count: conv_count,
                message_count: messages_per_project.get(project).copied().unwrap_or(0),
                percentage: if total_conversations > 0 {
                    conv_count as f64 / total_conversations as f64 * 100.0
                } else {
                    0.0
                },
            })
            .collect();
        top_projects.sort_by(|a, b| b.conversation_count.cmp(&a.conversation_count));

        ProjectAnalytics {
            conversations_per_project,
            messages_per_project,
            avg_conversation_length_per_project,
            project_activity_over_time,
            top_projects,
        }
    }

    /// Compute conversation quality metrics
    fn compute_quality_metrics(&self) -> QualityMetrics {
        let durations: Vec<i64> = self
            .conversations
            .iter()
            .filter_map(|c| c.duration())
            .map(|d| d.num_minutes())
            .collect();

        let average_conversation_duration = if !durations.is_empty() {
            Some(durations.iter().sum::<i64>() as f64 / durations.len() as f64)
        } else {
            None
        };

        // Calculate turn-taking patterns
        let total_turns: usize = self
            .conversations
            .iter()
            .map(|c| self.count_conversation_turns(c))
            .sum();
        let average_turns_per_conversation = if self.conversations.len() > 0 {
            total_turns as f64 / self.conversations.len() as f64
        } else {
            0.0
        };

        // Message length distribution
        let message_lengths: Vec<usize> = self
            .conversations
            .iter()
            .flat_map(|c| &c.messages)
            .map(|m| m.content.len())
            .collect();
        let message_length_distribution = Self::calculate_length_distribution(&message_lengths);

        // Simple completion rate (conversations with at least 2 messages)
        let completed_conversations = self
            .conversations
            .iter()
            .filter(|c| c.messages.len() >= 2)
            .count();
        let completion_rate = if self.conversations.len() > 0 {
            completed_conversations as f64 / self.conversations.len() as f64 * 100.0
        } else {
            0.0
        };

        QualityMetrics {
            average_conversation_duration,
            average_turns_per_conversation,
            message_length_distribution,
            completion_rate,
            response_patterns: ResponsePatterns {
                average_response_time_minutes: None, // Would need timestamp analysis
                response_time_distribution: None,
            },
        }
    }

    /// Calculate length distribution for a set of values
    fn calculate_length_distribution(values: &[usize]) -> LengthDistribution {
        if values.is_empty() {
            return LengthDistribution {
                min: 0,
                max: 0,
                mean: 0.0,
                median: 0,
                percentile_25: 0,
                percentile_75: 0,
                percentile_90: 0,
                percentile_95: 0,
            };
        }

        let mut sorted_values = values.to_vec();
        sorted_values.sort_unstable();

        let min = sorted_values[0];
        let max = sorted_values[sorted_values.len() - 1];
        let mean = sorted_values.iter().sum::<usize>() as f64 / sorted_values.len() as f64;

        let median = sorted_values[sorted_values.len() / 2];
        let percentile_25 = sorted_values[sorted_values.len() / 4];
        let percentile_75 = sorted_values[sorted_values.len() * 3 / 4];
        let percentile_90 = sorted_values[sorted_values.len() * 9 / 10];
        let percentile_95 = sorted_values[sorted_values.len() * 95 / 100];

        LengthDistribution {
            min,
            max,
            mean,
            median,
            percentile_25,
            percentile_75,
            percentile_90,
            percentile_95,
        }
    }

    /// Calculate date range of conversations
    fn calculate_date_range(&self) -> DateRange {
        let dates: Vec<DateTime<Utc>> = self
            .conversations
            .iter()
            .filter_map(|c| c.started_at)
            .collect();

        if dates.is_empty() {
            return DateRange {
                earliest: None,
                latest: None,
                span_days: None,
            };
        }

        let earliest = dates.iter().min().copied();
        let latest = dates.iter().max().copied();
        let span_days = if let (Some(earliest), Some(latest)) = (earliest, latest) {
            Some((latest - earliest).num_days())
        } else {
            None
        };

        DateRange {
            earliest,
            latest,
            span_days,
        }
    }

    /// Calculate activity trends
    fn calculate_activity_trends(
        &self,
        conversations_per_day: &BTreeMap<String, usize>,
    ) -> ActivityTrends {
        // Simplified trend calculation
        let weekly_growth_rate = 0.0; // TODO: Implement proper trend analysis
        let monthly_growth_rate = 0.0; // TODO: Implement proper trend analysis

        let most_active_month = conversations_per_day
            .iter()
            .max_by_key(|(_, &count)| count)
            .map(|(date, _)| date.clone())
            .unwrap_or_else(|| "N/A".to_string());

        let most_active_day = conversations_per_day
            .iter()
            .max_by_key(|(_, &count)| count)
            .map(|(date, _)| date.clone())
            .unwrap_or_else(|| "N/A".to_string());

        ActivityTrends {
            weekly_growth_rate,
            monthly_growth_rate,
            most_active_month,
            most_active_day,
        }
    }

    /// Count conversation turns (user-assistant exchanges)
    fn count_conversation_turns(&self, conversation: &Conversation) -> usize {
        let mut turns = 0;
        let mut last_role: Option<MessageRole> = None;

        for message in &conversation.messages {
            if last_role != Some(message.role.clone()) {
                turns += 1;
                last_role = Some(message.role.clone());
            }
        }

        turns
    }
}
