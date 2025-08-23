// Agent state detection logic

use crate::agent_lifecycle::types::*;
use crate::agent_lifecycle::traits::*;
use anyhow::Result;

/// Implementation of AgentStateDetector for real GitHub operations
pub struct AgentStateDetector<G: GitHubOperations, O: GitOperations> {
    github_ops: G,
    git_ops: O,
}

impl<G: GitHubOperations, O: GitOperations> AgentStateDetector<G, O> {
    /// Create new state detector
    pub fn new(github_ops: G, git_ops: O) -> Self {
        Self {
            github_ops,
            git_ops,
        }
    }
}

impl<G: GitHubOperations, O: GitOperations> StateDetector for AgentStateDetector<G, O> {
    /// Detect current agent state based on GitHub labels and git state
    fn detect_current_state(&self, agent_id: &str) -> Result<AgentState> {
        // Check if agent has any labeled issues
        let agent_issues = self.github_ops.get_issues_with_label(agent_id)?;
        
        if agent_issues.is_empty() {
            return Ok(AgentState::Idle);
        }
        
        // For now, check the first issue (single agent constraint)
        let issue_number = agent_issues[0];
        let issue_labels = self.github_ops.get_labels(issue_number)?;
        
        // Get current branch to determine if agent is working
        let current_branch = self.git_ops.get_current_branch().ok();
        let agent_branch_pattern = format!("{}/", agent_id);
        
        if let Some(branch) = &current_branch {
            if branch.starts_with(&agent_branch_pattern) {
                // Parse issue number from branch name
                if let Some((_, issue_from_branch)) = parse_agent_branch(branch) {
                    if issue_from_branch == issue_number {
                        // Check if there are commits on this branch
                        let commits_ahead = self.git_ops.get_commits_ahead("main").unwrap_or_default();
                        
                        if !commits_ahead.is_empty() {
                            return Ok(AgentState::Working {
                                agent_id: agent_id.to_string(),
                                issue: issue_number,
                                branch: branch.clone(),
                                commits_ahead: commits_ahead.len() as u32,
                            });
                        } else {
                            return Ok(AgentState::Assigned {
                                agent_id: agent_id.to_string(),
                                issue: issue_number,
                                branch: branch.clone(),
                            });
                        }
                    }
                }
            }
        }
        
        // Default to assigned state if agent has labeled issues but unclear git state
        Ok(AgentState::Assigned {
            agent_id: agent_id.to_string(),
            issue: issue_number,
            branch: format!("{}/{}", agent_id, issue_number),
        })
    }
    
    /// Detect pre-flight issues that could block agent operation
    fn detect_pre_flight_issues(&self, agent_id: &str) -> Result<Vec<PreFlightIssue>> {
        let mut issues = Vec::new();
        
        // Check current git state
        let current_branch = self.git_ops.get_current_branch().ok();
        let agent_branch_pattern = format!("{}/", agent_id);
        
        if let Some(branch) = &current_branch {
            if branch.starts_with(&agent_branch_pattern) {
                // Check for commits that haven't been pushed
                let commits_ahead = self.git_ops.get_commits_ahead("main")?;
                if !commits_ahead.is_empty() {
                    // Check if remote branch exists
                    if let Ok(false) = self.git_ops.remote_branch_exists("origin", branch) {
                        issues.push(PreFlightIssue::UnpushedCommits {
                            count: commits_ahead.len() as u32,
                        });
                    }
                }
                
                // Check if branch is behind main
                if let Ok(behind_count) = self.git_ops.get_commits_behind("main") {
                    if behind_count > 0 {
                        issues.push(PreFlightIssue::BehindMain {
                            commits: behind_count,
                        });
                    }
                }
                
                // Check for merge conflicts
                if let Ok(conflicts) = self.git_ops.get_merge_conflicts("main") {
                    if !conflicts.is_empty() {
                        issues.push(PreFlightIssue::MergeConflicts { files: conflicts });
                    }
                }
                
                // Check if working directory is clean
                if let Ok(false) = self.git_ops.is_clean() {
                    // This could indicate uncommitted work
                }
                
                // Parse issue from branch and check GitHub state
                if let Some((_, issue_number)) = parse_agent_branch(branch) {
                    let expected_labels = vec![agent_id.to_string()];
                    if let Ok(actual_labels) = self.github_ops.get_labels(issue_number) {
                        let has_agent_label = actual_labels.contains(&agent_id.to_string());
                        if !has_agent_label {
                            issues.push(PreFlightIssue::LabelMismatch {
                                expected: expected_labels,
                                actual: actual_labels,
                            });
                        }
                    }
                }
            } else if branch != "main" {
                // Agent is on non-agent branch - potential issue
                issues.push(PreFlightIssue::BranchMissing {
                    branch: format!("{}/unknown", agent_id),
                });
            }
        } else {
            // No current branch detected
            issues.push(PreFlightIssue::BranchMissing {
                branch: "unknown".to_string(),
            });
        }
        
        Ok(issues)
    }
    
    /// Validate that expected and actual states match
    fn validate_state(&self, expected: &AgentState, actual: &AgentState) -> Result<bool> {
        match (expected, actual) {
            (AgentState::Idle, AgentState::Idle) => Ok(true),
            (
                AgentState::Assigned { agent_id: e_agent, issue: e_issue, branch: e_branch },
                AgentState::Assigned { agent_id: a_agent, issue: a_issue, branch: a_branch },
            ) => Ok(e_agent == a_agent && e_issue == a_issue && e_branch == a_branch),
            (
                AgentState::Working { agent_id: e_agent, issue: e_issue, branch: e_branch, .. },
                AgentState::Working { agent_id: a_agent, issue: a_issue, branch: a_branch, .. },
            ) => Ok(e_agent == a_agent && e_issue == a_issue && e_branch == a_branch),
            _ => Ok(false), // Different state types don't match
        }
    }
}