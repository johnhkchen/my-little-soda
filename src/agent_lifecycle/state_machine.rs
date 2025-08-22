use serde::{Deserialize, Serialize};
use thiserror::Error;
use statig::prelude::*;
// Forward declare to avoid circular dependency
// use crate::agents::validation::{StateValidator, StateValidation};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentEvent {
    Assign { 
        agent_id: String, 
        issue: u64,
        branch: String,
    },
    StartWork { 
        commits_ahead: u32 
    },
    CompleteWork,
    Bundle { 
        bundle_pr: u64,
        issues: Vec<u64>,
    },
    Merge,
    Abandon,
    ForceReset,
}

#[derive(Debug, Error)]
pub enum TransitionError {
    #[error("Invalid transition with event {event:?}")]
    InvalidTransition { event: AgentEvent },
    #[error("Validation failed: {reason}")]
    ValidationFailed { reason: String },
    #[error("Agent ID mismatch: expected {expected}, got {actual}")]
    AgentIdMismatch { expected: String, actual: String },
}

#[derive(Debug, Error)]
pub enum StateError {
    #[error("GitHub API error: {0}")]
    GitHubError(String),
    #[error("Git operations error: {0}")]
    GitError(String),
    #[error("State validation error: {0}")]
    ValidationError(String),
}

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

#[derive(Debug)]
pub struct RecoveryReport {
    pub recovered: Vec<String>,
    pub failed: Vec<(String, String)>,
    pub skipped: Vec<String>,
}

#[derive(Default)]
pub struct AgentStateMachine {
    pub agent_id: String,
    pub current_issue: Option<u64>,
    pub current_branch: Option<String>,
    pub commits_ahead: u32,
    pub bundle_issues: Vec<u64>,
    pub bundle_pr: Option<u64>,
}

impl AgentStateMachine {
    pub fn new(agent_id: String) -> Self {
        Self {
            agent_id,
            ..Default::default()
        }
    }
    
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }
    
    fn validate_agent_id(&self, event_agent_id: &str) -> Result<(), TransitionError> {
        if self.agent_id != event_agent_id {
            return Err(TransitionError::AgentIdMismatch {
                expected: self.agent_id.clone(),
                actual: event_agent_id.to_string(),
            });
        }
        Ok(())
    }
}

#[state_machine(initial = "State::idle()")]
impl AgentStateMachine {
    #[state]
    fn idle(&mut self, event: &AgentEvent) -> Outcome<State> {
        match event {
            AgentEvent::Assign { agent_id, issue, branch } => {
                if let Err(e) = self.validate_agent_id(agent_id) {
                    tracing::error!("Agent ID validation failed: {}", e);
                    return Handled;
                }
                
                // Apply validation guards
                if let Err(e) = self.validate_assignment(*issue, branch) {
                    tracing::error!("Assignment validation failed: {}", e);
                    return Handled;
                }
                
                self.current_issue = Some(*issue);
                self.current_branch = Some(branch.clone());
                self.commits_ahead = 0;
                tracing::info!(
                    agent_id = %self.agent_id,
                    issue = %issue,
                    branch = %branch,
                    "Agent assigned to issue"
                );
                self.on_enter_assigned();
                Transition(State::assigned())
            }
            _ => Handled,
        }
    }
    
    #[state]
    fn assigned(&mut self, event: &AgentEvent) -> Outcome<State> {
        match event {
            AgentEvent::StartWork { commits_ahead } => {
                // Apply validation guards
                if let Err(e) = self.validate_start_work(*commits_ahead) {
                    tracing::error!("Start work validation failed: {}", e);
                    return Handled;
                }
                
                self.commits_ahead = *commits_ahead;
                tracing::info!(
                    agent_id = %self.agent_id,
                    issue = ?self.current_issue,
                    commits_ahead = %commits_ahead,
                    "Agent started working"
                );
                self.on_exit_assigned();
                self.on_enter_working();
                Transition(State::working())
            }
            AgentEvent::Abandon | AgentEvent::ForceReset => {
                self.reset_state();
                tracing::info!(agent_id = %self.agent_id, "Agent abandoned work");
                Transition(State::idle())
            }
            _ => Handled,
        }
    }
    
