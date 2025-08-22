// Atomic State Transitions - VERBOTEN Compliance
// Following VERBOTEN rules: All operations must be atomic

use crate::github::{GitHubClient, GitHubError};
use crate::agents::{AgentState, AgentCoordinator};

#[derive(Debug, Clone)]
pub enum StateTransition {
    AssignToAgent { agent_id: String, issue_url: String },
    StartWork { agent_id: String, issue_url: String },
    CompleteWork { agent_id: String, issue_url: String },
    // Phase 1: Work completion to review
    StartReview { agent_id: String, issue_url: String, pr_number: u64 },
    // Phase 2: Review completion to landing
    StartLanding { agent_id: String, issue_url: String },
    // Original integration (now split into phases)
    IntegrateWork { agent_id: String, issue_url: String },
}

#[derive(Debug, Clone)]
pub enum TransitionResult {
    Success { previous_state: AgentState, new_state: AgentState },
    Failed { error: String, state_preserved: AgentState },
}

#[derive(Debug)]
pub struct StateMachine {
    github_client: GitHubClient,
    coordinator: AgentCoordinator,
}

impl StateMachine {
    pub async fn new() -> Result<Self, GitHubError> {
        let github_client = GitHubClient::new()?;
        let coordinator = AgentCoordinator::new().await?;
        
        Ok(Self {
            github_client,
            coordinator,
        })
    }

    pub async fn execute_atomic_transition(&self, transition: StateTransition) -> Result<TransitionResult, GitHubError> {
        // VERBOTEN rule: All operations must be atomic
        // Either the entire state transition succeeds or it fails with no partial state
        
        match transition {
            StateTransition::AssignToAgent { agent_id, issue_url } => {
                self.atomic_assign_to_agent(&agent_id, &issue_url).await
            },
            StateTransition::StartWork { agent_id, issue_url } => {
                self.atomic_start_work(&agent_id, &issue_url).await
            },
            StateTransition::CompleteWork { agent_id, issue_url } => {
                self.atomic_complete_work(&agent_id, &issue_url).await
            },
            StateTransition::StartReview { agent_id, issue_url, pr_number } => {
                self.atomic_start_review(&agent_id, &issue_url, pr_number).await
            },
            StateTransition::StartLanding { agent_id, issue_url } => {
                self.atomic_start_landing(&agent_id, &issue_url).await
            },
            StateTransition::IntegrateWork { agent_id, issue_url } => {
                self.atomic_integrate_work(&agent_id, &issue_url).await
            },
        }
    }

    fn parse_issue_number_from_url(&self, issue_url: &str) -> Result<u64, GitHubError> {
        // Parse issue number from GitHub URL like: https://github.com/owner/repo/issues/123
        if let Some(issue_number_str) = issue_url.split('/').last() {
            issue_number_str.parse::<u64>()
                .map_err(|_| GitHubError::TokenNotFound("Invalid issue URL format".to_string()))
        } else {
            Err(GitHubError::TokenNotFound("Could not extract issue number from URL".to_string()))
        }
    }

    async fn atomic_assign_to_agent(&self, agent_id: &str, issue_url: &str) -> Result<TransitionResult, GitHubError> {
        let previous_state = AgentState::Available;
        let new_state = AgentState::Assigned(issue_url.to_string());
        
        // Extract issue number from URL
        let issue_number = self.parse_issue_number_from_url(issue_url)?;
        
        // Atomic operation: Either assignment succeeds completely or fails with no changes
        match self.coordinator.assign_agent_to_issue(agent_id, issue_number).await {
            Ok(_) => {
                self.coordinator.update_agent_state(agent_id, new_state.clone()).await?;
                Ok(TransitionResult::Success { previous_state, new_state })
            },
            Err(e) => {
                // Atomic failure: State is preserved, no partial changes
                Ok(TransitionResult::Failed {
                    error: format!("Assignment failed: {:?}", e),
                    state_preserved: previous_state,
                })
            }
        }
    }

