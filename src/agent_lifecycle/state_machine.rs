use serde::{Deserialize, Serialize};
use thiserror::Error;
use statig::prelude::*;

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

#[derive(Debug)]
pub struct Inconsistency {
    pub agent_id: String,
    pub pattern: StuckAgentPattern,
    pub detected_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug)]
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
                self.current_issue = Some(*issue);
                self.current_branch = Some(branch.clone());
                self.commits_ahead = 0;
                tracing::info!(
                    agent_id = %self.agent_id,
                    issue = %issue,
                    branch = %branch,
                    "Agent assigned to issue"
                );
                Transition(State::assigned())
            }
            _ => Handled,
        }
    }
    
    #[state]
    fn assigned(&mut self, event: &AgentEvent) -> Outcome<State> {
        match event {
            AgentEvent::StartWork { commits_ahead } => {
                self.commits_ahead = *commits_ahead;
                tracing::info!(
                    agent_id = %self.agent_id,
                    issue = ?self.current_issue,
                    commits_ahead = %commits_ahead,
                    "Agent started working"
                );
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
                tracing::info!(
                    agent_id = %self.agent_id,
                    issue = ?self.current_issue,
                    "Agent completed work"
                );
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_state_machine_basic_workflow() {
        let mut sm = AgentStateMachine::new("agent001".to_string()).state_machine();
        
        // Start in idle state
        assert!(sm.context().is_available());
        
        // Assign agent to issue
        sm.handle(&AgentEvent::Assign {
            agent_id: "agent001".to_string(),
            issue: 123,
            branch: "agent001/123-test".to_string(),
        });
        
        // Should be assigned now
        assert!(sm.context().is_assigned());
        assert_eq!(sm.context().current_issue(), Some(123));
        
        // Start work
        sm.handle(&AgentEvent::StartWork { commits_ahead: 2 });
        
        // Should be working now
        assert!(sm.context().is_working());
        assert_eq!(sm.context().commits_ahead(), 2);
        
        // Complete work
        sm.handle(&AgentEvent::CompleteWork);
        
        // Force reset to idle
        sm.handle(&AgentEvent::ForceReset);
        
        // Should be available again
        assert!(sm.context().is_available());
        assert_eq!(sm.context().current_issue(), None);
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
        assert!(sm.context().is_available());
        assert_eq!(sm.context().current_issue(), None);
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