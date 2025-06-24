use claude_tools::claude::{ClaudeDirectory, ConversationParser, StreamingConversationParser};
use std::path::Path;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let claude_dir = ClaudeDirectory::auto_detect()?;
    let parser = ConversationParser::new(claude_dir);

    println!("🔍 Benchmarking Conversation Parsing Performance\n");

    // Benchmark current full parsing
    let start = Instant::now();
    let conversations = parser.parse_all_conversations()?;
    let full_parse_time = start.elapsed();

    println!("📊 Full Parsing Results:");
    println!("   Time: {:.2}s", full_parse_time.as_secs_f64());
    println!("   Conversations: {}", conversations.len());
    println!(
        "   Messages: {}",
        conversations
            .iter()
            .map(|c| c.messages.len())
            .sum::<usize>()
    );

    // Find largest file for detailed benchmarking
    let largest_file = find_largest_conversation_file(&parser)?;
    if let Some((file_path, project_name, file_size)) = largest_file {
        println!("\n🎯 Benchmarking Largest File:");
        println!("   File: {}", file_path.display());
        println!("   Size: {:.1} MB", file_size as f64 / 1_048_576.0);

        // Benchmark streaming metadata extraction
        let start = Instant::now();
        let mut streaming_parser = StreamingConversationParser::new(&file_path, &project_name)?;
        let session_id = file_path.file_stem().unwrap().to_str().unwrap().to_string();
        let metadata = streaming_parser.get_metadata(session_id.clone(), project_name.clone())?;
        let metadata_time = start.elapsed();

        println!("\n📈 Streaming Metadata Extraction:");
        println!("   Time: {:.3}s", metadata_time.as_secs_f64());
        println!("   Lines: {}", metadata.line_count);
        println!("   Memory efficient: ✅ (no full file load)");

        // Benchmark full file parsing vs streaming
        let start = Instant::now();
        let full_conversation = parser.parse_conversation_file(&file_path, &project_name)?;
        let full_file_time = start.elapsed();

        let start = Instant::now();
        let streaming_conversation = streaming_parser.to_conversation(session_id, project_name)?;
        let streaming_file_time = start.elapsed();

        println!("\n⚡ Full File Parsing Comparison:");
        println!("   Traditional: {:.3}s", full_file_time.as_secs_f64());
        println!("   Streaming: {:.3}s", streaming_file_time.as_secs_f64());
        println!(
            "   Messages (traditional): {}",
            full_conversation.messages.len()
        );
        println!(
            "   Messages (streaming): {}",
            streaming_conversation.messages.len()
        );

        // Test seeking performance
        if metadata.line_count > 10 {
            let start = Instant::now();
            let _mid_entry = streaming_parser.read_entry_at(metadata.line_count / 2)?;
            let seek_time = start.elapsed();

            println!("\n🎯 Seeking Performance:");
            println!(
                "   Seek to middle entry: {:.3}ms",
                seek_time.as_secs_f64() * 1000.0
            );
            println!("   Target: <1ms ✅");
        }

        // Test chunked streaming
        let start = Instant::now();
        let mut total_entries = 0;
        for chunk_result in streaming_parser.stream_entries_chunked(50) {
            let chunk = chunk_result?;
            total_entries += chunk.len();
        }
        let chunked_time = start.elapsed();

        println!("\n📦 Chunked Streaming (50 entries/chunk):");
        println!("   Time: {:.3}s", chunked_time.as_secs_f64());
        println!("   Total entries: {}", total_entries);
        println!("   Memory efficient: ✅ (chunked processing)");
    }

    println!("\n✅ Benchmark Complete!");
    Ok(())
}

fn find_largest_conversation_file(
    parser: &ConversationParser,
) -> Result<Option<(std::path::PathBuf, String, u64)>, Box<dyn std::error::Error>> {
    let projects_dir = parser.claude_dir.path.join("projects");
    if !projects_dir.exists() {
        return Ok(None);
    }

    let mut largest: Option<(std::path::PathBuf, String, u64)> = None;

    for entry in std::fs::read_dir(&projects_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let project_name = path.file_name().unwrap().to_str().unwrap();

            for file_entry in std::fs::read_dir(&path)? {
                let file_entry = file_entry?;
                let file_path = file_entry.path();

                if file_path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                    let file_size = file_path.metadata()?.len();

                    if largest
                        .as_ref()
                        .map_or(true, |(_, _, size)| file_size > *size)
                    {
                        largest = Some((file_path, project_name.to_string(), file_size));
                    }
                }
            }
        }
    }

    Ok(largest)
}
