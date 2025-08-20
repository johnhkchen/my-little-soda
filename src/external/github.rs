//! GitHub CLI abstractions
//! 
//! Provides trait-based abstractions for GitHub operations, enabling testable
//! GitHub integrations through dependency injection.

use async_trait::async_trait;
use thiserror::Error;
use super::command::{CommandExecutor, CommandError};
use std::sync::Arc;

pub type IssueId = u64;
pub type PrId = u64;

#[derive(Debug, Clone)]
pub struct IssueOperation {
    pub add_labels: Vec<String>,
    pub remove_labels: Vec<String>,
    pub comment: Option<String>,
    pub close: bool,
}

#[derive(Debug, Clone)]
pub struct PrConfig {
    pub title: String,
    pub body: String,
    pub head_branch: String,
    pub base_branch: String,
}

#[derive(Debug, Clone)]
pub struct LabelConfig {
    pub name: String,
    pub description: String,
    pub color: String,
}

impl LabelConfig {
    pub fn new(name: &str, description: &str, color: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            color: color.to_string(),
        }
    }
}

#[derive(Debug, Error)]
pub enum GitHubError {
    #[error("GitHub authentication failed")]
    AuthenticationFailed,
    #[error("Issue not found: {issue_id}")]
    IssueNotFound { issue_id: IssueId },
    #[error("Repository not found or access denied")]
    RepositoryNotFound,
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Network error: {message}")]
    NetworkError { message: String },
    #[error("Command execution error: {source}")]
    CommandError {
        #[from]
        source: CommandError,
    },
    #[error("Invalid response from GitHub CLI: {message}")]
    InvalidResponse { message: String },
}

/// Trait for GitHub operations
/// 
/// This abstraction enables testing GitHub integrations without actual
/// GitHub API calls, while preserving the exact interface used by the
/// application code.
#[async_trait]
pub trait GitHubOperations: Send + Sync {
    /// Create a new GitHub issue
    async fn create_issue(
        &self,
        title: &str,
        body: &str,
        labels: &[&str],
    ) -> Result<IssueId, GitHubError>;

    /// Edit an existing GitHub issue
    async fn edit_issue(
        &self,
        issue_id: IssueId,
        operation: &IssueOperation,
    ) -> Result<(), GitHubError>;

    /// Create a pull request
    async fn create_pr(&self, config: &PrConfig) -> Result<PrId, GitHubError>;

    /// Create a repository label
    async fn create_label(&self, config: &LabelConfig) -> Result<(), GitHubError>;

    /// List issues with specific labels
    async fn list_issues(&self, labels: &[&str]) -> Result<Vec<IssueId>, GitHubError>;

    /// Get issue details
    async fn get_issue(&self, issue_id: IssueId) -> Result<IssueDetails, GitHubError>;
}

#[derive(Debug, Clone)]
pub struct IssueDetails {
    pub id: IssueId,
    pub title: String,
    pub body: String,
    pub labels: Vec<String>,
    pub assignee: Option<String>,
    pub url: String,
}

/// Real GitHub CLI implementation
pub struct GitHubClient {
    executor: Arc<dyn CommandExecutor>,
}

impl GitHubClient {
    pub fn new(executor: Arc<dyn CommandExecutor>) -> Self {
        Self { executor }
    }

    async fn execute_gh_command(&self, args: &[&str]) -> Result<String, GitHubError> {
        let output = self.executor.execute("gh", args).await?;
        
        if !output.success() {
            return Err(self.classify_gh_error(&output.stderr));
        }
        
        Ok(output.stdout)
    }

    fn classify_gh_error(&self, stderr: &str) -> GitHubError {
        if stderr.contains("authentication failed") || stderr.contains("not authenticated") {
            GitHubError::AuthenticationFailed
        } else if stderr.contains("rate limit") {
            GitHubError::RateLimitExceeded
        } else if stderr.contains("not found") {
            GitHubError::RepositoryNotFound
        } else {
            GitHubError::NetworkError {
                message: stderr.to_string(),
            }
        }
    }

    fn parse_issue_id_from_url(&self, url: &str) -> Result<IssueId, GitHubError> {
        url.split('/')
            .last()
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| GitHubError::InvalidResponse {
                message: format!("Could not parse issue ID from URL: {}", url),
            })
    }
}

#[async_trait]
impl GitHubOperations for GitHubClient {
    async fn create_issue(
        &self,
        title: &str,
        body: &str,
        labels: &[&str],
    ) -> Result<IssueId, GitHubError> {
        let mut args = vec!["issue", "create", "--title", title, "--body", body];
        
        if !labels.is_empty() {
            args.push("--label");
            for label in labels {
                args.push(label);
            }
        }

        let output = self.execute_gh_command(&args).await?;
        self.parse_issue_id_from_url(output.trim())
    }

    async fn edit_issue(
        &self,
        issue_id: IssueId,
        operation: &IssueOperation,
    ) -> Result<(), GitHubError> {
        let issue_str = issue_id.to_string();
        let mut args = vec!["issue", "edit", &issue_str];

        for label in &operation.add_labels {
            args.extend(&["--add-label", label]);
        }

        for label in &operation.remove_labels {
            args.extend(&["--remove-label", label]);
        }

        if let Some(comment) = &operation.comment {
            args.extend(&["--body", comment]);
        }

        if operation.close {
            args.push("--state");
            args.push("closed");
        }

        self.execute_gh_command(&args).await?;
        Ok(())
    }

