//! Integration tests for workflow state transitions
//!
//! These tests verify end-to-end workflow state transitions including
//! the bundling workflow and state machine integration to prevent
//! regressions in complex multi-step operations.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

mod fixtures;

// Workflow states for integration testing
#[derive(Debug, Clone, PartialEq)]
pub enum WorkflowState {
    Ready,          // issue ready for assignment
    Assigned,       // assigned to agent
    Working,        // agent working
    Completed,      // work completed (route:ready_to_merge)
    ReviewReady,    // ready for CodeRabbit review
    Reviewed,       // CodeRabbit review complete
    MergeReady,     // ready for merge
    Merged,         // fully integrated
    Failed(String), // workflow failed
}

#[derive(Debug, Clone)]
pub struct WorkflowTransition {
    pub from_state: WorkflowState,
    pub to_state: WorkflowState,
    pub trigger: TransitionTrigger,
    pub timestamp: Instant,
    pub duration: Duration,
}

#[derive(Debug, Clone)]
pub enum TransitionTrigger {
    AgentAssignment,
    WorkStarted,
    WorkCompleted,
    LandCommand,
    CodeRabbitReview,
    MergeCommand,
    FailureDetected(String),
}

// Mock workflow coordinator for integration testing
#[derive(Debug)]
pub struct MockWorkflowCoordinator {
    pub issue_workflows: Arc<Mutex<HashMap<u64, WorkflowState>>>,
    pub transitions: Arc<Mutex<Vec<WorkflowTransition>>>,
    pub bundling_enabled: bool,
    pub rate_limit_delays: Arc<Mutex<HashMap<String, Instant>>>,
}

