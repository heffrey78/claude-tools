use super::conversation::{Conversation, ConversationEntry};
use crate::errors::ClaudeToolsError;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;

/// Streaming parser for large conversation files with indexing support
pub struct StreamingConversationParser {
    file: File,
    line_index: Vec<u64>, // Byte positions of each line start
    metadata_cache: Option<ConversationMetadata>,
}

/// Minimal metadata extracted without full parsing
#[derive(Debug, Clone)]
pub struct ConversationMetadata {
    pub session_id: String,
    pub project_path: String,
    pub line_count: usize,
    pub file_size: u64,
    pub summary: Option<String>,
    pub first_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    pub last_timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

impl StreamingConversationParser {
    /// Create a new streaming parser for a conversation file
    pub fn new(file_path: &Path, _project_path: &str) -> Result<Self, ClaudeToolsError> {
        let mut file = File::open(file_path)?;
        let _session_id = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Build index of line positions for O(1) seeking
        let line_index = Self::build_line_index(&mut file)?;

        Ok(Self {
            file,
            line_index,
            metadata_cache: None,
        })
    }

    /// Build an index of byte positions for each line
    fn build_line_index(file: &mut File) -> Result<Vec<u64>, ClaudeToolsError> {
        file.seek(SeekFrom::Start(0))?;
        let reader = BufReader::new(file);
        let mut index = Vec::new();
        let mut pos = 0u64;

        for line_result in reader.lines() {
            index.push(pos);
            let line = line_result?;
            pos += line.len() as u64 + 1; // +1 for newline character
        }

        Ok(index)
    }

    /// Extract metadata without parsing entire file
    pub fn get_metadata(&mut self, session_id: String, project_path: String) -> Result<ConversationMetadata, ClaudeToolsError> {
        if let Some(ref cached) = self.metadata_cache {
            return Ok(cached.clone());
        }

        let file_size = self.file.metadata()?.len();
        let line_count = self.line_index.len();

        // Parse first and last entries only for timestamps
        let (first_timestamp, summary) = if !self.line_index.is_empty() {
            let first_entry = self.read_entry_at(0)?;
            let first_ts = Self::extract_timestamp(&first_entry);
            let summary = Self::extract_summary(&first_entry);
            (first_ts, summary)
        } else {
            (None, None)
        };

        let last_timestamp = if self.line_index.len() > 1 {
            let last_entry = self.read_entry_at(self.line_index.len() - 1)?;
            Self::extract_timestamp(&last_entry)
        } else {
            first_timestamp
        };

        let metadata = ConversationMetadata {
            session_id,
            project_path,
            line_count,
            file_size,
            summary,
            first_timestamp,
            last_timestamp,
        };

        self.metadata_cache = Some(metadata.clone());
        Ok(metadata)
    }

    /// Read a specific entry by line index
    pub fn read_entry_at(&mut self, index: usize) -> Result<ConversationEntry, ClaudeToolsError> {
        if index >= self.line_index.len() {
            return Err(ClaudeToolsError::Config(format!(
                "Index {} out of bounds (max: {})",
                index,
                self.line_index.len() - 1
            )));
        }

        // Seek to the specific line position
        self.file.seek(SeekFrom::Start(self.line_index[index]))?;
        let mut reader = BufReader::new(&self.file);
        let mut line = String::new();
        reader.read_line(&mut line)?;

        // Parse the JSON line
        serde_json::from_str(&line).map_err(|e| {
            ClaudeToolsError::Config(format!("Failed to parse JSON at line {}: {}", index, e))
        })
    }

    /// Stream entries in chunks for memory efficiency
    pub fn stream_entries_chunked(&mut self, chunk_size: usize) -> StreamingIterator {
        StreamingIterator {
            parser: self,
            current_index: 0,
            chunk_size,
        }
    }

    /// Get entry count without loading all entries
    pub fn entry_count(&self) -> usize {
        self.line_index.len()
    }