    #[state]
    fn working(&mut self, event: &AgentEvent) -> Outcome<State> {
        match event {
            AgentEvent::CompleteWork => {
                // Apply validation guards
                if let Err(e) = self.validate_complete_work() {
                    tracing::error!("Complete work validation failed: {}", e);
                    return Handled;
                }
                
                tracing::info!(
                    agent_id = %self.agent_id,
                    issue = ?self.current_issue,
                    "Agent completed work"
                );
                self.on_exit_working();
                self.on_enter_landed();
                Transition(State::landed())
            }
            AgentEvent::StartWork { commits_ahead } => {
                self.commits_ahead = *commits_ahead;
                tracing::info!(
                    agent_id = %self.agent_id,
                    commits_ahead = %commits_ahead,
                    "Agent updated commits ahead"
                );
                Handled
            }
            AgentEvent::Abandon | AgentEvent::ForceReset => {
                self.reset_state();
                tracing::info!(agent_id = %self.agent_id, "Agent abandoned work");
                Transition(State::idle())
            }
            _ => Handled,
        }
    }
    
    #[state]
    fn landed(&mut self, event: &AgentEvent) -> Outcome<State> {
        match event {
            AgentEvent::Bundle { bundle_pr, issues } => {
                self.bundle_pr = Some(*bundle_pr);
                self.bundle_issues = issues.clone();
                tracing::info!(
                    agent_id = %self.agent_id,
                    bundle_pr = %bundle_pr,
                    issues = ?issues,
                    "Work bundled"
                );
                Transition(State::bundled())
            }
            AgentEvent::Abandon | AgentEvent::ForceReset => {
                self.reset_state();
                tracing::info!(agent_id = %self.agent_id, "Agent reset to idle");
                Transition(State::idle())
            }
            _ => Handled,
        }
    }
    
    #[state]
    fn bundled(&mut self, event: &AgentEvent) -> Outcome<State> {
        match event {
            AgentEvent::Merge => {
                tracing::info!(
                    agent_id = %self.agent_id,
                    bundle_pr = ?self.bundle_pr,
                    issues = ?self.bundle_issues,
                    "Bundle merged"
                );
                Transition(State::merged())
            }
            AgentEvent::Abandon | AgentEvent::ForceReset => {
                self.reset_state();
                tracing::info!(agent_id = %self.agent_id, "Agent reset to idle");
                Transition(State::idle())
            }
            _ => Handled,
        }
    }
    
    #[state]
    fn merged(&mut self, event: &AgentEvent) -> Outcome<State> {
        match event {
            AgentEvent::Abandon | AgentEvent::ForceReset => {
                let completed_issues = self.bundle_issues.clone();
                self.reset_state();
                tracing::info!(
                    agent_id = %self.agent_id,
                    completed_issues = ?completed_issues,
                    "Agent completed merge and reset to idle"
                );
                Transition(State::idle())
            }
            _ => Handled,
        }
    }
}

impl AgentStateMachine {
    fn reset_state(&mut self) {
        self.current_issue = None;
        self.current_branch = None;
        self.commits_ahead = 0;
        self.bundle_issues.clear();
        self.bundle_pr = None;
    }
    
    /// Validation guard for assignment - check if issue is assignable
    fn validate_assignment(&self, issue: u64, branch: &str) -> Result<(), TransitionError> {
        // Check if branch name is valid
        if branch.is_empty() {
            return Err(TransitionError::ValidationFailed {
                reason: "Branch name cannot be empty".to_string(),
            });
        }
        
        // Check if issue number is valid
        if issue == 0 {
            return Err(TransitionError::ValidationFailed {
                reason: "Issue number must be greater than 0".to_string(),
            });
        }
        
        // Check if agent is already assigned to another issue
        if self.current_issue.is_some() && self.current_issue != Some(issue) {
            return Err(TransitionError::ValidationFailed {
                reason: format!(
                    "Agent already assigned to issue {}. Cannot assign to issue {}",
                    self.current_issue.unwrap(),
                    issue
                ),
            });
        }
        
        Ok(())
    }
    
    /// Validation guard for starting work - check if conditions are met
    fn validate_start_work(&self, _commits_ahead: u32) -> Result<(), TransitionError> {
        // Must be assigned to an issue first
        if self.current_issue.is_none() {
            return Err(TransitionError::ValidationFailed {
                reason: "Cannot start work without being assigned to an issue".to_string(),
            });
        }
        
        // Branch must exist
        if self.current_branch.is_none() {
            return Err(TransitionError::ValidationFailed {
                reason: "Cannot start work without a branch".to_string(),
            });
        }
        
        Ok(())
    }
    
