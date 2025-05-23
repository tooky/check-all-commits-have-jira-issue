// tests/integration_main.rs

//! Integration tests for the `my_rust_project` binary.
//!
//! These tests cover various scenarios by:
//! 1. Setting up mock Jira servers using `wiremock`.
//! 2. Creating temporary Git repositories with specific commit histories using helpers from `common::git_helpers`.
//! 3. Running the compiled binary (`my_rust_project`) using `assert_cmd`.
//! 4. Asserting the command's exit code, stdout, and stderr against expected outcomes using `predicates`.

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::path::Path; // For Path type in helper
use std::process::Command;
use wiremock::MockServer;
use anyhow::Result;

mod common;
use common::git_helpers::setup_repo_with_commits;
use common::jira_helpers::{mock_issue_exists, mock_issue_not_found, mock_auth_failure};

/// Helper function to set up the common parts of the command for integration tests.
fn setup_command<'a>(
    temp_repo_path: &'a Path,
    mock_server_uri: &'a str,
    start_ref: &'a str,
    end_ref: &'a str,
) -> anyhow::Result<Command> {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    cmd.current_dir(temp_repo_path);
    cmd.arg("--jira-url").arg(mock_server_uri)
        .arg("--username").arg("testuser") // Standardized test credentials
        .arg("--api-token").arg("testtoken")
        .arg("--start-ref").arg(start_ref)
        .arg("--end-ref").arg(end_ref);
    Ok(cmd)
}

#[tokio::test]
async fn test_successful_validation() -> Result<()> {
    // Scenario: All commits in the range are valid (Jira key present, Jira issue exists).
    // Expected: Program exits successfully (0), stdout indicates all commits valid.

    // 1. Setup Jira Mock Server
    let mock_server = MockServer::start().await;
    mock_issue_exists(&mock_server, "PROJ-123").await?;
    mock_issue_exists(&mock_server, "PROJ-124").await?;

    // 2. Setup Git Repo
    let temp_repo_dir = setup_repo_with_commits(&[
        "feat: PROJ-123 Implement feature X",
        "fix: PROJ-124 Address bug Y",
    ])?;
    let repo_path = temp_repo_dir.path();

    // 3. Run the validator command using the helper
    let mut cmd = setup_command(repo_path, &mock_server.uri(), "HEAD~2", "HEAD")?;

    // 4. Assertions
    cmd.assert()
        .success()
        .stdout(
            predicate::str::contains("Validating commit")
            .and(predicate::str::contains("feat: PROJ-123 Implement feature X").and(predicate::str::contains("VALID (Jira Key: PROJ-123)")))
            .and(predicate::str::contains("fix: PROJ-124 Address bug Y").and(predicate::str::contains("VALID (Jira Key: PROJ-124)")))
            .and(predicate::str::contains("Total commits scanned: 2"))
            .and(predicate::str::contains("Valid commits: 2"))
            .and(predicate::str::contains("Invalid commits: 0"))
            .and(predicate::str::contains(">>> Final Result: Validation SUCCESSFUL."))
        )
        .stderr(predicate::str::is_empty()); // Expect no errors on stderr for a fully successful run

    Ok(())
}

#[tokio::test]
async fn test_failed_invalid_jira_credentials() -> anyhow::Result<()> {
    // Scenario: Jira credentials provided are invalid, leading to a 401 error from Jira API.
    // Expected: Program exits with failure (1), stdout indicates the commit couldn't be validated due to auth error.

    // 1. Setup Jira Mock Server for auth failure
    let mock_server = MockServer::start().await;
    mock_auth_failure(&mock_server, r"^/rest/api/2/issue/.*").await?;

    // 2. Setup Git Repo
    let temp_repo_dir = setup_repo_with_commits(&[
        "feat: PROJ-456 Implement something",
    ])?;
    let repo_path = temp_repo_dir.path();

    // 3. Run the validator command using the helper (note: username/token in helper are standard,
    // but the mock server will cause failure regardless of these specific values).
    // For a more direct test of "baduser"/"badtoken", the helper would need to take them as args.
    // However, since `mock_auth_failure` mocks a 401 for *any* auth, this setup is fine.
    let mut cmd = setup_command(repo_path, &mock_server.uri(), "HEAD~1", "HEAD")?;
    // If we wanted to specifically test the tool's reaction to different *input* credentials,
    // we would modify `cmd` here, e.g., by removing and re-adding specific args.
    // For this test, the mock server forces the 401, so the helper's standard creds are okay.

    // 4. Assertions
    cmd.assert()
        .failure()
        .stdout(
            predicate::str::contains("PROJ-456") // The commit should still be listed
            // Check for the specific error message related to auth failure for the key
            .and(predicate::str::contains("INVALID - Error: Error validating Jira key 'PROJ-456': Unauthorized (401): Failed to authenticate with Jira at http://").or( // allow for http or https
                 predicate::str::contains("INVALID - Error: Error validating Jira key 'PROJ-456': Unauthorized (401): Failed to authenticate with Jira at https://")
            ))
            .and(predicate::str::contains("Validation FAILED."))
        );
        // Note: Depending on how wiremock handles unexpected requests or if the error from jira_utils is also printed to stderr,
        // stderr might not be completely empty. The primary check is the exit code and stdout.
        // For now, let's assume operational errors (like unable to connect to mock server) are not expected.
        // If the tool prints the Jira error to stderr as well as incorporating it into the "INVALID" reason on stdout,
        // then the stderr check would need adjustment. Given current main.rs, it prints detailed Jira check errors to stdout.
    Ok(())
}

