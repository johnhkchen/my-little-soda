//! Autonomous operation module for unattended development workflows
//!
//! This module provides the complete autonomous workflow state machine and error recovery
//! systems needed for true unattended operation of development agents.
//!
//! # Architecture
//!
//! The autonomous system consists of:
//! - **Workflow State Machine**: Models all possible states in the development workflow
//! - **Error Recovery System**: Handles autonomous recovery from various failure modes
//! - **Integration Layer**: Connects with existing agent coordination logic
//! - **Persistence Layer**: Maintains state across agent restarts
//!
//! # Key Features
//!
//! - Complete workflow modeling (assignment → work → review → integration)
//! - Autonomous error recovery with multiple strategies
//! - State persistence for restart resilience
//! - Comprehensive logging and monitoring
//! - Timeout and abandonment handling
//! - Integration with existing agent lifecycle

pub mod workflow_state_machine;
pub mod error_recovery;
pub mod integration;
pub mod persistence;
pub mod work_continuity;

pub use workflow_state_machine::{
    AutonomousWorkflowState, 
    AutonomousEvent, 
    AutonomousWorkflowMachine,
    AutonomousWorkflowError,
    AutonomousStatusReport,
    Issue, 
    AgentId, 
    WorkspaceState,
    BlockerType,
    PullRequest,
    ReviewFeedback,
    ConflictInfo,
    CIFailure,
    CompletedWork,
    AbandonmentReason,
    Priority,
    StateTransitionRecord,
};

pub use error_recovery::{
    AutonomousErrorRecovery,
    RecoveryStrategy,
    ErrorType,
    FixType,
    ConfidenceLevel,
    AutonomousRecoveryAttempt,
    AutonomousRecoveryReport,
};

pub use integration::{
    AutonomousIntegration,
    IntegrationCoordinator,
    AutonomousIntegrationFactory,
    IntegratedStatusReport,
    IntegrationConfig,
};

pub use persistence::{
    StatePersistenceManager,
    PersistentWorkflowState,
    PersistenceConfig,
    CheckpointReason,
    CheckpointInfo,
    PersistenceError,
};

pub use work_continuity::{
    WorkContinuityManager,
    PersistentAgentState,
    WorkContinuityConfig,
    WorkContinuityError,
    ResumeAction,
    ContinuityStatus,
    WorkspaceSnapshot,
    WorkProgress,
    PendingOperation,
    RecoveryContext,
};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};
use rand::Rng;

use crate::github::GitHubClient;
use crate::agents::recovery::AutomaticRecovery;

/// Main autonomous coordination system that orchestrates workflow state machine
/// and error recovery for unattended operation
pub struct AutonomousCoordinator {
    workflow_machine: Arc<RwLock<AutonomousWorkflowMachine>>,
    error_recovery: Arc<RwLock<AutonomousErrorRecovery>>,
    github_client: GitHubClient,
    agent_id: String,
    coordination_config: CoordinationConfig,
    is_running: Arc<RwLock<bool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationConfig {
    pub max_work_hours: u8,
    pub max_recovery_attempts: u8,
    pub recovery_timeout_minutes: u32,
    pub enable_aggressive_recovery: bool,
    pub enable_state_persistence: bool,
    pub monitoring_interval_minutes: u32,
}

impl Default for CoordinationConfig {
    fn default() -> Self {
        Self {
            max_work_hours: 8,
            max_recovery_attempts: 3,
            recovery_timeout_minutes: 30,
            enable_aggressive_recovery: false,
            enable_state_persistence: true,
            monitoring_interval_minutes: 5,
        }
    }
}

impl AutonomousCoordinator {
    /// Create new autonomous coordinator
    pub async fn new(
        github_client: GitHubClient,
        agent_id: String,
        recovery_client: Box<dyn AutomaticRecovery + Send + Sync>,
        config: CoordinationConfig,
    ) -> Result<Self, AutonomousWorkflowError> {
        let workflow_machine = AutonomousWorkflowMachine::new(config.max_work_hours)
            .with_github_client(github_client.clone());
        
        let error_recovery = AutonomousErrorRecovery::new(github_client.clone(), recovery_client)
            .with_max_attempts(config.max_recovery_attempts)
            .with_timeout(config.recovery_timeout_minutes)
            .with_aggressive_recovery(config.enable_aggressive_recovery);
        
        Ok(Self {
            workflow_machine: Arc::new(RwLock::new(workflow_machine)),
            error_recovery: Arc::new(RwLock::new(error_recovery)),
            github_client,
            agent_id,
            coordination_config: config,
            is_running: Arc::new(RwLock::new(false)),
        })
    }
    
