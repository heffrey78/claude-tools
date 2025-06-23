use super::conversation::{Conversation, ConversationMessage};
use crate::errors::ClaudeToolsError;
use chrono::{DateTime, Duration, Utc};
use lru::LruCache;
use rayon::prelude::*;
use regex::Regex;
use std::collections::HashMap;
use std::num::NonZeroUsize;

/// Advanced search engine for Claude Code conversations
pub struct SearchEngine {
    /// In-memory inverted index for fast text search
    content_index: InvertedIndex,
    /// Date-based index for temporal filtering
    date_index: DateIndex,
    /// Compiled regex cache for performance
    regex_cache: LruCache<String, Regex>,
    /// Search result cache
    result_cache: LruCache<u64, Vec<SearchResult>>,
    /// All conversations for reference
    conversations: Vec<Conversation>,
}

/// Inverted index for efficient text search
#[derive(Debug, Clone)]
pub struct InvertedIndex {
    /// Word -> List of occurrences
    word_index: HashMap<String, Vec<IndexEntry>>,
    /// Document frequencies for TF-IDF calculation
    document_frequencies: HashMap<String, usize>,
    /// Total number of conversations indexed
    total_conversations: usize,
}

/// Entry in the inverted index
#[derive(Debug, Clone)]
pub struct IndexEntry {
    pub conversation_id: String,
    pub message_index: usize,
    pub position: usize,
    pub term_frequency: usize,
}

/// Date-based index for temporal filtering
#[derive(Debug, Clone)]
pub struct DateIndex {
    /// Conversations sorted by date
    conversations_by_date: Vec<ConversationDateEntry>,
}

#[derive(Debug, Clone)]
pub struct ConversationDateEntry {
    pub conversation_id: String,
    pub start_date: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

/// Search query with various filtering options
#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub text: Option<String>,
    pub regex_pattern: Option<String>,
    pub boolean_query: Option<BooleanQuery>,
    pub date_range: Option<DateRange>,
    pub project_filter: Option<String>,
    pub model_filter: Option<String>,
    pub tool_filter: Option<String>,
    pub message_role_filter: Option<MessageRole>,
    pub min_messages: Option<usize>,
    pub max_messages: Option<usize>,
    pub min_duration_minutes: Option<u32>,
    pub max_duration_minutes: Option<u32>,
    pub search_mode: SearchMode,
    pub max_results: Option<usize>,
}

/// Boolean query representation for complex search logic
#[derive(Debug, Clone)]
pub enum BooleanQuery {
    /// Single search term
    Term(String),
    /// AND operation
    And(Box<BooleanQuery>, Box<BooleanQuery>),
    /// OR operation
    Or(Box<BooleanQuery>, Box<BooleanQuery>),
    /// NOT operation
    Not(Box<BooleanQuery>),
    /// Grouped query with parentheses
    Group(Box<BooleanQuery>),
}

/// Message role for filtering
#[derive(Debug, Clone, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

/// Search modes supported by the engine
#[derive(Debug, Clone, PartialEq)]
pub enum SearchMode {
    /// Simple case-insensitive text matching
    Text,
    /// Regular expression pattern matching
    Regex,
    /// Fuzzy search with typo tolerance
    Fuzzy,
    /// Advanced search combining multiple criteria
    Advanced,
}

/// Date range filter
#[derive(Debug, Clone)]
pub struct DateRange {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

/// Boolean query parser for complex search expressions
pub struct BooleanQueryParser;

impl BooleanQueryParser {
    /// Parse a boolean query string into a BooleanQuery AST
    pub fn parse(input: &str) -> Result<BooleanQuery, ClaudeToolsError> {
        let tokens = Self::tokenize(input)?;
        Self::parse_tokens(&tokens, 0).map(|(query, _)| query)
    }

    /// Tokenize the input string
    fn tokenize(input: &str) -> Result<Vec<Token>, ClaudeToolsError> {
        let mut tokens = Vec::new();
        let mut current_word = String::new();
        let mut chars = input.chars().peekable();
        let mut in_quotes = false;

        while let Some(ch) = chars.next() {
            match ch {
                '"' => {
                    in_quotes = !in_quotes;
                    if !current_word.is_empty() {
                        tokens.push(Token::Term(current_word.clone()));
                        current_word.clear();
                    }
                }
                ' ' | '\t' | '\n' if !in_quotes => {
                    if !current_word.is_empty() {
                        tokens.push(Self::word_to_token(&current_word));
                        current_word.clear();
                    }
                }
                '(' if !in_quotes => {
                    if !current_word.is_empty() {
                        tokens.push(Self::word_to_token(&current_word));
                        current_word.clear();
                    }
                    tokens.push(Token::LeftParen);
                }
                ')' if !in_quotes => {
                    if !current_word.is_empty() {
                        tokens.push(Self::word_to_token(&current_word));
                        current_word.clear();
                    }
                    tokens.push(Token::RightParen);
                }
                _ => current_word.push(ch),
            }
        }

        if in_quotes {
            return Err(ClaudeToolsError::Config(
                "Unclosed quote in search query".to_string(),
            ));
        }

        if !current_word.is_empty() {
            tokens.push(Self::word_to_token(&current_word));
        }

        Ok(tokens)
    }

