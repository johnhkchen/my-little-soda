use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// Local type definitions to avoid import issues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inconsistency {
    pub agent_id: String,
    pub pattern: StuckAgentPattern,
    pub detected_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StuckAgentPattern {
    LabeledButNoBranch { agent_id: String, issue: u64 },
    BranchButNoLabel { agent_id: String, branch: String },
    WorkingButNoCommits { agent_id: String, issue: u64 },
    LandedButNotFreed { agent_id: String, issue: u64 },
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Architectural error enum - fields reserved for future error context
pub enum StateError {
    GitHub(String),
    Git(String),
    Validation(String),
}
use crate::github::GitHubClient;

/// Validation report for a single agent's state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub agent_id: String,
    pub is_consistent: bool,
    pub inconsistencies: Vec<StuckAgentPattern>,
    pub github_labels: Vec<String>,
    pub github_branches: Vec<String>,
    pub git_commits_ahead: Option<u32>,
    pub validated_at: DateTime<Utc>,
}

/// Comprehensive state validation report for all agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemValidationReport {
    pub total_agents: usize,
    pub consistent_agents: usize,
    pub inconsistent_agents: usize,
    pub inconsistencies: Vec<Inconsistency>,
    pub validation_duration_ms: u64,
    pub validated_at: DateTime<Utc>,
}

/// Trait for state validation operations
#[async_trait]
pub trait StateValidation {
    /// Validate a specific agent's state against GitHub/Git reality
    #[allow(dead_code)] // Future agent state validation features
    async fn validate_agent_state(&self, agent_id: &str) -> Result<ValidationReport, StateError>;

    /// Detect all stuck agent patterns across the system
    async fn detect_all_inconsistencies(&self) -> Result<Vec<Inconsistency>, StateError>;

    /// Validate all agent states and generate comprehensive report
    #[allow(dead_code)] // Future system state validation features
    async fn validate_system_state(&self) -> Result<SystemValidationReport, StateError>;

    /// Check for specific stuck agent pattern
    #[allow(dead_code)] // Future pattern-specific validation features
    async fn check_specific_pattern(
        &self,
        pattern_type: &str,
    ) -> Result<Vec<StuckAgentPattern>, StateError>;
}

/// State validator implementation
pub struct StateValidator {
    github_client: GitHubClient,
}

impl StateValidator {
    pub fn new(github_client: GitHubClient) -> Self {
        Self { github_client }
    }

    /// Extract agent ID from label (e.g., "agent001" from labels)
    fn extract_agent_id_from_labels(labels: &[octocrab::models::Label]) -> Option<String> {
        labels
            .iter()
            .find(|label| label.name.starts_with("agent"))
            .map(|label| label.name.clone())
    }

    /// Check if agent has a corresponding branch for the issue
    async fn check_agent_branch_exists(
        &self,
        agent_id: &str,
        issue_number: u64,
    ) -> Result<bool, StateError> {
        // Expected branch name format: agent001/123-description
        let expected_branch_prefix = format!("{agent_id}/{issue_number}");

        // Use git command to check for branches since octocrab branch API is not fully implemented
        match std::process::Command::new("git")
            .args([
                "branch",
                "-r",
                "--list",
                &format!("origin/{expected_branch_prefix}*"),
            ])
            .output()
        {
            Ok(output) if output.status.success() => {
                let branches_output = String::from_utf8_lossy(&output.stdout);
                Ok(!branches_output.trim().is_empty())
            }
            Ok(_) => Ok(false),
            Err(e) => Err(StateError::Git(format!(
                "Failed to check git branches: {e}"
            ))),
        }
    }

