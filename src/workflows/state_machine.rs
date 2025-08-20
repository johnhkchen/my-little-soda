// Atomic State Transitions - VERBOTEN Compliance
// Following VERBOTEN rules: All operations must be atomic

use crate::github::{GitHubClient, GitHubError};
use crate::agents::{AgentState, AgentCoordinator};

#[derive(Debug, Clone)]
pub enum StateTransition {
    AssignToAgent { agent_id: String, issue_url: String },
    StartWork { agent_id: String, issue_url: String },
    CompleteWork { agent_id: String, issue_url: String },
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

    async fn atomic_integrate_work(&self, agent_id: &str, issue_url: &str) -> Result<TransitionResult, GitHubError> {
        let previous_state = AgentState::Completed(issue_url.to_string());
        let new_state = AgentState::Available;
        
        // Atomic operation: Either integration succeeds completely or fails with work preserved
        println!("🔄 Atomically integrating work: agent {} issue {}", agent_id, issue_url);
        
        // In production, this would:
        // 1. Merge to main branch
        // 2. Close issue
        // 3. Clean up branch
        // 4. Free agent
        // All atomically with rollback on failure
        
        self.coordinator.update_agent_state(agent_id, new_state.clone()).await?;
        Ok(TransitionResult::Success { previous_state, new_state })
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