    /// Convert a word to the appropriate token type
    fn word_to_token(word: &str) -> Token {
        match word.to_uppercase().as_str() {
            "AND" => Token::And,
            "OR" => Token::Or,
            "NOT" => Token::Not,
            _ => Token::Term(word.to_string()),
        }
    }

    /// Parse tokens into a boolean query AST
    fn parse_tokens(
        tokens: &[Token],
        mut pos: usize,
    ) -> Result<(BooleanQuery, usize), ClaudeToolsError> {
        let (mut left, new_pos) = Self::parse_term(tokens, pos)?;
        pos = new_pos;

        while pos < tokens.len() {
            match &tokens[pos] {
                Token::And => {
                    pos += 1;
                    let (right, new_pos) = Self::parse_term(tokens, pos)?;
                    left = BooleanQuery::And(Box::new(left), Box::new(right));
                    pos = new_pos;
                }
                Token::Or => {
                    pos += 1;
                    let (right, new_pos) = Self::parse_term(tokens, pos)?;
                    left = BooleanQuery::Or(Box::new(left), Box::new(right));
                    pos = new_pos;
                }
                Token::RightParen => break,
                _ => break,
            }
        }

        Ok((left, pos))
    }

    /// Parse a single term (including NOT and parentheses)
    fn parse_term(
        tokens: &[Token],
        mut pos: usize,
    ) -> Result<(BooleanQuery, usize), ClaudeToolsError> {
        if pos >= tokens.len() {
            return Err(ClaudeToolsError::Config(
                "Unexpected end of query".to_string(),
            ));
        }

        match &tokens[pos] {
            Token::Not => {
                pos += 1;
                let (term, new_pos) = Self::parse_term(tokens, pos)?;
                Ok((BooleanQuery::Not(Box::new(term)), new_pos))
            }
            Token::LeftParen => {
                pos += 1;
                let (query, new_pos) = Self::parse_tokens(tokens, pos)?;
                pos = new_pos;
                if pos >= tokens.len() || !matches!(tokens[pos], Token::RightParen) {
                    return Err(ClaudeToolsError::Config(
                        "Missing closing parenthesis".to_string(),
                    ));
                }
                pos += 1;
                Ok((BooleanQuery::Group(Box::new(query)), pos))
            }
            Token::Term(term) => {
                pos += 1;
                Ok((BooleanQuery::Term(term.clone()), pos))
            }
            _ => Err(ClaudeToolsError::Config(
                "Unexpected token in query".to_string(),
            )),
        }
    }
}

/// Tokens for boolean query parsing
#[derive(Debug, Clone)]
enum Token {
    Term(String),
    And,
    Or,
    Not,
    LeftParen,
    RightParen,
}

/// Search result with relevance scoring and highlights
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub conversation: Conversation,
    pub relevance_score: f64,
    pub match_highlights: Vec<MatchHighlight>,
    pub match_count: usize,
    pub matched_messages: Vec<usize>, // Indices of messages that matched
}

/// Result of boolean query evaluation
#[derive(Debug, Clone)]
struct BooleanResult {
    matches: bool,
    score: f64,
    highlights: Vec<MatchHighlight>,
    match_count: usize,
    matched_messages: Vec<usize>,
}

/// Type of highlight to distinguish visual styling
#[derive(Debug, Clone, PartialEq)]
pub enum HighlightType {
    /// Global search across all conversations
    GlobalSearch,
    /// Search within a specific conversation
    InConversationSearch,
}

/// Highlight information for search result display
#[derive(Debug, Clone)]
pub struct MatchHighlight {
    pub message_index: usize,
    pub start: usize,
    pub end: usize,
    pub matched_text: String,
    pub highlight_type: HighlightType,
}

