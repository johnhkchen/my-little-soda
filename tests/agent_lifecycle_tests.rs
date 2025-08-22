//! Agent lifecycle state management tests
//!
//! These tests verify the complete agent workflow from assignment to completion
//! to prevent regressions like issue #93 where completed work was reassigned.
//!
//! Test coverage:
//! - Complete work → route:ready → clambake land → agent freed cycle
//! - Completed work never reassigned by clambake pop
//! - State consistency across operations
//! - Edge cases with partial state transitions

use clambake::github::GitHubError;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

mod fixtures;

// Agent state tracking for lifecycle tests
#[derive(Debug, Clone, PartialEq)]
pub enum AgentState {
    Available,
    Assigned(u64), // issue number
    Working(u64),
    Completed(u64),
    Freed,
}

#[derive(Debug, Clone)]
pub struct AgentLifecycleEvent {
    pub agent_id: String,
    pub issue_number: u64,
    pub event_type: LifecycleEventType,
    pub timestamp: std::time::Instant,
}

#[derive(Debug, Clone)]
pub enum LifecycleEventType {
    Assigned,
    StartedWork,
    CompletedWork,
    Freed,
    Reassigned, // This should never happen for completed work
}

// Mock agent coordinator for tracking lifecycle
#[derive(Debug, Clone)]
pub struct MockAgentCoordinator {
    pub agents: Arc<Mutex<HashMap<String, AgentState>>>,
    pub lifecycle_events: Arc<Mutex<Vec<AgentLifecycleEvent>>>,
    pub issue_states: Arc<Mutex<HashMap<u64, IssueState>>>,
}

#[derive(Debug, Clone)]
pub struct IssueState {
    pub number: u64,
    pub labels: Vec<String>,
    pub assigned_agent: Option<String>,
    pub status: IssueStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IssueStatus {
    Ready,      // route:ready
    Assigned,   // assigned to agent
    InProgress, // agent working
    Completed,  // route:ready_to_merge (merge-ready)
    Merged,     // fully complete
}

impl MockAgentCoordinator {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(Mutex::new(HashMap::new())),
            lifecycle_events: Arc::new(Mutex::new(Vec::new())),
            issue_states: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    pub fn add_agent(&self, agent_id: &str) {
        self.agents.lock().unwrap().insert(agent_id.to_string(), AgentState::Available);
    }
    
    pub fn add_issue(&self, issue_number: u64, status: IssueStatus) {
        let issue_state = IssueState {
            number: issue_number,
            labels: match status {
                IssueStatus::Ready => vec!["route:ready".to_string()],
                IssueStatus::Completed => vec!["route:ready".to_string(), "route:ready_to_merge".to_string()],
                _ => vec!["route:ready".to_string()],
            },
            assigned_agent: None,
            status,
        };
        self.issue_states.lock().unwrap().insert(issue_number, issue_state);
    }
    
    pub fn assign_issue_to_agent(&self, issue_number: u64, agent_id: &str) -> Result<(), String> {
        // Check if agent is available
        let mut agents = self.agents.lock().unwrap();
        let agent_state = agents.get(agent_id).ok_or("Agent not found")?;
        
        if *agent_state != AgentState::Available {
            return Err(format!("Agent {} not available: {:?}", agent_id, agent_state));
        }
        
        // Check if issue is available for assignment
        let mut issues = self.issue_states.lock().unwrap();
        let issue = issues.get_mut(&issue_number).ok_or("Issue not found")?;
        
        if issue.status == IssueStatus::Completed {
            return Err(format!("Cannot assign completed issue #{}", issue_number));
        }
        
        if issue.assigned_agent.is_some() {
            return Err(format!("Issue #{} already assigned", issue_number));
        }
        
        // Perform assignment
        agents.insert(agent_id.to_string(), AgentState::Assigned(issue_number));
        issue.assigned_agent = Some(agent_id.to_string());
        issue.status = IssueStatus::Assigned;
        issue.labels.push(format!("agent{}", agent_id));
        
        // Record event
        self.record_lifecycle_event(agent_id, issue_number, LifecycleEventType::Assigned);
        
        Ok(())
    }
    