    async fn create_pr(&self, config: &PrConfig) -> Result<PrId, GitHubError> {
        let args = [
            "pr", "create",
            "--title", &config.title,
            "--body", &config.body,
            "--head", &config.head_branch,
            "--base", &config.base_branch,
        ];

        let output = self.execute_gh_command(&args).await?;
        self.parse_issue_id_from_url(output.trim())
    }

    async fn create_label(&self, config: &LabelConfig) -> Result<(), GitHubError> {
        let args = [
            "label", "create",
            &config.name,
            "--description", &config.description,
            "--color", &config.color,
        ];

        self.execute_gh_command(&args).await?;
        Ok(())
    }

    async fn list_issues(&self, labels: &[&str]) -> Result<Vec<IssueId>, GitHubError> {
        let mut args = vec!["issue", "list", "--json", "number"];
        let label_string;
        
        if !labels.is_empty() {
            args.push("--label");
            label_string = labels.join(",");
            args.push(&label_string);
        }

        let output = self.execute_gh_command(&args).await?;
        
        // Parse JSON response to extract issue numbers
        let issues: Vec<serde_json::Value> = serde_json::from_str(&output)
            .map_err(|e| GitHubError::InvalidResponse {
                message: format!("Failed to parse issue list JSON: {}", e),
            })?;

        let issue_ids = issues
            .into_iter()
            .filter_map(|issue| {
                issue.get("number")
                    .and_then(|n| n.as_u64())
            })
            .collect();

        Ok(issue_ids)
    }

    async fn get_issue(&self, issue_id: IssueId) -> Result<IssueDetails, GitHubError> {
        let issue_str = issue_id.to_string();
        let args = ["issue", "view", &issue_str, "--json", "title,body,labels,assignees,url"];

        let output = self.execute_gh_command(&args).await?;
        
        let issue_data: serde_json::Value = serde_json::from_str(&output)
            .map_err(|e| GitHubError::InvalidResponse {
                message: format!("Failed to parse issue JSON: {}", e),
            })?;

        let title = issue_data.get("title")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();

        let body = issue_data.get("body")
            .and_then(|b| b.as_str())
            .unwrap_or("")
            .to_string();

        let url = issue_data.get("url")
            .and_then(|u| u.as_str())
            .unwrap_or("")
            .to_string();

        let labels = issue_data.get("labels")
            .and_then(|l| l.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|label| label.get("name").and_then(|n| n.as_str()))
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        let assignee = issue_data.get("assignees")
            .and_then(|a| a.as_array())
            .and_then(|arr| arr.first())
            .and_then(|assignee| assignee.get("login"))
            .and_then(|login| login.as_str())
            .map(|s| s.to_string());

        Ok(IssueDetails {
            id: issue_id,
            title,
            body,
            labels,
            assignee,
            url,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use mockall::mock;
    use super::super::command::{CommandOutput, CommandError};

    // Simple mock for testing
    struct MockCommandExecutor {
        responses: std::collections::HashMap<String, Result<CommandOutput, CommandError>>,
    }

    impl MockCommandExecutor {
        fn new() -> Self {
            Self {
                responses: std::collections::HashMap::new(),
            }
        }

        fn expect_command(mut self, program: &str, args: &[&str], response: Result<CommandOutput, CommandError>) -> Self {
            let key = format!("{} {}", program, args.join(" "));
            self.responses.insert(key, response);
            self
        }
    }

    #[async_trait]
    impl CommandExecutor for MockCommandExecutor {
        async fn execute(&self, program: &str, args: &[&str]) -> Result<CommandOutput, CommandError> {
            let key = format!("{} {}", program, args.join(" "));
            self.responses.get(&key)
                .cloned()
                .unwrap_or(Err(CommandError::CommandNotFound {
                    command: program.to_string(),
                }))
        }
    }

    #[tokio::test]
    async fn test_create_issue_success() {
        let mock_executor = MockCommandExecutor::new()
            .expect_command("gh", &["issue", "create", "--title", "Test Issue", "--body", "Test body"], 
                Ok(CommandOutput {
                    status_code: 0,
                    stdout: "https://github.com/owner/repo/issues/123".to_string(),
                    stderr: String::new(),
                }));

        let client = GitHubClient::new(Arc::new(mock_executor));
        let result = client.create_issue("Test Issue", "Test body", &[]).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 123);
    }

    #[tokio::test]
    async fn test_create_issue_with_labels() {
        let mock_executor = MockCommandExecutor::new()
            .expect_command("gh", &["issue", "create", "--title", "Test", "--body", "Body", "--label", "bug", "urgent"], 
                Ok(CommandOutput {
                    status_code: 0,
                    stdout: "https://github.com/owner/repo/issues/456".to_string(),
                    stderr: String::new(),
                }));

        let client = GitHubClient::new(Arc::new(mock_executor));
        let result = client.create_issue("Test", "Body", &["bug", "urgent"]).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 456);
    }

    #[tokio::test]
    async fn test_create_issue_authentication_error() {
        let mock_executor = MockCommandExecutor::new()
            .expect_command("gh", &["issue", "create", "--title", "Test", "--body", "Body"], 
                Ok(CommandOutput {
                    status_code: 1,
                    stdout: String::new(),
                    stderr: "authentication failed".to_string(),
                }));

        let client = GitHubClient::new(Arc::new(mock_executor));
        let result = client.create_issue("Test", "Body", &[]).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), GitHubError::AuthenticationFailed));
    }
}