impl SearchEngine {
    /// Create a new search engine
    pub fn new() -> Self {
        Self {
            content_index: InvertedIndex::new(),
            date_index: DateIndex::new(),
            regex_cache: LruCache::new(NonZeroUsize::new(100).unwrap()),
            result_cache: LruCache::new(NonZeroUsize::new(50).unwrap()),
            conversations: Vec::new(),
        }
    }

    /// Build search index from a collection of conversations
    pub fn build_index(
        &mut self,
        conversations: Vec<Conversation>,
    ) -> Result<(), ClaudeToolsError> {
        // Build inverted index
        for conversation in &conversations {
            self.index_conversation(conversation)?;
        }

        // Store conversations after indexing
        self.conversations = conversations;

        // Build date index
        self.build_date_index();

        Ok(())
    }

    /// Index a single conversation
    fn index_conversation(&mut self, conversation: &Conversation) -> Result<(), ClaudeToolsError> {
        // Index each message content
        for (msg_idx, message) in conversation.messages.iter().enumerate() {
            self.index_message(&conversation.session_id, msg_idx, message);
        }

        // Index conversation summary if available
        if let Some(ref summary) = conversation.summary {
            self.index_text(&conversation.session_id, 0, summary);
        }

        Ok(())
    }

    /// Index a single message
    fn index_message(
        &mut self,
        conversation_id: &str,
        message_index: usize,
        message: &ConversationMessage,
    ) {
        self.index_text(conversation_id, message_index, &message.content);
    }

    /// Index text content by extracting and storing words
    fn index_text(&mut self, conversation_id: &str, message_index: usize, text: &str) {
        let words = Self::extract_words(text);
        let mut word_frequencies = HashMap::new();

        // Count word frequencies in this text
        for (_position, word) in words.iter().enumerate() {
            *word_frequencies.entry(word.clone()).or_insert(0) += 1;
        }

        // Update inverted index
        for (word, frequency) in word_frequencies {
            let entry = IndexEntry {
                conversation_id: conversation_id.to_string(),
                message_index,
                position: 0, // Simplified position tracking
                term_frequency: frequency,
            };

            self.content_index
                .word_index
                .entry(word.clone())
                .or_insert_with(Vec::new)
                .push(entry);

            // Update document frequency
            *self
                .content_index
                .document_frequencies
                .entry(word)
                .or_insert(0) += 1;
        }

        self.content_index.total_conversations = self.conversations.len();
    }

    /// Build date index for temporal filtering
    fn build_date_index(&mut self) {
        let mut entries = Vec::new();

        for conversation in &self.conversations {
            if let (Some(start), Some(end)) = (conversation.started_at, conversation.last_updated) {
                entries.push(ConversationDateEntry {
                    conversation_id: conversation.session_id.clone(),
                    start_date: start,
                    last_updated: end,
                });
            }
        }

        // Sort by start date for efficient range queries
        entries.sort_by(|a, b| a.start_date.cmp(&b.start_date));
        self.date_index.conversations_by_date = entries;
    }

    /// Execute a search query
    pub fn search(&mut self, query: &SearchQuery) -> Result<Vec<SearchResult>, ClaudeToolsError> {
        // Check cache first
        let query_hash = self.calculate_query_hash(query);
        if let Some(cached_results) = self.result_cache.get(&query_hash) {
            return Ok(cached_results.clone());
        }

        // Get candidate conversations based on filters
        let mut candidates = self.get_candidate_conversations(query)?;

        // Apply text/regex/boolean search
        let results = if let Some(ref boolean_query) = query.boolean_query {
            self.search_boolean_parallel(&mut candidates, boolean_query)?
        } else if let Some(ref text) = query.text {
            self.search_text_parallel(&mut candidates, text, query.search_mode.clone())?
        } else if let Some(ref pattern) = query.regex_pattern {
            self.search_regex_parallel(&mut candidates, pattern)?
        } else {
            // Just return filtered candidates with default scoring
            candidates
                .into_iter()
                .map(|conv| SearchResult {
                    relevance_score: 1.0,
                    match_highlights: Vec::new(),
                    match_count: 0,
                    matched_messages: Vec::new(),
                    conversation: conv,
                })
                .collect()
        };

        // Apply limits
        let limited_results = if let Some(max) = query.max_results {
            results.into_iter().take(max).collect()
        } else {
            results
        };

        // Cache results
        self.result_cache.put(query_hash, limited_results.clone());

        Ok(limited_results)
    }

