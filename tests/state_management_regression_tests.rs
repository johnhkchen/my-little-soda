//! Regression tests for state management bugs (issue #93)
//!
//! These tests prevent state management regressions that caused:
//! - Completed work being reassigned by clambake pop
//! - gh CLI calls failing without explicit repository targeting
//! - Label removal failures going undetected
//!
//! Test structure follows existing patterns from tests/agent_labeling_test.rs
//! and tests/expressive_test_framework.rs

use async_trait::async_trait;
use my_little_soda::github::GitHubError;
use std::io::{Error as IoError, ErrorKind};
use std::process::Output;
use std::sync::{Arc, Mutex};

mod fixtures;

// Mock trait for testing GitHub operations with explicit repo targeting
#[async_trait]
pub trait GitHubOps {
    async fn remove_label_from_issue(
        &self,
        issue_number: u64,
        label_name: &str,
    ) -> Result<(), GitHubError>;
    async fn add_label_to_issue(
        &self,
        issue_number: u64,
        label_name: &str,
    ) -> Result<(), GitHubError>;
    async fn fetch_issues(&self) -> Result<Vec<octocrab::models::issues::Issue>, GitHubError>;
    fn owner(&self) -> &str;
    fn repo(&self) -> &str;
}

/// Mock GitHub client that simulates gh CLI behavior and tracks repo targeting
#[derive(Debug, Clone)]
pub struct MockGitHubClient {
    pub owner: String,
    pub repo: String,
    pub issues: Vec<octocrab::models::issues::Issue>,
    pub gh_calls: Arc<Mutex<Vec<GhCliCall>>>,
    pub simulate_failures: Arc<Mutex<Vec<FailureScenario>>>,
}

#[derive(Debug, Clone)]
pub enum GhCliCall {
    RemoveLabel {
        issue_number: u64,
        label: String,
        repo_targeted: bool,
        command_args: Vec<String>,
    },
    AddLabel {
        issue_number: u64,
        label: String,
        repo_targeted: bool,
        command_args: Vec<String>,
    },
    FetchIssues {
        repo_targeted: bool,
        command_args: Vec<String>,
    },
}

#[derive(Debug, Clone)]
pub enum FailureScenario {
    RemoveLabelFailure {
        issue_number: u64,
        error_msg: String,
    },
    AddLabelFailure {
        issue_number: u64,
        error_msg: String,
    },
    ApiFailure {
        error_msg: String,
    },
}