#[tokio::test]
async fn test_failed_invalid_commit_range() -> anyhow::Result<()> {
    // Scenario: The specified commit range (e.g., start-ref) is invalid or doesn't exist.
    // Expected: Program exits with failure (1) early, stderr indicates Git error, stdout is empty.

    // 1. Setup Git Repo (Jira mock not strictly needed as error should be pre-Jira)
    let temp_repo_dir = setup_repo_with_commits(&[
        "feat: PROJ-123 Initial work", // A commit to make the repo non-empty
    ])?;
    let repo_path = temp_repo_dir.path();
    let mock_server = MockServer::start().await; // Still needed for setup_command helper

    // 3. Run the validator command with an invalid start-ref
    // The setup_command helper adds standard args; we then overwrite start-ref if needed,
    // but for this test, we pass the invalid ref directly to the helper.
    let mut cmd = setup_command(
        repo_path,
        &mock_server.uri(),
        "nonexistent-branch", // Invalid ref
        "HEAD"
    )?;

    // 4. Assertions
    cmd.assert()
        .failure()
        .stdout(predicate::str::is_empty()) // Expect no output on stdout as it should fail early
        .stderr(
            predicate::str::contains("Error fetching commit information from Git:") // General error prefix
            .and(predicate::str::contains("Failed to resolve start_ref 'nonexistent-branch'")) // Specific libgit2/git error
        );
    Ok(())
}

// Placeholder for future tests - test_failed_missing_key
#[tokio::test]
async fn test_failed_missing_key() -> Result<()> {
    // Scenario: One commit in the range is missing a Jira key. Other commits are valid.
    // Expected: Program exits with failure (1), stdout indicates the specific commit is invalid.

    // 1. Setup Git Repo
    let temp_repo_dir = setup_repo_with_commits(&[
        "docs: Update README with new instructions", // No Jira key
        "feat: PROJ-789 Implement another feature", // Valid key
    ])?;
    let repo_path = temp_repo_dir.path();
    
    // 2. Setup Jira Mock Server (only for the valid key)
    let mock_server = MockServer::start().await;
    mock_issue_exists(&mock_server, "PROJ-789").await?;

    // 3. Run the validator command
    let mut cmd = setup_command(repo_path, &mock_server.uri(), "HEAD~2", "HEAD")?;

    // 4. Assertions
    cmd.assert()
        .failure() // Expect the command to fail (exit code 1)
        .stdout(
            predicate::str::contains("docs: Update README with new instructions")
            .and(predicate::str::contains("INVALID - Error: No Jira keys found in commit summary."))
            .and(predicate::str::contains("feat: PROJ-789 Implement another feature").and(predicate::str::contains("VALID (Jira Key: PROJ-789)")))
            .and(predicate::str::contains("Total commits scanned: 2"))
            .and(predicate::str::contains("Valid commits: 1"))
            .and(predicate::str::contains("Invalid commits: 1"))
            .and(predicate::str::contains(">>> Final Result: Validation FAILED."))
        )
        .stderr(predicate::str::is_empty()); // Expect no operational errors on stderr

    Ok(())
}

// Placeholder for future tests - test_failed_key_not_found_in_jira
#[tokio::test]
async fn test_failed_key_not_found_in_jira() -> Result<()> {
    // Scenario: A commit contains a Jira key, but that issue is not found in Jira (404).
    // Expected: Program exits with failure (1), stdout indicates the specific commit is invalid.

    // 1. Setup Jira Mock Server
    let mock_server = MockServer::start().await;
    mock_issue_not_found(&mock_server, "PROJ-XYZ").await?; // This key will result in a 404
    mock_issue_exists(&mock_server, "PROJ-ABC").await?;    // This key is valid

    // 2. Setup Git Repo
    let temp_repo_dir = setup_repo_with_commits(&[
        "fix: PROJ-XYZ Resolve non-existent issue", // Uses key that won't be found
        "feat: PROJ-ABC Implement a real feature",  // Uses a valid key
    ])?;
    let repo_path = temp_repo_dir.path();

    // 3. Run the validator command
    let mut cmd = setup_command(repo_path, &mock_server.uri(), "HEAD~2", "HEAD")?;

    // 4. Assertions
    cmd.assert()
        .failure()
        .stdout(
            predicate::str::contains("fix: PROJ-XYZ Resolve non-existent issue")
            .and(predicate::str::contains("INVALID - Error: Jira key 'PROJ-XYZ' not found in Jira project (HTTP 404)."))
            .and(predicate::str::contains("feat: PROJ-ABC Implement a real feature").and(predicate::str::contains("VALID (Jira Key: PROJ-ABC)")))
            .and(predicate::str::contains("Total commits scanned: 2"))
            .and(predicate::str::contains("Valid commits: 1"))
            .and(predicate::str::contains("Invalid commits: 1"))
            .and(predicate::str::contains(">>> Final Result: Validation FAILED."))
        )
        .stderr(predicate::str::is_empty()); // Expect no operational errors on stderr

    Ok(())
}