    /// Parallel text search across conversations
    fn search_text_parallel(
        &self,
        conversations: &mut [Conversation],
        query: &str,
        mode: SearchMode,
    ) -> Result<Vec<SearchResult>, ClaudeToolsError> {
        let query_words = Self::extract_words(query);

        let results: Vec<SearchResult> = conversations
            .par_iter()
            .filter_map(|conv| {
                let result = self.score_conversation(conv, &query_words, query, &mode);
                if result.relevance_score > 0.0 || result.match_count > 0 {
                    Some(result)
                } else {
                    None
                }
            })
            .collect();

        // Sort by relevance score
        let mut sorted_results = results;
        sorted_results
            .par_sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());

        Ok(sorted_results)
    }

    /// Parallel regex search across conversations
    fn search_regex_parallel(
        &mut self,
        conversations: &mut [Conversation],
        pattern: &str,
    ) -> Result<Vec<SearchResult>, ClaudeToolsError> {
        // Get or compile regex
        let regex = if let Some(compiled) = self.regex_cache.get(pattern) {
            compiled.clone()
        } else {
            let compiled = Regex::new(pattern)
                .map_err(|e| ClaudeToolsError::Config(format!("Invalid regex: {}", e)))?;
            self.regex_cache.put(pattern.to_string(), compiled.clone());
            compiled
        };

        let results: Vec<SearchResult> = conversations
            .par_iter()
            .filter_map(|conv| {
                let mut highlights = Vec::new();
                let mut matched_messages = Vec::new();
                let mut match_count = 0;

                // Search in each message
                for (msg_idx, message) in conv.messages.iter().enumerate() {
                    for mat in regex.find_iter(&message.content) {
                        highlights.push(MatchHighlight {
                            message_index: msg_idx,
                            start: mat.start(),
                            end: mat.end(),
                            matched_text: mat.as_str().to_string(),
                            highlight_type: HighlightType::GlobalSearch,
                        });
                        match_count += 1;
                    }

                    if !highlights.is_empty() && !matched_messages.contains(&msg_idx) {
                        matched_messages.push(msg_idx);
                    }
                }

                // Search in summary
                if let Some(ref summary) = conv.summary {
                    for _mat in regex.find_iter(summary) {
                        match_count += 1;
                    }
                }

                if match_count > 0 {
                    Some(SearchResult {
                        conversation: conv.clone(),
                        relevance_score: match_count as f64, // Simple scoring for regex
                        match_highlights: highlights,
                        match_count,
                        matched_messages,
                    })
                } else {
                    None
                }
            })
            .collect();

        // Sort by match count
        let mut sorted_results = results;
        sorted_results.par_sort_by(|a, b| b.match_count.cmp(&a.match_count));

        Ok(sorted_results)
    }

    /// Parallel boolean search across conversations
    fn search_boolean_parallel(
        &self,
        conversations: &mut [Conversation],
        boolean_query: &BooleanQuery,
    ) -> Result<Vec<SearchResult>, ClaudeToolsError> {
        let results: Vec<SearchResult> = conversations
            .par_iter()
            .filter_map(|conv| {
                let result = self.evaluate_boolean_query(conv, boolean_query);
                if result.matches {
                    Some(SearchResult {
                        conversation: conv.clone(),
                        relevance_score: result.score,
                        match_highlights: result.highlights,
                        match_count: result.match_count,
                        matched_messages: result.matched_messages,
                    })
                } else {
                    None
                }
            })
            .collect();

        // Sort by relevance score
        let mut sorted_results = results;
        sorted_results
            .par_sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());

        Ok(sorted_results)
    }

    /// Evaluate a boolean query against a conversation
    fn evaluate_boolean_query(
        &self,
        conversation: &Conversation,
        query: &BooleanQuery,
    ) -> BooleanResult {
        match query {
            BooleanQuery::Term(term) => self.evaluate_term(conversation, term),
            BooleanQuery::And(left, right) => {
                let left_result = self.evaluate_boolean_query(conversation, left);
                let right_result = self.evaluate_boolean_query(conversation, right);

                BooleanResult {
                    matches: left_result.matches && right_result.matches,
                    score: if left_result.matches && right_result.matches {
                        left_result.score + right_result.score
                    } else {
                        0.0
                    },
                    highlights: [left_result.highlights, right_result.highlights].concat(),
                    match_count: left_result.match_count + right_result.match_count,
                    matched_messages: [left_result.matched_messages, right_result.matched_messages]
                        .concat()
                        .into_iter()
                        .collect::<std::collections::HashSet<_>>()
                        .into_iter()
                        .collect(),
                }
            }
            BooleanQuery::Or(left, right) => {
                let left_result = self.evaluate_boolean_query(conversation, left);
                let right_result = self.evaluate_boolean_query(conversation, right);

                BooleanResult {
                    matches: left_result.matches || right_result.matches,
                    score: left_result.score + right_result.score,
                    highlights: [left_result.highlights, right_result.highlights].concat(),
                    match_count: left_result.match_count + right_result.match_count,
                    matched_messages: [left_result.matched_messages, right_result.matched_messages]
                        .concat()
                        .into_iter()
                        .collect::<std::collections::HashSet<_>>()
                        .into_iter()
                        .collect(),
                }
            }
            BooleanQuery::Not(inner) => {
                let inner_result = self.evaluate_boolean_query(conversation, inner);
                BooleanResult {
                    matches: !inner_result.matches,
                    score: if !inner_result.matches { 1.0 } else { 0.0 },
                    highlights: Vec::new(), // NOT queries don't highlight
                    match_count: if !inner_result.matches { 1 } else { 0 },
                    matched_messages: Vec::new(),
                }
            }
            BooleanQuery::Group(inner) => self.evaluate_boolean_query(conversation, inner),
        }
    }

    /// Evaluate a single term against a conversation
    fn evaluate_term(&self, conversation: &Conversation, term: &str) -> BooleanResult {
        let term_lower = term.to_lowercase();
        let mut highlights = Vec::new();
        let mut matched_messages = Vec::new();
        let mut match_count = 0;
        let mut score = 0.0;

        // Search in messages
        for (msg_idx, message) in conversation.messages.iter().enumerate() {
            let content_lower = message.content.to_lowercase();
            let mut start = 0;

            while let Some(pos) = content_lower[start..].find(&term_lower) {
                let actual_pos = start + pos;
                highlights.push(MatchHighlight {
                    message_index: msg_idx,
                    start: actual_pos,
                    end: actual_pos + term.len(),
                    matched_text: term.to_string(),
                    highlight_type: HighlightType::GlobalSearch,
                });
                match_count += 1;

                if !matched_messages.contains(&msg_idx) {
                    matched_messages.push(msg_idx);
                }

                start = actual_pos + 1;
            }
        }

        // Search in summary
        if let Some(ref summary) = conversation.summary {
            if summary.to_lowercase().contains(&term_lower) {
                match_count += 1;
                score += 0.5; // Summary matches get some weight
            }
        }

        // Calculate TF-IDF score for the term
        if match_count > 0 {
            let tf = self.calculate_term_frequency(term, conversation);
            let idf = self.calculate_inverse_document_frequency(term);
            score += tf * idf;
        }

        BooleanResult {
            matches: match_count > 0,
            score,
            highlights,
            match_count,
            matched_messages,
        }
    }

    /// Score a conversation using TF-IDF
    fn score_conversation(
        &self,
        conversation: &Conversation,
        query_words: &[String],
        query_text: &str,
        mode: &SearchMode,
    ) -> SearchResult {
        let mut total_score = 0.0;
        let mut highlights = Vec::new();
        let mut matched_messages = Vec::new();
        let mut match_count = 0;

        // Calculate TF-IDF score for each query word
        for word in query_words {
            if let Some(entries) = self.content_index.word_index.get(word) {
                for entry in entries {
                    if entry.conversation_id == conversation.session_id {
                        let tf = self.calculate_term_frequency(word, conversation);
                        let idf = self.calculate_inverse_document_frequency(word);
                        total_score += tf * idf;
                    }
                }
            }
        }

        // Find actual matches for highlighting
        match mode {
            SearchMode::Text | SearchMode::Advanced => {
                let query_lower = query_text.to_lowercase();

                for (msg_idx, message) in conversation.messages.iter().enumerate() {
                    let content_lower = message.content.to_lowercase();

                    // Find all occurrences
                    let mut start = 0;
                    while let Some(pos) = content_lower[start..].find(&query_lower) {
                        let actual_start = start + pos;
                        let actual_end = actual_start + query_text.len();

                        highlights.push(MatchHighlight {
                            message_index: msg_idx,
                            start: actual_start,
                            end: actual_end,
                            matched_text: message.content[actual_start..actual_end].to_string(),
                            highlight_type: HighlightType::GlobalSearch,
                        });

                        match_count += 1;
                        if !matched_messages.contains(&msg_idx) {
                            matched_messages.push(msg_idx);
                        }

                        start = actual_start + 1;
                    }
                }

                // Check summary
                if let Some(ref summary) = conversation.summary {
                    if summary.to_lowercase().contains(&query_lower) {
                        match_count += 1;
                    }
                }
            }
            _ => {
                // For other modes, use basic scoring
                if match_count == 0 && total_score > 0.0 {
                    match_count = 1; // Ensure non-zero match count for scoring
                }
            }
        }

        // Apply additional scoring factors
        total_score *= self.calculate_recency_boost(conversation);
        total_score *= self.calculate_length_normalization(conversation);

        SearchResult {
            conversation: conversation.clone(),
            relevance_score: total_score,
            match_highlights: highlights,
            match_count,
            matched_messages,
        }
    }

    /// Calculate term frequency for TF-IDF
    fn calculate_term_frequency(&self, term: &str, conversation: &Conversation) -> f64 {
        let term_lower = term.to_lowercase();
        let term_count = conversation
            .messages
            .iter()
            .map(|msg| msg.content.to_lowercase().matches(&term_lower).count())
            .sum::<usize>() as f64;

        let total_words = conversation
            .messages
            .iter()
            .map(|msg| Self::extract_words(&msg.content).len())
            .sum::<usize>() as f64;

        if total_words > 0.0 {
            term_count / total_words
        } else {
            0.0
        }
    }

    /// Calculate inverse document frequency for TF-IDF
    fn calculate_inverse_document_frequency(&self, term: &str) -> f64 {
        let term_lower = term.to_lowercase();
        if let Some(&doc_freq) = self.content_index.document_frequencies.get(&term_lower) {
            let total_docs = self.content_index.total_conversations as f64;
            let ratio = total_docs / doc_freq as f64;
            if ratio > 0.0 {
                ratio.ln().max(0.0) // Ensure non-negative
            } else {
                0.0
            }
        } else {
            // If term not found, give it a low score but not zero
            1.0
        }
    }

    /// Calculate recency boost factor
    fn calculate_recency_boost(&self, conversation: &Conversation) -> f64 {
        if let Some(last_updated) = conversation.last_updated {
            let now = Utc::now();
            let age = now.signed_duration_since(last_updated);
            let days_old = age.num_days() as f64;

            // Boost recent conversations (decay over 30 days)
            if days_old < 30.0 {
                1.0 + (30.0 - days_old) / 30.0 * 0.5
            } else {
                1.0
            }
        } else {
            1.0
        }
    }

    /// Calculate length normalization factor
    fn calculate_length_normalization(&self, conversation: &Conversation) -> f64 {
        let message_count = conversation.messages.len() as f64;
        // Slight boost for conversations with reasonable length (5-50 messages)
        if message_count >= 5.0 && message_count <= 50.0 {
            1.1
        } else {
            1.0
        }
    }

    /// Get candidate conversations based on filters
    fn get_candidate_conversations(
        &self,
        query: &SearchQuery,
    ) -> Result<Vec<Conversation>, ClaudeToolsError> {
        let mut candidates = self.conversations.clone();

        // Apply date range filter
        if let Some(ref date_range) = query.date_range {
            candidates = self.filter_by_date_range(candidates, date_range);
        }

        // Apply project filter
        if let Some(ref project) = query.project_filter {
            candidates.retain(|conv| conv.project_path.contains(project));
        }

        // Apply model filter
        if let Some(ref model) = query.model_filter {
            candidates.retain(|conv| {
                conv.messages
                    .iter()
                    .any(|msg| msg.model.as_ref().map_or(false, |m| m.contains(model)))
            });
        }

        // Apply tool filter
        if let Some(ref tool) = query.tool_filter {
            candidates.retain(|conv| {
                conv.messages.iter().any(|msg| {
                    msg.tool_uses
                        .iter()
                        .any(|tool_use| tool_use.name.contains(tool))
                })
            });
        }

        // Apply message role filter
        if let Some(ref role) = query.message_role_filter {
            use crate::claude::conversation::MessageRole as ConvRole;
            match role {
                MessageRole::User => {
                    candidates
                        .retain(|conv| conv.messages.iter().any(|msg| msg.role == ConvRole::User));
                }
                MessageRole::Assistant => {
                    candidates.retain(|conv| {
                        conv.messages
                            .iter()
                            .any(|msg| msg.role == ConvRole::Assistant)
                    });
                }
                MessageRole::System => {
                    candidates.retain(|conv| {
                        conv.messages.iter().any(|msg| msg.role == ConvRole::System)
                    });
                }
                MessageRole::Tool => {
                    // Tool messages are typically Assistant messages with tool uses
                    candidates
                        .retain(|conv| conv.messages.iter().any(|msg| !msg.tool_uses.is_empty()));
                }
            }
        }

        // Apply message count filters
        if let Some(min_messages) = query.min_messages {
            candidates.retain(|conv| conv.messages.len() >= min_messages);
        }
        if let Some(max_messages) = query.max_messages {
            candidates.retain(|conv| conv.messages.len() <= max_messages);
        }

        // Apply duration filters (estimated from timestamp differences)
        if query.min_duration_minutes.is_some() || query.max_duration_minutes.is_some() {
            candidates.retain(|conv| {
                if let (Some(start), Some(end)) = (conv.started_at, conv.last_updated) {
                    let duration_minutes = (end - start).num_minutes() as u32;

                    let min_ok = query
                        .min_duration_minutes
                        .map_or(true, |min| duration_minutes >= min);
                    let max_ok = query
                        .max_duration_minutes
                        .map_or(true, |max| duration_minutes <= max);

                    min_ok && max_ok
                } else {
                    false
                }
            });
        }

        Ok(candidates)
    }

    /// Filter conversations by date range
    fn filter_by_date_range(
        &self,
        mut conversations: Vec<Conversation>,
        date_range: &DateRange,
    ) -> Vec<Conversation> {
        conversations.retain(|conv| {
            if let Some(start_date) = conv.started_at {
                let after_start = date_range.start.map_or(true, |start| start_date >= start);
                let before_end = date_range.end.map_or(true, |end| start_date <= end);
                after_start && before_end
            } else {
                false
            }
        });
        conversations
    }

    /// Extract words from text for indexing
    fn extract_words(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split_whitespace()
            .filter(|word| word.len() > 2) // Filter very short words
            .map(|word| {
                // Remove punctuation
                word.chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect::<String>()
            })
            .filter(|word| !word.is_empty())
            .collect()
    }

    /// Calculate hash for query caching
    fn calculate_query_hash(&self, query: &SearchQuery) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        query.text.hash(&mut hasher);
        query.regex_pattern.hash(&mut hasher);
        query.project_filter.hash(&mut hasher);
        // Note: DateRange doesn't implement Hash, so we'll hash the string representation
        if let Some(ref range) = query.date_range {
            format!("{:?}", range).hash(&mut hasher);
        }
        hasher.finish()
    }
}

