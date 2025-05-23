// tests/common/git_helpers.rs

//! Provides helper functions for setting up and managing temporary Git repositories
//! for integration testing purposes. This module aims to simplify the creation of
//! specific Git states (e.g., repositories with a defined commit history) that
//! can then be used as a basis for testing the main application's Git interaction logic.

use std::path::Path;
use std::process::Command;
use tempfile::TempDir;
use anyhow::{Context, Result};

/// Sets up a temporary Git repository with a series of commits.
///
/// This function performs the following steps:
/// 1. Creates a new temporary directory to house the Git repository.
/// 2. Initializes a new Git repository in this directory (`git init`).
/// 3. Configures a default user name (`Test User`) and email (`test@example.com`) for commits.
/// 4. Sets the `safe.directory` git config to the repository path to avoid ownership issues in CI/test environments.
/// 5. Creates an initial empty commit with the message "Initial commit". This serves as a baseline.
/// 6. For each message provided in the `commit_messages` slice:
///    a. Creates a new dummy file (e.g., `file0.txt`, `file1.txt`) with the commit message as its content.
///       This ensures each commit has unique content and is not empty.
///    b. Stages this file (`git add <filename>`).
///    c. Creates a commit with the provided message (`git commit -m "<message>"`).
///
/// The commit messages do not need to follow any specific format for this helper, as it
/// simply uses them for the `git commit -m` command. The main application's parsing logic
/// for Jira keys will be tested against these messages.
///
/// # Parameters
///
/// * `commit_messages`: A slice of string slices, where each inner string slice is a commit message
///   for a new commit to be created after the initial one. Commits are created in the order they appear.
///
/// # Returns
///
/// * `Result<TempDir>`: On success, returns a `tempfile::TempDir` object. This object represents
///   the temporary directory where the Git repository is located. The directory and its contents
///   (including the Git repository) will be automatically cleaned up when the `TempDir` object
///   goes out of scope (RAII). If any step in the setup fails, an `anyhow::Error` is returned
///   detailing the context of the failure.
pub fn setup_repo_with_commits(commit_messages: &[&str]) -> Result<TempDir> {
    let temp_dir = TempDir::new().context("Failed to create temporary directory for Git repo")?;
    let repo_path = temp_dir.path();

    // 1. Initialize a git repository
    run_git_command(&["init"], repo_path).context("Failed to initialize Git repository")?;

    // Configure user.name and user.email to ensure commits can be made
    run_git_command(&["config", "user.name", "Test User"], repo_path)
        .context("Failed to set Git user.name")?;
    run_git_command(&["config", "user.email", "test@example.com"], repo_path)
        .context("Failed to set Git user.email")?;
    
    // Configure safe.directory to avoid potential ownership issues in CI environments
    // This is important as the test runner might be a different user than the directory owner.
    run_git_command(&["config", "safe.directory", repo_path.to_str().unwrap_or("")], repo_path)
        .context("Failed to set Git safe.directory configuration")?;

    // Create an initial empty commit to establish a baseline (e.g., HEAD)
    run_git_command(&["commit", "--allow-empty", "-m", "Initial commit"], repo_path)
        .context("Failed to create initial empty commit")?;

    // Create subsequent commits based on the provided messages
    for (i, message) in commit_messages.iter().enumerate() {
        // Create a dummy file for each commit to have something to add
        // This makes the commits more realistic than just --allow-empty for all.
        let file_name = format!("file{}.txt", i);
        // Create a unique file for each commit to ensure it's not empty and has distinct content.
        let file_name = format!("file{}.txt", i);
        std::fs::write(repo_path.join(&file_name), format!("Content for commit: {}", message))
            .with_context(|| format!("Failed to write dummy file '{}'", file_name))?;
        run_git_command(&["add", &file_name], repo_path)
            .with_context(|| format!("Failed to `git add {}`", file_name))?;
        run_git_command(&["commit", "-m", message], repo_path)
            .with_context(|| format!("Failed to create commit with message: '{}'", message))?;
    }

    Ok(temp_dir)
}

/// Executes a Git command with the given arguments in the specified repository directory.
///
/// # Parameters
/// * `args`: A slice of string slices representing the arguments for the `git` command (e.g., `&["commit", "-m", "message"]`).
/// * `repo_path`: The path to the directory where the Git command should be executed.
///
/// # Returns
/// * `Result<()>`: Returns `Ok(())` if the command was successful. Otherwise, returns an `anyhow::Error`
///   containing details about the failure, including stdout and stderr from the command.
fn run_git_command(args: &[&str], repo_path: &Path) -> Result<()> {
    let command_str = format!("git {}", args.join(" "));
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .output()
        .with_context(|| format!("Failed to execute command: `{}` in {}", command_str, repo_path.display()))?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Command `{}` failed in {}:\nExit Status: {}\nStdout:\n{}\nStderr:\n{}",
            command_str,
            repo_path.display(),
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

// The example_git_helper can be removed as it's no longer a placeholder.
// pub fn example_git_helper() {
//     println!("Git helper placeholder updated (if needed)");
// }
