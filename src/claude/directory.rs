use crate::errors::{ClaudeToolsError, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ClaudeDirectory {
    pub path: PathBuf,
}

impl ClaudeDirectory {
    pub fn auto_detect() -> Result<Self> {
        let home_dir = dirs::home_dir().ok_or_else(|| {
            ClaudeToolsError::Config("Unable to determine home directory".to_string())
        })?;

        let claude_dir = home_dir.join(".claude");

        if claude_dir.exists() && claude_dir.is_dir() {
            Self::from_path(claude_dir)
        } else {
            Err(ClaudeToolsError::DirectoryNotFound {
                path: claude_dir.display().to_string(),
            })
        }
    }

    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        if !path.exists() {
            return Err(ClaudeToolsError::DirectoryNotFound {
                path: path.display().to_string(),
            });
        }

        if !path.is_dir() {
            return Err(ClaudeToolsError::InvalidDirectory {
                path: path.display().to_string(),
                reason: "Path is not a directory".to_string(),
            });
        }

        // Validate that this looks like a Claude directory
        Self::validate_claude_directory(&path)?;

        Ok(ClaudeDirectory { path })
    }

    fn validate_claude_directory(path: &Path) -> Result<()> {
        // Check for typical Claude directory structure
        let projects_dir = path.join("projects");

        if !projects_dir.exists() {
            eprintln!(
                "Warning: Directory {} doesn't contain 'projects' subdirectory",
                path.display()
            );
            eprintln!("This may not be a valid Claude Code directory.");
        }

        Ok(())
    }

    pub fn projects_dir(&self) -> PathBuf {
        self.path.join("projects")
    }

    pub fn todos_dir(&self) -> PathBuf {
        self.path.join("todos")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_from_path_nonexistent() {
        let result = ClaudeDirectory::from_path("/nonexistent/path");
        assert!(matches!(
            result,
            Err(ClaudeToolsError::DirectoryNotFound { .. })
        ));
    }

    #[test]
    fn test_from_path_valid_directory() {
        let temp_dir = TempDir::new().unwrap();
        let claude_dir = temp_dir.path().join("claude");
        std::fs::create_dir(&claude_dir).unwrap();
        std::fs::create_dir(claude_dir.join("projects")).unwrap();

        let result = ClaudeDirectory::from_path(&claude_dir);
        assert!(result.is_ok());
    }
}