impl InvertedIndex {
    fn new() -> Self {
        Self {
            word_index: HashMap::new(),
            document_frequencies: HashMap::new(),
            total_conversations: 0,
        }
    }
}

impl DateIndex {
    fn new() -> Self {
        Self {
            conversations_by_date: Vec::new(),
        }
    }
}

impl SearchQuery {
    /// Create a simple text search query
    pub fn text(query: &str) -> Self {
        Self {
            text: Some(query.to_string()),
            regex_pattern: None,
            boolean_query: None,
            date_range: None,
            project_filter: None,
            model_filter: None,
            tool_filter: None,
            message_role_filter: None,
            min_messages: None,
            max_messages: None,
            min_duration_minutes: None,
            max_duration_minutes: None,
            search_mode: SearchMode::Text,
            max_results: None,
        }
    }

    /// Create a regex search query
    pub fn regex(pattern: &str) -> Self {
        Self {
            text: None,
            regex_pattern: Some(pattern.to_string()),
            boolean_query: None,
            date_range: None,
            project_filter: None,
            model_filter: None,
            tool_filter: None,
            message_role_filter: None,
            min_messages: None,
            max_messages: None,
            min_duration_minutes: None,
            max_duration_minutes: None,
            search_mode: SearchMode::Regex,
            max_results: None,
        }
    }

