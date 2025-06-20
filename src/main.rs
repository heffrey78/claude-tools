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
                eprintln!("💡 Help: Run 'claude-tools --help' for usage information");
                std::process::exit(1);
            }
            ClaudeToolsError::Io(io_err) => {
                eprintln!("❌ IO error: {}", io_err);
                eprintln!("💡 Suggestions:");
                eprintln!("   • Check file permissions and disk space");
                eprintln!("   • Ensure the Claude directory is readable");
                eprintln!("   • Try running with --verbose for more details");
                std::process::exit(1);
            }
            ClaudeToolsError::Json(json_err) => {
                eprintln!("❌ JSON parsing error: {}", json_err);
                eprintln!("💡 This usually indicates corrupted conversation files");
                eprintln!("   • The conversation data may be incomplete or corrupted");
                eprintln!("   • Try refreshing the conversation list with 'r' in interactive mode");
                eprintln!("   • Check if Claude Code is currently running and try again");
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
