use serde::{Deserialize, Serialize};
use thiserror::Error;
use chrono::{DateTime, Utc, Duration};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{info, warn, error, debug};

use crate::github::{GitHubClient, errors::GitHubError};
use super::AutonomousWorkflowState;

/// State drift detection and auto-correction system for long-running operations
#[derive(Debug)]
pub struct StateDriftDetector {
    /// Monitoring configuration
    validation_interval: Duration,
    drift_thresholds: DriftThresholds,
    correction_strategies: Vec<CorrectionStrategy>,
    
    /// State tracking
    expected_state: ExpectedSystemState,
    last_validation: DateTime<Utc>,
    detected_drifts: Vec<StateDrift>,
    
    /// GitHub client for validation
    github_client: GitHubClient,
    
    /// Agent context
    agent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftThresholds {
    /// Maximum time between validations during active operations (minutes)
    pub max_validation_interval_active: i64,
    /// Maximum time between validations during idle periods (minutes)
    pub max_validation_interval_idle: i64,
    /// Maximum allowed divergence for branch commits before auto-correction
    pub max_commits_behind: u32,
    /// Critical drift types that require immediate correction
    pub critical_drift_types: Vec<StateDriftType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CorrectionStrategy {
    /// Trust GitHub state over local expectations
    GitHubAuthoritative,
    /// Preserve agent work at all costs during corrections
    WorkPreserving,
    /// Create issues for manual intervention but continue autonomously  
    EscalateAndContinue,
    /// Require manual intervention for complex scenarios
    RequireManualIntervention,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedSystemState {
    /// Expected issue assignments and states
    pub issues: HashMap<u64, ExpectedIssueState>,
    /// Expected branch states  
    pub branches: HashMap<String, ExpectedBranchState>,
    /// Expected PR states
    pub pull_requests: HashMap<u64, ExpectedPRState>,
    /// Expected local workspace state
    pub workspace: ExpectedWorkspaceState,
    /// Last update timestamp
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedIssueState {
    pub issue_id: u64,
    pub title: String,
    pub assignee: Option<String>,
    pub labels: Vec<String>,
    pub state: IssueState,
    pub last_modified: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedBranchState {
    pub branch_name: String,
    pub head_commit: String,
    pub exists: bool,
    pub behind_main: Option<u32>,
    pub ahead_main: Option<u32>,
    pub last_push: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]  
pub struct ExpectedPRState {
    pub pr_id: u64,
    pub title: String,
    pub state: PRState,
    pub head_branch: String,
    pub base_branch: String,
    pub review_state: ReviewState,
    pub mergeable: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedWorkspaceState {
    pub current_branch: String,
    pub uncommitted_changes: bool,
    pub head_commit: String,
    pub tracked_files: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IssueState {
    Open,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PRState {
    Open,
    Closed,
    Merged,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReviewState {
    Pending,
    Approved,
    ChangesRequested,
    Dismissed,
}

/// Detected state drift between expected and actual states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateDrift {
    /// Issue state drifts
    IssueUnexpectedlyAssigned { 
        issue_id: u64, 
        expected_assignee: Option<String>,
        actual_assignee: String,
        severity: DriftSeverity,
    },
    IssueUnexpectedlyClosed { 
        issue_id: u64, 
        closer: String,
        closed_at: DateTime<Utc>,
        severity: DriftSeverity,
    },
    LabelsChanged { 
        issue_id: u64, 
        expected: Vec<String>, 
        actual: Vec<String>,
        severity: DriftSeverity,
    },
    
    /// Branch state drifts  
    BranchDeleted { 
        branch_name: String,
        severity: DriftSeverity,
    },
    BranchDiverged { 
        branch_name: String, 
        commits_behind: u32,
        commits_ahead: u32,
        severity: DriftSeverity,
    },
    UnexpectedBranch { 
        branch_name: String, 
        creator: String,
        created_at: DateTime<Utc>,
        severity: DriftSeverity,
    },
    
    /// PR state drifts
    PRUnexpectedlyMerged { 
        pr_id: u64, 
        merger: String,
        merged_at: DateTime<Utc>,
        severity: DriftSeverity,
    },
    PRUnexpectedlyClosed { 
        pr_id: u64, 
        closer: String,
        closed_at: DateTime<Utc>,
        severity: DriftSeverity,
    },
    ReviewStateChanged { 
        pr_id: u64, 
        expected_state: ReviewState,
        new_state: ReviewState,
        reviewer: String,
        severity: DriftSeverity,
    },
    
    /// Workspace drifts
    WorkspaceFileChanges { 
        modified_files: Vec<PathBuf>,
        severity: DriftSeverity,
    },
    GitStateInconsistent { 
        expected_head: String, 
        actual_head: String,
        severity: DriftSeverity,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StateDriftType {
    IssueUnexpectedlyAssigned,
    IssueUnexpectedlyClosed,
    LabelsChanged,
    BranchDeleted,
    BranchDiverged,
    UnexpectedBranch,
    PRUnexpectedlyMerged,
    PRUnexpectedlyClosed,
    ReviewStateChanged,
    WorkspaceFileChanges,
    GitStateInconsistent,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum DriftSeverity {
    /// Minor drifts with no impact on agent operation
    Minor,
    /// Moderate drifts that affect productivity but are recoverable
    Moderate,
    /// Critical drifts that threaten work preservation or agent safety
    Critical,
}

/// Actions that can be taken to correct detected drifts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CorrectionAction {
    /// Safe corrections (no work loss)
    UpdateLocalState { 
        new_state: SystemStateField,
        reason: String,
    },
    SynchronizeWithGitHub { 
        fields: Vec<StateField>,
        backup_created: bool,
    },
    RefreshCachedData { 
        cache_keys: Vec<String> 
    },
    
    /// Work-preserving corrections
    PreserveWorkAndResync { 
        backup_location: String,
        sync_strategy: SyncStrategy,
    },
    CreateRecoveryBranch { 
        original_work: WorkSnapshot,
        recovery_branch: String,
    },
    
    /// Escalation with preservation
    DocumentDriftAndContinue { 
        drift_report: DriftReport,
        continue_autonomously: bool,
    },
    CreateDriftIssue { 
        affected_work: Option<WorkContext>,
        issue_number: u64,
    },
    
    /// Terminal (rare)
    RequireManualIntervention { 
        reason: String, 
        preserved_state: StateSnapshot,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemStateField {
    Issue(ExpectedIssueState),
    Branch(ExpectedBranchState),
    PullRequest(ExpectedPRState),
    Workspace(ExpectedWorkspaceState),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateField {
    IssueAssignment,
    IssueLabels,
    BranchHead,
    PRState,
    ReviewState,
    WorkspaceFiles,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncStrategy {
    GitHubWins,
    LocalWins,
    MergeStrategic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkSnapshot {
    pub commits: Vec<String>,
    pub modified_files: HashMap<PathBuf, String>, // file path -> content hash
    pub branch_name: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftReport {
    pub drift_id: String,
    pub detected_at: DateTime<Utc>,
    pub drift_type: StateDriftType,
    pub severity: DriftSeverity,
    pub description: String,
    pub correction_taken: Option<String>, // Changed to String to avoid circular dependency
    pub resolution_time: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkContext {
    pub issue_number: u64,
    pub branch_name: String,
    pub commits: Vec<String>,
    pub files_modified: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub timestamp: DateTime<Utc>,
    pub expected_state: ExpectedSystemState,
    pub actual_state: HashMap<String, serde_json::Value>,
    pub agent_context: AgentContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContext {
    pub agent_id: String,
    pub current_workflow_state: Option<String>, // Serialized AutonomousWorkflowState
    pub active_operations: Vec<String>,
}

#[derive(Error, Debug)]
pub enum StateDriftError {
    #[error("GitHub API error: {0}")]
    GitHubError(#[from] GitHubError),
    
    #[error("State validation failed: {message}")]
    ValidationFailed { message: String },
    
    #[error("Correction strategy failed: {strategy:?}, reason: {reason}")]
    CorrectionFailed { 
        strategy: CorrectionStrategy,
        reason: String 
    },
    
    #[error("Critical drift detected requiring manual intervention: {drift_type:?}")]
    CriticalDriftDetected { drift_type: StateDriftType },
    
    #[error("Workspace inconsistency: {details}")]
    WorkspaceInconsistent { details: String },
}

impl Default for DriftThresholds {
    fn default() -> Self {
        Self {
            max_validation_interval_active: 5,  // 5 minutes during active work
            max_validation_interval_idle: 30,   // 30 minutes during idle
            max_commits_behind: 10,
            critical_drift_types: vec![
                StateDriftType::IssueUnexpectedlyClosed,
                StateDriftType::BranchDeleted,
                StateDriftType::PRUnexpectedlyMerged,
                StateDriftType::GitStateInconsistent,
            ],
        }
    }
}

impl StateDriftDetector {
    /// Create new state drift detector
    pub fn new(
        github_client: GitHubClient,
        agent_id: String,
    ) -> Self {
        Self {
            validation_interval: Duration::minutes(5),
            drift_thresholds: DriftThresholds::default(),
            correction_strategies: vec![
                CorrectionStrategy::WorkPreserving,
                CorrectionStrategy::GitHubAuthoritative,
                CorrectionStrategy::EscalateAndContinue,
            ],
            expected_state: ExpectedSystemState {
                issues: HashMap::new(),
                branches: HashMap::new(),
                pull_requests: HashMap::new(),
                workspace: ExpectedWorkspaceState {
                    current_branch: "main".to_string(),
                    uncommitted_changes: false,
                    head_commit: String::new(),
                    tracked_files: vec![],
                },
                last_updated: Utc::now(),
            },
            last_validation: Utc::now() - Duration::hours(1), // Force initial validation
            detected_drifts: Vec::new(),
            github_client,
            agent_id,
        }
    }

    /// Configure validation interval based on operation mode
    pub fn with_validation_interval(mut self, interval: Duration) -> Self {
        self.validation_interval = interval;
        self
    }

    /// Configure drift thresholds
    pub fn with_drift_thresholds(mut self, thresholds: DriftThresholds) -> Self {
        self.drift_thresholds = thresholds;
        self
    }

    /// Configure correction strategies
    pub fn with_correction_strategies(mut self, strategies: Vec<CorrectionStrategy>) -> Self {
        self.correction_strategies = strategies;
        self
    }

    /// Update expected state from current workflow state
    pub async fn update_expected_state(
        &mut self, 
        workflow_state: &AutonomousWorkflowState
    ) -> Result<(), StateDriftError> {
        debug!(
            agent_id = %self.agent_id,
            "Updating expected state from workflow state"
        );

        match workflow_state {
            AutonomousWorkflowState::Assigned { issue, agent, workspace: _ } => {
                self.expected_state.issues.insert(
                    issue.number,
                    ExpectedIssueState {
                        issue_id: issue.number,
                        title: issue.title.clone(),
                        assignee: Some(agent.0.clone()),
                        labels: issue.labels.clone(),
                        state: IssueState::Open,
                        last_modified: Utc::now(),
                    }
                );
            }
            AutonomousWorkflowState::ReadyForReview {  pr, .. } => {
                self.expected_state.pull_requests.insert(
                    pr.number,
                    ExpectedPRState {
                        pr_id: pr.number,
                        title: pr.title.clone(),
                        state: PRState::Open,
                        head_branch: pr.branch.clone(),
                        base_branch: "main".to_string(),
                        review_state: ReviewState::Pending,
                        mergeable: None,
                    }
                );
            }
            // Add more state mappings as needed
            _ => {}
        }

        self.expected_state.last_updated = Utc::now();
        Ok(())
    }

    /// Check if validation is needed based on timing and activity
    pub fn needs_validation(&self, is_active: bool) -> bool {
        let now = Utc::now();
        let time_since_last = now - self.last_validation;
        
        let threshold_minutes = if is_active {
            self.drift_thresholds.max_validation_interval_active
        } else {
            self.drift_thresholds.max_validation_interval_idle
        };

        time_since_last > Duration::minutes(threshold_minutes)
    }

    /// Perform comprehensive state validation
    pub async fn validate_state(&mut self) -> Result<Vec<StateDrift>, StateDriftError> {
        info!(
            agent_id = %self.agent_id,
            "Starting comprehensive state validation"
        );

        let mut detected_drifts = Vec::new();

        // Validate issue states
        detected_drifts.extend(self.validate_issue_states().await?);

        // Validate branch states  
        detected_drifts.extend(self.validate_branch_states().await?);

        // Validate PR states
        detected_drifts.extend(self.validate_pr_states().await?);

        // Validate workspace state
        detected_drifts.extend(self.validate_workspace_state().await?);

        self.last_validation = Utc::now();
        self.detected_drifts.extend(detected_drifts.clone());

        info!(
            agent_id = %self.agent_id,
            drifts_count = detected_drifts.len(),
            "State validation completed"
        );

        Ok(detected_drifts)
    }

    /// Validate issue states against expectations
    async fn validate_issue_states(&self) -> Result<Vec<StateDrift>, StateDriftError> {
        let mut drifts = Vec::new();

        for (issue_id, expected) in &self.expected_state.issues {
            match self.github_client.issues.fetch_issue(*issue_id).await {
                Ok(actual_issue) => {
                    // Check assignment drift
                    let actual_assignee = actual_issue.assignee
                        .as_ref()
                        .map(|a| a.login.clone());
                    
                    if expected.assignee != actual_assignee {
                        let severity = if actual_assignee.is_some() 
                            && actual_assignee.as_ref() != Some(&self.agent_id) {
                            DriftSeverity::Critical
                        } else {
                            DriftSeverity::Moderate
                        };

                        drifts.push(StateDrift::IssueUnexpectedlyAssigned {
                            issue_id: *issue_id,
                            expected_assignee: expected.assignee.clone(),
                            actual_assignee: actual_assignee.unwrap_or("none".to_string()),
                            severity,
                        });
                    }

                    // Check state drift (open/closed)
                    let actual_state = if matches!(actual_issue.state, octocrab::models::IssueState::Closed) {
                        IssueState::Closed
                    } else {
                        IssueState::Open
                    };

                    if expected.state != actual_state && actual_state == IssueState::Closed {
                        drifts.push(StateDrift::IssueUnexpectedlyClosed {
                            issue_id: *issue_id,
                            closer: actual_issue.closed_by
                                .as_ref()
                                .map(|u| u.login.clone())
                                .unwrap_or("unknown".to_string()),
                            closed_at: actual_issue.closed_at.unwrap_or(Utc::now()),
                            severity: DriftSeverity::Critical,
                        });
                    }

                    // Check label drift
                    let actual_labels: Vec<String> = actual_issue.labels
                        .iter()
                        .map(|l| l.name.clone())
                        .collect();
                    
                    if expected.labels != actual_labels {
                        let severity = if actual_labels.contains(&self.agent_id) 
                            != expected.labels.contains(&self.agent_id) {
                            DriftSeverity::Moderate
                        } else {
                            DriftSeverity::Minor
                        };

                        drifts.push(StateDrift::LabelsChanged {
                            issue_id: *issue_id,
                            expected: expected.labels.clone(),
                            actual: actual_labels,
                            severity,
                        });
                    }
                }
                Err(e) => {
                    warn!(
                        agent_id = %self.agent_id,
                        issue_id = *issue_id,
                        error = ?e,
                        "Failed to fetch issue for validation"
                    );
                }
            }
        }

        Ok(drifts)
    }

    /// Validate branch states against expectations  
    async fn validate_branch_states(&self) -> Result<Vec<StateDrift>, StateDriftError> {
        let mut drifts = Vec::new();

        for (branch_name, expected) in &self.expected_state.branches {
            match self.github_client.branches.get_branch_info(branch_name).await {
                Ok(actual_branch) => {
                    // Check head commit drift
                    let actual_head = actual_branch.sha.clone();
                    if expected.head_commit != actual_head {
                        // Determine if we're behind, ahead, or diverged
                        // This would require more complex git analysis in practice
                        drifts.push(StateDrift::BranchDiverged {
                            branch_name: branch_name.clone(),
                            commits_behind: 0, // Would be calculated
                            commits_ahead: 0,  // Would be calculated  
                            severity: DriftSeverity::Moderate,
                        });
                    }
                }
                Err(e) => {
                    // Branch doesn't exist - check if we expected it to exist
                    if expected.exists {
                        drifts.push(StateDrift::BranchDeleted {
                            branch_name: branch_name.clone(),
                            severity: DriftSeverity::Critical,
                        });
                    } else {
                        warn!(
                            agent_id = %self.agent_id,
                            branch_name = branch_name,
                            error = ?e,
                            "Failed to fetch branch for validation"
                        );
                    }
                }
            }
        }

        Ok(drifts)
    }

    /// Validate PR states against expectations
    async fn validate_pr_states(&self) -> Result<Vec<StateDrift>, StateDriftError> {
        let mut drifts = Vec::new();

        for (pr_id, expected) in &self.expected_state.pull_requests {
            match self.github_client.pulls.get_pull_request(*pr_id).await {
                Ok(actual_pr) => {
                    let actual_state = if actual_pr.merged_at.is_some() {
                        PRState::Merged
                    } else if actual_pr.closed_at.is_some() {
                        PRState::Closed
                    } else {
                        PRState::Open
                    };

                    // Check for unexpected merge/close
                    if expected.state != actual_state {
                        match actual_state {
                            PRState::Merged => {
                                drifts.push(StateDrift::PRUnexpectedlyMerged {
                                    pr_id: *pr_id,
                                    merger: "unknown".to_string(), // Merged by info not available in this API
                                    merged_at: actual_pr.merged_at.unwrap_or(Utc::now()),
                                    severity: DriftSeverity::Critical,
                                });
                            }
                            PRState::Closed => {
                                drifts.push(StateDrift::PRUnexpectedlyClosed {
                                    pr_id: *pr_id,
                                    closer: "unknown".to_string(), // Closed by info not available in this API
                                    closed_at: actual_pr.closed_at.unwrap_or(Utc::now()),
                                    severity: DriftSeverity::Moderate,
                                });
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        agent_id = %self.agent_id,
                        pr_id = *pr_id,
                        error = ?e,
                        "Failed to fetch PR for validation"
                    );
                }
            }
        }

        Ok(drifts)
    }

    /// Validate local workspace state
    async fn validate_workspace_state(&self) -> Result<Vec<StateDrift>, StateDriftError> {
        let mut drifts = Vec::new();

        // Check git status for uncommitted changes
        if let Ok(output) = tokio::process::Command::new("git")
            .args(&["status", "--porcelain"])
            .output()
            .await
        {
            let has_changes = !output.stdout.is_empty();
            if has_changes != self.expected_state.workspace.uncommitted_changes {
                let modified_files = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .map(|line| PathBuf::from(line.trim_start_matches(|c: char| !c.is_whitespace())))
                    .collect();

                drifts.push(StateDrift::WorkspaceFileChanges {
                    modified_files,
                    severity: DriftSeverity::Minor,
                });
            }
        }

        // Check current HEAD
        if let Ok(output) = tokio::process::Command::new("git")
            .args(&["rev-parse", "HEAD"])
            .output()
            .await
        {
            let actual_head = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if actual_head != self.expected_state.workspace.head_commit 
                && !self.expected_state.workspace.head_commit.is_empty() {
                drifts.push(StateDrift::GitStateInconsistent {
                    expected_head: self.expected_state.workspace.head_commit.clone(),
                    actual_head,
                    severity: DriftSeverity::Moderate,
                });
            }
        }

        Ok(drifts)
    }

    /// Execute automatic correction for detected drifts
    pub async fn correct_drifts(&mut self, drifts: Vec<StateDrift>) -> Result<Vec<CorrectionAction>, StateDriftError> {
        let mut corrections = Vec::new();

        for drift in drifts {
            let severity = drift.get_severity();
            
            info!(
                agent_id = %self.agent_id,
                drift_type = ?drift.get_type(),
                severity = ?severity,
                "Attempting to correct detected drift"
            );

            let correction = self.correct_single_drift(drift).await?;
            corrections.push(correction);
        }

        Ok(corrections)
    }

    /// Correct a single detected drift
    async fn correct_single_drift(&mut self, drift: StateDrift) -> Result<CorrectionAction, StateDriftError> {
        match drift {
            StateDrift::IssueUnexpectedlyAssigned { issue_id, ref actual_assignee, severity, .. } => {
                if severity == DriftSeverity::Critical && actual_assignee != &self.agent_id {
                    // Another agent/person took our issue - escalate but preserve work
                    let work_context = WorkContext {
                        issue_number: issue_id,
                        branch_name: format!("{}/{}", self.agent_id, issue_id),
                        commits: vec![], // Would be populated from git
                        files_modified: vec![], // Would be populated from git status
                    };

                    let issue_number = self.create_drift_issue(
                        &format!("Issue #{} unexpectedly reassigned to {}", issue_id, actual_assignee),
                        &drift,
                        Some(work_context.clone())
                    ).await?;

                    Ok(CorrectionAction::CreateDriftIssue {
                        affected_work: Some(work_context),
                        issue_number,
                    })
                } else {
                    // Minor assignment change - update our expectations
                    if let Some(expected_issue) = self.expected_state.issues.get_mut(&issue_id) {
                        expected_issue.assignee = Some(actual_assignee.clone());
                        expected_issue.last_modified = Utc::now();
                    }

                    Ok(CorrectionAction::UpdateLocalState {
                        new_state: SystemStateField::Issue(
                            self.expected_state.issues[&issue_id].clone()
                        ),
                        reason: format!("Issue assigned to {}", actual_assignee),
                    })
                }
            }

            StateDrift::IssueUnexpectedlyClosed { issue_id, closer, .. } => {
                // Critical: Issue was closed while we were working on it
                let preserved_state = self.create_state_snapshot().await;
                
                Ok(CorrectionAction::RequireManualIntervention {
                    reason: format!("Issue #{} was closed by {} while agent was working", issue_id, closer),
                    preserved_state,
                })
            }

            StateDrift::BranchDeleted { branch_name, .. } => {
                // Critical: Our work branch was deleted
                Ok(CorrectionAction::RequireManualIntervention {
                    reason: format!("Work branch '{}' was deleted", branch_name),
                    preserved_state: self.create_state_snapshot().await,
                })
            }

            StateDrift::LabelsChanged { issue_id, actual, .. } => {
                // Update local expectations to match GitHub
                if let Some(expected_issue) = self.expected_state.issues.get_mut(&issue_id) {
                    expected_issue.labels = actual.clone();
                    expected_issue.last_modified = Utc::now();
                }

                Ok(CorrectionAction::SynchronizeWithGitHub {
                    fields: vec![StateField::IssueLabels],
                    backup_created: false,
                })
            }

            StateDrift::WorkspaceFileChanges { modified_files, .. } => {
                // Document the changes but continue
                Ok(CorrectionAction::DocumentDriftAndContinue {
                    drift_report: DriftReport {
                        drift_id: uuid::Uuid::new_v4().to_string(),
                        detected_at: Utc::now(),
                        drift_type: StateDriftType::WorkspaceFileChanges,
                        severity: DriftSeverity::Minor,
                        description: format!("Workspace files changed: {:?}", modified_files),
                        correction_taken: Some("DocumentDriftAndContinue".to_string()),
                        resolution_time: None,
                    },
                    continue_autonomously: true,
                })
            }

            // Handle other drift types...
            _ => {
                Ok(CorrectionAction::DocumentDriftAndContinue {
                    drift_report: DriftReport {
                        drift_id: uuid::Uuid::new_v4().to_string(),
                        detected_at: Utc::now(),
                        drift_type: drift.get_type(),
                        severity: drift.get_severity(),
                        description: format!("Unhandled drift: {:?}", drift),
                        correction_taken: Some("DocumentDriftAndContinue".to_string()),
                        resolution_time: None,
                    },
                    continue_autonomously: true,
                })
            }
        }
    }

    /// Create a GitHub issue to document drift that needs manual attention
    async fn create_drift_issue(
        &self,
        title: &str,
        drift: &StateDrift,
        affected_work: Option<WorkContext>,
    ) -> Result<u64, StateDriftError> {
        let body = format!(
            "## State Drift Detected\n\n\
             **Type**: {:?}\n\
             **Severity**: {:?}\n\
             **Agent**: {}\n\
             **Detected At**: {}\n\n\
             ### Details\n\
             {:?}\n\n\
             {}",
            drift.get_type(),
            drift.get_severity(),
            self.agent_id,
            Utc::now(),
            drift,
            affected_work
                .map(|w| format!("### Affected Work\n- Issue: #{}\n- Branch: {}\n- Files: {:?}", 
                               w.issue_number, w.branch_name, w.files_modified))
                .unwrap_or_default()
        );

        let labels = vec![
            "drift-detection".to_string(),
            "autonomous-agent".to_string(),
            format!("severity-{:?}", drift.get_severity()).to_lowercase(),
        ];

        let created_issue = self.github_client.issues.create_issue(title, &body, labels).await?;
        
        info!(
            agent_id = %self.agent_id,
            issue_number = created_issue.number,
            "Created drift tracking issue"
        );

        Ok(created_issue.number)
    }

    /// Create comprehensive state snapshot for preservation
    async fn create_state_snapshot(&self) -> StateSnapshot {
        StateSnapshot {
            timestamp: Utc::now(),
            expected_state: self.expected_state.clone(),
            actual_state: HashMap::new(), // Would be populated with actual GitHub state
            agent_context: AgentContext {
                agent_id: self.agent_id.clone(),
                current_workflow_state: None, // Would be serialized from actual state
                active_operations: vec![], // Would be populated from agent state
            },
        }
    }

    /// Get all detected drifts
    pub fn get_detected_drifts(&self) -> &[StateDrift] {
        &self.detected_drifts
    }

    /// Clear resolved drifts
    pub fn clear_resolved_drifts(&mut self) {
        self.detected_drifts.clear();
    }

    /// Check if there are any critical drifts that require immediate attention
    pub fn has_critical_drifts(&self) -> bool {
        self.detected_drifts.iter().any(|d| d.get_severity() == DriftSeverity::Critical)
    }

    /// Generate drift detection report
    pub fn generate_drift_report(&self) -> DriftDetectionReport {
        DriftDetectionReport {
            agent_id: self.agent_id.clone(),
            total_drifts_detected: self.detected_drifts.len(),
            critical_drifts: self.detected_drifts.iter()
                .filter(|d| d.get_severity() == DriftSeverity::Critical)
                .cloned()
                .collect(),
            last_validation: self.last_validation,
            next_validation: self.last_validation + self.validation_interval,
            validation_health: if self.detected_drifts.is_empty() {
                ValidationHealth::Healthy
            } else if self.has_critical_drifts() {
                ValidationHealth::Critical
            } else {
                ValidationHealth::Warning
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftDetectionReport {
    pub agent_id: String,
    pub total_drifts_detected: usize,
    pub critical_drifts: Vec<StateDrift>,
    pub last_validation: DateTime<Utc>,
    pub next_validation: DateTime<Utc>,
    pub validation_health: ValidationHealth,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ValidationHealth {
    Healthy,
    Warning,
    Critical,
}

// Helper trait implementations
impl StateDrift {
    pub fn get_type(&self) -> StateDriftType {
        match self {
            StateDrift::IssueUnexpectedlyAssigned { .. } => StateDriftType::IssueUnexpectedlyAssigned,
            StateDrift::IssueUnexpectedlyClosed { .. } => StateDriftType::IssueUnexpectedlyClosed,
            StateDrift::LabelsChanged { .. } => StateDriftType::LabelsChanged,
            StateDrift::BranchDeleted { .. } => StateDriftType::BranchDeleted,
            StateDrift::BranchDiverged { .. } => StateDriftType::BranchDiverged,
            StateDrift::UnexpectedBranch { .. } => StateDriftType::UnexpectedBranch,
            StateDrift::PRUnexpectedlyMerged { .. } => StateDriftType::PRUnexpectedlyMerged,
            StateDrift::PRUnexpectedlyClosed { .. } => StateDriftType::PRUnexpectedlyClosed,
            StateDrift::ReviewStateChanged { .. } => StateDriftType::ReviewStateChanged,
            StateDrift::WorkspaceFileChanges { .. } => StateDriftType::WorkspaceFileChanges,
            StateDrift::GitStateInconsistent { .. } => StateDriftType::GitStateInconsistent,
        }
    }

    pub fn get_severity(&self) -> DriftSeverity {
        match self {
            StateDrift::IssueUnexpectedlyAssigned { severity, .. } |
            StateDrift::IssueUnexpectedlyClosed { severity, .. } |
            StateDrift::LabelsChanged { severity, .. } |
            StateDrift::BranchDeleted { severity, .. } |
            StateDrift::BranchDiverged { severity, .. } |
            StateDrift::UnexpectedBranch { severity, .. } |
            StateDrift::PRUnexpectedlyMerged { severity, .. } |
            StateDrift::PRUnexpectedlyClosed { severity, .. } |
            StateDrift::ReviewStateChanged { severity, .. } |
            StateDrift::WorkspaceFileChanges { severity, .. } |
            StateDrift::GitStateInconsistent { severity, .. } => *severity,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_drift_detector_creation() {
        let github_client = GitHubClient::new().unwrap();
        let detector = StateDriftDetector::new(github_client, "test-agent".to_string());
        
        assert_eq!(detector.agent_id, "test-agent");
        assert!(!detector.has_critical_drifts());
    }

    #[tokio::test]
    async fn test_needs_validation_timing() {
        let github_client = GitHubClient::new().unwrap();
        let mut detector = StateDriftDetector::new(github_client, "test-agent".to_string());
        
        // Should need validation initially (last_validation is 1 hour ago)
        assert!(detector.needs_validation(true));
        assert!(detector.needs_validation(false));
        
        // After updating last_validation to now
        detector.last_validation = Utc::now();
        assert!(!detector.needs_validation(true));
        assert!(!detector.needs_validation(false));
    }

    #[test]
    fn test_drift_severity_classification() {
        let critical_drift = StateDrift::IssueUnexpectedlyClosed {
            issue_id: 123,
            closer: "someone".to_string(),
            closed_at: Utc::now(),
            severity: DriftSeverity::Critical,
        };

        assert_eq!(critical_drift.get_severity(), DriftSeverity::Critical);
        assert_eq!(critical_drift.get_type(), StateDriftType::IssueUnexpectedlyClosed);
    }
}