    /// Create a boolean search query
    pub fn boolean(query: &str) -> Result<Self, ClaudeToolsError> {
        let boolean_query = BooleanQueryParser::parse(query)?;
        Ok(Self {
            text: None,
            regex_pattern: None,
            boolean_query: Some(boolean_query),
            date_range: None,
            project_filter: None,
            model_filter: None,
            tool_filter: None,
            message_role_filter: None,
            min_messages: None,
            max_messages: None,
            min_duration_minutes: None,
            max_duration_minutes: None,
            search_mode: SearchMode::Advanced,
            max_results: None,
        })
    }

    /// Add date range filter
    pub fn with_date_range(
        mut self,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
    ) -> Self {
        self.date_range = Some(DateRange { start, end });
        self
    }

    /// Add project filter
    pub fn with_project(mut self, project: &str) -> Self {
        self.project_filter = Some(project.to_string());
        self
    }

    /// Add model filter
    pub fn with_model(mut self, model: &str) -> Self {
        self.model_filter = Some(model.to_string());
        self
    }

    /// Add tool filter
    pub fn with_tool(mut self, tool: &str) -> Self {
        self.tool_filter = Some(tool.to_string());
        self
    }

    /// Add message role filter
    pub fn with_role(mut self, role: MessageRole) -> Self {
        self.message_role_filter = Some(role);
        self
    }

