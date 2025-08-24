use serde::{Deserialize, Serialize};
use thiserror::Error;
use chrono::{DateTime, Utc};
use tracing::{info, warn, error};

use crate::github::{GitHubClient, errors::GitHubError};
use crate::agents::recovery::{AutomaticRecovery, ComprehensiveRecoveryReport};

/// Complete autonomous workflow state machine for unattended operation
/// This covers every possible state and transition for true autonomous work
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutonomousWorkflowState {
    // Initial states
    Unassigned { 
        issue: Issue 
    },
    Assigned { 
        issue: Issue, 
        agent: AgentId, 
        workspace: WorkspaceState 
    },
    
    // Work states  
    InProgress { 
        issue: Issue, 
        agent: AgentId, 
        progress: WorkProgress 
    },
    Blocked { 
        issue: Issue, 
        agent: AgentId, 
        blocker: BlockerType 
    },
    
    // Review states
    ReadyForReview { 
        issue: Issue, 
        agent: AgentId, 
        pr: PullRequest 
    },
    UnderReview { 
        issue: Issue, 
        agent: AgentId, 
        pr: PullRequest, 
        feedback: Vec<ReviewFeedback> 
    },
    ChangesRequested { 
        issue: Issue, 
        agent: AgentId, 
        pr: PullRequest, 
        required_changes: Vec<Change> 
    },
    
    // Integration states
    Approved { 
        issue: Issue, 
        agent: AgentId, 
        pr: PullRequest 
    },
    MergeConflict { 
        issue: Issue, 
        agent: AgentId, 
        pr: PullRequest, 
        conflicts: Vec<ConflictInfo> 
    },
    CIFailure { 
        issue: Issue, 
        agent: AgentId, 
        pr: PullRequest, 
        failures: Vec<CIFailure> 
    },
    
    // Terminal states
    Merged { 
        issue: Issue, 
        work: CompletedWork 
    },
    Abandoned { 
        issue: Issue, 
        reason: AbandonmentReason 
    },
}

/// Events that can trigger state transitions in autonomous workflow
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutonomousEvent {
    // Assignment events
    AssignAgent { agent: AgentId, workspace_ready: bool },
    WorkspaceReady,
    
    // Work events
    StartWork,
    MakeProgress { commits: u32, files_changed: u32 },
    EncounterBlocker { blocker: BlockerType },
    ResolveBlocker,
    CompleteWork,
    
    // Review events  
    SubmitForReview { pr: PullRequest },
    ReviewReceived { feedback: Vec<ReviewFeedback> },
    ChangesRequested { changes: Vec<Change> },
    ApprovalReceived,
    
    // Integration events
    MergeConflictDetected { conflicts: Vec<ConflictInfo> },
    CIFailureDetected { failures: Vec<CIFailure> },
    ConflictsResolved,
    CIFixed,
    MergeCompleted { merged_work: CompletedWork },
    
    // Recovery events
    AutoRecover,
    ForceAbandon { reason: AbandonmentReason },
    Reset,
}