    /// Validation guard for completing work - check if work can be completed
    fn validate_complete_work(&self) -> Result<(), TransitionError> {
        // Must be working on an issue
        if self.current_issue.is_none() {
            return Err(TransitionError::ValidationFailed {
                reason: "Cannot complete work without being assigned to an issue".to_string(),
            });
        }
        
        // Must have started work (commits_ahead > 0)
        if self.commits_ahead == 0 {
            return Err(TransitionError::ValidationFailed {
                reason: "Cannot complete work without having made commits".to_string(),
            });
        }
        
        Ok(())
    }
    
    /// Entry action for assigned state - called when transitioning to assigned
    fn on_enter_assigned(&mut self) {
        tracing::info!(
            agent_id = %self.agent_id,
            issue = ?self.current_issue,
            branch = ?self.current_branch,
            "Entering assigned state - agent is now assigned to work"
        );
    }
    
    /// Entry action for working state - called when transitioning to working
    fn on_enter_working(&mut self) {
        tracing::info!(
            agent_id = %self.agent_id,
            issue = ?self.current_issue,
            commits_ahead = %self.commits_ahead,
            "Entering working state - agent has started making commits"
        );
    }
    
    /// Entry action for landed state - called when transitioning to landed
    fn on_enter_landed(&mut self) {
        tracing::info!(
            agent_id = %self.agent_id,
            issue = ?self.current_issue,
            "Entering landed state - work is complete and ready for bundling"
        );
    }
    
    /// Exit action for assigned state - called when leaving assigned
    fn on_exit_assigned(&mut self) {
        tracing::debug!(
            agent_id = %self.agent_id,
            issue = ?self.current_issue,
            "Exiting assigned state"
        );
    }
    
    /// Exit action for working state - called when leaving working
    fn on_exit_working(&mut self) {
        tracing::debug!(
            agent_id = %self.agent_id,
            issue = ?self.current_issue,
            commits_ahead = %self.commits_ahead,
            "Exiting working state"
        );
    }
    
    /// Exit action for landed state - called when leaving landed
    fn on_exit_landed(&mut self) {
        tracing::debug!(
            agent_id = %self.agent_id,
            issue = ?self.current_issue,
            "Exiting landed state"
        );
    }
    
    pub fn current_issue(&self) -> Option<u64> {
        self.current_issue
    }
    
    pub fn current_branch(&self) -> Option<&str> {
        self.current_branch.as_deref()
    }
    
    pub fn commits_ahead(&self) -> u32 {
        self.commits_ahead
    }
    
    pub fn is_available(&self) -> bool {
        // Will be implemented once we understand the generated API
        // For now, check based on internal state
        self.current_issue.is_none()
    }
    
    pub fn is_working(&self) -> bool {
        // Will be implemented once we understand the generated API
        // For now, check based on internal state  
        self.current_issue.is_some() && self.commits_ahead > 0
    }
    
    pub fn is_assigned(&self) -> bool {
        // Will be implemented once we understand the generated API
        // For now, check based on internal state
        self.current_issue.is_some() && self.commits_ahead == 0
    }

    /// Attempt automatic recovery from detected inconsistencies
    /// This method serves as the integration point for the recovery system
    pub async fn attempt_automatic_recovery(&self, github_client: crate::github::GitHubClient) -> Result<crate::agents::recovery::ComprehensiveRecoveryReport, StateError> {
        use crate::agents::recovery::{AutoRecovery, AutomaticRecovery};
        
        let recovery = AutoRecovery::new(github_client, true); // preserve_work = true for safety
        
        recovery.recover_all_inconsistencies().await
            .map_err(|e| StateError::ValidationError(format!("Recovery failed: {:?}", e)))
    }