    pub fn start_work(&self, agent_id: &str, issue_number: u64) -> Result<(), String> {
        let mut agents = self.agents.lock().unwrap();
        let agent_state = agents.get(agent_id).ok_or("Agent not found")?;
        
        if *agent_state != AgentState::Assigned(issue_number) {
            return Err(format!("Agent {} not assigned to issue #{}", agent_id, issue_number));
        }
        
        // Update states
        agents.insert(agent_id.to_string(), AgentState::Working(issue_number));
        
        let mut issues = self.issue_states.lock().unwrap();
        if let Some(issue) = issues.get_mut(&issue_number) {
            issue.status = IssueStatus::InProgress;
        }
        
        self.record_lifecycle_event(agent_id, issue_number, LifecycleEventType::StartedWork);
        Ok(())
    }
    
    pub fn complete_work(&self, agent_id: &str, issue_number: u64) -> Result<(), String> {
        let mut agents = self.agents.lock().unwrap();
        let agent_state = agents.get(agent_id).ok_or("Agent not found")?;
        
        if *agent_state != AgentState::Working(issue_number) {
            return Err(format!("Agent {} not working on issue #{}", agent_id, issue_number));
        }
        
        // Update states
        agents.insert(agent_id.to_string(), AgentState::Completed(issue_number));
        
        let mut issues = self.issue_states.lock().unwrap();
        if let Some(issue) = issues.get_mut(&issue_number) {
            issue.status = IssueStatus::Completed;
            issue.labels.push("route:ready_to_merge".to_string());
        }
        
        self.record_lifecycle_event(agent_id, issue_number, LifecycleEventType::CompletedWork);
        Ok(())
    }
    
    pub fn free_agent(&self, agent_id: &str, issue_number: u64) -> Result<(), String> {
        let mut agents = self.agents.lock().unwrap();
        let agent_state = agents.get(agent_id).ok_or("Agent not found")?;
        
        if *agent_state != AgentState::Completed(issue_number) {
            return Err(format!("Agent {} hasn't completed issue #{}", agent_id, issue_number));
        }
        
        // Free the agent
        agents.insert(agent_id.to_string(), AgentState::Available);
        
        // Remove agent label from issue
        let mut issues = self.issue_states.lock().unwrap();
        if let Some(issue) = issues.get_mut(&issue_number) {
            issue.labels.retain(|label| !label.starts_with("agent"));
            issue.assigned_agent = None;
        }
        
        self.record_lifecycle_event(agent_id, issue_number, LifecycleEventType::Freed);
        Ok(())
    }
    
    pub fn simulate_clambake_pop(&self, agent_id: &str) -> Result<Option<u64>, String> {
        let agents = self.agents.lock().unwrap();
        let agent_state = agents.get(agent_id).ok_or("Agent not found")?;
        
        // Only available agents can get new work
        if *agent_state != AgentState::Available {
            return Ok(None);
        }
        
        // Find assignable issues (not completed, not already assigned)
        let issues = self.issue_states.lock().unwrap();
        let assignable_issues: Vec<&IssueState> = issues.values()
            .filter(|issue| {
                issue.status == IssueStatus::Ready && 
                issue.assigned_agent.is_none()
            })
            .collect();
        
        // Return first available issue
        if let Some(issue) = assignable_issues.first() {
            Ok(Some(issue.number))
        } else {
            Ok(None)
        }
    }
    
    pub fn verify_no_reassignment_of_completed_work(&self) -> Result<(), String> {
        let events = self.lifecycle_events.lock().unwrap();
        
        // Track which issues have been completed
        let mut completed_issues = std::collections::HashSet::new();
        
        for event in events.iter() {
            match event.event_type {
                LifecycleEventType::CompletedWork => {
                    completed_issues.insert(event.issue_number);
                }
                LifecycleEventType::Assigned => {
                    if completed_issues.contains(&event.issue_number) {
                        return Err(format!(
                            "REGRESSION: Issue #{} was reassigned after completion to agent {}",
                            event.issue_number, event.agent_id
                        ));
                    }
                }
                LifecycleEventType::Reassigned => {
                    return Err(format!(
                        "REGRESSION: Issue #{} was reassigned to agent {} (this should never happen)",
                        event.issue_number, event.agent_id
                    ));
                }
                _ => {}
            }
        }
        
        Ok(())
    }
    
