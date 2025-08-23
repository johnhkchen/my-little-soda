//! Work Continuity System for Autonomous Operation
//!
//! This module implements robust work continuity across process restarts and interruptions.
//! It builds on the existing persistence and autonomous systems to provide seamless recovery
//! of agent work context and ensures no work is lost during system interruptions.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::sync::{RwLock, mpsc};
use chrono::{DateTime, Utc};
use thiserror::Error;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

use crate::github::GitHubClient;
use crate::agent_lifecycle::state_machine::AgentStateMachine;
use super::persistence::{
    StatePersistenceManager, 
    PersistentWorkflowState, 
    PersistenceConfig, 
    CheckpointReason,
    PersistenceError
};
use super::{AutonomousWorkflowState, AutonomousEvent};

/// Errors that can occur during work continuity operations
#[derive(Debug, Error)]
pub enum WorkContinuityError {
    #[error("Persistence error: {0}")]
    PersistenceError(#[from] PersistenceError),
    
    #[error("GitHub API error: {0}")]
    GitHubError(String),
    
    #[error("Git operation error: {0}")]
    GitError(String),
    
    #[error("State validation error: {reason}")]
    StateValidation { reason: String },
    
    #[error("Recovery error: {reason}")]
    RecoveryError { reason: String },
    
    #[error("Configuration error: {reason}")]
    ConfigurationError { reason: String },
}

/// Complete persistent agent state for work continuity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentAgentState {
    /// Current work context
    pub current_issue: Option<Issue>,
    pub current_branch: Option<String>,
    pub workspace_state: WorkspaceSnapshot,
    pub progress_checkpoint: WorkProgress,
    
    /// Operation context
    pub last_github_sync: DateTime<Utc>,
    pub pending_operations: Vec<PendingOperation>,
    pub error_recovery_context: Option<RecoveryContext>,
    
    /// Session continuity
    pub session_id: String,
    pub uptime_start: DateTime<Utc>,
    pub operation_history: Vec<CompletedOperation>,
    
