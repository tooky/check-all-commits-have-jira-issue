use regex::Regex;

pub fn extract_jira_keys(commit_message: &str) -> Vec<String> {
    // Regex to find Jira-like keys (e.g., ABC-123, JIRA-4567)
    // This regex looks for one or more uppercase letters, followed by a hyphen,
    // followed by one or more digits.
    let re = Regex::new(r"([A-Z]+-[0-9]+)").unwrap(); // unwrap is okay for a known-good regex

    // Find all captures in the commit message.
    // `captures_iter` returns an iterator over all non-overlapping matches.
    // For each match, we take the first capture group (the whole key).
    re.captures_iter(commit_message)
        .map(|cap| cap[1].to_string()) // cap[0] is the whole match, cap[1] is the first group
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_single_key() {
        assert_eq!(extract_jira_keys("Fix for ABC-123"), vec!["ABC-123"]);
    }

    #[test]
    fn test_extract_multiple_keys() {
        let message = "Related to XYZ-789, fixed PROJ-001 and also TEST-100.";
        let expected = vec!["XYZ-789", "PROJ-001", "TEST-100"];
        assert_eq!(extract_jira_keys(message), expected);
    }

    #[test]
    fn test_extract_no_keys() {
        assert_eq!(extract_jira_keys("A regular commit message without keys."), Vec::<String>::new());
    }

    #[test]
    fn test_extract_keys_with_lowercase() {
        // Regex is case-sensitive for the project key part by default.
        // This test ensures it doesn't match 'abc-123' unless regex is modified.
        assert_eq!(extract_jira_keys("Fix for abc-123"), Vec::<String>::new());
    }

    #[test]
    fn test_extract_keys_mixed_with_text() {
        assert_eq!(extract_jira_keys("ABC-123 is the main ticket, but also see DEF-456."), vec!["ABC-123", "DEF-456"]);
    }

    #[test]
    fn test_extract_key_at_start() {
        assert_eq!(extract_jira_keys("PROJ-007: Implement feature"), vec!["PROJ-007"]);
    }

    #[test]
    fn test_extract_key_at_end() {
        assert_eq!(extract_jira_keys("Implemented feature for TASK-321"), vec!["TASK-321"]);
    }
     #[test]
    fn test_extract_key_with_no_surrounding_text() {
        assert_eq!(extract_jira_keys("MYKEY-111"), vec!["MYKEY-111"]);
    }
}

pub async fn check_jira_issue_exists(
    jira_url: &str,
    username: &str,
    api_token: &str,
    issue_key: &str,
) -> Result<bool, String> {
    // Construct the Jira API URL
    let api_url = format!("{}/rest/api/2/issue/{}", jira_url.trim_end_matches('/'), issue_key);

    // Create a reqwest client
    let client = reqwest::Client::new();

    // Make the GET request with Basic Authentication
    let response = client
        .get(&api_url)
        .basic_auth(username, Some(api_token))
        .send()
        .await
        .map_err(|e| format!("Request to {} failed: {}", api_url, e))?;

    // Check the response status
    match response.status() {
        reqwest::StatusCode::OK => Ok(true), // 200 OK implies issue exists
        reqwest::StatusCode::NOT_FOUND => Ok(false), // 404 Not Found implies issue does not exist
        reqwest::StatusCode::UNAUTHORIZED => Err(format!(
            "Unauthorized (401): Failed to authenticate with Jira at {}. Check credentials.",
            api_url
        )),
        reqwest::StatusCode::FORBIDDEN => Err(format!(
            "Forbidden (403): Insufficient permissions for Jira issue {} at {}.",
            issue_key, api_url
        )),
        status if status.is_server_error() => Err(format!(
            "Jira server error ({}) for issue {} at {}.",
            status, issue_key, api_url
        )),
        status => Err(format!(
            "Unexpected response status ({}) for issue {} at {}.",
            status, issue_key, api_url
        )),
    }
}

#[cfg(test)]
mod async_tests {
    // Note: These tests require a running Jira instance and valid credentials/issue keys to pass.
    // They are also ignored by default to prevent issues in CI environments without Jira.
    // For local testing:
    // 1. Set JIRA_TEST_URL, JIRA_TEST_USER, JIRA_TEST_TOKEN, JIRA_TEST_EXISTING_ISSUE, JIRA_TEST_NONEXISTENT_ISSUE environment variables.
    // 2. Run with `cargo test -- --ignored` or enable the tests.

    use super::*;
    use std::env;

    #[tokio::test]
    #[ignore]
    async fn test_check_existing_jira_issue() {
        let jira_url = env::var("JIRA_TEST_URL").expect("JIRA_TEST_URL not set");
        let username = env::var("JIRA_TEST_USER").expect("JIRA_TEST_USER not set");
        let api_token = env::var("JIRA_TEST_TOKEN").expect("JIRA_TEST_TOKEN not set");
        let issue_key = env::var("JIRA_TEST_EXISTING_ISSUE").expect("JIRA_TEST_EXISTING_ISSUE not set");

        let result = check_jira_issue_exists(&jira_url, &username, &api_token, &issue_key).await;
        assert!(result.is_ok(), "Expected Ok, got Err: {:?}", result.err());
        assert_eq!(result.unwrap(), true, "Expected issue {} to exist", issue_key);
    }

    #[tokio::test]
    #[ignore]
    async fn test_check_nonexistent_jira_issue() {
        let jira_url = env::var("JIRA_TEST_URL").expect("JIRA_TEST_URL not set");
        let username = env::var("JIRA_TEST_USER").expect("JIRA_TEST_USER not set");
        let api_token = env::var("JIRA_TEST_TOKEN").expect("JIRA_TEST_TOKEN not set");
        // Use a clearly non-existent issue key format if possible, or one known to be non-existent
        let issue_key = env::var("JIRA_TEST_NONEXISTENT_ISSUE").unwrap_or_else(|_| "NONEXISTENT-000".to_string());


        let result = check_jira_issue_exists(&jira_url, &username, &api_token, &issue_key).await;
        assert!(result.is_ok(), "Expected Ok, got Err: {:?}", result.err());
        assert_eq!(result.unwrap(), false, "Expected issue {} to not exist", issue_key);
    }

    #[tokio::test]
    #[ignore]
    async fn test_check_jira_issue_auth_failure() {
        let jira_url = env::var("JIRA_TEST_URL").expect("JIRA_TEST_URL not set");
        let username = "invaliduser";
        let api_token = "invalidtoken";
        let issue_key = env::var("JIRA_TEST_EXISTING_ISSUE").unwrap_or_else(|_| "ANYKEY-1".to_string());

        let result = check_jira_issue_exists(&jira_url, username, api_token, &issue_key).await;
        assert!(result.is_err(), "Expected Err for auth failure, got Ok");
        let error_message = result.err().unwrap();
        assert!(error_message.contains("Unauthorized (401)") || error_message.contains("401"), "Error message did not indicate an auth error: {}", error_message);
    }
}