    /// Extract timestamp from an entry
    fn extract_timestamp(entry: &ConversationEntry) -> Option<chrono::DateTime<chrono::Utc>> {
        match entry {
            ConversationEntry::User { timestamp, .. } => Some(*timestamp),
            ConversationEntry::Assistant { timestamp, .. } => Some(*timestamp),
            ConversationEntry::Summary { .. } => None,
        }
    }

    /// Extract summary from an entry if it's a summary type
    fn extract_summary(entry: &ConversationEntry) -> Option<String> {
        match entry {
            ConversationEntry::Summary { summary, .. } => Some(summary.clone()),
            _ => None,
        }
    }

    /// Convert to full conversation (for backward compatibility)
    pub fn to_conversation(&mut self, session_id: String, project_path: String) -> Result<Conversation, ClaudeToolsError> {
        let mut entries = Vec::new();
        
        // Read all entries (this loads everything into memory)
        for i in 0..self.line_index.len() {
            match self.read_entry_at(i) {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    eprintln!("Warning: Failed to parse entry {}: {}", i, e);
                }
            }
        }

        Ok(Conversation::from_entries(session_id, project_path, entries))
    }
}

/// Iterator for streaming entries in chunks
pub struct StreamingIterator<'a> {
    parser: &'a mut StreamingConversationParser,
    current_index: usize,
    chunk_size: usize,
}

impl<'a> Iterator for StreamingIterator<'a> {
    type Item = Result<Vec<ConversationEntry>, ClaudeToolsError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.parser.entry_count() {
            return None;
        }

        let end_index = (self.current_index + self.chunk_size).min(self.parser.entry_count());
        let mut chunk = Vec::new();

        // Read chunk of entries
        for i in self.current_index..end_index {
            match self.parser.read_entry_at(i) {
                Ok(entry) => chunk.push(entry),
                Err(e) => return Some(Err(e)),
            }
        }

        self.current_index = end_index;
        Some(Ok(chunk))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    fn create_test_jsonl() -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, r#"{{"type":"summary","summary":"Test conversation","leafUuid":"uuid1"}}"#).unwrap();
        writeln!(file, r#"{{"type":"user","sessionId":"test","message":{{"role":"user","content":"Hello"}},"uuid":"uuid2","timestamp":"2024-01-01T00:00:00Z","parentUuid":null}}"#).unwrap();
        writeln!(file, r#"{{"type":"assistant","sessionId":"test","message":{{"id":"msg1","type":"message","role":"assistant","content":[{{"type":"text","text":"Hi there!"}}]}},"uuid":"uuid3","timestamp":"2024-01-01T00:01:00Z","parentUuid":"uuid2"}}"#).unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_streaming_parser_creation() {
        let test_file = create_test_jsonl();
        let parser = StreamingConversationParser::new(test_file.path(), "test_project");
        assert!(parser.is_ok());
        
        let parser = parser.unwrap();
        assert_eq!(parser.entry_count(), 3);
    }

    #[test]
    fn test_metadata_extraction() {
        let test_file = create_test_jsonl();
        let mut parser = StreamingConversationParser::new(test_file.path(), "test_project").unwrap();
        
        let metadata = parser.get_metadata("test_session".to_string(), "test_project".to_string()).unwrap();
        assert_eq!(metadata.line_count, 3);
        assert_eq!(metadata.session_id, "test_session");
        assert_eq!(metadata.summary, Some("Test conversation".to_string()));
    }

    #[test]
    fn test_entry_reading() {
        let test_file = create_test_jsonl();
        let mut parser = StreamingConversationParser::new(test_file.path(), "test_project").unwrap();
        
        let first_entry = parser.read_entry_at(0).unwrap();
        match first_entry {
            ConversationEntry::Summary { summary, .. } => {
                assert_eq!(summary, "Test conversation");
            }
            _ => panic!("Expected summary entry"),
        }
    }
}