    /// Start autonomous operation
    pub async fn start_autonomous_operation(&self) -> Result<(), AutonomousWorkflowError> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Err(AutonomousWorkflowError::InvalidTransition { 
                event: AutonomousEvent::StartWork 
            });
        }
        
        *is_running = true;
        drop(is_running);
        
        info!(
            agent_id = %self.agent_id,
            config = ?self.coordination_config,
            "Starting autonomous operation"
        );
        
        // Start monitoring task
        let monitoring_handle = self.spawn_monitoring_task().await;
        
        // Initialize workflow state machine
        let mut workflow = self.workflow_machine.write().await;
        workflow.handle_event(AutonomousEvent::AssignAgent {
            agent: AgentId(self.agent_id.clone()),
            workspace_ready: true,
        }).await?;
        
        drop(workflow);
        
        // Start main coordination loop
        let coordination_result = self.run_coordination_loop().await;
        
        // Stop monitoring
        monitoring_handle.abort();
        
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        
        coordination_result
    }
    
    /// Stop autonomous operation
    pub async fn stop_autonomous_operation(&self) -> Result<(), AutonomousWorkflowError> {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        
        info!(agent_id = %self.agent_id, "Stopping autonomous operation");
        
        // Gracefully transition to stopped state
        let mut workflow = self.workflow_machine.write().await;
        workflow.handle_event(AutonomousEvent::ForceAbandon {
            reason: AbandonmentReason::RequirementsChanged,
        }).await?;
        
        Ok(())
    }
    
    /// Main coordination loop for autonomous operation
    async fn run_coordination_loop(&self) -> Result<(), AutonomousWorkflowError> {
        while *self.is_running.read().await {
            let current_state = {
                let workflow = self.workflow_machine.read().await;
                workflow.current_state().cloned()
            };
            
            match current_state {
                Some(AutonomousWorkflowState::Assigned { .. }) => {
                    self.handle_assigned_state().await?;
                }
                Some(AutonomousWorkflowState::InProgress { .. }) => {
                    self.handle_in_progress_state().await?;
                }
                Some(AutonomousWorkflowState::Blocked { blocker, .. }) => {
                    self.handle_blocked_state(&blocker).await?;
                }
                Some(AutonomousWorkflowState::ReadyForReview { .. }) => {
                    self.handle_ready_for_review_state().await?;
                }
                Some(AutonomousWorkflowState::UnderReview { .. }) => {
                    self.handle_under_review_state().await?;
                }
                Some(AutonomousWorkflowState::ChangesRequested { .. }) => {
                    self.handle_changes_requested_state().await?;
                }
                Some(AutonomousWorkflowState::Approved { .. }) => {
                    self.handle_approved_state().await?;
                }
                Some(AutonomousWorkflowState::MergeConflict { conflicts, .. }) => {
                    self.handle_merge_conflict_state(&conflicts).await?;
                }
                Some(AutonomousWorkflowState::CIFailure { failures, .. }) => {
                    self.handle_ci_failure_state(&failures).await?;
                }
                Some(AutonomousWorkflowState::Merged { .. }) => {
                    info!(agent_id = %self.agent_id, "Work completed successfully");
                    break;
                }
                Some(AutonomousWorkflowState::Abandoned { reason, .. }) => {
                    warn!(
                        agent_id = %self.agent_id,
                        reason = ?reason,
                        "Work abandoned"
                    );
                    break;
                }
                None => {
                    // No work assigned, wait for assignment
                    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                }
                _ => {
                    // Handle other states
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
            
            // Check if we can continue autonomously
            let workflow = self.workflow_machine.read().await;
            if !workflow.can_continue_autonomously() {
                warn!(
                    agent_id = %self.agent_id,
                    "Cannot continue autonomously, stopping"
                );
                break;
            }
        }
        
        Ok(())
    }
    
    /// Handle assigned state - start work
    async fn handle_assigned_state(&self) -> Result<(), AutonomousWorkflowError> {
        info!(agent_id = %self.agent_id, "Starting work on assigned issue");
        
        let mut workflow = self.workflow_machine.write().await;
        workflow.handle_event(AutonomousEvent::StartWork).await?;
        
        Ok(())
    }
    
    /// Handle in-progress state - simulate work and progress
    async fn handle_in_progress_state(&self) -> Result<(), AutonomousWorkflowError> {
        info!(agent_id = %self.agent_id, "Making progress on work");
        
        // Simulate work progress
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        
        let mut workflow = self.workflow_machine.write().await;
        
        // Simulate encountering a blocker occasionally
        if rand::thread_rng().gen::<f64>() < 0.1 { // 10% chance
            workflow.handle_event(AutonomousEvent::EncounterBlocker {
                blocker: BlockerType::TestFailure {
                    test_name: "integration_test".to_string(),
                    error: "Connection timeout".to_string(),
                }
            }).await?;
        } else if rand::random::<f64>() < 0.3 { // 30% chance to complete
            workflow.handle_event(AutonomousEvent::CompleteWork).await?;
        } else {
            // Make progress
            workflow.handle_event(AutonomousEvent::MakeProgress {
                commits: 1,
                files_changed: 2,
            }).await?;
        }
        
        Ok(())
    }
    
    /// Handle blocked state - attempt recovery
    async fn handle_blocked_state(&self, blocker: &BlockerType) -> Result<(), AutonomousWorkflowError> {
        warn!(
            agent_id = %self.agent_id,
            blocker = ?blocker,
            "Handling blocked state"
        );
        
        // Determine error type from blocker
        let error_type = match blocker {
            BlockerType::TestFailure { test_name, error } => {
                ErrorType::TestFailure {
                    test_suite: "integration".to_string(),
                    failed_tests: vec![test_name.clone()],
                }
            }
            BlockerType::BuildFailure { error } => {
                ErrorType::BuildFailure {
                    stage: "compile".to_string(),
                    error: error.clone(),
                }
            }
            BlockerType::DependencyIssue { dependency, error } => {
                ErrorType::DependencyIssue {
                    dependency: dependency.clone(),
                    version_conflict: error.contains("version"),
                }
            }
            _ => {
                ErrorType::StateInconsistency {
                    expected_state: "working".to_string(),
                    actual_state: "blocked".to_string(),
                }
            }
        };
        
        // Attempt recovery
        let recovery_result = {
            let mut error_recovery = self.error_recovery.write().await;
            let strategy = error_recovery.determine_recovery_strategy(&error_type);
            let current_state = self.workflow_machine.read().await.current_state().cloned()
                .unwrap_or_else(|| AutonomousWorkflowState::Abandoned {
                    issue: Issue {
                        number: 0,
                        title: "Recovery context".to_string(),
                        body: "".to_string(),
                        labels: vec![],
                        priority: Priority::Medium,
                        estimated_hours: None,
                    },
                    reason: AbandonmentReason::CriticalFailure {
                        error: "No state available".to_string(),
                    }
                });
            
            error_recovery.execute_recovery_strategy(error_type, strategy, &current_state).await
        };
        
        match recovery_result {
            Ok(attempt) if attempt.success => {
                info!(
                    agent_id = %self.agent_id,
                    attempt_id = %attempt.attempt_id,
                    "Recovery successful, resolving blocker"
                );
                
                let mut workflow = self.workflow_machine.write().await;
                workflow.handle_event(AutonomousEvent::ResolveBlocker).await?;
            }
            Ok(attempt) => {
                warn!(
                    agent_id = %self.agent_id,
                    attempt_id = %attempt.attempt_id,
                    error = ?attempt.error_message,
                    "Recovery failed"
                );
                
                // Escalate or abandon based on strategy
                if matches!(attempt.strategy, RecoveryStrategy::Escalate { .. }) {
                    // Continue waiting for manual intervention
                    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                } else {
                    let mut workflow = self.workflow_machine.write().await;
                    workflow.handle_event(AutonomousEvent::ForceAbandon {
                        reason: AbandonmentReason::UnresolvableBlocker { blocker: blocker.clone() },
                    }).await?;
                }
            }
            Err(e) => {
                error!(
                    agent_id = %self.agent_id,
                    error = ?e,
                    "Recovery system error"
                );
                
                let mut workflow = self.workflow_machine.write().await;
                workflow.handle_event(AutonomousEvent::ForceAbandon {
                    reason: AbandonmentReason::CriticalFailure {
                        error: format!("Recovery system failed: {:?}", e),
                    },
                }).await?;
            }
        }
        
        Ok(())
    }
    
    /// Handle ready for review state - submit PR
    async fn handle_ready_for_review_state(&self) -> Result<(), AutonomousWorkflowError> {
        info!(agent_id = %self.agent_id, "Submitting work for review");
        
        let pr = PullRequest {
            number: 123, // Would be created via GitHub API
            title: "Autonomous work completion".to_string(),
            branch: format!("{}/autonomous-work", self.agent_id),
            commits: 5,
            files_changed: 3,
        };
        
        let mut workflow = self.workflow_machine.write().await;
        workflow.handle_event(AutonomousEvent::SubmitForReview { pr }).await?;
        
        Ok(())
    }
    
    /// Handle under review state - wait for feedback
    async fn handle_under_review_state(&self) -> Result<(), AutonomousWorkflowError> {
        info!(agent_id = %self.agent_id, "Waiting for review feedback");
        
        // Simulate waiting for review
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        
        // Simulate receiving feedback
        if rand::thread_rng().gen::<f64>() < 0.7 { // 70% chance of approval
            let mut workflow = self.workflow_machine.write().await;
            workflow.handle_event(AutonomousEvent::ApprovalReceived).await?;
        }
        
        Ok(())
    }
    
    /// Handle changes requested state - address feedback
    async fn handle_changes_requested_state(&self) -> Result<(), AutonomousWorkflowError> {
        info!(agent_id = %self.agent_id, "Addressing requested changes");
        
        // Simulate making changes
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        
        let mut workflow = self.workflow_machine.write().await;
        workflow.handle_event(AutonomousEvent::ApprovalReceived).await?;
        
        Ok(())
    }
    
    /// Handle approved state - proceed to merge
    async fn handle_approved_state(&self) -> Result<(), AutonomousWorkflowError> {
        info!(agent_id = %self.agent_id, "Work approved, proceeding to merge");
        
        // Check for potential issues
        if rand::thread_rng().gen::<f64>() < 0.1 { // 10% chance of merge conflict
            let conflicts = vec![ConflictInfo {
                file: "src/main.rs".to_string(),
                conflict_markers: 2,
                auto_resolvable: true,
            }];
            
            let mut workflow = self.workflow_machine.write().await;
            workflow.handle_event(AutonomousEvent::MergeConflictDetected { conflicts }).await?;
        } else if rand::random::<f64>() < 0.1 { // 10% chance of CI failure
            let failures = vec![CIFailure {
                job_name: "test".to_string(),
                step: "unit-tests".to_string(),
                error: "Test timeout".to_string(),
                auto_fixable: true,
            }];
            
            let mut workflow = self.workflow_machine.write().await;
            workflow.handle_event(AutonomousEvent::CIFailureDetected { failures }).await?;
        } else {
            // Successful merge
            let completed_work = CompletedWork {
                issue: Issue {
                    number: 123,
                    title: "Autonomous work".to_string(),
                    body: "Completed autonomously".to_string(),
                    labels: vec!["autonomous".to_string()],
                    priority: Priority::Medium,
                    estimated_hours: Some(2),
                },
                commits: 5,
                files_changed: 3,
                tests_added: 2,
                completion_time: Utc::now(),
            };
            
            let mut workflow = self.workflow_machine.write().await;
            workflow.handle_event(AutonomousEvent::MergeCompleted { merged_work: completed_work }).await?;
        }
        
        Ok(())
    }
    
    /// Handle merge conflict state - attempt resolution
    async fn handle_merge_conflict_state(&self, conflicts: &[ConflictInfo]) -> Result<(), AutonomousWorkflowError> {
        warn!(
            agent_id = %self.agent_id,
            conflicts_count = %conflicts.len(),
            "Handling merge conflicts"
        );
        
        // Check if conflicts are auto-resolvable
        let auto_resolvable = conflicts.iter().all(|c| c.auto_resolvable);
        
        if auto_resolvable {
            // Simulate conflict resolution
            tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
            
            let mut workflow = self.workflow_machine.write().await;
            workflow.handle_event(AutonomousEvent::ConflictsResolved).await?;
        } else {
            // Escalate complex conflicts
            let mut workflow = self.workflow_machine.write().await;
            workflow.handle_event(AutonomousEvent::ForceAbandon {
                reason: AbandonmentReason::CriticalFailure {
                    error: "Complex merge conflicts require human intervention".to_string(),
                },
            }).await?;
        }
        
        Ok(())
    }
    
    /// Handle CI failure state - attempt fixes
    async fn handle_ci_failure_state(&self, failures: &[CIFailure]) -> Result<(), AutonomousWorkflowError> {
        warn!(
            agent_id = %self.agent_id,
            failures_count = %failures.len(),
            "Handling CI failures"
        );
        
        // Check if failures are auto-fixable
        let auto_fixable = failures.iter().all(|f| f.auto_fixable);
        
        if auto_fixable {
            // Simulate fixing CI issues
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            
            let mut workflow = self.workflow_machine.write().await;
            workflow.handle_event(AutonomousEvent::CIFixed).await?;
        } else {
            // Escalate complex CI failures
            let mut workflow = self.workflow_machine.write().await;
            workflow.handle_event(AutonomousEvent::ForceAbandon {
                reason: AbandonmentReason::CriticalFailure {
                    error: "Complex CI failures require human intervention".to_string(),
                },
            }).await?;
        }
        
        Ok(())
    }
    
    /// Spawn monitoring task for autonomous operation
    async fn spawn_monitoring_task(&self) -> tokio::task::JoinHandle<()> {
        let workflow_machine = Arc::clone(&self.workflow_machine);
        let error_recovery = Arc::clone(&self.error_recovery);
        let is_running = Arc::clone(&self.is_running);
        let agent_id = self.agent_id.clone();
        let monitoring_interval = self.coordination_config.monitoring_interval_minutes;
        
        tokio::spawn(async move {
            while *is_running.read().await {
                // Generate and log status report
                let status_report = {
                    let workflow = workflow_machine.read().await;
                    workflow.generate_status_report()
                };
                
                let recovery_report = {
                    let recovery = error_recovery.read().await;
                    recovery.generate_recovery_report()
                };
                
                info!(
                    agent_id = %agent_id,
                    current_state = ?status_report.current_state,
                    uptime_minutes = ?status_report.uptime_minutes,
                    can_continue = %status_report.can_continue,
                    transitions_count = %status_report.transitions_count,
                    recovery_success_rate = %recovery_report.success_rate,
                    "Autonomous operation status"
                );
                
                tokio::time::sleep(tokio::time::Duration::from_secs(monitoring_interval as u64 * 60)).await;
            }
        })
    }
    
    /// Get current status report
    pub async fn get_status_report(&self) -> AutonomousStatusReport {
        let workflow = self.workflow_machine.read().await;
        workflow.generate_status_report()
    }
    
    /// Get recovery report
    pub async fn get_recovery_report(&self) -> AutonomousRecoveryReport {
        let recovery = self.error_recovery.read().await;
        recovery.generate_recovery_report()
    }
    
    /// Check if autonomous operation is running
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::recovery::AutoRecovery;

    #[tokio::test]
    async fn test_autonomous_coordinator_creation() {
        let github_client = GitHubClient::new().unwrap();
        let recovery_client = Box::new(AutoRecovery::new(github_client.clone(), true));
        let config = CoordinationConfig::default();
        
        let coordinator = AutonomousCoordinator::new(
            github_client,
            "test-agent".to_string(),
            recovery_client,
            config,
        ).await;
        
        assert!(coordinator.is_ok());
        
        let coordinator = coordinator.unwrap();
        assert!(!coordinator.is_running().await);
    }
    
    #[tokio::test]
    async fn test_status_report_generation() {
        let github_client = GitHubClient::new().unwrap();
        let recovery_client = Box::new(AutoRecovery::new(github_client.clone(), true));
        let config = CoordinationConfig::default();
        
        let coordinator = AutonomousCoordinator::new(
            github_client,
            "test-agent".to_string(),
            recovery_client,
            config,
        ).await.unwrap();
        
        let status_report = coordinator.get_status_report().await;
        assert!(status_report.agent_id.is_some());
        
        let recovery_report = coordinator.get_recovery_report().await;
        assert_eq!(recovery_report.total_attempts, 0);
    }
}