// tests/common/jira_helpers.rs

//! Provides helper functions for configuring a `wiremock::MockServer` to simulate
//! various Jira API responses. These helpers are used in integration tests to
//! control the behavior of the Jira API that the main application interacts with,
//! allowing for predictable testing of different scenarios (e.g., issue exists,
//! issue not found, authentication failure).

use wiremock::{MockServer, Mock, ResponseTemplate, matchers::{method, path, path_matches}};
use anyhow::Result; // Used for Result return types, though not strictly necessary for simple mock setups.

/// Configures the `MockServer` to simulate a successful Jira API response (HTTP 200 OK)
/// for a specific issue key.
///
/// This mock indicates that the Jira issue identified by `issue_key` exists.
/// The response body includes a minimal JSON structure for the issue.
///
/// # Parameters
/// * `server`: A reference to the `MockServer` instance to configure.
/// * `issue_key`: The Jira issue key (e.g., "PROJ-123") for which the mock response should be set up.
///
/// # Returns
/// * `Result<()>`: Returns `Ok(())` if the mock was successfully mounted on the server.
pub async fn mock_issue_exists(server: &MockServer, issue_key: &str) -> Result<()> {
    Mock::given(method("GET"))
        .and(path(format!("/rest/api/2/issue/{}", issue_key))) // Matches exact path for the issue
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "key": issue_key,
            "fields": {
                "summary": "Mocked issue summary",
            }
        })))
        .mount(server)
        .await;
    Ok(())
}

/// Configures the `MockServer` to simulate a Jira API response indicating that an issue
/// was not found (HTTP 404 Not Found) for a specific issue key.
///
/// # Parameters
/// * `server`: A reference to the `MockServer` instance to configure.
/// * `issue_key`: The Jira issue key (e.g., "PROJ-123") for which the "not found" response
///   should be set up.
///
/// # Returns
/// * `Result<()>`: Returns `Ok(())` if the mock was successfully mounted on the server.
pub async fn mock_issue_not_found(server: &MockServer, issue_key: &str) -> Result<()> {
    Mock::given(method("GET"))
        .and(path(format!("/rest/api/2/issue/{}", issue_key))) // Matches exact path for the issue
        .respond_with(ResponseTemplate::new(404)) // HTTP 404 Not Found
        .mount(server)
        .await;
    Ok(())
}

/// Configures the `MockServer` to simulate a Jira API response indicating an authentication
/// failure (HTTP 401 Unauthorized) for any GET request whose path matches the provided regex pattern.
///
/// This is useful for testing how the application handles invalid Jira credentials.
///
/// # Parameters
/// * `server`: A reference to the `MockServer` instance to configure.
/// * `path_pattern`: A string slice containing a regular expression (e.g., `r"^/rest/api/2/issue/.*"`)
///   that will be used to match request paths. Any GET request to a path matching this pattern
///   will trigger the 401 response.
///
/// # Returns
/// * `Result<()>`: Returns `Ok(())` if the mock was successfully mounted on the server.
pub async fn mock_auth_failure(server: &MockServer, path_pattern: &str) -> Result<()> {
    Mock::given(method("GET"))
        .and(path_matches(path_pattern)) // Matches any path conforming to the regex pattern
        .respond_with(ResponseTemplate::new(401)) // HTTP 401 Unauthorized
        .mount(server)
        .await;
    Ok(())
}


// Placeholder for example_jira_helper, can be removed if not used.
// pub async fn example_jira_helper(mock_server: &MockServer) {
//     println!("Jira helper placeholder for server: {}", mock_server.uri());
// }
