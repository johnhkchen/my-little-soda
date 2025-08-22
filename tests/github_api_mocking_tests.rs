//! GitHub API mocking infrastructure tests
//!
//! These tests use wiremock to create deterministic HTTP mocking for GitHub API calls,
//! eliminating network dependencies and making tests fast and reliable.

use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path, header, query_param, body_json};
use serde_json::{json, Value};
use std::collections::HashMap;

/// GitHub API mock server for deterministic testing
pub struct GitHubApiMock {
    pub server: MockServer,
    pub base_url: String,
}

impl GitHubApiMock {
    /// Create a new GitHub API mock server
    pub async fn new() -> Self {
        let server = MockServer::start().await;
        let base_url = server.uri();
        
        Self { server, base_url }
    }
    
    /// Mock GitHub issue retrieval
    pub async fn mock_issue_view(&self, issue_number: u64, title: &str, state: &str, labels: Vec<&str>) {
        let labels_json: Vec<Value> = labels.iter()
            .map(|label| json!({"name": label}))
            .collect();
            
        let response = json!({
            "number": issue_number,
            "title": title,
            "state": state.to_uppercase(),
            "labels": labels_json
        });
        
        Mock::given(method("GET"))
            .and(path(format!("/repos/test-owner/test-repo/issues/{}", issue_number)))
            .and(header("authorization", "token mock-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&self.server)
            .await;
    }
    
    /// Mock GitHub pull request creation
    pub async fn mock_create_pull_request(&self, expected_title: &str, expected_head: &str, expected_base: &str, pr_number: u64) {
        let response = json!({
            "number": pr_number,
            "title": expected_title,
            "head": {"ref": expected_head},
            "base": {"ref": expected_base},
            "html_url": format!("https://github.com/test-owner/test-repo/pull/{}", pr_number)
        });
        
        Mock::given(method("POST"))
            .and(path("/repos/test-owner/test-repo/pulls"))
            .and(header("authorization", "token mock-token"))
            .and(header("content-type", "application/json"))
            .respond_with(ResponseTemplate::new(201).set_body_json(response))
            .mount(&self.server)
            .await;
    }
    
    /// Mock GitHub label addition to issue
    pub async fn mock_add_label_to_issue(&self, issue_number: u64, label: &str) {
        let response = json!([{"name": label}]);
        
        Mock::given(method("POST"))
            .and(path(format!("/repos/test-owner/test-repo/issues/{}/labels", issue_number)))
            .and(header("authorization", "token mock-token"))
            .and(body_json(json!([label])))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&self.server)
            .await;
    }
    
    /// Mock GitHub repository information
    pub async fn mock_repository_info(&self, owner: &str, repo: &str) {
        let response = json!({
            "name": repo,
            "full_name": format!("{}/{}", owner, repo),
            "owner": {
                "login": owner
            },
            "default_branch": "main"
        });
        
        Mock::given(method("GET"))
            .and(path(format!("/repos/{}/{}", owner, repo)))
            .and(header("authorization", "token mock-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&self.server)
            .await;
    }
    
    /// Mock GitHub API rate limiting responses
    pub async fn mock_rate_limit_response(&self) {
        let response = json!({
            "message": "API rate limit exceeded",
            "documentation_url": "https://docs.github.com/rest/overview/resources-in-the-rest-api#rate-limiting"
        });
        
        Mock::given(method("GET"))
            .and(path("/rate_limit"))
            .respond_with(
                ResponseTemplate::new(403)
                    .set_body_json(response)
                    .append_header("X-RateLimit-Limit", "5000")
                    .append_header("X-RateLimit-Remaining", "0")
                    .append_header("X-RateLimit-Reset", "1640995200")
            )
            .mount(&self.server)
            .await;
    }
    
    /// Mock GitHub search API for finding PRs
    pub async fn mock_search_pulls(&self, query: &str, items: Vec<Value>) {
        let response = json!({
            "total_count": items.len(),
            "incomplete_results": false,
            "items": items
        });
        
        Mock::given(method("GET"))
            .and(path("/search/issues"))
            .and(query_param("q", query))
            .and(header("authorization", "token mock-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&self.server)
            .await;
    }
    
    /// Mock GitHub API error responses for testing failure scenarios
    pub async fn mock_api_error(&self, path_pattern: &str, status_code: u16, error_message: &str) {
        let response = json!({
            "message": error_message,
            "status": status_code
        });
        
        Mock::given(method("GET"))
            .and(path(path_pattern))
            .respond_with(ResponseTemplate::new(status_code).set_body_json(response))
            .mount(&self.server)
            .await;
    }
}

/// Test helper for creating mock GitHub scenarios
pub struct GitHubScenarioBuilder {
    pub mock: GitHubApiMock,
    pub issues: HashMap<u64, (String, String, Vec<String>)>, // number -> (title, state, labels)
}

impl GitHubScenarioBuilder {
    pub async fn new() -> Self {
        Self {
            mock: GitHubApiMock::new().await,
            issues: HashMap::new(),
        }
    }
    
    /// Add an issue to the scenario
    pub fn with_issue(mut self, number: u64, title: &str, state: &str, labels: Vec<&str>) -> Self {
        self.issues.insert(number, (title.to_string(), state.to_string(), labels.iter().map(|s| s.to_string()).collect()));
        self
    }
    
    /// Build the scenario by setting up all mocks
    pub async fn build(self) -> GitHubApiMock {
        // Mock repository info
        self.mock.mock_repository_info("test-owner", "test-repo").await;
        
        // Mock all issues
        for (number, (title, state, labels)) in self.issues {
            let label_refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
            self.mock.mock_issue_view(number, &title, &state, label_refs).await;
        }
        
        self.mock
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_github_api_mock_setup() {
        // Create a mock GitHub API server
        let mock = GitHubApiMock::new().await;
        
        // Set up a mock issue
        mock.mock_issue_view(123, "Test Issue", "open", vec!["bug", "priority-high"]).await;
        
        // Verify the server is running and accessible
        assert!(mock.base_url.starts_with("http://127.0.0.1:"));
        
        // Test that we can make a request to the mock server
        let client = reqwest::Client::new();
        let response = client
            .get(&format!("{}/repos/test-owner/test-repo/issues/123", mock.base_url))
            .header("authorization", "token mock-token")
            .send()
            .await
            .unwrap();
        
        assert_eq!(response.status(), 200);
        
        let issue: Value = response.json().await.unwrap();
        assert_eq!(issue["number"], 123);
        assert_eq!(issue["title"], "Test Issue");
        assert_eq!(issue["state"], "OPEN");
    }

    #[tokio::test]
    async fn test_pull_request_creation_mock() {
        let mock = GitHubApiMock::new().await;
        
        // Set up mock for PR creation
        mock.mock_create_pull_request(
            "Test PR", 
            "feature-branch", 
            "main", 
            456
        ).await;
        
        // Test PR creation request
        let client = reqwest::Client::new();
        let pr_request = json!({
            "title": "Test PR",
            "head": "feature-branch",
            "base": "main",
            "body": "Test PR body"
        });
        
        let response = client
            .post(&format!("{}/repos/test-owner/test-repo/pulls", mock.base_url))
            .header("authorization", "token mock-token")
            .header("content-type", "application/json")
            .json(&pr_request)
            .send()
            .await
            .unwrap();
        
        assert_eq!(response.status(), 201);
        
        let pr: Value = response.json().await.unwrap();
        assert_eq!(pr["number"], 456);
        assert_eq!(pr["title"], "Test PR");
    }

    #[tokio::test]
    async fn test_label_addition_mock() {
        let mock = GitHubApiMock::new().await;
        
        // Set up mock for adding label
        mock.mock_add_label_to_issue(789, "route:review").await;
        
        // Test label addition request
        let client = reqwest::Client::new();
        let label_request = json!(["route:review"]);
        
        let response = client
            .post(&format!("{}/repos/test-owner/test-repo/issues/789/labels", mock.base_url))
            .header("authorization", "token mock-token")
            .header("content-type", "application/json")
            .json(&label_request)
            .send()
            .await
            .unwrap();
        
        assert_eq!(response.status(), 200);
        
        let labels: Value = response.json().await.unwrap();
        assert_eq!(labels[0]["name"], "route:review");
    }

    #[tokio::test]
    async fn test_scenario_builder() {
        // Use scenario builder to create complex GitHub state
        let mock = GitHubScenarioBuilder::new()
            .await
            .with_issue(100, "Feature Request", "open", vec!["enhancement"])
            .with_issue(101, "Bug Fix", "open", vec!["bug", "priority-high"])
            .with_issue(102, "Documentation", "closed", vec!["documentation"])
            .build()
            .await;
        
        // Test that all issues are properly mocked
        let client = reqwest::Client::new();
        
        // Check issue 100
        let response = client
            .get(&format!("{}/repos/test-owner/test-repo/issues/100", mock.base_url))
            .header("authorization", "token mock-token")
            .send()
            .await
            .unwrap();
        let issue: Value = response.json().await.unwrap();
        assert_eq!(issue["title"], "Feature Request");
        assert_eq!(issue["labels"][0]["name"], "enhancement");
        
        // Check issue 101
        let response = client
            .get(&format!("{}/repos/test-owner/test-repo/issues/101", mock.base_url))
            .header("authorization", "token mock-token")
            .send()
            .await
            .unwrap();
        let issue: Value = response.json().await.unwrap();
        assert_eq!(issue["title"], "Bug Fix");
        assert_eq!(issue["labels"][0]["name"], "bug");
    }

    #[tokio::test]
    async fn test_rate_limiting_simulation() {
        let mock = GitHubApiMock::new().await;
        
        // Set up rate limit response
        mock.mock_rate_limit_response().await;
        
        // Test rate limit endpoint
        let client = reqwest::Client::new();
        let response = client
            .get(&format!("{}/rate_limit", mock.base_url))
            .send()
            .await
            .unwrap();
        
        assert_eq!(response.status(), 403);
        assert_eq!(response.headers().get("X-RateLimit-Remaining").unwrap(), "0");
        
        let error: Value = response.json().await.unwrap();
        assert!(error["message"].as_str().unwrap().contains("rate limit exceeded"));
    }

    #[tokio::test]
    async fn test_error_scenarios() {
        let mock = GitHubApiMock::new().await;
        
        // Mock API error for non-existent issue
        mock.mock_api_error(
            "/repos/test-owner/test-repo/issues/999", 
            404, 
            "Not Found"
        ).await;
        
        let client = reqwest::Client::new();
        let response = client
            .get(&format!("{}/repos/test-owner/test-repo/issues/999", mock.base_url))
            .send()
            .await
            .unwrap();
        
        assert_eq!(response.status(), 404);
        
        let error: Value = response.json().await.unwrap();
        assert_eq!(error["message"], "Not Found");
    }

    #[tokio::test]
    async fn test_multiple_concurrent_mocks() {
        // Test that multiple mock servers can run concurrently
        let mock1 = GitHubApiMock::new().await;
        let mock2 = GitHubApiMock::new().await;
        
        // Each should have different ports
        assert_ne!(mock1.base_url, mock2.base_url);
        
        // Set up different responses on each
        mock1.mock_issue_view(100, "Issue on Server 1", "open", vec!["server1"]).await;
        mock2.mock_issue_view(100, "Issue on Server 2", "closed", vec!["server2"]).await;
        
        let client = reqwest::Client::new();
        
        // Test server 1
        let response1 = client
            .get(&format!("{}/repos/test-owner/test-repo/issues/100", mock1.base_url))
            .header("authorization", "token mock-token")
            .send()
            .await
            .unwrap();
        let issue1: Value = response1.json().await.unwrap();
        assert_eq!(issue1["title"], "Issue on Server 1");
        
        // Test server 2
        let response2 = client
            .get(&format!("{}/repos/test-owner/test-repo/issues/100", mock2.base_url))
            .header("authorization", "token mock-token")
            .send()
            .await
            .unwrap();
        let issue2: Value = response2.json().await.unwrap();
        assert_eq!(issue2["title"], "Issue on Server 2");
    }
}