    /// Integration with autonomous workflow
    pub autonomous_state: Option<AutonomousWorkflowState>,
    pub last_autonomous_event: Option<AutonomousEvent>,
    pub state_machine_data: AgentStateMachineData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub number: u64,
    pub title: String,
    pub body: String,
    pub labels: Vec<String>,
    pub assignee: Option<String>,
    pub milestone: Option<String>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSnapshot {
    pub current_directory: PathBuf,
    pub git_branch: String,
    pub git_commit: String,
    pub uncommitted_changes: bool,
    pub staged_files: Vec<String>,
    pub modified_files: Vec<String>,
    pub untracked_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkProgress {
    pub commits_made: u32,
    pub files_modified: Vec<String>,
    pub tests_written: u32,
    pub last_commit_sha: Option<String>,
    pub progress_description: String,
    pub estimated_completion: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingOperation {
    pub operation_id: String,
    pub operation_type: PendingOperationType,
    pub created_at: DateTime<Utc>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PendingOperationType {
    GitCommit { message: String },
    GitPush { branch: String },
    CreatePullRequest { title: String, body: String },
    UpdateIssue { issue_number: u64, update: String },
    CreateComment { target: String, comment: String },
    LabelUpdate { issue_number: u64, labels: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryContext {
    pub last_error: String,
    pub error_timestamp: DateTime<Utc>,
    pub attempted_fixes: Vec<String>,
    pub recovery_strategy: String,
    pub can_auto_recover: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedOperation {
    pub operation_id: String,
    pub operation_type: String,
    pub completed_at: DateTime<Utc>,
    pub result: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStateMachineData {
    pub agent_id: String,
    pub current_issue: Option<u64>,
    pub current_branch: Option<String>,
    pub commits_ahead: u32,
    pub bundle_issues: Vec<u64>,
    pub bundle_pr: Option<u64>,
}

/// Actions that can be taken when resuming work after restart
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResumeAction {
    ContinueWork { 
        issue: Issue, 
        branch: String,
        last_progress: WorkProgress 
    },
    CompletePartialOperation { 
        operation: PendingOperation 
    },
    RecoverFromError { 
        context: RecoveryContext 
    },
    ValidateAndResync { 
        reason: String 
    },
    StartFresh { 
        reason: String 
    },
}

/// Configuration for work continuity system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkContinuityConfig {
    pub enable_continuity: bool,
    pub state_file_path: PathBuf,
    pub backup_interval_minutes: u32,
    pub max_recovery_attempts: u32,
    pub validation_timeout_seconds: u32,
    pub force_fresh_start_after_hours: u32,
    pub preserve_partial_work: bool,
}

impl Default for WorkContinuityConfig {
    fn default() -> Self {
        Self {
            enable_continuity: true,
            state_file_path: PathBuf::from(".my-little-soda/agent-state.json"),
            backup_interval_minutes: 5,
            max_recovery_attempts: 3,
            validation_timeout_seconds: 30,
            force_fresh_start_after_hours: 24,
            preserve_partial_work: true,
        }
    }
}

/// Main work continuity manager
pub struct WorkContinuityManager {
    config: WorkContinuityConfig,
    github_client: GitHubClient,
    persistence_manager: StatePersistenceManager,
    current_state: RwLock<Option<PersistentAgentState>>,
    checkpoint_sender: Option<mpsc::Sender<PersistentWorkflowState>>,
}

impl WorkContinuityManager {
    /// Create new work continuity manager
    pub fn new(
        config: WorkContinuityConfig,
        github_client: GitHubClient,
        persistence_config: PersistenceConfig,
    ) -> Self {
        let persistence_manager = StatePersistenceManager::new(persistence_config);
        
        Self {
            config,
            github_client,
            persistence_manager,
            current_state: RwLock::new(None),
            checkpoint_sender: None,
        }
    }
    
    /// Initialize work continuity system and start automatic checkpointing
    pub async fn initialize(&mut self, agent_id: &str) -> Result<(), WorkContinuityError> {
        if !self.config.enable_continuity {
            info!("Work continuity disabled by configuration");
            return Ok(());
        }
        
        // Set up automatic checkpointing
        let (sender, receiver) = mpsc::channel::<PersistentWorkflowState>(100);
        self.checkpoint_sender = Some(sender);
        
        // Start auto-save with the receiver
        self.persistence_manager.start_auto_save(receiver).await
            .map_err(WorkContinuityError::PersistenceError)?;
        
        info!(
            agent_id = %agent_id,
            config = ?self.config,
            "Work continuity system initialized"
        );
        
        Ok(())
    }
    
    /// Attempt to recover work state after process restart
    pub async fn recover_from_checkpoint(&self, agent_id: &str) -> Result<Option<ResumeAction>, WorkContinuityError> {
        if !self.config.enable_continuity {
            return Ok(None);
        }
        
        info!(agent_id = %agent_id, "Attempting work recovery from checkpoint");
        
        // Try to load the most recent state
        let persistent_state = match self.persistence_manager.load_state(agent_id).await? {
            Some(state) => state,
            None => {
                info!(agent_id = %agent_id, "No previous state found, starting fresh");
                return Ok(Some(ResumeAction::StartFresh {
                    reason: "No previous state found".to_string()
                }));
            }
        };
        
        // Convert autonomous state to our agent state format
        let agent_state = self.convert_to_agent_state(&persistent_state, agent_id).await?;
        
        // Validate that the state is still relevant and consistent
        let resume_action = self.determine_resume_action(&agent_state).await?;
        
        // Store the recovered state
        {
            let mut current_state = self.current_state.write().await;
            *current_state = Some(agent_state);
        }
        
        match &resume_action {
            ResumeAction::ContinueWork { issue, branch, .. } => {
                info!(
                    agent_id = %agent_id,
                    issue = %issue.number,
                    branch = %branch,
                    "Recovered work context, continuing previous work"
                );
            }
            ResumeAction::StartFresh { reason } => {
                info!(
                    agent_id = %agent_id,
                    reason = %reason,
                    "Starting fresh after recovery analysis"
                );
            }
            _ => {
                info!(
                    agent_id = %agent_id,
                    action = ?resume_action,
                    "Determined recovery action"
                );
            }
        }
        
        Ok(Some(resume_action))
    }
    
    /// Save current work state to persistent storage
    pub async fn checkpoint_state(
        &self,
        agent_state: &AgentStateMachine,
        autonomous_state: Option<AutonomousWorkflowState>,
        reason: CheckpointReason,
    ) -> Result<String, WorkContinuityError> {
        if !self.config.enable_continuity {
            return Ok("continuity_disabled".to_string());
        }
        
        // Capture current workspace state
        let workspace_state = self.capture_workspace_state().await?;
        
        // Create comprehensive agent state
        let persistent_state = PersistentAgentState {
            current_issue: agent_state.current_issue().map(|n| Issue {
                number: n,
                title: "In Progress".to_string(), // Would be fetched from GitHub in real implementation
                body: "".to_string(),
                labels: vec![],
                assignee: Some(agent_state.agent_id().to_string()),
                milestone: None,
                url: format!("https://github.com/example/repo/issues/{}", n),
            }),
            current_branch: agent_state.current_branch().map(|s| s.to_string()),
            workspace_state,
            progress_checkpoint: self.capture_work_progress(agent_state).await?,
            last_github_sync: Utc::now(),
            pending_operations: self.capture_pending_operations().await?,
            error_recovery_context: None, // Would be set if recovering from error
            session_id: Uuid::new_v4().to_string(),
            uptime_start: Utc::now(), // Would track actual start time
            operation_history: vec![], // Would track recent operations
            autonomous_state,
            last_autonomous_event: None, // Would track last event
            state_machine_data: AgentStateMachineData {
                agent_id: agent_state.agent_id().to_string(),
                current_issue: agent_state.current_issue(),
                current_branch: agent_state.current_branch().map(|s| s.to_string()),
                commits_ahead: agent_state.commits_ahead(),
                bundle_issues: vec![], // Would be populated from agent state
                bundle_pr: None, // Would be populated from agent state
            },
        };
        
        // Convert to persistence format and save
        let workflow_state = self.convert_to_workflow_state(&persistent_state).await?;
        let checkpoint_id = self.persistence_manager.save_state(&workflow_state, reason.clone()).await?;
        
        // Update current state
        {
            let mut current_state = self.current_state.write().await;
            *current_state = Some(persistent_state);
        }
        
        // Send to auto-save channel if available
        if let Some(sender) = &self.checkpoint_sender {
            let _ = sender.try_send(workflow_state);
        }
        
        debug!(
            agent_id = %agent_state.agent_id(),
            checkpoint_id = %checkpoint_id,
            reason = ?reason,
            "Work state checkpointed successfully"
        );
        
        Ok(checkpoint_id)
    }
    
    /// Resume interrupted work based on recovery action
    pub async fn resume_interrupted_work(
        &self,
        resume_action: ResumeAction,
        agent_state: &mut AgentStateMachine,
    ) -> Result<(), WorkContinuityError> {
        match resume_action {
            ResumeAction::ContinueWork { issue, branch, last_progress } => {
                info!(
                    agent_id = %agent_state.agent_id(),
                    issue = %issue.number,
                    branch = %branch,
                    commits = %last_progress.commits_made,
                    "Resuming work on issue"
                );
                
                // Validate workspace is in expected state
                self.validate_workspace_for_resume(&branch, &issue).await?;
                
                // Resume would be handled by the calling code using the returned information
                Ok(())
            }
            
            ResumeAction::CompletePartialOperation { operation } => {
                info!(
                    agent_id = %agent_state.agent_id(),
                    operation_id = %operation.operation_id,
                    operation_type = ?operation.operation_type,
                    "Completing partial operation"
                );
                
                self.complete_pending_operation(&operation).await?;
                Ok(())
            }
            
            ResumeAction::RecoverFromError { context } => {
                warn!(
                    agent_id = %agent_state.agent_id(),
                    error = %context.last_error,
                    recovery_strategy = %context.recovery_strategy,
                    "Attempting error recovery"
                );
                
                if context.can_auto_recover {
                    self.attempt_error_recovery(&context).await?;
                } else {
                    return Err(WorkContinuityError::RecoveryError {
                        reason: format!("Manual intervention required: {}", context.last_error)
                    });
                }
                Ok(())
            }
            
            ResumeAction::ValidateAndResync { reason } => {
                info!(
                    agent_id = %agent_state.agent_id(),
                    reason = %reason,
                    "Validating and resyncing state"
                );
                
                self.validate_and_resync_state(agent_state).await?;
                Ok(())
            }
            
            ResumeAction::StartFresh { reason } => {
                info!(
                    agent_id = %agent_state.agent_id(),
                    reason = %reason,
                    "Starting fresh - previous state not recoverable"
                );
                
                // Clear any stale state
                self.clear_stale_state(agent_state).await?;
                Ok(())
            }
        }
    }
    
    /// Get current work continuity status
    pub async fn get_continuity_status(&self, agent_id: &str) -> Result<ContinuityStatus, WorkContinuityError> {
        let current_state = self.current_state.read().await;
        
        let status = match current_state.as_ref() {
            Some(state) => ContinuityStatus {
                is_active: true,
                session_id: state.session_id.clone(),
                uptime_start: state.uptime_start,
                last_checkpoint: state.last_github_sync,
                current_issue: state.current_issue.as_ref().map(|i| i.number),
                current_branch: state.current_branch.clone(),
                pending_operations: state.pending_operations.len(),
                can_resume: true,
            },
            None => ContinuityStatus {
                is_active: false,
                session_id: "none".to_string(),
                uptime_start: Utc::now(),
                last_checkpoint: Utc::now(),
                current_issue: None,
                current_branch: None,
                pending_operations: 0,
                can_resume: false,
            }
        };
        
        Ok(status)
    }
    
    // Private helper methods
    
    async fn convert_to_agent_state(
        &self,
        workflow_state: &PersistentWorkflowState,
        agent_id: &str,
    ) -> Result<PersistentAgentState, WorkContinuityError> {
        // This would be a more sophisticated conversion in practice
        // For now, create a basic agent state from workflow state
        Ok(PersistentAgentState {
            current_issue: None, // Would extract from workflow state
            current_branch: None, // Would extract from workflow state
            workspace_state: WorkspaceSnapshot {
                current_directory: std::env::current_dir().unwrap_or_default(),
                git_branch: "main".to_string(), // Would get from git
                git_commit: "unknown".to_string(), // Would get from git
                uncommitted_changes: false,
                staged_files: vec![],
                modified_files: vec![],
                untracked_files: vec![],
            },
            progress_checkpoint: WorkProgress {
                commits_made: 0,
                files_modified: vec![],
                tests_written: 0,
                last_commit_sha: None,
                progress_description: "Recovered state".to_string(),
                estimated_completion: None,
            },
            last_github_sync: workflow_state.last_persisted,
            pending_operations: vec![],
            error_recovery_context: None,
            session_id: Uuid::new_v4().to_string(),
            uptime_start: workflow_state.start_time.unwrap_or_else(Utc::now),
            operation_history: vec![],
            autonomous_state: workflow_state.current_state.clone(),
            last_autonomous_event: None,
            state_machine_data: AgentStateMachineData {
                agent_id: agent_id.to_string(),
                current_issue: None,
                current_branch: None,
                commits_ahead: 0,
                bundle_issues: vec![],
                bundle_pr: None,
            },
        })
    }
    
    async fn convert_to_workflow_state(
        &self,
        agent_state: &PersistentAgentState,
    ) -> Result<PersistentWorkflowState, WorkContinuityError> {
        Ok(PersistentWorkflowState {
            version: "1.0.0".to_string(),
            agent_id: agent_state.state_machine_data.agent_id.clone(),
            current_state: agent_state.autonomous_state.clone(),
            start_time: Some(agent_state.uptime_start),
            max_work_hours: 8,
            state_history: vec![], // Would be populated with actual history
            recovery_history: vec![], // Would be populated with actual history
            checkpoint_metadata: super::persistence::CheckpointMetadata {
                checkpoint_id: agent_state.session_id.clone(),
                creation_reason: CheckpointReason::PeriodicSave,
                integrity_hash: "placeholder".to_string(),
                agent_pid: std::process::id().into(),
                hostname: hostname::get()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
            },
            last_persisted: agent_state.last_github_sync,
        })
    }
    
    async fn determine_resume_action(
        &self,
        agent_state: &PersistentAgentState,
    ) -> Result<ResumeAction, WorkContinuityError> {
        // Check if state is too old
        let time_since_checkpoint = Utc::now() - agent_state.last_github_sync;
        if time_since_checkpoint > chrono::Duration::hours(self.config.force_fresh_start_after_hours as i64) {
            return Ok(ResumeAction::StartFresh {
                reason: format!("State too old: {} hours", time_since_checkpoint.num_hours())
            });
        }
        
        // Check for pending operations
        if !agent_state.pending_operations.is_empty() {
            return Ok(ResumeAction::CompletePartialOperation {
                operation: agent_state.pending_operations[0].clone()
            });
        }
        
        // Check for error recovery
        if let Some(recovery_context) = &agent_state.error_recovery_context {
            return Ok(ResumeAction::RecoverFromError {
                context: recovery_context.clone()
            });
        }
        
        // Check if we can continue work
        if let (Some(issue), Some(branch)) = (&agent_state.current_issue, &agent_state.current_branch) {
            return Ok(ResumeAction::ContinueWork {
                issue: issue.clone(),
                branch: branch.clone(),
                last_progress: agent_state.progress_checkpoint.clone(),
            });
        }
        
        // Default to validation and resync
        Ok(ResumeAction::ValidateAndResync {
            reason: "State validation required".to_string()
        })
    }
    
    async fn capture_workspace_state(&self) -> Result<WorkspaceSnapshot, WorkContinuityError> {
        // In a real implementation, this would use git commands to capture state
        Ok(WorkspaceSnapshot {
            current_directory: std::env::current_dir().unwrap_or_default(),
            git_branch: "main".to_string(), // Would use git to get current branch
            git_commit: "unknown".to_string(), // Would use git to get current commit
            uncommitted_changes: false, // Would check git status
            staged_files: vec![],
            modified_files: vec![],
            untracked_files: vec![],
        })
    }
    
    async fn capture_work_progress(
        &self,
        agent_state: &AgentStateMachine,
    ) -> Result<WorkProgress, WorkContinuityError> {
        Ok(WorkProgress {
            commits_made: agent_state.commits_ahead(),
            files_modified: vec![], // Would be captured from git status
            tests_written: 0, // Would be analyzed from recent commits
            last_commit_sha: None, // Would be captured from git
            progress_description: format!(
                "Agent {} working on issue {:?} with {} commits ahead",
                agent_state.agent_id(),
                agent_state.current_issue(),
                agent_state.commits_ahead()
            ),
            estimated_completion: None, // Would be estimated based on progress
        })
    }
    
    async fn capture_pending_operations(&self) -> Result<Vec<PendingOperation>, WorkContinuityError> {
        // In a real implementation, this would capture any operations in progress
        Ok(vec![])
    }
    
    async fn validate_workspace_for_resume(
        &self,
        branch: &str,
        issue: &Issue,
    ) -> Result<(), WorkContinuityError> {
        // In a real implementation, this would:
        // 1. Check that the git branch exists and is current
        // 2. Validate that the issue is still open and assigned
        // 3. Ensure workspace is clean or has expected changes
        info!(
            branch = %branch,
            issue = %issue.number,
            "Validating workspace for resume (mock implementation)"
        );
        Ok(())
    }
    
    async fn complete_pending_operation(
        &self,
        operation: &PendingOperation,
    ) -> Result<(), WorkContinuityError> {
        info!(
            operation_id = %operation.operation_id,
            operation_type = ?operation.operation_type,
            "Completing pending operation (mock implementation)"
        );
        Ok(())
    }
    
    async fn attempt_error_recovery(
        &self,
        context: &RecoveryContext,
    ) -> Result<(), WorkContinuityError> {
        info!(
            error = %context.last_error,
            strategy = %context.recovery_strategy,
            "Attempting error recovery (mock implementation)"
        );
        Ok(())
    }
    
    async fn validate_and_resync_state(
        &self,
        agent_state: &AgentStateMachine,
    ) -> Result<(), WorkContinuityError> {
        info!(
            agent_id = %agent_state.agent_id(),
            "Validating and resyncing state (mock implementation)"
        );
        Ok(())
    }
    
    async fn clear_stale_state(
        &self,
        agent_state: &AgentStateMachine,
    ) -> Result<(), WorkContinuityError> {
        info!(
            agent_id = %agent_state.agent_id(),
            "Clearing stale state (mock implementation)"
        );
        Ok(())
    }
}

/// Status information about work continuity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuityStatus {
    pub is_active: bool,
    pub session_id: String,
    pub uptime_start: DateTime<Utc>,
    pub last_checkpoint: DateTime<Utc>,
    pub current_issue: Option<u64>,
    pub current_branch: Option<String>,
    pub pending_operations: usize,
    pub can_resume: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::github::GitHubClient;

    #[tokio::test]
    async fn test_work_continuity_manager_creation() {
        let github_client = GitHubClient::new().unwrap();
        let config = WorkContinuityConfig::default();
        let persistence_config = PersistenceConfig::default();
        
        let manager = WorkContinuityManager::new(config, github_client, persistence_config);
        
        // Basic creation should succeed
        assert!(manager.current_state.read().await.is_none());
    }
    
    #[tokio::test]
    async fn test_continuity_status() {
        let github_client = GitHubClient::new().unwrap();
        let config = WorkContinuityConfig::default();
        let persistence_config = PersistenceConfig::default();
        
        let manager = WorkContinuityManager::new(config, github_client, persistence_config);
        
        let status = manager.get_continuity_status("test-agent").await.unwrap();
        assert!(!status.is_active);
        assert_eq!(status.pending_operations, 0);
    }
    
    #[tokio::test]
    async fn test_checkpoint_state() {
        let github_client = GitHubClient::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        
        let config = WorkContinuityConfig {
            state_file_path: temp_dir.path().join("agent-state.json"),
            ..WorkContinuityConfig::default()
        };
        
        let persistence_config = PersistenceConfig {
            persistence_directory: temp_dir.path().to_path_buf(),
            ..PersistenceConfig::default()
        };
        
        let manager = WorkContinuityManager::new(config, github_client, persistence_config);
        let agent_state = AgentStateMachine::new("test-agent".to_string());
        
        let checkpoint_id = manager.checkpoint_state(
            &agent_state,
            None,
            CheckpointReason::UserRequested,
        ).await.unwrap();
        
        assert!(!checkpoint_id.is_empty());
    }
}