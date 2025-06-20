use clap::Parser;
use claude_tools::claude::ClaudeDirectory;
use claude_tools::cli::{execute_command, Cli};
use claude_tools::errors::ClaudeToolsError;

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        match e {
            ClaudeToolsError::DirectoryNotFound { path } => {
                eprintln!("❌ Error: Claude directory not found: {}", path);
                eprintln!("💡 Suggestions:");
                eprintln!("   • Make sure Claude Code has been run at least once");
                eprintln!("   • Use --claude-dir to specify a custom directory");
                eprintln!("   • Check that ~/.claude/ exists and contains conversation data");
                std::process::exit(1);
            }
            ClaudeToolsError::InvalidDirectory { path, reason } => {
                eprintln!("❌ Error: Invalid Claude directory: {}", path);
                eprintln!("   Reason: {}", reason);
                eprintln!("💡 Use --claude-dir to specify a valid Claude Code directory");
                std::process::exit(1);
            }
            ClaudeToolsError::Config(msg) => {
                eprintln!("❌ Configuration error: {}", msg);
                std::process::exit(1);
            }
            ClaudeToolsError::Io(io_err) => {
                eprintln!("❌ IO error: {}", io_err);
                std::process::exit(1);
            }
            ClaudeToolsError::Json(json_err) => {
                eprintln!("❌ JSON parsing error: {}", json_err);
                std::process::exit(1);
            }
        }
    }
}

fn run(cli: Cli) -> Result<(), ClaudeToolsError> {
    // Determine Claude directory
    let claude_dir = if let Some(path) = cli.claude_dir {
        ClaudeDirectory::from_path(path)?
    } else {
        ClaudeDirectory::auto_detect()?
    };

    if cli.verbose {
        eprintln!("📁 Using Claude directory: {}", claude_dir.path.display());
    }

    // Execute the command
    execute_command(claude_dir, cli.command, cli.verbose)?;

    Ok(())
}