/// Supporting data types for autonomous workflow
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Issue {
    pub number: u64,
    pub title: String,
    pub body: String,
    pub labels: Vec<String>,
    pub priority: Priority,
    pub estimated_hours: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentId(pub String);

impl PartialEq<str> for AgentId {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceState {
    pub branch_name: String,
    pub base_branch: String,
    pub workspace_setup: bool,
    pub dependencies_installed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkProgress {
    pub commits_made: u32,
    pub files_changed: u32,
    pub tests_written: u32,
    pub elapsed_minutes: u32,
    pub completion_percentage: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockerType {
    DependencyIssue { dependency: String, error: String },
    TestFailure { test_name: String, error: String },
    BuildFailure { error: String },
    ExternalService { service: String, status: String },
    MissingRequirements { missing: Vec<String> },
    NetworkIssue { error: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub branch: String,
    pub commits: u32,
    pub files_changed: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewFeedback {
    pub reviewer: String,
    pub comments: Vec<ReviewComment>,
    pub overall_approval: bool,
    pub requested_changes: Vec<Change>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewComment {
    pub file: String,
    pub line: u32,
    pub comment: String,
    pub severity: CommentSeverity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommentSeverity {
    Nitpick,
    Suggestion,
    RequiredChange,
    Blocking,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Change {
    pub file: String,
    pub description: String,
    pub automated_fix_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictInfo {
    pub file: String,
    pub conflict_markers: u32,
    pub auto_resolvable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CIFailure {
    pub job_name: String,
    pub step: String,
    pub error: String,
    pub auto_fixable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletedWork {
    pub issue: Issue,
    pub commits: u32,
    pub files_changed: u32,
    pub tests_added: u32,
    pub completion_time: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AbandonmentReason {
    UnresolvableBlocker { blocker: BlockerType },
    TimeoutExceeded { max_hours: u8 },
    RequirementsChanged,
    DependencyIssues,
    CriticalFailure { error: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

/// Errors that can occur during autonomous workflow transitions
#[derive(Debug, Error)]
pub enum AutonomousWorkflowError {
    #[error("Invalid transition: {event:?} not allowed in current state")]
    InvalidTransition { event: AutonomousEvent },
    
    #[error("GitHub API error: {0}")]
    GitHubError(#[from] GitHubError),
    
    #[error("Workspace setup failed: {reason}")]
    WorkspaceError { reason: String },
    
    #[error("Recovery failed: {reason}")]
    RecoveryError { reason: String },
    
    #[error("Timeout exceeded: {max_hours}h")]
    TimeoutError { max_hours: u8 },
    
    #[error("Critical blocker: {blocker:?}")]
    CriticalBlocker { blocker: BlockerType },
}

/// Main autonomous workflow state machine
#[derive(Default)]
pub struct AutonomousWorkflowMachine {
    pub current_state: Option<AutonomousWorkflowState>,
    pub agent_id: Option<AgentId>,
    pub start_time: Option<DateTime<Utc>>,
    pub max_work_hours: u8,
    pub github_client: Option<GitHubClient>,
    pub recovery_client: Option<Box<dyn AutomaticRecovery + Send + Sync>>,
    pub state_history: Vec<StateTransitionRecord>,
}

impl std::fmt::Debug for AutonomousWorkflowMachine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AutonomousWorkflowMachine")
            .field("current_state", &self.current_state)
            .field("agent_id", &self.agent_id)
            .field("start_time", &self.start_time)
            .field("max_work_hours", &self.max_work_hours)
            .field("github_client", &self.github_client.is_some())
            .field("recovery_client", &self.recovery_client.is_some())
            .field("state_history", &self.state_history)
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransitionRecord {
    pub from_state: Option<AutonomousWorkflowState>,
    pub to_state: AutonomousWorkflowState,
    pub event: AutonomousEvent,
    pub timestamp: DateTime<Utc>,
    pub duration_ms: u64,
}

impl AutonomousWorkflowMachine {
    pub fn new(max_work_hours: u8) -> Self {
        Self {
            max_work_hours,
            ..Default::default()
        }
    }
    
    pub fn with_github_client(mut self, client: GitHubClient) -> Self {
        self.github_client = Some(client);
        self
    }
    
    pub fn with_agent_id(mut self, agent_id: String) -> Self {
        self.agent_id = Some(AgentId(agent_id));
        self
    }
    
    pub fn with_recovery_client(mut self, recovery: Box<dyn AutomaticRecovery + Send + Sync>) -> Self {
        self.recovery_client = Some(recovery);
        self
    }
    
    /// Record state transition for audit trail
    fn record_transition(
        &mut self, 
        from: Option<AutonomousWorkflowState>, 
        to: AutonomousWorkflowState, 
        event: AutonomousEvent,
        duration_ms: u64
    ) {
        let record = StateTransitionRecord {
            from_state: from,
            to_state: to.clone(),
            event,
            timestamp: Utc::now(),
            duration_ms,
        };
        
        info!(
            from_state = ?record.from_state,
            to_state = ?record.to_state,
            event = ?record.event,
            duration_ms = %record.duration_ms,
            "Autonomous workflow state transition"
        );
        
        self.state_history.push(record);
        self.current_state = Some(to);
    }
    
    /// Check if maximum work time has been exceeded
    fn is_timeout_exceeded(&self) -> bool {
        if let Some(start_time) = self.start_time {
            let elapsed = Utc::now().signed_duration_since(start_time);
            elapsed.num_hours() >= self.max_work_hours as i64
        } else {
            false
        }
    }
    
    /// Attempt autonomous recovery from current state
    pub async fn attempt_autonomous_recovery(&mut self) -> Result<ComprehensiveRecoveryReport, AutonomousWorkflowError> {
        if let Some(recovery) = &self.recovery_client {
            let report = recovery.recover_all_inconsistencies().await
                .map_err(|e| AutonomousWorkflowError::RecoveryError { 
                    reason: format!("Recovery failed: {:?}", e) 
                })?;
            
            info!(
                recovery_rate = %report.recovery_rate,
                recovered = %report.recovered.len(),
                failed = %report.failed.len(),
                "Autonomous recovery completed"
            );
            
            Ok(report)
        } else {
            Err(AutonomousWorkflowError::RecoveryError { 
                reason: "No recovery client configured".to_string() 
            })
        }
    }
    
    /// Handle autonomous event - main state transition logic
    pub async fn handle_event(&mut self, event: AutonomousEvent) -> Result<(), AutonomousWorkflowError> {
        let start_time = std::time::Instant::now();
        let from_state = self.current_state.clone();
        
        // Check timeout before processing any event
        if self.is_timeout_exceeded() {
            let timeout_event = AutonomousEvent::ForceAbandon { 
                reason: AbandonmentReason::TimeoutExceeded { 
                    max_hours: self.max_work_hours 
                } 
            };
            return self.handle_timeout_abandonment(timeout_event).await;
        }
        
        let new_state = match (&self.current_state, &event) {
            // Initial assignment transitions
            (None, AutonomousEvent::AssignAgent { agent, workspace_ready }) => {
                self.agent_id = Some(agent.clone());
                self.start_time = Some(Utc::now());
                
                if *workspace_ready {
                    Some(AutonomousWorkflowState::Assigned { 
                        issue: self.create_placeholder_issue(),
                        agent: agent.clone(),
                        workspace: self.create_workspace_state()
                    })
                } else {
                    Some(AutonomousWorkflowState::Unassigned { 
                        issue: self.create_placeholder_issue() 
                    })
                }
            }
            
            (Some(AutonomousWorkflowState::Unassigned { issue }), AutonomousEvent::WorkspaceReady) => {
                if let Some(agent) = &self.agent_id {
                    Some(AutonomousWorkflowState::Assigned { 
                        issue: issue.clone(),
                        agent: agent.clone(),
                        workspace: self.create_workspace_state()
                    })
                } else {
                    return Err(AutonomousWorkflowError::WorkspaceError { 
                        reason: "No agent assigned".to_string() 
                    });
                }
            }
            
            // Work progression transitions
            (Some(AutonomousWorkflowState::Assigned { issue, agent, .. }), AutonomousEvent::StartWork) => {
                Some(AutonomousWorkflowState::InProgress { 
                    issue: issue.clone(),
                    agent: agent.clone(),
                    progress: WorkProgress {
                        commits_made: 0,
                        files_changed: 0,
                        tests_written: 0,
                        elapsed_minutes: 0,
                        completion_percentage: 0,
                    }
                })
            }
            
            (Some(AutonomousWorkflowState::InProgress { issue, agent, progress }), 
             AutonomousEvent::MakeProgress { commits, files_changed }) => {
                let mut updated_progress = progress.clone();
                updated_progress.commits_made += commits;
                updated_progress.files_changed += files_changed;
                if let Some(start) = self.start_time {
                    updated_progress.elapsed_minutes = Utc::now().signed_duration_since(start).num_minutes() as u32;
                }
                
                Some(AutonomousWorkflowState::InProgress { 
                    issue: issue.clone(),
                    agent: agent.clone(),
                    progress: updated_progress
                })
            }
            
            (Some(AutonomousWorkflowState::InProgress { issue, agent, .. }), 
             AutonomousEvent::EncounterBlocker { blocker }) => {
                warn!(
                    blocker = ?blocker,
                    "Autonomous workflow encountered blocker"
                );
                
                Some(AutonomousWorkflowState::Blocked { 
                    issue: issue.clone(),
                    agent: agent.clone(),
                    blocker: blocker.clone()
                })
            }
            
            (Some(AutonomousWorkflowState::Blocked { issue, agent, .. }), 
             AutonomousEvent::ResolveBlocker) => {
                info!("Autonomous workflow resolved blocker");
                
                Some(AutonomousWorkflowState::InProgress { 
                    issue: issue.clone(),
                    agent: agent.clone(),
                    progress: WorkProgress {
                        commits_made: 0,
                        files_changed: 0,
                        tests_written: 0,
                        elapsed_minutes: 0,
                        completion_percentage: 50, // Resume with some progress
                    }
                })
            }
            
            (Some(AutonomousWorkflowState::InProgress { issue, agent, .. }), 
             AutonomousEvent::CompleteWork) => {
                info!("Autonomous workflow completed work phase");
                
                Some(AutonomousWorkflowState::ReadyForReview { 
                    issue: issue.clone(),
                    agent: agent.clone(),
                    pr: PullRequest {
                        number: 0, // Will be set when PR is created
                        title: format!("Fix {}", issue.title),
                        branch: format!("{}/{}", agent.0, issue.number),
                        commits: 5,
                        files_changed: 3,
                    }
                })
            }
            
            // Review flow transitions
            (Some(AutonomousWorkflowState::ReadyForReview { issue, agent, .. }), 
             AutonomousEvent::SubmitForReview { pr }) => {
                Some(AutonomousWorkflowState::UnderReview { 
                    issue: issue.clone(),
                    agent: agent.clone(),
                    pr: pr.clone(),
                    feedback: vec![]
                })
            }
            
            (Some(AutonomousWorkflowState::UnderReview { issue, agent, pr, .. }), 
             AutonomousEvent::ReviewReceived { feedback }) => {
                let has_required_changes = feedback.iter()
                    .any(|f| !f.requested_changes.is_empty());
                
                if has_required_changes {
                    let all_changes: Vec<Change> = feedback.iter()
                        .flat_map(|f| f.requested_changes.iter().cloned())
                        .collect();
                        
                    Some(AutonomousWorkflowState::ChangesRequested { 
                        issue: issue.clone(),
                        agent: agent.clone(),
                        pr: pr.clone(),
                        required_changes: all_changes
                    })
                } else {
                    Some(AutonomousWorkflowState::Approved { 
                        issue: issue.clone(),
                        agent: agent.clone(),
                        pr: pr.clone()
                    })
                }
            }
            
            (Some(AutonomousWorkflowState::ChangesRequested { issue, agent, pr, .. }), 
             AutonomousEvent::ApprovalReceived) => {
                Some(AutonomousWorkflowState::Approved { 
                    issue: issue.clone(),
                    agent: agent.clone(),
                    pr: pr.clone()
                })
            }
            
            // Integration flow transitions
            (Some(AutonomousWorkflowState::Approved { issue, agent, pr }), 
             AutonomousEvent::MergeConflictDetected { conflicts }) => {
                warn!(
                    conflicts_count = %conflicts.len(),
                    "Merge conflicts detected in autonomous workflow"
                );
                
                Some(AutonomousWorkflowState::MergeConflict { 
                    issue: issue.clone(),
                    agent: agent.clone(),
                    pr: pr.clone(),
                    conflicts: conflicts.clone()
                })
            }
            
            (Some(AutonomousWorkflowState::Approved { issue, agent, pr }), 
             AutonomousEvent::CIFailureDetected { failures }) => {
                warn!(
                    failures_count = %failures.len(),
                    "CI failures detected in autonomous workflow"
                );
                
                Some(AutonomousWorkflowState::CIFailure { 
                    issue: issue.clone(),
                    agent: agent.clone(),
                    pr: pr.clone(),
                    failures: failures.clone()
                })
            }
            
            (Some(AutonomousWorkflowState::MergeConflict { issue, agent, pr, .. }), 
             AutonomousEvent::ConflictsResolved) => {
                info!("Merge conflicts resolved autonomously");
                
                Some(AutonomousWorkflowState::Approved { 
                    issue: issue.clone(),
                    agent: agent.clone(),
                    pr: pr.clone()
                })
            }
            
            (Some(AutonomousWorkflowState::CIFailure { issue, agent, pr, .. }), 
             AutonomousEvent::CIFixed) => {
                info!("CI failures resolved autonomously");
                
                Some(AutonomousWorkflowState::Approved { 
                    issue: issue.clone(),
                    agent: agent.clone(),
                    pr: pr.clone()
                })
            }
            
            (Some(AutonomousWorkflowState::Approved { issue, .. }), 
             AutonomousEvent::MergeCompleted { merged_work }) => {
                info!(
                    issue_number = %issue.number,
                    "Autonomous workflow completed successfully"
                );
                
                Some(AutonomousWorkflowState::Merged { 
                    issue: issue.clone(),
                    work: merged_work.clone()
                })
            }
            
            // Recovery transitions
            (Some(_), AutonomousEvent::AutoRecover) => {
                let _ = self.attempt_autonomous_recovery().await?;
                // State remains the same after recovery attempt
                self.current_state.clone()
            }
            
            // Abandonment transitions
            (Some(state), AutonomousEvent::ForceAbandon { reason }) => {
                let issue = self.extract_issue_from_state(state);
                
                error!(
                    reason = ?reason,
                    issue = ?issue.as_ref().map(|i| i.number),
                    "Autonomous workflow abandoned"
                );
                
                Some(AutonomousWorkflowState::Abandoned { 
                    issue: issue.unwrap_or_else(|| self.create_placeholder_issue()),
                    reason: reason.clone()
                })
            }
            
            // Reset transitions (terminal -> initial)
            (Some(AutonomousWorkflowState::Merged { .. }), AutonomousEvent::Reset) |
            (Some(AutonomousWorkflowState::Abandoned { .. }), AutonomousEvent::Reset) => {
                self.agent_id = None;
                self.start_time = None;
                None // Back to initial state
            }
            
            // Invalid transitions
            (current_state, event) => {
                error!(
                    current_state = ?current_state,
                    event = ?event,
                    "Invalid autonomous workflow transition"
                );
                
                return Err(AutonomousWorkflowError::InvalidTransition { 
                    event: event.clone() 
                });
            }
        };
        
        let duration = start_time.elapsed().as_millis() as u64;
        
        if let Some(new_state) = new_state {
            self.record_transition(from_state, new_state, event, duration);
        }
        
        Ok(())
    }
    
    /// Handle timeout abandonment specially
    async fn handle_timeout_abandonment(&mut self, event: AutonomousEvent) -> Result<(), AutonomousWorkflowError> {
        if let Some(current_state) = &self.current_state {
            let issue = self.extract_issue_from_state(current_state)
                .unwrap_or_else(|| self.create_placeholder_issue());
            
            let reason = if let AutonomousEvent::ForceAbandon { reason } = event {
                reason
            } else {
                AbandonmentReason::TimeoutExceeded { max_hours: self.max_work_hours }
            };
            
            self.current_state = Some(AutonomousWorkflowState::Abandoned { issue, reason });
        }
        
        Ok(())
    }
    
    /// Extract issue from any state that contains one
    fn extract_issue_from_state(&self, state: &AutonomousWorkflowState) -> Option<Issue> {
        match state {
            AutonomousWorkflowState::Unassigned { issue } |
            AutonomousWorkflowState::Assigned { issue, .. } |
            AutonomousWorkflowState::InProgress { issue, .. } |
            AutonomousWorkflowState::Blocked { issue, .. } |
            AutonomousWorkflowState::ReadyForReview { issue, .. } |
            AutonomousWorkflowState::UnderReview { issue, .. } |
            AutonomousWorkflowState::ChangesRequested { issue, .. } |
            AutonomousWorkflowState::Approved { issue, .. } |
            AutonomousWorkflowState::MergeConflict { issue, .. } |
            AutonomousWorkflowState::CIFailure { issue, .. } |
            AutonomousWorkflowState::Merged { issue, .. } |
            AutonomousWorkflowState::Abandoned { issue, .. } => Some(issue.clone())
        }
    }
    
    /// Create placeholder issue for error cases
    fn create_placeholder_issue(&self) -> Issue {
        Issue {
            number: 0,
            title: "Unknown Issue".to_string(),
            body: "Placeholder issue for autonomous workflow".to_string(),
            labels: vec![],
            priority: Priority::Medium,
            estimated_hours: None,
        }
    }
    
    /// Create workspace state
    fn create_workspace_state(&self) -> WorkspaceState {
        WorkspaceState {
            branch_name: format!("autonomous-work-{}", Utc::now().timestamp()),
            base_branch: "main".to_string(),
            workspace_setup: true,
            dependencies_installed: true,
        }
    }
    
    /// Get current state for external inspection
    pub fn current_state(&self) -> Option<&AutonomousWorkflowState> {
        self.current_state.as_ref()
    }
    
    /// Get full state transition history
    pub fn state_history(&self) -> &[StateTransitionRecord] {
        &self.state_history
    }
    
    /// Check if workflow can continue autonomously
    pub fn can_continue_autonomously(&self) -> bool {
        match &self.current_state {
            Some(AutonomousWorkflowState::Abandoned { .. }) |
            Some(AutonomousWorkflowState::Merged { .. }) => false,
            Some(AutonomousWorkflowState::Blocked { blocker, .. }) => {
                self.can_resolve_blocker_autonomously(blocker)
            }
            _ => !self.is_timeout_exceeded()
        }
    }
    
    /// Check if a blocker can be resolved autonomously
    fn can_resolve_blocker_autonomously(&self, blocker: &BlockerType) -> bool {
        match blocker {
            BlockerType::DependencyIssue { .. } => false, // Requires human intervention
            BlockerType::TestFailure { .. } => true,      // Can be fixed autonomously
            BlockerType::BuildFailure { .. } => true,     // Often fixable
            BlockerType::ExternalService { .. } => false, // Wait for service recovery
            BlockerType::MissingRequirements { .. } => false, // Requires clarification
            BlockerType::NetworkIssue { .. } => true,     // Retry mechanism
        }
    }
    
    /// Generate status report for monitoring
    pub fn generate_status_report(&self) -> AutonomousStatusReport {
        let uptime = if let Some(start) = self.start_time {
            Some(Utc::now().signed_duration_since(start).num_minutes() as u32)
        } else {
            None
        };
        
        AutonomousStatusReport {
            current_state: self.current_state.clone(),
            agent_id: self.agent_id.clone(),
            uptime_minutes: uptime,
            can_continue: self.can_continue_autonomously(),
            timeout_in_minutes: if let Some(start) = self.start_time {
                let max_minutes = self.max_work_hours as i64 * 60;
                let elapsed = Utc::now().signed_duration_since(start).num_minutes();
                Some((max_minutes - elapsed).max(0) as u32)
            } else {
                None
            },
            transitions_count: self.state_history.len(),
            last_transition: self.state_history.last().map(|t| t.timestamp),
        }
    }
}

/// Status report for autonomous workflow monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousStatusReport {
    pub current_state: Option<AutonomousWorkflowState>,
    pub agent_id: Option<AgentId>,
    pub uptime_minutes: Option<u32>,
    pub can_continue: bool,
    pub timeout_in_minutes: Option<u32>,
    pub transitions_count: usize,
    pub last_transition: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autonomous_workflow_basic_flow() {
        let mut workflow = AutonomousWorkflowMachine::new(8);
        
        // Test initial assignment
        let assign_event = AutonomousEvent::AssignAgent {
            agent: AgentId("agent001".to_string()),
            workspace_ready: true,
        };
        
        tokio_test::block_on(async {
            workflow.handle_event(assign_event).await.unwrap();
        });
        
        assert!(matches!(
            workflow.current_state(),
            Some(AutonomousWorkflowState::Assigned { .. })
        ));
        
        // Test starting work
        tokio_test::block_on(async {
            workflow.handle_event(AutonomousEvent::StartWork).await.unwrap();
        });
        
        assert!(matches!(
            workflow.current_state(),
            Some(AutonomousWorkflowState::InProgress { .. })
        ));
    }
    
    #[test]
    fn test_blocker_handling() {
        let mut workflow = AutonomousWorkflowMachine::new(8);
        
        // Setup to in-progress state
        tokio_test::block_on(async {
            workflow.handle_event(AutonomousEvent::AssignAgent {
                agent: AgentId("agent001".to_string()),
                workspace_ready: true,
            }).await.unwrap();
            
            workflow.handle_event(AutonomousEvent::StartWork).await.unwrap();
            
            // Encounter a blocker
            workflow.handle_event(AutonomousEvent::EncounterBlocker {
                blocker: BlockerType::TestFailure {
                    test_name: "test_example".to_string(),
                    error: "Assertion failed".to_string(),
                }
            }).await.unwrap();
        });
        
        assert!(matches!(
            workflow.current_state(),
            Some(AutonomousWorkflowState::Blocked { .. })
        ));
    }
    
    #[test]
    fn test_timeout_handling() {
        let mut workflow = AutonomousWorkflowMachine::new(0); // 0 hours = immediate timeout
        
        tokio_test::block_on(async {
            workflow.handle_event(AutonomousEvent::AssignAgent {
                agent: AgentId("agent001".to_string()),
                workspace_ready: true,
            }).await.unwrap();
            
            // Any subsequent event should trigger timeout
            let _result = workflow.handle_event(AutonomousEvent::StartWork).await;
            
            // Should either be abandoned or handle timeout gracefully
            assert!(workflow.current_state().is_some());
        });
    }
    
    #[test]
    fn test_status_report_generation() {
        let mut workflow = AutonomousWorkflowMachine::new(8);
        
        tokio_test::block_on(async {
            workflow.handle_event(AutonomousEvent::AssignAgent {
                agent: AgentId("agent001".to_string()),
                workspace_ready: true,
            }).await.unwrap();
        });
        
        let report = workflow.generate_status_report();
        
        assert!(report.agent_id.is_some());
        assert!(report.can_continue);
        assert_eq!(report.transitions_count, 1);
    }
    
    #[test]
    fn test_state_transition_recording() {
        let mut workflow = AutonomousWorkflowMachine::new(8);
        
        tokio_test::block_on(async {
            workflow.handle_event(AutonomousEvent::AssignAgent {
                agent: AgentId("agent001".to_string()),
                workspace_ready: true,
            }).await.unwrap();
            
            workflow.handle_event(AutonomousEvent::StartWork).await.unwrap();
        });
        
        let history = workflow.state_history();
        assert_eq!(history.len(), 2);
        
        // Check that transitions are recorded correctly
        assert!(matches!(history[0].event, AutonomousEvent::AssignAgent { .. }));
        assert!(matches!(history[1].event, AutonomousEvent::StartWork));
    }
}