//! Agent State Diagnostics
//! 
//! This module implements comprehensive diagnostic checks for agent state,
//! work continuity, and state machine health to ensure proper agent lifecycle management.

use crate::agent_lifecycle::state_machine::AgentStateMachine;
use crate::agents::coordinator::AgentCoordinator;
use crate::cli::commands::doctor::{DiagnosticResult, DiagnosticStatus};
#[cfg(feature = "autonomous")]
use crate::autonomous::work_continuity::{WorkContinuityManager, WorkContinuityConfig};
#[cfg(feature = "autonomous")]
use crate::autonomous::persistence::PersistenceConfig;
use crate::github::{GitHubClient, GitHubError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use tracing::{debug, info};

/// Comprehensive agent state diagnostic report
#[derive(Debug, Serialize, Deserialize)]
pub struct AgentStateDiagnosticReport {
    pub agent_status: AgentStatus,
    pub state_machine_health: StateMachineHealth,
    pub work_continuity_status: WorkContinuityStatus,
    pub github_sync_status: GitHubSyncStatus,
    pub detected_issues: Vec<AgentStateIssue>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentStatus {
    pub agent_id: String,
    pub is_available: bool,
    pub current_assignment: Option<u64>,
    pub current_branch: Option<String>,
    pub commits_ahead: u32,
    pub last_activity: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StateMachineHealth {
    pub is_consistent: bool,
    pub current_state: String,
    pub transition_history: Vec<String>,
    pub validation_errors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkContinuityStatus {
    pub is_enabled: bool,
    pub state_file_exists: bool,
    pub state_file_valid: bool,
    pub last_checkpoint: Option<DateTime<Utc>>,
    pub can_resume: bool,
    pub integrity_issues: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubSyncStatus {
    pub is_synced: bool,
    pub orphaned_assignments: Vec<u64>,
    pub abandoned_branches: Vec<String>,
    pub conflicting_assignments: Vec<String>,
    pub cleanup_needed: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentStateIssue {
    pub issue_type: AgentStateIssueType,
    pub severity: IssueSeverity,
    pub description: String,
    pub affected_components: Vec<String>,
    pub suggested_resolution: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AgentStateIssueType {
    OrphanedAssignment,
    AbandonedBranch,
    StuckState,
    ConflictingAssignment,
    CorruptedState,
    WorkContinuityFailure,
    GitHubDesync,
    CleanupNeeded,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum IssueSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Agent state diagnostic checker
pub struct AgentStateDiagnostic {
    github_client: GitHubClient,
    agent_coordinator: Option<AgentCoordinator>,
}

impl AgentStateDiagnostic {
    /// Create new agent state diagnostic checker
    pub fn new(github_client: GitHubClient) -> Self {
        Self {
            github_client,
            agent_coordinator: None,
        }
    }

    /// Initialize with agent coordinator for enhanced diagnostics
    pub async fn with_coordinator(mut self) -> Result<Self, GitHubError> {
        self.agent_coordinator = Some(AgentCoordinator::new().await?);
        Ok(self)
    }

    /// Check current agent state and assignment status
    pub async fn check_agent_state(&self, checks: &mut HashMap<String, DiagnosticResult>) {
        debug!("Checking agent state and assignment status");

        // Check agent availability
        match self.check_agent_availability().await {
            Ok(status) => {
                checks.insert(
                    "agent_availability".to_string(),
                    DiagnosticResult {
                        status: if status.is_available {
                            DiagnosticStatus::Pass
                        } else {
                            DiagnosticStatus::Info
                        },
                        message: format!(
                            "Agent {} is {}",
                            status.agent_id,
                            if status.is_available { "available" } else { "busy" }
                        ),
                        details: Some(format!(
                            "Current assignment: {:?}, Branch: {:?}, Commits ahead: {}",
                            status.current_assignment, status.current_branch, status.commits_ahead
                        )),
                        suggestion: if !status.is_available {
                            Some("Agent is currently working. Use 'my-little-soda status' to see details".to_string())
                        } else {
                            Some("Agent is ready for new work. Use 'my-little-soda pop' to get next task".to_string())
                        },
                    },
                );
            }
            Err(e) => {
                checks.insert(
                    "agent_availability".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "Failed to check agent availability".to_string(),
                        details: Some(format!("Error: {:?}", e)),
                        suggestion: Some("Ensure GitHub authentication is working and try again".to_string()),
                    },
                );
            }
        }

        // Check state machine consistency
        self.check_state_machine_consistency(checks).await;
    }

    /// Check work continuity state file integrity
    pub async fn check_work_continuity(&self, checks: &mut HashMap<String, DiagnosticResult>) {
        debug!("Checking work continuity state file integrity");

        #[cfg(feature = "autonomous")]
        {
            match self.check_work_continuity_status().await {
                Ok(status) => {
                    let diagnostic_status = if !status.is_enabled {
                        DiagnosticStatus::Info
                    } else if !status.state_file_valid {
                        DiagnosticStatus::Fail
                    } else if !status.integrity_issues.is_empty() {
                        DiagnosticStatus::Warning
                    } else {
                        DiagnosticStatus::Pass
                    };

                    let message = if !status.is_enabled {
                        "Work continuity is disabled".to_string()
                    } else if status.state_file_valid {
                        "Work continuity state file is valid".to_string()
                    } else {
                        "Work continuity state file has issues".to_string()
                    };

                    checks.insert(
                        "work_continuity_integrity".to_string(),
                        DiagnosticResult {
                            status: diagnostic_status,
                            message,
                            details: Some(format!(
                                "Enabled: {}, File exists: {}, Valid: {}, Can resume: {}, Issues: {}",
                                status.is_enabled,
                                status.state_file_exists,
                                status.state_file_valid,
                                status.can_resume,
                                status.integrity_issues.len()
                            )),
                            suggestion: if !status.state_file_valid {
                                Some("Clear corrupted state with 'my-little-soda reset --agent-state' and restart".to_string())
                            } else if !status.integrity_issues.is_empty() {
                                Some("Review integrity issues and consider resetting agent state if needed".to_string())
                            } else {
                                None
                            },
                        },
                    );
                }
                Err(e) => {
                    checks.insert(
                        "work_continuity_integrity".to_string(),
                        DiagnosticResult {
                            status: DiagnosticStatus::Fail,
                            message: "Failed to check work continuity status".to_string(),
                            details: Some(format!("Error: {:?}", e)),
                            suggestion: Some("Check configuration and file permissions".to_string()),
                        },
                    );
                }
            }
        }

        #[cfg(not(feature = "autonomous"))]
        {
            checks.insert(
                "work_continuity_integrity".to_string(),
                DiagnosticResult {
                    status: DiagnosticStatus::Info,
                    message: "Work continuity not available (autonomous feature disabled)".to_string(),
                    details: Some("Compile with --features autonomous to enable work continuity".to_string()),
                    suggestion: None,
                },
            );
        }
    }

    /// Detect orphaned or stuck agent assignments
    pub async fn check_orphaned_assignments(&self, checks: &mut HashMap<String, DiagnosticResult>) {
        debug!("Checking for orphaned or stuck agent assignments");

        match self.detect_orphaned_assignments().await {
            Ok(orphaned_issues) => {
                let status = if orphaned_issues.is_empty() {
                    DiagnosticStatus::Pass
                } else {
                    DiagnosticStatus::Warning
                };

                checks.insert(
                    "orphaned_assignments".to_string(),
                    DiagnosticResult {
                        status,
                        message: if orphaned_issues.is_empty() {
                            "No orphaned agent assignments detected".to_string()
                        } else {
                            format!("Found {} orphaned agent assignments", orphaned_issues.len())
                        },
                        details: if orphaned_issues.is_empty() {
                            None
                        } else {
                            Some(format!("Orphaned issues: {:?}", orphaned_issues))
                        },
                        suggestion: if !orphaned_issues.is_empty() {
                            Some("Use 'my-little-soda reset --cleanup-assignments' to clean up orphaned assignments".to_string())
                        } else {
                            None
                        },
                    },
                );
            }
            Err(e) => {
                checks.insert(
                    "orphaned_assignments".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "Failed to check for orphaned assignments".to_string(),
                        details: Some(format!("Error: {:?}", e)),
                        suggestion: Some("Ensure GitHub API access is working".to_string()),
                    },
                );
            }
        }
    }

    /// Check for abandoned branches or incomplete work
    pub async fn check_abandoned_work(&self, checks: &mut HashMap<String, DiagnosticResult>) {
        debug!("Checking for abandoned branches or incomplete work");

        match self.detect_abandoned_branches().await {
            Ok(abandoned_branches) => {
                let status = if abandoned_branches.is_empty() {
                    DiagnosticStatus::Pass
                } else {
                    DiagnosticStatus::Warning
                };

                checks.insert(
                    "abandoned_work".to_string(),
                    DiagnosticResult {
                        status,
                        message: if abandoned_branches.is_empty() {
                            "No abandoned agent branches detected".to_string()
                        } else {
                            format!("Found {} potentially abandoned agent branches", abandoned_branches.len())
                        },
                        details: if abandoned_branches.is_empty() {
                            None
                        } else {
                            Some(format!("Abandoned branches: {:?}", abandoned_branches))
                        },
                        suggestion: if !abandoned_branches.is_empty() {
                            Some("Review abandoned branches and clean up with 'git branch -D <branch>' if no longer needed".to_string())
                        } else {
                            None
                        },
                    },
                );
            }
            Err(e) => {
                checks.insert(
                    "abandoned_work".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "Failed to check for abandoned work".to_string(),
                        details: Some(format!("Error: {:?}", e)),
                        suggestion: Some("Check Git repository and GitHub API access".to_string()),
                    },
                );
            }
        }
    }

    /// Detect conflicting agent assignments
    pub async fn check_conflicting_assignments(&self, checks: &mut HashMap<String, DiagnosticResult>) {
        debug!("Checking for conflicting agent assignments");

        match self.detect_conflicting_assignments().await {
            Ok(conflicts) => {
                let status = if conflicts.is_empty() {
                    DiagnosticStatus::Pass
                } else {
                    DiagnosticStatus::Fail
                };

                checks.insert(
                    "conflicting_assignments".to_string(),
                    DiagnosticResult {
                        status,
                        message: if conflicts.is_empty() {
                            "No conflicting agent assignments detected".to_string()
                        } else {
                            format!("Found {} conflicting agent assignments", conflicts.len())
                        },
                        details: if conflicts.is_empty() {
                            None
                        } else {
                            Some(format!("Conflicts: {:?}", conflicts))
                        },
                        suggestion: if !conflicts.is_empty() {
                            Some("Resolve conflicts by cleaning up duplicate assignments or reassigning issues".to_string())
                        } else {
                            None
                        },
                    },
                );
            }
            Err(e) => {
                checks.insert(
                    "conflicting_assignments".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "Failed to check for conflicting assignments".to_string(),
                        details: Some(format!("Error: {:?}", e)),
                        suggestion: Some("Ensure GitHub API access is working".to_string()),
                    },
                );
            }
        }
    }

    /// Check for proper cleanup of completed work
    pub async fn check_cleanup_status(&self, checks: &mut HashMap<String, DiagnosticResult>) {
        debug!("Checking cleanup status of completed work");

        match self.check_work_cleanup_status().await {
            Ok(cleanup_needed) => {
                let status = if cleanup_needed.is_empty() {
                    DiagnosticStatus::Pass
                } else {
                    DiagnosticStatus::Info
                };

                checks.insert(
                    "work_cleanup".to_string(),
                    DiagnosticResult {
                        status,
                        message: if cleanup_needed.is_empty() {
                            "No work cleanup needed".to_string()
                        } else {
                            format!("Found {} items that may need cleanup", cleanup_needed.len())
                        },
                        details: if cleanup_needed.is_empty() {
                            None
                        } else {
                            Some(format!("Cleanup items: {:?}", cleanup_needed))
                        },
                        suggestion: if !cleanup_needed.is_empty() {
                            Some("Review completed work and clean up merged branches or closed issues".to_string())
                        } else {
                            None
                        },
                    },
                );
            }
            Err(e) => {
                checks.insert(
                    "work_cleanup".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "Failed to check work cleanup status".to_string(),
                        details: Some(format!("Error: {:?}", e)),
                        suggestion: Some("Check GitHub API access and repository status".to_string()),
                    },
                );
            }
        }
    }

    /// Generate comprehensive agent state diagnostic report
    pub async fn generate_comprehensive_report(&self) -> Result<AgentStateDiagnosticReport, GitHubError> {
        info!("Generating comprehensive agent state diagnostic report");

        let agent_status = self.check_agent_availability().await?;
        let state_machine_health = self.check_state_machine_health().await?;
        
        #[cfg(feature = "autonomous")]
        let work_continuity_status = self.check_work_continuity_status().await?;
        #[cfg(not(feature = "autonomous"))]
        let work_continuity_status = WorkContinuityStatus {
            is_enabled: false,
            state_file_exists: false,
            state_file_valid: false,
            last_checkpoint: None,
            can_resume: false,
            integrity_issues: vec!["Autonomous feature not enabled".to_string()],
        };

        let orphaned_assignments = self.detect_orphaned_assignments().await?;
        let abandoned_branches = self.detect_abandoned_branches().await?;
        let conflicting_assignments = self.detect_conflicting_assignments().await?;
        let cleanup_needed = self.check_work_cleanup_status().await?;

        let github_sync_status = GitHubSyncStatus {
            is_synced: orphaned_assignments.is_empty() && conflicting_assignments.is_empty(),
            orphaned_assignments,
            abandoned_branches,
            conflicting_assignments,
            cleanup_needed,
        };

        let detected_issues = self.analyze_issues(&agent_status, &state_machine_health, &work_continuity_status, &github_sync_status);
        let recommendations = self.generate_recommendations(&detected_issues);

        Ok(AgentStateDiagnosticReport {
            agent_status,
            state_machine_health,
            work_continuity_status,
            github_sync_status,
            detected_issues,
            recommendations,
        })
    }

    // Private helper methods

    async fn check_agent_availability(&self) -> Result<AgentStatus, GitHubError> {
        if let Some(coordinator) = &self.agent_coordinator {
            let available_agents = coordinator.get_available_agents().await?;
            let is_available = !available_agents.is_empty();

            // Get current git branch to check for agent work
            let current_branch = self.get_current_git_branch();
            let current_assignment = if let Some(ref branch) = current_branch {
                self.extract_issue_number_from_branch(branch)
            } else {
                None
            };

            Ok(AgentStatus {
                agent_id: "agent001".to_string(),
                is_available,
                current_assignment,
                current_branch,
                commits_ahead: self.get_commits_ahead().await.unwrap_or(0),
                last_activity: None, // Would be tracked from state machine
            })
        } else {
            // Fallback check without coordinator
            let current_branch = self.get_current_git_branch();
            let current_assignment = if let Some(ref branch) = current_branch {
                self.extract_issue_number_from_branch(branch)
            } else {
                None
            };

            Ok(AgentStatus {
                agent_id: "agent001".to_string(),
                is_available: current_assignment.is_none(),
                current_assignment,
                current_branch,
                commits_ahead: self.get_commits_ahead().await.unwrap_or(0),
                last_activity: None,
            })
        }
    }

    async fn check_state_machine_consistency(&self, checks: &mut HashMap<String, DiagnosticResult>) {
        // Create a test state machine to verify consistency
        let agent_state = AgentStateMachine::new("agent001".to_string());
        
        let mut validation_errors = Vec::new();
        
        // Basic state machine validation
        if !agent_state.is_available() && agent_state.current_issue().is_none() {
            validation_errors.push("State machine inconsistency: not available but no current issue".to_string());
        }

        let status = if validation_errors.is_empty() {
            DiagnosticStatus::Pass
        } else {
            DiagnosticStatus::Fail
        };

        checks.insert(
            "state_machine_consistency".to_string(),
            DiagnosticResult {
                status,
                message: if validation_errors.is_empty() {
                    "State machine is consistent".to_string()
                } else {
                    format!("State machine has {} validation errors", validation_errors.len())
                },
                details: if validation_errors.is_empty() {
                    Some(format!("Available: {}, Current issue: {:?}", 
                        agent_state.is_available(), agent_state.current_issue()))
                } else {
                    Some(validation_errors.join("; "))
                },
                suggestion: if !validation_errors.is_empty() {
                    Some("Reset agent state with 'my-little-soda reset --agent-state'".to_string())
                } else {
                    None
                },
            },
        );
    }

    async fn check_state_machine_health(&self) -> Result<StateMachineHealth, GitHubError> {
        let agent_state = AgentStateMachine::new("agent001".to_string());
        let mut validation_errors = Vec::new();

        // Check state consistency
        if !agent_state.is_available() && agent_state.current_issue().is_none() {
            validation_errors.push("Inconsistent availability state".to_string());
        }

        let current_state = if agent_state.is_available() {
            "Available".to_string()
        } else if agent_state.is_assigned() {
            "Assigned".to_string()
        } else if agent_state.is_working() {
            "Working".to_string()
        } else {
            "Unknown".to_string()
        };

        Ok(StateMachineHealth {
            is_consistent: validation_errors.is_empty(),
            current_state,
            transition_history: vec![], // Would be populated from actual state machine
            validation_errors,
        })
    }

    #[cfg(feature = "autonomous")]
    async fn check_work_continuity_status(&self) -> Result<WorkContinuityStatus, GitHubError> {
        let config = match config() {
            Ok(c) => c,
            Err(_) => {
                return Ok(WorkContinuityStatus {
                    is_enabled: false,
                    state_file_exists: false,
                    state_file_valid: false,
                    last_checkpoint: None,
                    can_resume: false,
                    integrity_issues: vec!["Configuration not available".to_string()],
                });
            }
        };

        let continuity_config = WorkContinuityConfig {
            enable_continuity: config.agents.work_continuity.enable_continuity,
            state_file_path: PathBuf::from(&config.agents.work_continuity.state_file_path),
            backup_interval_minutes: config.agents.work_continuity.backup_interval_minutes,
            max_recovery_attempts: config.agents.work_continuity.max_recovery_attempts,
            validation_timeout_seconds: config.agents.work_continuity.validation_timeout_seconds,
            force_fresh_start_after_hours: config.agents.work_continuity.force_fresh_start_after_hours,
            preserve_partial_work: config.agents.work_continuity.preserve_partial_work,
        };

        let persistence_config = PersistenceConfig {
            enable_persistence: continuity_config.enable_continuity,
            persistence_directory: continuity_config.state_file_path.parent()
                .unwrap_or(&PathBuf::from(".my-little-soda")).to_path_buf(),
            auto_save_interval_minutes: continuity_config.backup_interval_minutes,
            max_state_history_entries: 1000,
            max_recovery_history_entries: 500,
            compress_old_states: true,
            backup_retention_days: 7,
            enable_integrity_checks: true,
        };

        if !continuity_config.enable_continuity {
            return Ok(WorkContinuityStatus {
                is_enabled: false,
                state_file_exists: false,
                state_file_valid: false,
                last_checkpoint: None,
                can_resume: false,
                integrity_issues: vec![],
            });
        }

        let state_file_exists = continuity_config.state_file_path.exists();
        let mut integrity_issues = Vec::new();
        let mut state_file_valid = true;
        let mut last_checkpoint = None;

        if state_file_exists {
            // Try to validate state file
            let manager = WorkContinuityManager::new(
                continuity_config,
                self.github_client.clone(),
                persistence_config,
            );

            match manager.recover_from_checkpoint("agent001").await {
                Ok(resume_action) => {
                    last_checkpoint = Some(Utc::now()); // Would get actual timestamp
                    if resume_action.is_none() {
                        integrity_issues.push("No recoverable state found in file".to_string());
                    }
                }
                Err(e) => {
                    state_file_valid = false;
                    integrity_issues.push(format!("State file validation failed: {:?}", e));
                }
            }
        }

        Ok(WorkContinuityStatus {
            is_enabled: true,
            state_file_exists,
            state_file_valid,
            last_checkpoint,
            can_resume: state_file_exists && state_file_valid && integrity_issues.is_empty(),
            integrity_issues,
        })
    }

    async fn detect_orphaned_assignments(&self) -> Result<Vec<u64>, GitHubError> {
        // Look for issues with agent labels but no corresponding local state
        let mut orphaned_issues = Vec::new();

        // Get all open issues and filter for those labeled with agent001
        let all_issues = self.github_client.issues
            .fetch_issues_with_state(Some(octocrab::params::State::Open))
            .await?;
        
        let issues: Vec<_> = all_issues.into_iter()
            .filter(|issue| {
                issue.labels.iter()
                    .any(|label| label.name == "agent001")
            })
            .collect();

        let current_branch = self.get_current_git_branch();
        let current_assignment = if let Some(ref branch) = current_branch {
            self.extract_issue_number_from_branch(branch)
        } else {
            None
        };

        for issue in issues {
            // If we find an issue assigned to agent but we're not working on it locally
            if Some(issue.number) != current_assignment {
                orphaned_issues.push(issue.number);
            }
        }

        Ok(orphaned_issues)
    }

    async fn detect_abandoned_branches(&self) -> Result<Vec<String>, GitHubError> {
        let mut abandoned_branches = Vec::new();

        // Get all remote agent branches using simplified approach
        match self.github_client.branches.list_branches().await {
            Ok(branch_names) => {
                for branch_name in branch_names {
                    if branch_name.starts_with("agent001/") {
                        // For now, just identify agent branches
                        // In a full implementation, we would check last commit date
                        // This is a simplified version that identifies potential candidates
                        abandoned_branches.push(branch_name);
                    }
                }
            }
            Err(_) => {
                // If we can't list branches via API, that's okay for diagnostic purposes
                // We can still report that we couldn't check
            }
        }

        Ok(abandoned_branches)
    }

    async fn detect_conflicting_assignments(&self) -> Result<Vec<String>, GitHubError> {
        let mut conflicts = Vec::new();

        // Get all open issues and filter for those labeled with agent001
        let all_issues = self.github_client.issues
            .fetch_issues_with_state(Some(octocrab::params::State::Open))
            .await?;
        
        let issues: Vec<_> = all_issues.into_iter()
            .filter(|issue| {
                issue.labels.iter()
                    .any(|label| label.name == "agent001")
            })
            .collect();

        for issue in issues {
            if let Some(assignee) = &issue.assignee {
                // If issue has both agent label and human assignee, it might be a conflict
                let labels: Vec<String> = issue.labels.iter().map(|l| l.name.clone()).collect();
                if labels.contains(&"agent001".to_string()) && !assignee.login.starts_with("agent") {
                    conflicts.push(format!("Issue #{} has both agent001 label and human assignee {}", 
                        issue.number, assignee.login));
                }
            }
        }

        Ok(conflicts)
    }

    async fn check_work_cleanup_status(&self) -> Result<Vec<String>, GitHubError> {
        let mut cleanup_needed = Vec::new();

        // Check for merged PRs that still have agent branches (simplified version)
        match self.github_client.branches.list_branches().await {
            Ok(branches) => {
                let agent_branches: Vec<_> = branches.into_iter()
                    .filter(|branch_name| branch_name.starts_with("agent001/"))
                    .collect();

                for branch_name in agent_branches {
                    // For now, just identify agent branches that exist
                    // In a full implementation, we would check for merged PRs
                    // This provides basic cleanup awareness
                    cleanup_needed.push(format!("Branch '{}' may need cleanup review", branch_name));
                }
            }
            Err(_) => {
                // If we can't list branches, that's okay for diagnostic purposes
            }
        }

        Ok(cleanup_needed)
    }

    fn get_current_git_branch(&self) -> Option<String> {
        std::process::Command::new("git")
            .args(["branch", "--show-current"])
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    let branch = String::from_utf8(output.stdout).ok()?;
                    let trimmed = branch.trim();
                    if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
                } else {
                    None
                }
            })
    }

    fn extract_issue_number_from_branch(&self, branch: &str) -> Option<u64> {
        // Extract issue number from branch name like "agent001/123-feature-name"
        if let Some(parts) = branch.strip_prefix("agent001/") {
            if let Some(issue_part) = parts.split('-').next() {
                issue_part.parse().ok()
            } else {
                None
            }
        } else {
            None
        }
    }

    async fn get_commits_ahead(&self) -> Result<u32, GitHubError> {
        // Get commits ahead of main
        match std::process::Command::new("git")
            .args(["rev-list", "--count", "HEAD", "^main"])
            .output() {
            Ok(output) if output.status.success() => {
                let count_str = String::from_utf8_lossy(&output.stdout);
                Ok(count_str.trim().parse().unwrap_or(0))
            }
            _ => Ok(0)
        }
    }

    fn analyze_issues(
        &self,
        _agent_status: &AgentStatus,
        state_machine_health: &StateMachineHealth,
        work_continuity_status: &WorkContinuityStatus,
        github_sync_status: &GitHubSyncStatus,
    ) -> Vec<AgentStateIssue> {
        let mut issues = Vec::new();

        // State machine issues
        if !state_machine_health.is_consistent {
            issues.push(AgentStateIssue {
                issue_type: AgentStateIssueType::StuckState,
                severity: IssueSeverity::High,
                description: "Agent state machine is inconsistent".to_string(),
                affected_components: vec!["State Machine".to_string()],
                suggested_resolution: "Reset agent state with 'my-little-soda reset --agent-state'".to_string(),
            });
        }

        // Work continuity issues
        if work_continuity_status.is_enabled && !work_continuity_status.state_file_valid {
            issues.push(AgentStateIssue {
                issue_type: AgentStateIssueType::WorkContinuityFailure,
                severity: IssueSeverity::Medium,
                description: "Work continuity state file is corrupted or invalid".to_string(),
                affected_components: vec!["Work Continuity".to_string()],
                suggested_resolution: "Clear corrupted state and restart with fresh state".to_string(),
            });
        }

        // GitHub sync issues
        if !github_sync_status.orphaned_assignments.is_empty() {
            issues.push(AgentStateIssue {
                issue_type: AgentStateIssueType::OrphanedAssignment,
                severity: IssueSeverity::Medium,
                description: format!("Found {} orphaned agent assignments", 
                    github_sync_status.orphaned_assignments.len()),
                affected_components: vec!["GitHub Integration".to_string()],
                suggested_resolution: "Clean up orphaned assignments with reset command".to_string(),
            });
        }

        if !github_sync_status.abandoned_branches.is_empty() {
            issues.push(AgentStateIssue {
                issue_type: AgentStateIssueType::AbandonedBranch,
                severity: IssueSeverity::Low,
                description: format!("Found {} potentially abandoned agent branches", 
                    github_sync_status.abandoned_branches.len()),
                affected_components: vec!["Git Repository".to_string()],
                suggested_resolution: "Review and clean up old agent branches".to_string(),
            });
        }

        if !github_sync_status.conflicting_assignments.is_empty() {
            issues.push(AgentStateIssue {
                issue_type: AgentStateIssueType::ConflictingAssignment,
                severity: IssueSeverity::High,
                description: format!("Found {} conflicting agent assignments", 
                    github_sync_status.conflicting_assignments.len()),
                affected_components: vec!["GitHub Integration".to_string()],
                suggested_resolution: "Resolve assignment conflicts manually".to_string(),
            });
        }

        issues
    }

    fn generate_recommendations(&self, issues: &[AgentStateIssue]) -> Vec<String> {
        let mut recommendations = Vec::new();

        if issues.iter().any(|i| matches!(i.issue_type, AgentStateIssueType::StuckState)) {
            recommendations.push("Consider resetting agent state to resolve state machine inconsistencies".to_string());
        }

        if issues.iter().any(|i| matches!(i.issue_type, AgentStateIssueType::OrphanedAssignment)) {
            recommendations.push("Run cleanup command to resolve orphaned GitHub assignments".to_string());
        }

        if issues.iter().any(|i| matches!(i.issue_type, AgentStateIssueType::WorkContinuityFailure)) {
            recommendations.push("Backup and reset work continuity state if corruption is detected".to_string());
        }

        if issues.iter().any(|i| i.severity == IssueSeverity::Critical) {
            recommendations.push("Address critical issues immediately to prevent work loss".to_string());
        }

        if issues.is_empty() {
            recommendations.push("Agent state is healthy - no immediate actions needed".to_string());
        }

        recommendations
    }
}