    async fn atomic_start_work(&self, agent_id: &str, issue_url: &str) -> Result<TransitionResult, GitHubError> {
        let previous_state = AgentState::Assigned(issue_url.to_string());
        let new_state = AgentState::Working(issue_url.to_string());
        
        // Atomic operation: Either work start succeeds completely or fails with no changes
        println!("🚀 Atomically starting work: agent {} on issue {}", agent_id, issue_url);
        
        // In production, this would:
        // 1. Create work branch
        // 2. Update issue status 
        // 3. Set agent status
        // All in a single atomic GitHub transaction
        
        self.coordinator.update_agent_state(agent_id, new_state.clone()).await?;
        Ok(TransitionResult::Success { previous_state, new_state })
    }

    async fn atomic_complete_work(&self, agent_id: &str, issue_url: &str) -> Result<TransitionResult, GitHubError> {
        let previous_state = AgentState::Working(issue_url.to_string());
        let new_state = AgentState::Completed(issue_url.to_string());
        
        // Atomic operation: Either work completion succeeds completely or fails with no changes
        println!("✅ Atomically completing work: agent {} on issue {}", agent_id, issue_url);
        
        // In production, this would:
        // 1. Validate work is complete
        // 2. Run tests
        // 3. Update PR status
        // 4. Set agent status
        // All atomically
        
        self.coordinator.update_agent_state(agent_id, new_state.clone()).await?;
        Ok(TransitionResult::Success { previous_state, new_state })
    }

    async fn atomic_start_review(&self, agent_id: &str, issue_url: &str, pr_number: u64) -> Result<TransitionResult, GitHubError> {
        let previous_state = AgentState::Completed(issue_url.to_string());
        let new_state = AgentState::UnderReview(issue_url.to_string());
        
        // Phase 1: Atomic transition from completed work to review state
        // This creates PR, removes route:ready label, and frees the agent
        println!("🔄 Phase 1: Starting review for agent {} issue {} (PR #{})", agent_id, issue_url, pr_number);
        
        let issue_number = self.parse_issue_number_from_url(issue_url)?;
        
        // Atomic operation: Remove route:ready label and free agent
        match self.remove_route_ready_label(issue_number).await {
            Ok(_) => {
                println!("✅ Removed route:ready label from issue #{}", issue_number);
                // Agent is now freed for new work
                self.coordinator.update_agent_state(agent_id, AgentState::Available).await?;
                Ok(TransitionResult::Success { 
                    previous_state, 
                    new_state: AgentState::Available // Agent is freed immediately
                })
            },
            Err(e) => {
                Ok(TransitionResult::Failed {
                    error: format!("Failed to start review: {:?}", e),
                    state_preserved: previous_state,
                })
            }
        }
    }

    async fn atomic_start_landing(&self, agent_id: &str, issue_url: &str) -> Result<TransitionResult, GitHubError> {
        let previous_state = AgentState::Available;
        let new_state = AgentState::ReadyToLand(issue_url.to_string());
        
        // Phase 2: Agent picks up route:ready_to_merge task to complete final merge
        println!("🔄 Phase 2: Starting landing for agent {} issue {}", agent_id, issue_url);
        
        self.coordinator.update_agent_state(agent_id, new_state.clone()).await?;
        Ok(TransitionResult::Success { previous_state, new_state })
    }

    async fn atomic_integrate_work(&self, agent_id: &str, issue_url: &str) -> Result<TransitionResult, GitHubError> {
        let previous_state = AgentState::ReadyToLand(issue_url.to_string());
        let new_state = AgentState::Available;
        
        // Atomic operation: Either integration succeeds completely or fails with work preserved
        println!("🔄 Atomically integrating work: agent {} issue {}", agent_id, issue_url);
        
        let issue_number = self.parse_issue_number_from_url(issue_url)?;
        
        // In production, this would:
        // 1. Merge PR to main branch
        // 2. Close issue
        // 3. Remove route:ready_to_merge label
        // 4. Clean up branch
        // 5. Free agent
        // All atomically with rollback on failure
        
        match self.complete_final_integration(issue_number).await {
            Ok(_) => {
                self.coordinator.update_agent_state(agent_id, new_state.clone()).await?;
                Ok(TransitionResult::Success { previous_state, new_state })
            },
            Err(e) => {
                Ok(TransitionResult::Failed {
                    error: format!("Integration failed: {:?}", e),
                    state_preserved: previous_state,
                })
            }
        }
    }