    /// Apply a recovery action to this state machine instance
    /// This allows the recovery system to trigger state machine events
    pub fn apply_recovery_action(&mut self, action: &crate::agents::recovery::RecoveryAction) -> Result<(), StateError> {
        match action {
            crate::agents::recovery::RecoveryAction::ResetToAssigned { agent_id, issue } => {
                if self.agent_id != *agent_id {
                    return Err(StateError::ValidationError(format!(
                        "Agent ID mismatch: expected {}, got {}", 
                        self.agent_id, 
                        agent_id
                    )));
                }
                
                // Reset to assigned state
                self.current_issue = Some(*issue);
                self.commits_ahead = 0;
                
                tracing::info!(
                    agent_id = %self.agent_id,
                    issue = %issue,
                    "Applied recovery action: reset to assigned state"
                );
                
                Ok(())
            }
            crate::agents::recovery::RecoveryAction::ForceReset { agent_id } => {
                if self.agent_id != *agent_id {
                    return Err(StateError::ValidationError(format!(
                        "Agent ID mismatch: expected {}, got {}", 
                        self.agent_id, 
                        agent_id
                    )));
                }
                
                // Force reset to idle
                self.reset_state();
                
                tracing::info!(
                    agent_id = %self.agent_id,
                    "Applied recovery action: force reset to idle"
                );
                
                Ok(())
            }
            _ => {
                // Other recovery actions (label/branch operations) are handled by the recovery system
                // and don't require state machine changes
                Ok(())
            }
        }
    }
    
    // Note: Validation methods will be added via extension trait to avoid circular dependencies
    // /// Validate this agent's state against external GitHub/Git reality
    // pub async fn validate_state(&self, validator: &StateValidator) -> Result<crate::agents::validation::ValidationReport, StateError> {
    //     validator.validate_agent_state(&self.agent_id).await
    // }
    // 
    // /// Detect inconsistencies for this specific agent
    // pub async fn detect_inconsistencies(&self, validator: &StateValidator) -> Result<Vec<Inconsistency>, StateError> {
    //     let all_inconsistencies = validator.detect_all_inconsistencies().await?;
    //     
    //     // Filter to only this agent's inconsistencies
    //     Ok(all_inconsistencies.into_iter()
    //         .filter(|inc| inc.agent_id == self.agent_id)
    //         .collect())
    // }
    // 
    // /// Check if this agent's current state is consistent with external reality
    // pub async fn is_state_consistent(&self, validator: &StateValidator) -> Result<bool, StateError> {
    //     let report = self.validate_state(validator).await?;
    //     Ok(report.is_consistent)
    // }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_state_machine_basic_workflow() {
        let mut sm = AgentStateMachine::new("agent001".to_string()).state_machine();
        
        // Start in idle state
        assert!(sm.inner().is_available());
        
        // Assign agent to issue
        sm.handle(&AgentEvent::Assign {
            agent_id: "agent001".to_string(),
            issue: 123,
            branch: "agent001/123-test".to_string(),
        });
        
        // Should be assigned now
        assert!(sm.inner().is_assigned());
        assert_eq!(sm.inner().current_issue(), Some(123));
        
        // Start work
        sm.handle(&AgentEvent::StartWork { commits_ahead: 2 });
        
        // Should be working now
        assert!(sm.inner().is_working());
        assert_eq!(sm.inner().commits_ahead(), 2);
        
        // Complete work
        sm.handle(&AgentEvent::CompleteWork);
        
        // Force reset to idle
        sm.handle(&AgentEvent::ForceReset);
        
        // Should be available again
        assert!(sm.inner().is_available());
        assert_eq!(sm.inner().current_issue(), None);
    }
    
    #[test]
    fn test_agent_id_validation() {
        let mut sm = AgentStateMachine::new("agent001".to_string()).state_machine();
        
        // Try to assign with wrong agent ID - should be ignored
        sm.handle(&AgentEvent::Assign {
            agent_id: "agent002".to_string(),
            issue: 123,
            branch: "agent002/123-test".to_string(),
        });
        
        // Should still be available (assignment was rejected)
        assert!(sm.inner().is_available());
        assert_eq!(sm.inner().current_issue(), None);
    }
    
    #[test]
    fn test_simple_state_machine_functionality() {
        // Test basic state machine functionality without the complex API
        let mut agent = AgentStateMachine::new("agent001".to_string());
        
        // Initially should be available
        assert!(agent.is_available());
        assert_eq!(agent.current_issue(), None);
        
        // Simulate assignment
        agent.current_issue = Some(123);
        agent.current_branch = Some("agent001/123-test".to_string());
        assert!(agent.is_assigned());
        assert_eq!(agent.current_issue(), Some(123));
        
        // Simulate starting work
        agent.commits_ahead = 2;
        assert!(agent.is_working());
        assert_eq!(agent.commits_ahead(), 2);
        
        // Reset to idle
        agent.reset_state();
        assert!(agent.is_available());
        assert_eq!(agent.current_issue(), None);
    }
}