    /// Add message count range filter
    pub fn with_message_count(mut self, min: Option<usize>, max: Option<usize>) -> Self {
        self.min_messages = min;
        self.max_messages = max;
        self
    }

    /// Add duration filter (in minutes)
    pub fn with_duration(mut self, min_minutes: Option<u32>, max_minutes: Option<u32>) -> Self {
        self.min_duration_minutes = min_minutes;
        self.max_duration_minutes = max_minutes;
        self
    }

    /// Set maximum results
    pub fn with_max_results(mut self, max: usize) -> Self {
        self.max_results = Some(max);
        self
    }
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            text: None,
            regex_pattern: None,
            boolean_query: None,
            date_range: None,
            project_filter: None,
            model_filter: None,
            tool_filter: None,
            message_role_filter: None,
            min_messages: None,
            max_messages: None,
            min_duration_minutes: None,
            max_duration_minutes: None,
            search_mode: SearchMode::Text,
            max_results: None,
        }
    }
}

impl DateRange {
    /// Create date range for last N days
    pub fn last_days(days: i64) -> Self {
        let end = Utc::now();
        let start = end - Duration::days(days);
        Self {
            start: Some(start),
            end: Some(end),
        }
    }

    /// Create date range for last week
    pub fn last_week() -> Self {
        Self::last_days(7)
    }