    /// Check if there are any commits ahead on the agent's branch
    async fn check_commits_ahead(
        &self,
        agent_id: &str,
        issue_number: u64,
    ) -> Result<Option<u32>, StateError> {
        let expected_branch_prefix = format!("{agent_id}/{issue_number}");

        // First, find the exact branch name
        let branch_output = std::process::Command::new("git")
            .args([
                "branch",
                "-r",
                "--list",
                &format!("origin/{expected_branch_prefix}*"),
            ])
            .output()
            .map_err(|e| StateError::Git(format!("Failed to list branches: {e}")))?;

        if !branch_output.status.success() {
            return Ok(None);
        }

        let branches_output = String::from_utf8_lossy(&branch_output.stdout);
        let branch_line = branches_output.lines().next();

        if let Some(branch_line) = branch_line {
            let branch_name = branch_line
                .trim()
                .strip_prefix("origin/")
                .unwrap_or(branch_line.trim());

            // Check commits ahead of main
            let commits_output = std::process::Command::new("git")
                .args([
                    "rev-list",
                    "--count",
                    &format!("origin/main..origin/{branch_name}"),
                ])
                .output()
                .map_err(|e| StateError::Git(format!("Failed to count commits: {e}")))?;

            if commits_output.status.success() {
                let count_str = String::from_utf8_lossy(&commits_output.stdout)
                    .trim()
                    .to_string();
                if let Ok(count) = count_str.parse::<u32>() {
                    return Ok(Some(count));
                }
            }
        }

        Ok(None)
    }

    /// Detect LabeledButNoBranch pattern
    async fn detect_labeled_but_no_branch(&self) -> Result<Vec<StuckAgentPattern>, StateError> {
        let issues = self
            .github_client
            .fetch_issues_with_state(Some(octocrab::params::State::Open))
            .await
            .map_err(|e| StateError::GitHub(format!("Failed to fetch issues: {e}")))?;

        let mut patterns = Vec::new();

        for issue in issues {
            if let Some(agent_id) = Self::extract_agent_id_from_labels(&issue.labels) {
                let has_branch = self
                    .check_agent_branch_exists(&agent_id, issue.number)
                    .await?;

                if !has_branch {
                    patterns.push(StuckAgentPattern::LabeledButNoBranch {
                        agent_id,
                        issue: issue.number,
                    });
                }
            }
        }

        Ok(patterns)
    }