impl MockWorkflowCoordinator {
    pub fn new(bundling_enabled: bool) -> Self {
        Self {
            issue_workflows: Arc::new(Mutex::new(HashMap::new())),
            transitions: Arc::new(Mutex::new(Vec::new())),
            bundling_enabled,
            rate_limit_delays: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add_issue(&self, issue_number: u64, initial_state: WorkflowState) {
        self.issue_workflows
            .lock()
            .unwrap()
            .insert(issue_number, initial_state);
    }

    pub fn transition_issue(
        &self,
        issue_number: u64,
        trigger: TransitionTrigger,
    ) -> Result<WorkflowState, String> {
        let start_time = Instant::now();

        let mut workflows = self.issue_workflows.lock().unwrap();
        let current_state = workflows
            .get(&issue_number)
            .ok_or_else(|| format!("Issue #{issue_number} not found"))?
            .clone();

        // Determine new state based on current state and trigger
        let new_state = self.compute_next_state(&current_state, &trigger)?;

        // Apply rate limiting if bundling is enabled
        if self.bundling_enabled {
            self.apply_rate_limiting(&trigger)?;
        }

        // Record transition
        let transition = WorkflowTransition {
            from_state: current_state.clone(),
            to_state: new_state.clone(),
            trigger,
            timestamp: start_time,
            duration: start_time.elapsed(),
        };

        // Update state
        workflows.insert(issue_number, new_state.clone());
        self.transitions.lock().unwrap().push(transition);

        Ok(new_state)
    }

    pub fn get_issue_state(&self, issue_number: u64) -> Option<WorkflowState> {
        self.issue_workflows
            .lock()
            .unwrap()
            .get(&issue_number)
            .cloned()
    }

    pub fn get_transitions(&self) -> Vec<WorkflowTransition> {
        self.transitions.lock().unwrap().clone()
    }

    pub fn simulate_clambake_pop(&self) -> Result<Vec<u64>, String> {
        let workflows = self.issue_workflows.lock().unwrap();
        let available_issues: Vec<u64> = workflows
            .iter()
            .filter(|(_, state)| **state == WorkflowState::Ready)
            .map(|(issue_num, _)| *issue_num)
            .collect();

        if self.bundling_enabled && available_issues.len() > 1 {
            // Bundle multiple issues for efficiency
            Ok(available_issues.into_iter().take(3).collect())
        } else {
            // Single issue assignment
            Ok(available_issues.into_iter().take(1).collect())
        }
    }

    pub fn simulate_clambake_land(&self, issue_number: u64) -> Result<(), String> {
        let current_state = self
            .get_issue_state(issue_number)
            .ok_or_else(|| format!("Issue #{issue_number} not found"))?;

        match current_state {
            WorkflowState::Working => {
                // Complete work and transition to land state
                self.transition_issue(issue_number, TransitionTrigger::WorkCompleted)?;
                self.transition_issue(issue_number, TransitionTrigger::LandCommand)?;
                Ok(())
            }
            WorkflowState::Completed => {
                // Already completed, just land
                self.transition_issue(issue_number, TransitionTrigger::LandCommand)?;
                Ok(())
            }
            _ => Err(format!(
                "Cannot land issue #{issue_number} in state {current_state:?}"
            )),
        }
    }

    pub fn verify_workflow_consistency(&self) -> Result<(), String> {
        let workflows = self.issue_workflows.lock().unwrap();
        let transitions = self.transitions.lock().unwrap();

        // Verify no invalid state transitions occurred
        for transition in transitions.iter() {
            if !self.is_valid_transition(
                &transition.from_state,
                &transition.to_state,
                &transition.trigger,
            ) {
                return Err(format!(
                    "Invalid transition: {:?} -> {:?} via {:?}",
                    transition.from_state, transition.to_state, transition.trigger
                ));
            }
        }

        // Verify no orphaned states
        for (issue_num, state) in workflows.iter() {
            if let WorkflowState::Failed(msg) = state {
                // Failed states should have a corresponding failure transition
                let has_failure = transitions
                    .iter()
                    .any(|t| matches!(t.trigger, TransitionTrigger::FailureDetected(_)));
                if !has_failure {
                    return Err(format!(
                        "Issue #{issue_num} in failed state without failure transition"
                    ));
                }
            }
        }

        Ok(())
    }

    fn compute_next_state(
        &self,
        current: &WorkflowState,
        trigger: &TransitionTrigger,
    ) -> Result<WorkflowState, String> {
        match (current, trigger) {
            (WorkflowState::Ready, TransitionTrigger::AgentAssignment) => {
                Ok(WorkflowState::Assigned)
            }
            (WorkflowState::Assigned, TransitionTrigger::WorkStarted) => Ok(WorkflowState::Working),
            (WorkflowState::Working, TransitionTrigger::WorkCompleted) => {
                Ok(WorkflowState::Completed)
            }
            (WorkflowState::Completed, TransitionTrigger::LandCommand) => {
                Ok(WorkflowState::ReviewReady)
            }
            (WorkflowState::ReviewReady, TransitionTrigger::CodeRabbitReview) => {
                Ok(WorkflowState::Reviewed)
            }
            (WorkflowState::Reviewed, TransitionTrigger::MergeCommand) => {
                Ok(WorkflowState::MergeReady)
            }
            (WorkflowState::MergeReady, TransitionTrigger::MergeCommand) => {
                Ok(WorkflowState::Merged)
            }
            (_, TransitionTrigger::FailureDetected(msg)) => Ok(WorkflowState::Failed(msg.clone())),
            _ => Err(format!(
                "Invalid transition: {current:?} with trigger {trigger:?}"
            )),
        }
    }

    fn is_valid_transition(
        &self,
        from: &WorkflowState,
        to: &WorkflowState,
        trigger: &TransitionTrigger,
    ) -> bool {
        self.compute_next_state(from, trigger)
            .map(|expected| expected == *to)
            .unwrap_or(false)
    }

    fn apply_rate_limiting(&self, trigger: &TransitionTrigger) -> Result<(), String> {
        let api_category = match trigger {
            TransitionTrigger::AgentAssignment | TransitionTrigger::LandCommand => "github_api",
            TransitionTrigger::CodeRabbitReview => "coderabbit_api",
            _ => return Ok(()),
        };

        let mut delays = self.rate_limit_delays.lock().unwrap();
        if let Some(last_call) = delays.get(api_category) {
            let elapsed = last_call.elapsed();
            if elapsed < Duration::from_millis(100) {
                // Simulate rate limiting delay
                std::thread::sleep(Duration::from_millis(100) - elapsed);
            }
        }

        delays.insert(api_category.to_string(), Instant::now());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complete_workflow_without_bundling() {
        // Given: A workflow coordinator without bundling
        let coordinator = MockWorkflowCoordinator::new(false);
        coordinator.add_issue(95, WorkflowState::Ready);

        // When: We execute the complete workflow
        let result1 = coordinator.transition_issue(95, TransitionTrigger::AgentAssignment);
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap(), WorkflowState::Assigned);

        let result2 = coordinator.transition_issue(95, TransitionTrigger::WorkStarted);
        assert!(result2.is_ok());
        assert_eq!(result2.unwrap(), WorkflowState::Working);

        let result3 = coordinator.transition_issue(95, TransitionTrigger::WorkCompleted);
        assert!(result3.is_ok());
        assert_eq!(result3.unwrap(), WorkflowState::Completed);

        let result4 = coordinator.transition_issue(95, TransitionTrigger::LandCommand);
        assert!(result4.is_ok());
        assert_eq!(result4.unwrap(), WorkflowState::ReviewReady);

        let result5 = coordinator.transition_issue(95, TransitionTrigger::CodeRabbitReview);
        assert!(result5.is_ok());
        assert_eq!(result5.unwrap(), WorkflowState::Reviewed);

        let result6 = coordinator.transition_issue(95, TransitionTrigger::MergeCommand);
        assert!(result6.is_ok());
        assert_eq!(result6.unwrap(), WorkflowState::MergeReady);

        let result7 = coordinator.transition_issue(95, TransitionTrigger::MergeCommand);
        assert!(result7.is_ok());
        assert_eq!(result7.unwrap(), WorkflowState::Merged);

        // Then: Workflow should be consistent
        let verification = coordinator.verify_workflow_consistency();
        assert!(verification.is_ok(), "Workflow should be consistent");

        // And: All transitions should be recorded
        let transitions = coordinator.get_transitions();
        assert_eq!(transitions.len(), 7);
    }

    #[test]
    fn test_bundled_workflow_for_rate_limiting() {
        // Given: A workflow coordinator with bundling enabled
        let coordinator = MockWorkflowCoordinator::new(true);
        coordinator.add_issue(95, WorkflowState::Ready);
        coordinator.add_issue(96, WorkflowState::Ready);
        coordinator.add_issue(97, WorkflowState::Ready);
        coordinator.add_issue(98, WorkflowState::Ready);

        // When: We simulate clambake pop with bundling
        let available_work = coordinator.simulate_clambake_pop().unwrap();

        // Then: Multiple issues should be bundled
        assert!(
            available_work.len() > 1 && available_work.len() <= 3,
            "Should bundle multiple issues for efficiency"
        );

        // When: We process bundled transitions with rate limiting
        let start_time = Instant::now();

        for issue_num in &available_work {
            coordinator
                .transition_issue(*issue_num, TransitionTrigger::AgentAssignment)
                .unwrap();
        }

        let elapsed = start_time.elapsed();

        // Then: Rate limiting should add delays between API calls
        let expected_min_duration = Duration::from_millis(100 * (available_work.len() - 1) as u64);
        assert!(
            elapsed >= expected_min_duration,
            "Rate limiting should enforce delays between API calls"
        );
    }

    #[test]
    fn test_clambake_land_integration() {
        // Given: An issue in working state
        let coordinator = MockWorkflowCoordinator::new(false);
        coordinator.add_issue(95, WorkflowState::Ready);

        // Set up issue in working state
        coordinator
            .transition_issue(95, TransitionTrigger::AgentAssignment)
            .unwrap();
        coordinator
            .transition_issue(95, TransitionTrigger::WorkStarted)
            .unwrap();

        // When: We execute clambake land
        let land_result = coordinator.simulate_clambake_land(95);

        // Then: Issue should transition to review-ready state
        assert!(land_result.is_ok(), "Clambake land should succeed");
        assert_eq!(
            coordinator.get_issue_state(95).unwrap(),
            WorkflowState::ReviewReady
        );

        // And: Transitions should include both completion and land
        let transitions = coordinator.get_transitions();
        let completion_transitions: Vec<_> = transitions
            .iter()
            .filter(|t| {
                matches!(
                    t.trigger,
                    TransitionTrigger::WorkCompleted | TransitionTrigger::LandCommand
                )
            })
            .collect();
        assert_eq!(
            completion_transitions.len(),
            2,
            "Should have both completion and land transitions"
        );
    }

    #[test]
    fn test_invalid_workflow_transitions() {
        // Given: A workflow coordinator
        let coordinator = MockWorkflowCoordinator::new(false);
        coordinator.add_issue(95, WorkflowState::Ready);

        // When: We attempt invalid transitions

        // Cannot start work before assignment
        let invalid1 = coordinator.transition_issue(95, TransitionTrigger::WorkStarted);
        assert!(invalid1.is_err(), "Cannot start work before assignment");

        // Cannot complete work before starting
        let invalid2 = coordinator.transition_issue(95, TransitionTrigger::WorkCompleted);
        assert!(invalid2.is_err(), "Cannot complete work before starting");

        // Cannot merge before review
        let invalid3 = coordinator.transition_issue(95, TransitionTrigger::MergeCommand);
        assert!(invalid3.is_err(), "Cannot merge before review");

        // Then: State should remain unchanged
        assert_eq!(
            coordinator.get_issue_state(95).unwrap(),
            WorkflowState::Ready
        );
    }

    #[test]
    fn test_workflow_failure_handling() {
        // Given: An issue in working state
        let coordinator = MockWorkflowCoordinator::new(false);
        coordinator.add_issue(95, WorkflowState::Ready);

        coordinator
            .transition_issue(95, TransitionTrigger::AgentAssignment)
            .unwrap();
        coordinator
            .transition_issue(95, TransitionTrigger::WorkStarted)
            .unwrap();

        // When: A failure occurs
        let failure_trigger = TransitionTrigger::FailureDetected("GitHub API timeout".to_string());
        let failure_result = coordinator.transition_issue(95, failure_trigger);

        // Then: Issue should transition to failed state
        assert!(failure_result.is_ok());
        match coordinator.get_issue_state(95).unwrap() {
            WorkflowState::Failed(msg) => {
                assert_eq!(msg, "GitHub API timeout");
            }
            _ => panic!("Expected failed state"),
        }

        // And: Workflow consistency should still be maintained
        let verification = coordinator.verify_workflow_consistency();
        assert!(
            verification.is_ok(),
            "Failed workflow should still be consistent"
        );
    }

    #[test]
    fn test_multiple_issues_different_workflow_phases() {
        // Given: Multiple issues in different phases
        let coordinator = MockWorkflowCoordinator::new(false);
        coordinator.add_issue(95, WorkflowState::Ready); // New issue
        coordinator.add_issue(96, WorkflowState::Ready); // Will be assigned
        coordinator.add_issue(97, WorkflowState::Ready); // Will be in progress
        coordinator.add_issue(98, WorkflowState::Ready); // Will be completed

        // When: Issues progress through different phases

        // Issue 96: Assignment
        coordinator
            .transition_issue(96, TransitionTrigger::AgentAssignment)
            .unwrap();

        // Issue 97: Working
        coordinator
            .transition_issue(97, TransitionTrigger::AgentAssignment)
            .unwrap();
        coordinator
            .transition_issue(97, TransitionTrigger::WorkStarted)
            .unwrap();

        // Issue 98: Completed and ready for review
        coordinator
            .transition_issue(98, TransitionTrigger::AgentAssignment)
            .unwrap();
        coordinator
            .transition_issue(98, TransitionTrigger::WorkStarted)
            .unwrap();
        coordinator
            .transition_issue(98, TransitionTrigger::WorkCompleted)
            .unwrap();
        coordinator
            .transition_issue(98, TransitionTrigger::LandCommand)
            .unwrap();

        // Then: Each issue should be in the correct state
        assert_eq!(
            coordinator.get_issue_state(95).unwrap(),
            WorkflowState::Ready
        );
        assert_eq!(
            coordinator.get_issue_state(96).unwrap(),
            WorkflowState::Assigned
        );
        assert_eq!(
            coordinator.get_issue_state(97).unwrap(),
            WorkflowState::Working
        );
        assert_eq!(
            coordinator.get_issue_state(98).unwrap(),
            WorkflowState::ReviewReady
        );

        // And: clambake pop should only return ready issues
        let available_work = coordinator.simulate_clambake_pop().unwrap();
        assert_eq!(
            available_work,
            vec![95],
            "Only issue 95 should be available for assignment"
        );

        // And: Workflow should be consistent across all issues
        let verification = coordinator.verify_workflow_consistency();
        assert!(
            verification.is_ok(),
            "Multi-issue workflow should be consistent"
        );
    }

    #[test]
    fn test_coderabbit_review_integration() {
        // Given: An issue ready for review
        let coordinator = MockWorkflowCoordinator::new(false);
        coordinator.add_issue(95, WorkflowState::Ready);

        // Progress to review-ready state
        coordinator
            .transition_issue(95, TransitionTrigger::AgentAssignment)
            .unwrap();
        coordinator
            .transition_issue(95, TransitionTrigger::WorkStarted)
            .unwrap();
        coordinator
            .transition_issue(95, TransitionTrigger::WorkCompleted)
            .unwrap();
        coordinator
            .transition_issue(95, TransitionTrigger::LandCommand)
            .unwrap();

        assert_eq!(
            coordinator.get_issue_state(95).unwrap(),
            WorkflowState::ReviewReady
        );

        // When: CodeRabbit review is triggered
        let review_result = coordinator.transition_issue(95, TransitionTrigger::CodeRabbitReview);

        // Then: Issue should be reviewed and ready for merge
        assert!(review_result.is_ok());
        assert_eq!(
            coordinator.get_issue_state(95).unwrap(),
            WorkflowState::Reviewed
        );

        // When: Merge is triggered
        let merge_result = coordinator.transition_issue(95, TransitionTrigger::MergeCommand);

        // Then: Issue should be merge-ready
        assert!(merge_result.is_ok());
        assert_eq!(
            coordinator.get_issue_state(95).unwrap(),
            WorkflowState::MergeReady
        );

        // Final merge
        let final_merge = coordinator.transition_issue(95, TransitionTrigger::MergeCommand);
        assert!(final_merge.is_ok());
        assert_eq!(
            coordinator.get_issue_state(95).unwrap(),
            WorkflowState::Merged
        );
    }

    #[test]
    fn test_state_machine_compatibility() {
        // Given: A workflow coordinator that simulates state machine behavior
        let coordinator = MockWorkflowCoordinator::new(false);
        coordinator.add_issue(95, WorkflowState::Ready);

        // When: We test state machine-like atomic transitions
        let mut successful_transitions = 0;
        let mut failed_transitions = 0;

        // Valid sequence
        if coordinator
            .transition_issue(95, TransitionTrigger::AgentAssignment)
            .is_ok()
        {
            successful_transitions += 1;
        } else {
            failed_transitions += 1;
        }

        if coordinator
            .transition_issue(95, TransitionTrigger::WorkStarted)
            .is_ok()
        {
            successful_transitions += 1;
        } else {
            failed_transitions += 1;
        }

        // Invalid transition attempt (should fail)
        if coordinator
            .transition_issue(95, TransitionTrigger::MergeCommand)
            .is_err()
        {
            // This is expected - counting as successful validation
            successful_transitions += 1;
        }

        // Continue valid sequence
        if coordinator
            .transition_issue(95, TransitionTrigger::WorkCompleted)
            .is_ok()
        {
            successful_transitions += 1;
        } else {
            failed_transitions += 1;
        }

        // Then: Atomic behavior should be preserved
        assert_eq!(failed_transitions, 0, "Valid transitions should not fail");
        assert!(
            successful_transitions >= 4,
            "Should have successful state validation"
        );

        // And: Final state should reflect valid transitions only
        assert_eq!(
            coordinator.get_issue_state(95).unwrap(),
            WorkflowState::Completed
        );
    }
}