    /// Create date range for last month
    pub fn last_month() -> Self {
        Self::last_days(30)
    }

    /// Create date range for last year
    pub fn last_year() -> Self {
        Self::last_days(365)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_conversation() -> Conversation {
        use crate::claude::conversation::{ConversationMessage, MessageRole};

        Conversation {
            session_id: "test-123".to_string(),
            project_path: "test-project".to_string(),
            summary: Some("Test conversation about Rust programming".to_string()),
            messages: vec![
                ConversationMessage {
                    uuid: "msg1".to_string(),
                    parent_uuid: None,
                    role: MessageRole::User,
                    content: "How do I implement error handling in Rust?".to_string(),
                    timestamp: Utc::now(),
                    model: None,
                    tool_uses: vec![],
                },
                ConversationMessage {
                    uuid: "msg2".to_string(),
                    parent_uuid: Some("msg1".to_string()),
                    role: MessageRole::Assistant,
                    content: "Rust has excellent error handling with Result and Option types. You can use match or ? operator.".to_string(),
                    timestamp: Utc::now(),
                    model: Some("claude-3".to_string()),
                    tool_uses: vec![],
                },
            ],
            started_at: Some(Utc::now()),
            last_updated: Some(Utc::now()),
        }
    }

    #[test]
    fn test_search_engine_creation() {
        let engine = SearchEngine::new();
        assert_eq!(engine.conversations.len(), 0);
    }

    #[test]
    fn test_search_engine_indexing() {
        let mut engine = SearchEngine::new();
        let conversations = vec![create_test_conversation()];

        let result = engine.build_index(conversations);
        assert!(result.is_ok());
        assert_eq!(engine.conversations.len(), 1);
    }

    #[test]
    fn test_text_search() {
        let mut engine = SearchEngine::new();
        let conversations = vec![create_test_conversation()];
        engine.build_index(conversations).unwrap();

        // Test basic text search
        let query = SearchQuery::text("rust");
        let results = engine.search(&query).unwrap();

        assert!(!results.is_empty());
        assert!(results[0].match_count > 0);

        // Test another search term
        let query2 = SearchQuery::text("error handling");
        let results2 = engine.search(&query2).unwrap();

        assert!(!results2.is_empty());
        assert!(results2[0].match_count > 0);
    }

    #[test]
    fn test_date_range_query() {
        let query = SearchQuery::text("test")
            .with_date_range(Some(Utc::now() - Duration::days(1)), Some(Utc::now()));

        assert!(query.date_range.is_some());
    }

    #[test]
    fn test_word_extraction() {
        let words = SearchEngine::extract_words("Hello, world! This is a test.");
        assert!(words.contains(&"hello".to_string()));
        assert!(words.contains(&"world".to_string()));
        assert!(words.contains(&"test".to_string()));
        // Short words like "is" and "a" should be filtered out
        assert!(!words.contains(&"is".to_string()));
    }
}