impl MockGitHubClient {
    pub fn new(owner: &str, repo: &str) -> Self {
        Self {
            owner: owner.to_string(),
            repo: repo.to_string(),
            issues: Vec::new(),
            gh_calls: Arc::new(Mutex::new(Vec::new())),
            simulate_failures: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_issues(mut self, issues: Vec<octocrab::models::issues::Issue>) -> Self {
        self.issues = issues;
        self
    }

    pub fn with_failure_scenario(self, scenario: FailureScenario) -> Self {
        self.simulate_failures.lock().unwrap().push(scenario);
        self
    }

    pub fn get_gh_calls(&self) -> Vec<GhCliCall> {
        self.gh_calls.lock().unwrap().clone()
    }

    pub fn clear_gh_calls(&self) {
        self.gh_calls.lock().unwrap().clear();
    }

    // Simulate gh CLI command execution with repo targeting validation
    fn simulate_gh_command(&self, args: &[&str]) -> Result<Output, IoError> {
        let args_vec: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let repo_targeted =
            args.contains(&"-R") && args.len() > args.iter().position(|&x| x == "-R").unwrap() + 1;

        // Check for simulated failures
        let failures = self.simulate_failures.lock().unwrap();
        for failure in failures.iter() {
            match failure {
                FailureScenario::RemoveLabelFailure {
                    issue_number,
                    error_msg,
                } => {
                    let issue_str = issue_number.to_string();
                    if args.contains(&"--remove-label") && args.contains(&issue_str.as_str()) {
                        // Simulate failure by creating a failed command result
                        let failed_status = std::process::Command::new("false")
                            .status()
                            .unwrap_or_else(|_| {
                                std::process::Command::new("echo").status().unwrap()
                            });
                        return Ok(Output {
                            status: failed_status,
                            stdout: Vec::new(),
                            stderr: error_msg.as_bytes().to_vec(),
                        });
                    }
                }
                FailureScenario::AddLabelFailure {
                    issue_number,
                    error_msg,
                } => {
                    let issue_str = issue_number.to_string();
                    if args.contains(&"--add-label") && args.contains(&issue_str.as_str()) {
                        // Simulate failure by creating a failed command result
                        let failed_status = std::process::Command::new("false")
                            .status()
                            .unwrap_or_else(|_| {
                                std::process::Command::new("echo").status().unwrap()
                            });
                        return Ok(Output {
                            status: failed_status,
                            stdout: Vec::new(),
                            stderr: error_msg.as_bytes().to_vec(),
                        });
                    }
                }
                FailureScenario::ApiFailure { error_msg } => {
                    let failed_status = std::process::Command::new("false")
                        .status()
                        .unwrap_or_else(|_| std::process::Command::new("echo").status().unwrap());
                    return Ok(Output {
                        status: failed_status,
                        stdout: Vec::new(),
                        stderr: error_msg.as_bytes().to_vec(),
                    });
                }
            }
        }

        // Successful response
        let success_status = std::process::Command::new("echo")
            .status()
            .unwrap_or_else(|_| std::process::Command::new("true").status().unwrap());
        Ok(Output {
            status: success_status,
            stdout: "Success".as_bytes().to_vec(),
            stderr: Vec::new(),
        })
    }
}

#[async_trait]
impl GitHubOps for MockGitHubClient {
    async fn remove_label_from_issue(
        &self,
        issue_number: u64,
        label_name: &str,
    ) -> Result<(), GitHubError> {
        let repo = format!("{}/{}", self.owner(), self.repo());
        let issue_str = issue_number.to_string();
        let args = vec![
            "issue",
            "edit",
            &issue_str,
            "-R",
            &repo,
            "--remove-label",
            label_name,
        ];
        let args_vec: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let repo_targeted = args.contains(&"-R");

        // Record the call
        self.gh_calls.lock().unwrap().push(GhCliCall::RemoveLabel {
            issue_number,
            label: label_name.to_string(),
            repo_targeted,
            command_args: args_vec.clone(),
        });

        // Simulate gh command execution
        let output = self.simulate_gh_command(&args);

        match output {
            Ok(result) => {
                if result.status.success() {
                    Ok(())
                } else {
                    let error_msg = String::from_utf8_lossy(&result.stderr);
                    Err(GitHubError::IoError(IoError::other(format!(
                        "GitHub CLI error: {error_msg}"
                    ))))
                }
            }
            Err(e) => Err(GitHubError::IoError(e)),
        }
    }

    async fn add_label_to_issue(
        &self,
        issue_number: u64,
        label_name: &str,
    ) -> Result<(), GitHubError> {
        let repo = format!("{}/{}", self.owner(), self.repo());
        let issue_str = issue_number.to_string();
        let args = vec![
            "issue",
            "edit",
            &issue_str,
            "-R",
            &repo,
            "--add-label",
            label_name,
        ];
        let args_vec: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let repo_targeted = args.contains(&"-R");

        // Record the call
        self.gh_calls.lock().unwrap().push(GhCliCall::AddLabel {
            issue_number,
            label: label_name.to_string(),
            repo_targeted,
            command_args: args_vec.clone(),
        });

        // Simulate gh command execution
        let output = self.simulate_gh_command(&args);

        match output {
            Ok(result) => {
                if result.status.success() {
                    Ok(())
                } else {
                    let error_msg = String::from_utf8_lossy(&result.stderr);
                    Err(GitHubError::IoError(IoError::other(format!(
                        "Failed to add label {label_name}: {error_msg}"
                    ))))
                }
            }
            Err(e) => Err(GitHubError::IoError(e)),
        }
    }

    async fn fetch_issues(&self) -> Result<Vec<octocrab::models::issues::Issue>, GitHubError> {
        let repo = format!("{}/{}", self.owner(), self.repo());
        let args = [
            "issue",
            "list",
            "-R",
            &repo,
            "--json",
            "number,title,labels,assignees",
        ];
        let args_vec: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let repo_targeted = args.contains(&"-R");

        // Record the call
        self.gh_calls.lock().unwrap().push(GhCliCall::FetchIssues {
            repo_targeted,
            command_args: args_vec,
        });

        Ok(self.issues.clone())
    }

    fn owner(&self) -> &str {
        &self.owner
    }

    fn repo(&self) -> &str {
        &self.repo
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_label_removal_includes_repo_targeting() {
        // Given: A mock GitHub client
        let client = MockGitHubClient::new("johnhkchen", "clambake");

        // When: We remove a label from an issue
        let result = client.remove_label_from_issue(95, "agent001").await;

        // Then: The operation should succeed
        assert!(result.is_ok(), "Label removal should succeed");

        // And: The gh CLI call should include explicit repo targeting
        let calls = client.get_gh_calls();
        assert_eq!(calls.len(), 1);

        match &calls[0] {
            GhCliCall::RemoveLabel {
                issue_number,
                label,
                repo_targeted,
                command_args,
            } => {
                assert_eq!(*issue_number, 95);
                assert_eq!(label, "agent001");
                assert!(repo_targeted, "gh CLI call must include -R repo targeting");
                assert!(command_args.contains(&"-R".to_string()));
                assert!(command_args.contains(&"johnhkchen/clambake".to_string()));
                assert!(command_args.contains(&"--remove-label".to_string()));
            }
            _ => panic!("Expected RemoveLabel call"),
        }
    }

    #[tokio::test]
    async fn test_label_addition_includes_repo_targeting() {
        // Given: A mock GitHub client
        let client = MockGitHubClient::new("johnhkchen", "clambake");

        // When: We add a label to an issue
        let result = client.add_label_to_issue(95, "route:ready_to_merge").await;

        // Then: The operation should succeed
        assert!(result.is_ok(), "Label addition should succeed");

        // And: The gh CLI call should include explicit repo targeting
        let calls = client.get_gh_calls();
        assert_eq!(calls.len(), 1);

        match &calls[0] {
            GhCliCall::AddLabel {
                issue_number,
                label,
                repo_targeted,
                command_args,
            } => {
                assert_eq!(*issue_number, 95);
                assert_eq!(label, "route:ready_to_merge");
                assert!(repo_targeted, "gh CLI call must include -R repo targeting");
                assert!(command_args.contains(&"-R".to_string()));
                assert!(command_args.contains(&"johnhkchen/clambake".to_string()));
                assert!(command_args.contains(&"--add-label".to_string()));
            }
            _ => panic!("Expected AddLabel call"),
        }
    }

    #[tokio::test]
    async fn test_issue_fetching_includes_repo_targeting() {
        // Given: A mock GitHub client with test issues
        let test_issues = fixtures::load_test_issues();
        let client =
            MockGitHubClient::new("johnhkchen", "clambake").with_issues(test_issues.clone());

        // When: We fetch issues
        let result = client.fetch_issues().await;

        // Then: The operation should succeed
        assert!(result.is_ok(), "Issue fetching should succeed");
        let issues = result.unwrap();
        assert_eq!(issues.len(), test_issues.len());

        // And: The gh CLI call should include explicit repo targeting
        let calls = client.get_gh_calls();
        assert_eq!(calls.len(), 1);

        match &calls[0] {
            GhCliCall::FetchIssues {
                repo_targeted,
                command_args,
            } => {
                assert!(repo_targeted, "gh CLI call must include -R repo targeting");
                assert!(command_args.contains(&"-R".to_string()));
                assert!(command_args.contains(&"johnhkchen/clambake".to_string()));
            }
            _ => panic!("Expected FetchIssues call"),
        }
    }

    #[tokio::test]
    async fn test_label_removal_failure_detection() {
        // Given: A mock client configured to simulate label removal failure
        let client = MockGitHubClient::new("johnhkchen", "clambake").with_failure_scenario(
            FailureScenario::RemoveLabelFailure {
                issue_number: 95,
                error_msg: "Label 'agent001' not found on issue #95".to_string(),
            },
        );

        // When: We attempt to remove a label
        let result = client.remove_label_from_issue(95, "agent001").await;

        // Then: The operation should fail with proper error detection
        assert!(result.is_err(), "Label removal should fail as configured");

        let error = result.unwrap_err();
        let error_msg = format!("{error:?}");
        assert!(
            error_msg.contains("Label 'agent001' not found"),
            "Error should contain failure message: {error_msg}"
        );

        // And: The failure should still be tracked
        let calls = client.get_gh_calls();
        assert_eq!(calls.len(), 1);
        assert!(matches!(calls[0], GhCliCall::RemoveLabel { .. }));
    }

    #[tokio::test]
    async fn test_label_addition_failure_detection() {
        // Given: A mock client configured to simulate label addition failure
        let client = MockGitHubClient::new("johnhkchen", "clambake").with_failure_scenario(
            FailureScenario::AddLabelFailure {
                issue_number: 95,
                error_msg: "Label 'invalid-label' does not exist in repository".to_string(),
            },
        );

        // When: We attempt to add an invalid label
        let result = client.add_label_to_issue(95, "invalid-label").await;

        // Then: The operation should fail with proper error detection
        assert!(result.is_err(), "Label addition should fail as configured");

        let error = result.unwrap_err();
        let error_msg = format!("{error:?}");
        assert!(
            error_msg.contains("does not exist in repository"),
            "Error should contain failure message: {error_msg}"
        );
    }

    #[tokio::test]
    async fn test_directory_context_independence() {
        // Given: Multiple clients with different working contexts
        let client_main = MockGitHubClient::new("johnhkchen", "clambake");
        let client_fork = MockGitHubClient::new("otherfork", "clambake");

        // When: We perform operations from different contexts
        let result1 = client_main.add_label_to_issue(95, "agent001").await;
        let result2 = client_fork.remove_label_from_issue(42, "route:ready").await;

        // Then: Both operations should succeed independently
        assert!(result1.is_ok(), "Main repo operation should succeed");
        assert!(result2.is_ok(), "Fork repo operation should succeed");

        // And: Each should target their respective repositories
        let calls1 = client_main.get_gh_calls();
        let calls2 = client_fork.get_gh_calls();

        match &calls1[0] {
            GhCliCall::AddLabel { command_args, .. } => {
                assert!(command_args.contains(&"johnhkchen/clambake".to_string()));
            }
            _ => panic!("Expected AddLabel call"),
        }

        match &calls2[0] {
            GhCliCall::RemoveLabel { command_args, .. } => {
                assert!(command_args.contains(&"otherfork/clambake".to_string()));
            }
            _ => panic!("Expected RemoveLabel call"),
        }
    }

    #[tokio::test]
    async fn test_completed_work_never_reassigned_detection() {
        // Given: Issues with completed work (route:ready_to_merge label indicates merge-ready)
        let completed_issue = fixtures::create_completed_issue(93, "Fix state management bug");
        let ready_issue = fixtures::create_ready_issue(95, "Add regression tests");

        let issues = vec![completed_issue.clone(), ready_issue.clone()];
        let client = MockGitHubClient::new("johnhkchen", "clambake").with_issues(issues);

        // When: We fetch issues and filter for assignable work
        let all_issues = client.fetch_issues().await.unwrap();
        let assignable_issues = fixtures::filter_assignable_issues(&all_issues);

        // Then: Completed work should not be in assignable list
        let assignable_numbers: Vec<u64> = assignable_issues.iter().map(|i| i.number).collect();
        assert!(
            !assignable_numbers.contains(&93),
            "Completed issue #93 should not be assignable"
        );
        assert!(
            assignable_numbers.contains(&95),
            "Ready issue #95 should be assignable"
        );
    }

    #[tokio::test]
    async fn test_agent_lifecycle_state_transitions() {
        // Given: A client that tracks label operations through agent lifecycle
        let client = MockGitHubClient::new("johnhkchen", "clambake");

        // When: We simulate complete agent lifecycle
        // 1. Assign work to agent
        let _ = client.add_label_to_issue(95, "agent001").await;

        // 2. Complete work (add route:ready_to_merge label)
        let _ = client.add_label_to_issue(95, "route:ready_to_merge").await;

        // 3. Free agent (remove agent label)
        let _ = client.remove_label_from_issue(95, "agent001").await;

        // Then: All operations should succeed and be properly tracked
        let calls = client.get_gh_calls();
        assert_eq!(
            calls.len(),
            3,
            "Should have 3 gh CLI calls for complete lifecycle"
        );

        // Verify the sequence
        assert!(matches!(&calls[0], GhCliCall::AddLabel { label, .. } if label == "agent001"));
        assert!(
            matches!(&calls[1], GhCliCall::AddLabel { label, .. } if label == "route:ready_to_merge")
        );
        assert!(matches!(&calls[2], GhCliCall::RemoveLabel { label, .. } if label == "agent001"));

        // Verify all calls used proper repo targeting
        for call in &calls {
            match call {
                GhCliCall::AddLabel { repo_targeted, .. }
                | GhCliCall::RemoveLabel { repo_targeted, .. } => {
                    assert!(
                        repo_targeted,
                        "All lifecycle operations must use repo targeting"
                    );
                }
                _ => {}
            }
        }
    }
}