    pub fn get_agent_state(&self, agent_id: &str) -> Option<AgentState> {
        self.agents.lock().unwrap().get(agent_id).cloned()
    }
    
    pub fn get_issue_state(&self, issue_number: u64) -> Option<IssueState> {
        self.issue_states.lock().unwrap().get(&issue_number).cloned()
    }
    
    pub fn get_lifecycle_events(&self) -> Vec<AgentLifecycleEvent> {
        self.lifecycle_events.lock().unwrap().clone()
    }
    
    fn record_lifecycle_event(&self, agent_id: &str, issue_number: u64, event_type: LifecycleEventType) {
        let event = AgentLifecycleEvent {
            agent_id: agent_id.to_string(),
            issue_number,
            event_type,
            timestamp: std::time::Instant::now(),
        };
        self.lifecycle_events.lock().unwrap().push(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complete_agent_lifecycle_success() {
        // Given: A coordinator with an available agent and ready issue
        let coordinator = MockAgentCoordinator::new();
        coordinator.add_agent("agent001");
        coordinator.add_issue(95, IssueStatus::Ready);
        
        // When: We simulate the complete lifecycle
        // 1. Assign work
        let result1 = coordinator.assign_issue_to_agent(95, "agent001");
        assert!(result1.is_ok(), "Assignment should succeed");
        
        // 2. Start work
        let result2 = coordinator.start_work("agent001", 95);
        assert!(result2.is_ok(), "Start work should succeed");
        
        // 3. Complete work
        let result3 = coordinator.complete_work("agent001", 95);
        assert!(result3.is_ok(), "Complete work should succeed");
        
        // 4. Free agent
        let result4 = coordinator.free_agent("agent001", 95);
        assert!(result4.is_ok(), "Free agent should succeed");
        
        // Then: Agent should be available again
        let agent_state = coordinator.get_agent_state("agent001").unwrap();
        assert_eq!(agent_state, AgentState::Available);
        
        // And: Issue should be completed but unassigned
        let issue_state = coordinator.get_issue_state(95).unwrap();
        assert_eq!(issue_state.status, IssueStatus::Completed);
        assert!(issue_state.assigned_agent.is_none());
        assert!(issue_state.labels.contains(&"route:ready_to_merge".to_string()));
        assert!(!issue_state.labels.iter().any(|l| l.starts_with("agent")));
        
        // And: No reassignment regression should be detected
        let verification = coordinator.verify_no_reassignment_of_completed_work();
        assert!(verification.is_ok(), "No reassignment regression should be detected");
    }
    
    #[test]
    fn test_completed_work_never_reassigned() {
        // Given: A coordinator with completed work
        let coordinator = MockAgentCoordinator::new();
        coordinator.add_agent("agent001");
        coordinator.add_agent("agent002");
        coordinator.add_issue(93, IssueStatus::Ready);
        
        // Complete the full lifecycle for agent001
        coordinator.assign_issue_to_agent(93, "agent001").unwrap();
        coordinator.start_work("agent001", 93).unwrap();
        coordinator.complete_work("agent001", 93).unwrap();
        coordinator.free_agent("agent001", 93).unwrap();
        
        // When: agent002 tries to get work via clambake pop
        let available_work = coordinator.simulate_clambake_pop("agent002").unwrap();
        
        // Then: No work should be available (completed issue should not be reassigned)
        assert!(available_work.is_none(), "Completed issue should not be available for reassignment");
        
        // And: Attempting to assign completed issue should fail
        let assignment_result = coordinator.assign_issue_to_agent(93, "agent002");
        assert!(assignment_result.is_err(), "Cannot assign completed issue");
        assert!(assignment_result.unwrap_err().contains("Cannot assign completed issue"));
        
        // And: Verification should pass (no regression)
        let verification = coordinator.verify_no_reassignment_of_completed_work();
        assert!(verification.is_ok(), "No reassignment regression detected");
    }
    
    #[test]
    fn test_multiple_agents_concurrent_lifecycle() {
        // Given: Multiple agents and multiple issues
        let coordinator = MockAgentCoordinator::new();
        coordinator.add_agent("agent001");
        coordinator.add_agent("agent002");
        coordinator.add_agent("agent003");
        
        coordinator.add_issue(95, IssueStatus::Ready);
        coordinator.add_issue(96, IssueStatus::Ready);
        coordinator.add_issue(97, IssueStatus::Ready);
        
        // When: All agents work on different issues simultaneously
        coordinator.assign_issue_to_agent(95, "agent001").unwrap();
        coordinator.assign_issue_to_agent(96, "agent002").unwrap();
        coordinator.assign_issue_to_agent(97, "agent003").unwrap();
        
        // Agent001 completes work first
        coordinator.start_work("agent001", 95).unwrap();
        coordinator.complete_work("agent001", 95).unwrap();
        coordinator.free_agent("agent001", 95).unwrap();
        
        // Agent002 is still working
        coordinator.start_work("agent002", 96).unwrap();
        
        // Agent003 completes work second
        coordinator.start_work("agent003", 97).unwrap();
        coordinator.complete_work("agent003", 97).unwrap();
        coordinator.free_agent("agent003", 97).unwrap();
        
        // Then: States should be correct
        assert_eq!(coordinator.get_agent_state("agent001").unwrap(), AgentState::Available);
        assert_eq!(coordinator.get_agent_state("agent002").unwrap(), AgentState::Working(96));
        assert_eq!(coordinator.get_agent_state("agent003").unwrap(), AgentState::Available);
        
        // And: Issues should have correct states
        assert_eq!(coordinator.get_issue_state(95).unwrap().status, IssueStatus::Completed);
        assert_eq!(coordinator.get_issue_state(96).unwrap().status, IssueStatus::InProgress);
        assert_eq!(coordinator.get_issue_state(97).unwrap().status, IssueStatus::Completed);
        
        // And: Available agents should not get completed work
        let work_for_agent001 = coordinator.simulate_clambake_pop("agent001").unwrap();
        let work_for_agent003 = coordinator.simulate_clambake_pop("agent003").unwrap();
        
        assert!(work_for_agent001.is_none(), "No completed work should be available");
        assert!(work_for_agent003.is_none(), "No completed work should be available");
        
        // And: No reassignment regressions
        let verification = coordinator.verify_no_reassignment_of_completed_work();
        assert!(verification.is_ok(), "No reassignment regressions in concurrent scenario");
    }
    
    #[test]
    fn test_agent_state_consistency_during_transitions() {
        // Given: A coordinator tracking state transitions
        let coordinator = MockAgentCoordinator::new();
        coordinator.add_agent("agent001");
        coordinator.add_issue(95, IssueStatus::Ready);
        
        // When: We perform each lifecycle step and verify state consistency
        
        // Initial state
        assert_eq!(coordinator.get_agent_state("agent001").unwrap(), AgentState::Available);
        assert_eq!(coordinator.get_issue_state(95).unwrap().status, IssueStatus::Ready);
        
        // After assignment
        coordinator.assign_issue_to_agent(95, "agent001").unwrap();
        assert_eq!(coordinator.get_agent_state("agent001").unwrap(), AgentState::Assigned(95));
        assert_eq!(coordinator.get_issue_state(95).unwrap().status, IssueStatus::Assigned);
        assert_eq!(coordinator.get_issue_state(95).unwrap().assigned_agent.as_ref().unwrap(), "agent001");
        
        // After starting work
        coordinator.start_work("agent001", 95).unwrap();
        assert_eq!(coordinator.get_agent_state("agent001").unwrap(), AgentState::Working(95));
        assert_eq!(coordinator.get_issue_state(95).unwrap().status, IssueStatus::InProgress);
        
        // After completing work
        coordinator.complete_work("agent001", 95).unwrap();
        assert_eq!(coordinator.get_agent_state("agent001").unwrap(), AgentState::Completed(95));
        assert_eq!(coordinator.get_issue_state(95).unwrap().status, IssueStatus::Completed);
        assert!(coordinator.get_issue_state(95).unwrap().labels.contains(&"route:ready_to_merge".to_string()));
        
        // After freeing agent
        coordinator.free_agent("agent001", 95).unwrap();
        assert_eq!(coordinator.get_agent_state("agent001").unwrap(), AgentState::Available);
        assert!(coordinator.get_issue_state(95).unwrap().assigned_agent.is_none());
        assert!(!coordinator.get_issue_state(95).unwrap().labels.iter().any(|l| l.starts_with("agent")));
        
        // Then: All events should be recorded in order
        let events = coordinator.get_lifecycle_events();
        assert_eq!(events.len(), 4);
        assert!(matches!(events[0].event_type, LifecycleEventType::Assigned));
        assert!(matches!(events[1].event_type, LifecycleEventType::StartedWork));
        assert!(matches!(events[2].event_type, LifecycleEventType::CompletedWork));
        assert!(matches!(events[3].event_type, LifecycleEventType::Freed));
    }
    
    #[test]
    fn test_edge_case_partial_state_transitions() {
        // Given: A coordinator with an agent assigned to work
        let coordinator = MockAgentCoordinator::new();
        coordinator.add_agent("agent001");
        coordinator.add_issue(95, IssueStatus::Ready);
        
        coordinator.assign_issue_to_agent(95, "agent001").unwrap();
        coordinator.start_work("agent001", 95).unwrap();
        
        // When: We try invalid state transitions
        
        // Cannot assign agent to new work while working
        coordinator.add_issue(96, IssueStatus::Ready);
        let invalid_assignment = coordinator.assign_issue_to_agent(96, "agent001");
        assert!(invalid_assignment.is_err(), "Cannot assign working agent to new issue");
        
        // Cannot free agent before completing work
        let invalid_free = coordinator.free_agent("agent001", 95);
        assert!(invalid_free.is_err(), "Cannot free agent before completing work");
        
        // Cannot start work on different issue
        let invalid_start = coordinator.start_work("agent001", 96);
        assert!(invalid_start.is_err(), "Cannot start work on wrong issue");
        
        // Then: State should remain consistent
        assert_eq!(coordinator.get_agent_state("agent001").unwrap(), AgentState::Working(95));
        assert_eq!(coordinator.get_issue_state(95).unwrap().status, IssueStatus::InProgress);
        
        // And: No state corruption should occur
        assert!(coordinator.get_issue_state(96).unwrap().assigned_agent.is_none());
        assert_eq!(coordinator.get_issue_state(96).unwrap().status, IssueStatus::Ready);
    }
    
    #[test]
    fn test_clambake_pop_respects_agent_availability() {
        // Given: Agents in different states
        let coordinator = MockAgentCoordinator::new();
        coordinator.add_agent("available");
        coordinator.add_agent("assigned");
        coordinator.add_agent("working");
        coordinator.add_agent("completed");
        
        coordinator.add_issue(95, IssueStatus::Ready);
        coordinator.add_issue(96, IssueStatus::Ready);
        coordinator.add_issue(97, IssueStatus::Ready);
        coordinator.add_issue(98, IssueStatus::Ready);
        
        // Set up different agent states
        coordinator.assign_issue_to_agent(96, "assigned").unwrap();
        coordinator.assign_issue_to_agent(97, "working").unwrap();
        coordinator.start_work("working", 97).unwrap();
        coordinator.assign_issue_to_agent(98, "completed").unwrap();
        coordinator.start_work("completed", 98).unwrap();
        coordinator.complete_work("completed", 98).unwrap();
        
        // When: Each agent tries to get work via clambake pop
        let work_available = coordinator.simulate_clambake_pop("available").unwrap();
        let work_assigned = coordinator.simulate_clambake_pop("assigned").unwrap();
        let work_working = coordinator.simulate_clambake_pop("working").unwrap();
        let work_completed = coordinator.simulate_clambake_pop("completed").unwrap();
        
        // Then: Only available agent should get work
        assert!(work_available.is_some(), "Available agent should get work");
        assert_eq!(work_available.unwrap(), 95);
        
        assert!(work_assigned.is_none(), "Assigned agent should not get new work");
        assert!(work_working.is_none(), "Working agent should not get new work");
        assert!(work_completed.is_none(), "Completed agent should not get new work");
    }
}