    /// Detect BranchButNoLabel pattern
    async fn detect_branch_but_no_label(&self) -> Result<Vec<StuckAgentPattern>, StateError> {
        // Get all agent branches
        let branch_output = std::process::Command::new("git")
            .args(["branch", "-r", "--list", "origin/agent*"])
            .output()
            .map_err(|e| StateError::Git(format!("Failed to list agent branches: {e}")))?;

        if !branch_output.status.success() {
            return Ok(Vec::new());
        }

        let branches_output = String::from_utf8_lossy(&branch_output.stdout);
        let mut patterns = Vec::new();

        for branch_line in branches_output.lines() {
            let branch_name = branch_line
                .trim()
                .strip_prefix("origin/")
                .unwrap_or(branch_line.trim());

            // Parse agent ID from branch name (e.g., "agent001/123-description")
            if let Some((agent_id, issue_part)) = branch_name.split_once('/') {
                if let Some(issue_number_str) = issue_part.split('-').next() {
                    if let Ok(issue_number) = issue_number_str.parse::<u64>() {
                        // Check if the issue has the corresponding agent label
                        match self.github_client.fetch_issue(issue_number).await {
                            Ok(issue) => {
                                if !Self::extract_agent_id_from_labels(&issue.labels)
                                    .map(|id| id == agent_id)
                                    .unwrap_or(false)
                                {
                                    patterns.push(StuckAgentPattern::BranchButNoLabel {
                                        agent_id: agent_id.to_string(),
                                        branch: branch_name.to_string(),
                                    });
                                }
                            }
                            Err(_) => {
                                // Issue might not exist, this is also a stuck state
                                patterns.push(StuckAgentPattern::BranchButNoLabel {
                                    agent_id: agent_id.to_string(),
                                    branch: branch_name.to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(patterns)
    }

    /// Detect WorkingButNoCommits pattern
    async fn detect_working_but_no_commits(&self) -> Result<Vec<StuckAgentPattern>, StateError> {
        let issues = self
            .github_client
            .fetch_issues_with_state(Some(octocrab::params::State::Open))
            .await
            .map_err(|e| StateError::GitHub(format!("Failed to fetch issues: {e}")))?;

        let mut patterns = Vec::new();

        for issue in issues {
            if let Some(agent_id) = Self::extract_agent_id_from_labels(&issue.labels) {
                // Check if agent has branch but no commits
                let has_branch = self
                    .check_agent_branch_exists(&agent_id, issue.number)
                    .await?;

                if has_branch {
                    let commits_ahead = self.check_commits_ahead(&agent_id, issue.number).await?;

                    if commits_ahead == Some(0) {
                        patterns.push(StuckAgentPattern::WorkingButNoCommits {
                            agent_id,
                            issue: issue.number,
                        });
                    }
                }
            }
        }

        Ok(patterns)
    }
}

#[async_trait]
impl StateValidation for StateValidator {
    async fn validate_agent_state(&self, agent_id: &str) -> Result<ValidationReport, StateError> {
        let start_time = Utc::now();

        // Find issues assigned to this agent
        let issues = self
            .github_client
            .fetch_issues_with_state(Some(octocrab::params::State::Open))
            .await
            .map_err(|e| StateError::GitHub(format!("Failed to fetch issues: {e}")))?;

        let agent_issues: Vec<_> = issues
            .into_iter()
            .filter(|issue| {
                Self::extract_agent_id_from_labels(&issue.labels)
                    .map(|id| id == agent_id)
                    .unwrap_or(false)
            })
            .collect();

        let mut inconsistencies = Vec::new();
        let mut github_labels = Vec::new();
        let mut github_branches = Vec::new();
        let mut git_commits_ahead = None;

        // Check each assigned issue for inconsistencies
        for issue in &agent_issues {
            // Collect labels
            github_labels.extend(issue.labels.iter().map(|l| l.name.clone()));

            // Check branch existence
            let has_branch = self
                .check_agent_branch_exists(agent_id, issue.number)
                .await?;

            if has_branch {
                github_branches.push(format!("{}/{}", agent_id, issue.number));

                // Check commits ahead
                git_commits_ahead = self.check_commits_ahead(agent_id, issue.number).await?;

                // Detect WorkingButNoCommits
                if git_commits_ahead == Some(0) {
                    inconsistencies.push(StuckAgentPattern::WorkingButNoCommits {
                        agent_id: agent_id.to_string(),
                        issue: issue.number,
                    });
                }
            } else {
                // Detect LabeledButNoBranch
                inconsistencies.push(StuckAgentPattern::LabeledButNoBranch {
                    agent_id: agent_id.to_string(),
                    issue: issue.number,
                });
            }
        }

        let is_consistent = inconsistencies.is_empty();

        Ok(ValidationReport {
            agent_id: agent_id.to_string(),
            is_consistent,
            inconsistencies,
            github_labels,
            github_branches,
            git_commits_ahead,
            validated_at: start_time,
        })
    }

    async fn detect_all_inconsistencies(&self) -> Result<Vec<Inconsistency>, StateError> {
        let now = Utc::now();

        // Collect all stuck patterns
        let mut all_patterns = Vec::new();

        all_patterns.extend(self.detect_labeled_but_no_branch().await?);
        all_patterns.extend(self.detect_branch_but_no_label().await?);
        all_patterns.extend(self.detect_working_but_no_commits().await?);

        // Convert patterns to inconsistencies
        let inconsistencies = all_patterns
            .into_iter()
            .map(|pattern| {
                let agent_id = match &pattern {
                    StuckAgentPattern::LabeledButNoBranch { agent_id, .. } => agent_id.clone(),
                    StuckAgentPattern::BranchButNoLabel { agent_id, .. } => agent_id.clone(),
                    StuckAgentPattern::WorkingButNoCommits { agent_id, .. } => agent_id.clone(),
                    StuckAgentPattern::LandedButNotFreed { agent_id, .. } => agent_id.clone(),
                };

                Inconsistency {
                    agent_id,
                    pattern,
                    detected_at: now,
                }
            })
            .collect();

        Ok(inconsistencies)
    }

    async fn validate_system_state(&self) -> Result<SystemValidationReport, StateError> {
        let start_time = std::time::Instant::now();
        let validation_start = Utc::now();

        let inconsistencies = self.detect_all_inconsistencies().await?;

        // Count unique agents
        let mut agent_set = std::collections::HashSet::new();
        for inconsistency in &inconsistencies {
            agent_set.insert(&inconsistency.agent_id);
        }

        // For now, assume we have a fixed set of agents (agent001-agent010)
        // In a real implementation, this would be discovered from configuration
        let total_agents: usize = 1; // Currently only agent001 is active
        let inconsistent_agents = agent_set.len();
        let consistent_agents = total_agents.saturating_sub(inconsistent_agents);

        let validation_duration_ms = start_time.elapsed().as_millis() as u64;

        Ok(SystemValidationReport {
            total_agents,
            consistent_agents,
            inconsistent_agents,
            inconsistencies,
            validation_duration_ms,
            validated_at: validation_start,
        })
    }

    async fn check_specific_pattern(
        &self,
        pattern_type: &str,
    ) -> Result<Vec<StuckAgentPattern>, StateError> {
        match pattern_type {
            "LabeledButNoBranch" => self.detect_labeled_but_no_branch().await,
            "BranchButNoLabel" => self.detect_branch_but_no_label().await,
            "WorkingButNoCommits" => self.detect_working_but_no_commits().await,
            _ => Err(StateError::Validation(format!(
                "Unknown pattern type: {pattern_type}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a mock label - we'll use a simplified approach
    fn create_mock_label(name: &str) -> octocrab::models::Label {
        // Since Label is non-exhaustive, we'll use serde_json to create it
        let json = format!(
            r#"{{
            "id": 1,
            "node_id": "MDU6TGFiZWwx",
            "url": "https://api.github.com/repos/owner/repo/labels/{name}",
            "name": "{name}",
            "description": null,
            "color": "000000",
            "default": false
        }}"#
        );

        serde_json::from_str(&json).expect("Failed to create mock label")
    }

    #[test]
    fn test_extract_agent_id_from_labels() {
        let labels = vec![
            create_mock_label("agent001"),
            create_mock_label("route:ready"),
        ];

        let agent_id = StateValidator::extract_agent_id_from_labels(&labels);
        assert_eq!(agent_id, Some("agent001".to_string()));
    }

    #[test]
    fn test_extract_agent_id_no_agent_label() {
        let labels = vec![create_mock_label("route:ready")];

        let agent_id = StateValidator::extract_agent_id_from_labels(&labels);
        assert_eq!(agent_id, None);
    }

    #[test]
    fn test_stuck_agent_patterns_serialization() {
        let patterns = vec![
            StuckAgentPattern::LabeledButNoBranch {
                agent_id: "agent001".to_string(),
                issue: 123,
            },
            StuckAgentPattern::BranchButNoLabel {
                agent_id: "agent001".to_string(),
                branch: "agent001/123-test".to_string(),
            },
            StuckAgentPattern::WorkingButNoCommits {
                agent_id: "agent001".to_string(),
                issue: 456,
            },
        ];

        // Test that patterns can be serialized and deserialized
        for pattern in patterns {
            let serialized = serde_json::to_string(&pattern).expect("Serialization should work");
            let deserialized: StuckAgentPattern =
                serde_json::from_str(&serialized).expect("Deserialization should work");

            match (&pattern, &deserialized) {
                (
                    StuckAgentPattern::LabeledButNoBranch {
                        agent_id: a1,
                        issue: i1,
                    },
                    StuckAgentPattern::LabeledButNoBranch {
                        agent_id: a2,
                        issue: i2,
                    },
                ) => {
                    assert_eq!(a1, a2);
                    assert_eq!(i1, i2);
                }
                (
                    StuckAgentPattern::BranchButNoLabel {
                        agent_id: a1,
                        branch: b1,
                    },
                    StuckAgentPattern::BranchButNoLabel {
                        agent_id: a2,
                        branch: b2,
                    },
                ) => {
                    assert_eq!(a1, a2);
                    assert_eq!(b1, b2);
                }
                (
                    StuckAgentPattern::WorkingButNoCommits {
                        agent_id: a1,
                        issue: i1,
                    },
                    StuckAgentPattern::WorkingButNoCommits {
                        agent_id: a2,
                        issue: i2,
                    },
                ) => {
                    assert_eq!(a1, a2);
                    assert_eq!(i1, i2);
                }
                _ => panic!("Pattern types do not match"),
            }
        }
    }

    #[test]
    fn test_inconsistency_serialization() {
        let inconsistency = Inconsistency {
            agent_id: "agent001".to_string(),
            pattern: StuckAgentPattern::LabeledButNoBranch {
                agent_id: "agent001".to_string(),
                issue: 123,
            },
            detected_at: chrono::Utc::now(),
        };

        let serialized = serde_json::to_string(&inconsistency).expect("Serialization should work");
        let deserialized: Inconsistency =
            serde_json::from_str(&serialized).expect("Deserialization should work");

        assert_eq!(inconsistency.agent_id, deserialized.agent_id);
        // Note: We don't check the exact timestamp due to potential precision differences
    }

    #[test]
    fn test_validation_report_structure() {
        let report = ValidationReport {
            agent_id: "agent001".to_string(),
            is_consistent: false,
            inconsistencies: vec![StuckAgentPattern::LabeledButNoBranch {
                agent_id: "agent001".to_string(),
                issue: 123,
            }],
            github_labels: vec!["agent001".to_string(), "route:ready".to_string()],
            github_branches: vec!["agent001/123".to_string()],
            git_commits_ahead: Some(0),
            validated_at: chrono::Utc::now(),
        };

        assert_eq!(report.agent_id, "agent001");
        assert!(!report.is_consistent);
        assert_eq!(report.inconsistencies.len(), 1);
        assert_eq!(report.github_labels.len(), 2);
        assert_eq!(report.github_branches.len(), 1);
        assert_eq!(report.git_commits_ahead, Some(0));
    }

    #[test]
    fn test_system_validation_report_structure() {
        let report = SystemValidationReport {
            total_agents: 5,
            consistent_agents: 3,
            inconsistent_agents: 2,
            inconsistencies: vec![Inconsistency {
                agent_id: "agent001".to_string(),
                pattern: StuckAgentPattern::LabeledButNoBranch {
                    agent_id: "agent001".to_string(),
                    issue: 123,
                },
                detected_at: chrono::Utc::now(),
            }],
            validation_duration_ms: 1500,
            validated_at: chrono::Utc::now(),
        };

        assert_eq!(report.total_agents, 5);
        assert_eq!(report.consistent_agents, 3);
        assert_eq!(report.inconsistent_agents, 2);
        assert_eq!(report.inconsistencies.len(), 1);
        assert_eq!(report.validation_duration_ms, 1500);
    }

    #[test]
    fn test_state_error_types() {
        let github_error = StateError::GitHub("GitHub API failed".to_string());
        let git_error = StateError::Git("Git command failed".to_string());
        let validation_error = StateError::Validation("Invalid state".to_string());

        match github_error {
            StateError::GitHub(msg) => assert_eq!(msg, "GitHub API failed"),
            _ => panic!("Expected GitHub error"),
        }

        match git_error {
            StateError::Git(msg) => assert_eq!(msg, "Git command failed"),
            _ => panic!("Expected Git error"),
        }

        match validation_error {
            StateError::Validation(msg) => assert_eq!(msg, "Invalid state"),
            _ => panic!("Expected Validation error"),
        }
    }
}