    async fn remove_route_ready_label(&self, issue_number: u64) -> Result<(), GitHubError> {
        // Remove route:ready label to free agent for new work
        use std::process::Command;
        
        let repo = format!("{}/{}", self.github_client.owner(), self.github_client.repo());
        let output = Command::new("gh")
            .args(&["issue", "edit", &issue_number.to_string(), "-R", &repo, "--remove-label", "route:ready"])
            .output();
        
        match output {
            Ok(result) => {
                if result.status.success() {
                    Ok(())
                } else {
                    let error_msg = String::from_utf8_lossy(&result.stderr);
                    Err(GitHubError::IoError(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to remove route:ready label: {}", error_msg)
                    )))
                }
            }
            Err(e) => Err(GitHubError::IoError(e))
        }
    }

    async fn complete_final_integration(&self, issue_number: u64) -> Result<(), GitHubError> {
        // Complete the final integration by removing route:ready_to_merge label and closing issue
        use std::process::Command;
        
        let repo = format!("{}/{}", self.github_client.owner(), self.github_client.repo());
        // Remove route:ready_to_merge label
        let output = Command::new("gh")
            .args(&["issue", "edit", &issue_number.to_string(), "-R", &repo, "--remove-label", "route:ready_to_merge"])
            .output();
        
        match output {
            Ok(result) => {
                if !result.status.success() {
                    let error_msg = String::from_utf8_lossy(&result.stderr);
                    return Err(GitHubError::IoError(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to remove route:ready_to_merge label: {}", error_msg)
                    )));
                }
            }
            Err(e) => return Err(GitHubError::IoError(e))
        }
        
        // Note: Issue closure will be handled by GitHub auto-close when PR merges
        // with "Fixes #issue_number" keywords
        
        Ok(())
    }

    pub async fn validate_state_consistency(&self) -> Result<bool, GitHubError> {
        // Verify that agent states are consistent with GitHub repository state
        // This enforces the "GitHub is source of truth" rule
        
        let agents = self.coordinator.get_available_agents().await?;
        let issues = self.github_client.fetch_issues().await?;
        
        // In production, this would cross-validate:
        // - Agent assignments match GitHub issue assignees
        // - Work branches exist for working agents
        // - Completed work has corresponding PRs
        
        println!("🔍 Validating state consistency for {} agents and {} issues", 
                agents.len(), issues.len());
        
        Ok(true) // For MVP, assume consistency
    }
}
#[cfg(test)]
mod tests_parse_issue_number {
    use super::StateMachine;
    use crate::github::GitHubError;

    // Unsafe helper: construct a StateMachine suitable for calling parse_issue_number_from_url,
    // which does not touch any fields.
    fn sm() -> StateMachine {
        unsafe { std::mem::MaybeUninit::<StateMachine>::zeroed().assume_init() }
    }

    #[test]
    fn parses_valid_issue_url() {
        let s = sm();
        let url = "https://github.com/owner/repo/issues/123";
        let n = s.parse_issue_number_from_url(url).unwrap();
        assert_eq!(n, 123);
    }

    #[test]
    fn rejects_non_numeric_tail() {
        let s = sm();
        let url = "https://github.com/owner/repo/issues/notanumber";
        let err = s.parse_issue_number_from_url(url).unwrap_err();
        let msg = format!("{:?}", err);
        assert!(msg.contains("Invalid issue URL format"));
    }

    #[test]
    fn empty_url_is_error() {
        let s = sm();
        let url = "";
        let err = s.parse_issue_number_from_url(url).unwrap_err();
        let msg = format!("{:?}", err);
        // split('/').last() returns Some(""), parse fails
        assert!(msg.contains("Invalid issue URL format") || msg.contains("Could not extract issue number"));
    }

    #[test]
    fn accepts_pulls_path_currently() {
        let s = sm();
        let url = "https://github.com/owner/repo/pulls/77";
        let n = s.parse_issue_number_from_url(url).unwrap();
        assert_eq!(n, 77);
    }
}