use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_help_output() {
    let mut cmd = Command::cargo_bin("claude-tools").unwrap();
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Claude Tools provides utilities"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("show"))
        .stdout(predicate::str::contains("search"))
        .stdout(predicate::str::contains("stats"));
}

#[test]
fn test_version_output() {
    let mut cmd = Command::cargo_bin("claude-tools").unwrap();
    cmd.arg("--version");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("claude-tools"));
}

#[test]
fn test_list_subcommand_help() {
    let mut cmd = Command::cargo_bin("claude-tools").unwrap();
    cmd.args(["list", "--help"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("List conversations"))
        .stdout(predicate::str::contains("--since"))
        .stdout(predicate::str::contains("--project"))
        .stdout(predicate::str::contains("--detailed"));
}

#[test]
fn test_missing_directory_error() {
    let mut cmd = Command::cargo_bin("claude-tools").unwrap();
    cmd.args(["--claude-dir", "/nonexistent", "list"]);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Claude directory not found"))
        .stderr(predicate::str::contains("💡 Suggestions"));
}

#[test]
fn test_invalid_subcommand() {
    let mut cmd = Command::cargo_bin("claude-tools").unwrap();
    cmd.arg("invalid-command");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

#[test]
fn test_list_with_valid_directory() {
    let temp_dir = TempDir::new().unwrap();
    let claude_dir = temp_dir.path().join("claude");
    std::fs::create_dir(&claude_dir).unwrap();
    std::fs::create_dir(claude_dir.join("projects")).unwrap();

    let mut cmd = Command::cargo_bin("claude-tools").unwrap();
    cmd.args(["--claude-dir", claude_dir.to_str().unwrap(), "list"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No conversations found"));
}

#[test]
fn test_show_placeholder() {
    let temp_dir = TempDir::new().unwrap();
    let claude_dir = temp_dir.path().join("claude");
    std::fs::create_dir(&claude_dir).unwrap();
    std::fs::create_dir(claude_dir.join("projects")).unwrap();

    let mut cmd = Command::cargo_bin("claude-tools").unwrap();
    cmd.args([
        "--claude-dir",
        claude_dir.to_str().unwrap(),
        "show",
        "test-id",
    ]);

    cmd.assert().success().stdout(predicate::str::contains(
        "❌ Conversation not found: test-id",
    ));
}

#[test]
fn test_search_placeholder() {
    let temp_dir = TempDir::new().unwrap();
    let claude_dir = temp_dir.path().join("claude");
    std::fs::create_dir(&claude_dir).unwrap();
    std::fs::create_dir(claude_dir.join("projects")).unwrap();

    let mut cmd = Command::cargo_bin("claude-tools").unwrap();
    cmd.args([
        "--claude-dir",
        claude_dir.to_str().unwrap(),
        "search",
        "test query",
    ]);

    cmd.assert().success().stdout(predicate::str::contains(
        "No conversations found matching: test query",
    ));
}
