// Agent State Management - GitHub-native coordination
// Following VERBOTEN rules: GitHub is source of truth, no local state files

use crate::agent_lifecycle::{AgentEvent, AgentStateMachine};
#[cfg(feature = "autonomous")]
use crate::autonomous::CheckpointReason;
#[cfg(feature = "autonomous")]
use crate::autonomous::ContinuityStatus;
#[cfg(feature = "autonomous")]
use crate::autonomous::PersistenceConfig;
#[cfg(feature = "autonomous")]
use crate::autonomous::ResumeAction;
#[cfg(feature = "autonomous")]
use crate::autonomous::WorkContinuityConfig as AutonomousWorkContinuityConfig;
#[cfg(feature = "autonomous")]
use crate::autonomous::WorkContinuityManager;
use crate::github::{GitHubActions, GitHubClient, GitHubError};
#[cfg(feature = "metrics")]
use crate::metrics::MetricsTracker;
use crate::telemetry::{create_coordination_span, generate_correlation_id};
use serde_json::json;
use statig::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tracing::{info, warn, Instrument};

#[derive(Debug, Clone)]
#[allow(dead_code)] // Architectural enum variants - data fields reserved for future use
pub enum AgentState {
    Available,
    Assigned(String),         // GitHub issue URL
    Working(String),          // GitHub issue URL
}

#[derive(Debug, Clone)]
pub struct Agent {
    pub id: String,
}

pub struct AgentCoordinator {
    github_client: GitHubClient,
    // Single agent state tracking
    current_assignment: Arc<Mutex<Option<u64>>>, // Currently assigned issue
    #[cfg(feature = "metrics")]
    metrics_tracker: MetricsTracker,
    // State machine for the single agent lifecycle
    agent_state_machine: Arc<Mutex<StateMachine<AgentStateMachine>>>,
    // Work continuity manager for persistent state across restarts
    #[cfg(feature = "autonomous")]
    work_continuity: Arc<Mutex<Option<WorkContinuityManager>>>,
    // Verbose mode for debugging output
    verbose: bool,
}

impl AgentCoordinator {
    pub async fn new() -> Result<Self, GitHubError> {
        Self::with_verbose(true).await
    }

    pub async fn with_verbose(verbose: bool) -> Result<Self, GitHubError> {
        let github_client = GitHubClient::with_verbose(verbose)?;
        #[cfg(feature = "metrics")]
        let metrics_tracker = MetricsTracker::new();

        // Initialize state machine for the single agent
        let agent_state_machine = AgentStateMachine::new("agent001".to_string()).state_machine();

        Ok(Self {
            github_client,
            current_assignment: Arc::new(Mutex::new(None)),
            #[cfg(feature = "metrics")]
            metrics_tracker,
            agent_state_machine: Arc::new(Mutex::new(agent_state_machine)),
            #[cfg(feature = "autonomous")]
            work_continuity: Arc::new(Mutex::new(None)),
            verbose,
        })
    }

    pub async fn get_available_agents(&self) -> Result<Vec<Agent>, GitHubError> {
        // Single agent system: Check if agent001 is available
        let mut agents = Vec::new();

        // Check current assignment
        let current_assignment = self.current_assignment.lock().await;
        let is_assigned = current_assignment.is_some();

        // Check current git branch to see if agent is actively working
        let current_branch = self.get_current_git_branch();
        let is_agent_working = current_branch
            .as_ref()
            .map(|branch| branch.starts_with("agent001/"))
            .unwrap_or(false);

        // Check bundling status for additional context
        let bundling_status = self.get_bundling_status().await;

        let _agent_state = if is_agent_working {
            AgentState::Working(format!(
                "Active on branch: {}",
                current_branch.as_ref().unwrap()
            ))
        } else if is_assigned {
            AgentState::Assigned(format!(
                "Assigned to issue: {}",
                current_assignment.unwrap()
            ))
        } else {
            AgentState::Available
        };

        // Agent is available unless actively working
        let is_available = !is_agent_working && !is_assigned;

        if is_available {
            agents.push(Agent {
                id: "agent001".to_string(),
            });
        }

        let available_count = agents.len();
        if self.verbose {
            println!("📊 Available agents: {available_count} of 1 total");
        }

        // Show bundling status in verbose mode for operational visibility
        if let Some(bundling_info) = bundling_status {
            if !bundling_info.queued_branches.is_empty() {
                println!(
                    "🚄 Bundling status: {} branches queued for next departure",
                    bundling_info.queued_branches.len()
                );
            }
        }

        Ok(agents)
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
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                } else {
                    None
                }
            })
    }

    /// Atomic assignment operation with conflict detection
    pub async fn assign_agent_to_issue(
        &self,
        agent_id: &str,
        issue_number: u64,
    ) -> Result<(), GitHubError> {
        // Fetch issue to get title for descriptive branch name
        let issue_title = match self.github_client.fetch_issue(issue_number).await {
            Ok(issue) => issue.title,
            Err(_) => format!("issue-{issue_number}"), // Fallback title
        };

        self.assign_agent_to_issue_with_title(agent_id, issue_number, &issue_title)
            .await
    }

    /// Assignment with descriptive branch name
    async fn assign_agent_to_issue_with_title(
        &self,
        agent_id: &str,
        issue_number: u64,
        issue_title: &str,
    ) -> Result<(), GitHubError> {
        let _execution_start = Instant::now();
        let correlation_id = generate_correlation_id();
        let span = create_coordination_span(
            "assign_agent_to_issue",
            Some(agent_id),
            Some(issue_number),
            Some(&correlation_id),
        );

        async move {
            tracing::info!(
                agent_id = %agent_id,
                issue_number = issue_number,
                correlation_id = %correlation_id,
                "Starting atomic agent assignment with descriptive branch"
            );

            println!("🤖 Attempting atomic assignment: agent {agent_id} -> issue #{issue_number}");

        // Generate descriptive branch name
        let branch_name = self.generate_descriptive_branch_name(agent_id, issue_number, issue_title);

        // STATE MACHINE TRANSITION: Try to assign agent using state machine
        {
            let mut state_machine = self.agent_state_machine.lock().await;
            
            // Validate agent_id is agent001 (single agent system)
            if agent_id != "agent001" {
                return Err(GitHubError::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Only agent001 is supported in single-agent mode, got: {agent_id}")
                )));
            }

            // Check if agent is available before attempting assignment
            if !state_machine.inner().is_available() {
                // Track failed coordination decision
                #[cfg(feature = "metrics")]
                let _ = self.metrics_tracker.track_coordination_decision(
                    correlation_id.clone(),
                    "assign_agent_to_issue",
                    Some(agent_id),
                    Some(issue_number),
                    &format!("Agent {} not available for assignment (current state: available={})", agent_id, state_machine.inner().is_available()),
                        execution_start,
                        false,
                        HashMap::new(),
                    ).await;

                    return Err(GitHubError::IoError(std::io::Error::new(
                        std::io::ErrorKind::ResourceBusy,
                        format!("Agent {agent_id} is not available for assignment")
                    )));
                }

                // Attempt the state machine transition
                state_machine.handle(&AgentEvent::Assign {
                    agent_id: agent_id.to_string(),
                    issue: issue_number,
                    branch: branch_name.clone(),
                });

                // Verify the transition succeeded
                if state_machine.inner().current_issue() != Some(issue_number) {
                    // Track failed coordination decision
                    #[cfg(feature = "metrics")]
                    let _ = self.metrics_tracker.track_coordination_decision(
                        correlation_id.clone(),
                        "assign_agent_to_issue",
                        Some(agent_id),
                        Some(issue_number),
                        &format!("State machine transition failed for agent {agent_id}"),
                        execution_start,
                        false,
                        HashMap::new(),
                    ).await;

                    return Err(GitHubError::IoError(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("Failed to assign agent {agent_id} to issue {issue_number} - state machine transition failed")
                    )));
                }

                tracing::info!(
                    agent_id = %agent_id,
                    issue = %issue_number,
                    branch = %branch_name,
                    "State machine assign event triggered successfully"
                );
        }

        // ASSIGNMENT TRACKING: Track the assignment in single-agent mode
        {
            let mut current_assignment = self.current_assignment.lock().await;

            // Check if agent is already assigned
            if current_assignment.is_some() {
                let existing_issue = current_assignment.unwrap();

                // Track failed coordination decision
                #[cfg(feature = "metrics")]
        let _ = self.metrics_tracker.track_coordination_decision(
                    correlation_id.clone(),
                    "assign_agent_to_issue",
                    Some(agent_id),
                    Some(issue_number),
                    &format!("Assignment conflict: Agent already assigned to issue #{existing_issue}"),
                    execution_start,
                    false,
                    HashMap::new(),
                ).await;

                return Err(GitHubError::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Agent {agent_id} already assigned to issue #{existing_issue}")
                )));
            }

            // Reserve the assignment
            *current_assignment = Some(issue_number);

            println!("✅ Reserved assignment: agent {agent_id} -> issue #{issue_number}");
        }

        // GITHUB OPERATIONS: Perform actual GitHub API calls
        let github_user = self.github_client.owner();

        // Step 1: Assign the issue to the real GitHub user (with retry logic)
        match self.github_client.assign_issue(issue_number, github_user).await {
            Ok(_) => {
                println!("✅ Issue #{issue_number} assigned to GitHub user: {github_user}");
            },
            Err(e) => {
                // ROLLBACK: Remove reservation on failure
                self.rollback_assignment(agent_id, issue_number).await;
                println!("❌ Failed to assign issue #{issue_number}: {e:?}");

                // Track failed coordination decision
                let mut metadata = HashMap::new();
                metadata.insert("error_type".to_string(), "github_assignment_failed".to_string());
                metadata.insert("error_message".to_string(), format!("{e:?}"));

                #[cfg(feature = "metrics")]
        let _ = self.metrics_tracker.track_coordination_decision(
                    correlation_id.clone(),
                    "assign_agent_to_issue",
                    Some(agent_id),
                    Some(issue_number),
                    &format!("GitHub assignment failed: {e:?}"),
                    execution_start,
                    false,
                    metadata,
                ).await;

                return Err(e);
            }
        }

        // Step 2: Add agent label to track which agent is working on this
        println!("🏷️  Adding agent label: {agent_id}");
        match self.github_client.add_label_to_issue(issue_number, agent_id).await {
            Ok(_) => {
                println!("✅ Added agent label: {agent_id}");
            },
            Err(e) => {
                println!("⚠️  Agent labeling failed but assignment succeeded: {e:?}");
            }
        }

        // Step 3: Create agent branch using descriptive naming scheme
        println!("🌿 Creating agent branch: {branch_name}");

        match self.github_client.create_branch(&branch_name, "main").await {
            Ok(_) => {
                println!("✅ Branch '{branch_name}' created successfully");
            },
            Err(e) => {
                println!("⚠️  Branch creation failed: {e}");
                println!("   📝 Note: Branch may already exist, or you can create it manually");
                // Don't rollback - the issue assignment is the important part
                // Agent can still work without automatic branch creation
            }
        }

        println!("🎯 ATOMIC ASSIGNMENT COMPLETE: agent {agent_id} -> issue #{issue_number}");
        tracing::info!(
            agent_id = %agent_id,
            issue_number = issue_number,
            "Successfully completed atomic agent assignment"
        );

        // Track coordination decision
        let mut metadata = HashMap::new();
        metadata.insert("branch_name".to_string(), branch_name.clone());
        metadata.insert("github_user".to_string(), github_user.to_string());

        #[cfg(feature = "metrics")]
        let _ = self.metrics_tracker.track_coordination_decision(
            correlation_id.clone(),
            "assign_agent_to_issue",
            Some(agent_id),
            Some(issue_number),
            &format!("Successfully assigned agent {agent_id} to issue #{issue_number}"),
            execution_start,
            true,
            metadata,
        ).await;

        // Checkpoint work state after successful assignment
        #[cfg(feature = "autonomous")]
        if let Err(e) = self.checkpoint_work_state(agent_id).await {
            warn!("Failed to checkpoint after assignment: {:?}", e);
        }

        Ok(())
        }.instrument(span).await
    }


    /// Generate descriptive branch name from issue title
    fn generate_descriptive_branch_name(
        &self,
        agent_id: &str,
        issue_number: u64,
        issue_title: &str,
    ) -> String {
        let slug = self.create_intelligent_slug(issue_title, 30);
        format!("{agent_id}/{issue_number}-{slug}")
    }

    /// Create an intelligently truncated slug that preserves word boundaries
    fn create_intelligent_slug(&self, title: &str, max_length: usize) -> String {
        // Step 1: Convert to lowercase and filter characters
        let filtered: String = title
            .to_lowercase()
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || c.is_ascii_whitespace() || *c == '-')
            .collect();

        // Step 2: Replace multiple whitespace/dashes with single dash, trim
        let normalized = filtered
            .split(|c: char| c.is_whitespace() || c == '-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<&str>>()
            .join("-");

        // Step 3: If already short enough, return as-is
        if normalized.len() <= max_length {
            return normalized;
        }

        // Step 4: For single very long words, truncate at character boundary
        if !normalized.contains('-') {
            return normalized.chars().take(max_length).collect();
        }

        // Step 5: Truncate at word boundaries to avoid cutting words in half
        let mut result = String::new();
        for word in normalized.split('-') {
            // Check if adding this word would exceed the limit
            let next_length = if result.is_empty() {
                word.len()
            } else {
                result.len() + 1 + word.len() // +1 for the dash
            };

            if next_length > max_length {
                break;
            }

            if !result.is_empty() {
                result.push('-');
            }
            result.push_str(word);
        }

        // Step 6: Ensure we don't end with a dash
        result.trim_end_matches('-').to_string()
    }

    /// Rollback assignment reservation on failure
    async fn rollback_assignment(&self, _agent_id: &str, _issue_number: u64) {
        let mut current_assignment = self.current_assignment.lock().await;
        *current_assignment = None;
        println!("🔄 Rolled back assignment: agent available again");
    }

    /// Get agent utilization for single agent system
    pub async fn get_agent_utilization(&self) -> HashMap<String, (u32, u32)> {
        let mut utilization = HashMap::new();
        let current_assignment = self.current_assignment.lock().await;
        let is_assigned = current_assignment.is_some();
        
        // Single agent: 0 or 1 assignment
        utilization.insert("agent001".to_string(), (if is_assigned { 1 } else { 0 }, 1));
        
        utilization
    }




    /// Handle agent completing work - triggers state machine transition to landed state
    pub async fn complete_work(&self, agent_id: &str) -> Result<(), GitHubError> {
        // Validate agent_id is agent001 (single agent system)
        if agent_id != "agent001" {
            return Err(GitHubError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Only agent001 is supported in single-agent mode, got: {agent_id}"),
            )));
        }

        let mut state_machine = self.agent_state_machine.lock().await;
        state_machine.handle(&AgentEvent::CompleteWork);

        tracing::info!(
            agent_id = %agent_id,
            "Agent completed work via state machine"
        );

        // Trigger GitHub Actions bundling workflow after work completion
        if let Err(e) = self.trigger_bundling_workflow_async(agent_id).await {
            warn!(
                agent_id = %agent_id,
                error = %e,
                "Failed to trigger bundling workflow after work completion"
            );
            // Don't fail the work completion if bundling trigger fails
            // The periodic bundling workflow will catch any missed work
        }

        Ok(())
    }

    /// Handle agent abandoning work - triggers state machine transition back to idle
    pub async fn abandon_work(&self, agent_id: &str) -> Result<(), GitHubError> {
        // Validate agent_id is agent001 (single agent system)
        if agent_id != "agent001" {
            return Err(GitHubError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Only agent001 is supported in single-agent mode, got: {agent_id}"),
            )));
        }

        let mut state_machine = self.agent_state_machine.lock().await;
        state_machine.handle(&AgentEvent::Abandon);

        // Clear internal state tracking
        {
            let mut current_assignment = self.current_assignment.lock().await;
            *current_assignment = None;
        }

        tracing::info!(
            agent_id = %agent_id,
            "Agent abandoned work via state machine"
        );

        Ok(())
    }

    /// Trigger GitHub Actions bundling workflow asynchronously
    /// This enables real agents to trigger cloud bundling immediately after completion
    async fn trigger_bundling_workflow_async(&self, agent_id: &str) -> Result<(), GitHubError> {
        info!(
            agent_id = %agent_id,
            "Triggering GitHub Actions bundling workflow after agent work completion"
        );

        // Prepare workflow inputs to indicate this was triggered by agent completion
        let workflow_inputs = json!({
            "force_bundle": "false",
            "dry_run": "false",
            "verbose": "true",
            "triggered_by": format!("agent_completion:{}", agent_id)
        });

        // Trigger the bundling workflow
        self.github_client
            .actions
            .trigger_workflow("clambake-bundling.yml", Some(workflow_inputs))
            .await?;

        info!(
            agent_id = %agent_id,
            "Successfully triggered GitHub Actions bundling workflow"
        );

        Ok(())
    }


    pub async fn trigger_bundling_workflow_with_ci_mode(
        &self,
        force: bool,
        dry_run: bool,
        verbose: bool,
        ci_mode: bool,
    ) -> Result<(), GitHubError> {
        info!(
            force = force,
            dry_run = dry_run,
            verbose = verbose,
            ci_mode = ci_mode,
            "Manually triggering GitHub Actions bundling workflow"
        );

        let mut workflow_inputs = json!({
            "force_bundle": force.to_string(),
            "dry_run": dry_run.to_string(),
            "verbose": verbose.to_string(),
            "triggered_by": "manual_cli"
        });

        // Add CI mode optimization inputs
        if ci_mode {
            workflow_inputs["ci_mode"] = json!("true");
            workflow_inputs["artifact_handling"] = json!("optimized");
            workflow_inputs["github_token_strategy"] = json!("ci_optimized");

            info!("CI mode enabled - optimizing for GitHub Actions environment");
        }

        self.github_client
            .actions
            .trigger_workflow("clambake-bundling.yml", Some(workflow_inputs))
            .await?;

        info!(
            ci_mode = ci_mode,
            "Successfully triggered GitHub Actions bundling workflow"
        );

        Ok(())
    }



    /// Get current bundling status for operational visibility
    async fn get_bundling_status(&self) -> Option<BundlingStatus> {
        // Skip expensive bundling status checks in non-verbose mode
        if !self.verbose {
            return None;
        }

        use crate::train_schedule::TrainSchedule;

        // Get queued branches ready for bundling
        if let Ok(queued_branches) = TrainSchedule::get_queued_branches().await {
            let schedule = TrainSchedule::calculate_next_schedule();

            Some(BundlingStatus {
                queued_branches,
                next_departure: schedule.next_departure,
                minutes_until_departure: schedule.minutes_until_departure,
                status: schedule.status,
            })
        } else {
            None
        }
    }

    /// Initialize work continuity for the specified agent
    pub async fn initialize_work_continuity(&self, agent_id: &str) -> Result<(), GitHubError> {
        #[cfg(feature = "autonomous")]
        {
            let config = match config() {
                Ok(c) => c,
                Err(e) => {
                    warn!("Failed to load configuration for work continuity: {}", e);
                    return Ok(()); // Continue without work continuity
                }
            };

            // Convert config structures
            let continuity_config = AutonomousWorkContinuityConfig {
                enable_continuity: config.agents.work_continuity.enable_continuity,
                state_file_path: std::path::PathBuf::from(
                    &config.agents.work_continuity.state_file_path,
                ),
                backup_interval_minutes: config.agents.work_continuity.backup_interval_minutes,
                max_recovery_attempts: config.agents.work_continuity.max_recovery_attempts,
                validation_timeout_seconds: config.agents.work_continuity.validation_timeout_seconds,
                force_fresh_start_after_hours: config
                    .agents
                    .work_continuity
                    .force_fresh_start_after_hours,
                preserve_partial_work: config.agents.work_continuity.preserve_partial_work,
            };

            let persistence_config = PersistenceConfig {
                enable_persistence: continuity_config.enable_continuity,
                persistence_directory: continuity_config
                    .state_file_path
                    .parent()
                    .unwrap_or(&std::path::PathBuf::from(".my-little-soda"))
                    .to_path_buf(),
                auto_save_interval_minutes: continuity_config.backup_interval_minutes,
                max_state_history_entries: 1000,
                max_recovery_history_entries: 500,
                compress_old_states: true,
                backup_retention_days: 7,
                enable_integrity_checks: true,
            };

            if !continuity_config.enable_continuity {
                info!("Work continuity disabled for agent {}", agent_id);
                return Ok(());
            }

            let mut continuity_manager = WorkContinuityManager::new(
                continuity_config,
                self.github_client.clone(),
                persistence_config,
            );

            match continuity_manager.initialize(agent_id).await {
                Ok(_) => {
                    info!("Work continuity initialized for agent {}", agent_id);
                    let mut work_continuity = self.work_continuity.lock().await;
                    *work_continuity = Some(continuity_manager);
                    Ok(())
                }
                Err(e) => {
                    warn!(
                        "Failed to initialize work continuity for agent {}: {:?}",
                        agent_id, e
                    );
                    // Continue without work continuity rather than failing
                    Ok(())
                }
            }
        }
        #[cfg(not(feature = "autonomous"))]
        {
            info!("Work continuity not initialized for agent {}", agent_id);
            Ok(())
        }
    }

    /// Attempt to recover work state after process restart
    #[cfg(feature = "autonomous")]
    pub async fn attempt_work_recovery(
        &self,
        agent_id: &str,
    ) -> Result<Option<ResumeAction>, GitHubError> {
        let work_continuity_guard = self.work_continuity.lock().await;
        let continuity_manager = match work_continuity_guard.as_ref() {
            Some(manager) => manager,
            None => {
                info!("Work continuity not initialized for agent {}", agent_id);
                return Ok(None);
            }
        };

        match continuity_manager.recover_from_checkpoint(agent_id).await {
            Ok(resume_action) => {
                info!(
                    "Work recovery attempted for agent {}: {:?}",
                    agent_id,
                    resume_action.is_some()
                );
                Ok(resume_action)
            }
            Err(e) => {
                warn!("Work recovery failed for agent {}: {:?}", agent_id, e);
                // Return None to indicate fresh start rather than failing
                Ok(None)
            }
        }
    }

    /// Save current work state to persistent storage
    #[cfg(feature = "autonomous")]
    pub async fn checkpoint_work_state(&self, agent_id: &str) -> Result<(), GitHubError> {
        let work_continuity_guard = self.work_continuity.lock().await;
        let continuity_manager = match work_continuity_guard.as_ref() {
            Some(manager) => manager,
            None => return Ok(()), // Continuity not enabled
        };

        // Validate agent_id is agent001 (single agent system)
        if agent_id != "agent001" {
            warn!("Only agent001 is supported in single-agent mode, got: {}", agent_id);
            return Ok(());
        }

        let state_machine = self.agent_state_machine.lock().await;
        let agent_state = state_machine.inner();

        match continuity_manager
            .checkpoint_state(
                agent_state,
                None, // Would be populated with autonomous state if available
                CheckpointReason::StateTransition,
            )
            .await
        {
            Ok(checkpoint_id) => {
                info!(
                    "Work state checkpointed for agent {} ({})",
                    agent_id, checkpoint_id
                );
                Ok(())
            }
            Err(e) => {
                warn!(
                    "Failed to checkpoint work state for agent {}: {:?}",
                    agent_id, e
                );
                // Continue without checkpointing rather than failing
                Ok(())
            }
        }
    }

    /// Resume interrupted work based on recovery action
    #[cfg(feature = "autonomous")]
    pub async fn resume_interrupted_work(
        &self,
        agent_id: &str,
        resume_action: ResumeAction,
    ) -> Result<(), GitHubError> {
        let work_continuity_guard = self.work_continuity.lock().await;
        let continuity_manager = match work_continuity_guard.as_ref() {
            Some(manager) => manager,
            None => return Ok(()), // Continuity not enabled
        };

        // Validate agent_id is agent001 (single agent system)
        if agent_id != "agent001" {
            warn!("Only agent001 is supported in single-agent mode, got: {}", agent_id);
            return Ok(());
        }

        let mut state_machine = self.agent_state_machine.lock().await;
        let agent_state = unsafe { state_machine.inner_mut() };

        match continuity_manager
            .resume_interrupted_work(resume_action, agent_state)
            .await
        {
            Ok(_) => {
                info!(
                    "Successfully resumed interrupted work for agent {}",
                    agent_id
                );
                Ok(())
            }
            Err(e) => {
                warn!(
                    "Failed to resume interrupted work for agent {}: {:?}",
                    agent_id, e
                );
                // Continue with fresh start rather than failing
                Ok(())
            }
        }
    }

    /// Get current work continuity status
    #[cfg(feature = "autonomous")]
    pub async fn get_work_continuity_status(&self, agent_id: &str) -> Option<ContinuityStatus> {
        let work_continuity_guard = self.work_continuity.lock().await;
        let continuity_manager = work_continuity_guard.as_ref()?;

        match continuity_manager.get_continuity_status(agent_id).await {
            Ok(status) => Some(status),
            Err(e) => {
                warn!(
                    "Failed to get continuity status for agent {}: {:?}",
                    agent_id, e
                );
                None
            }
        }
    }
}

impl std::fmt::Debug for AgentCoordinator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug_struct = f.debug_struct("AgentCoordinator");
        debug_struct
            .field("github_client", &"GitHubClient")
            .field("current_assignment", &"Arc<Mutex<Option<u64>>>");
        
        #[cfg(feature = "metrics")]
        debug_struct.field("metrics_tracker", &"MetricsTracker");
        
        debug_struct
            .field("agent_state_machine", &"Arc<Mutex<StateMachine<AgentStateMachine>>>")
            .finish()
    }
}

/// Bundling status information for agent coordination
#[derive(Debug)]
struct BundlingStatus {
    queued_branches: Vec<crate::train_schedule::QueuedBranch>,
    #[allow(dead_code)]
    next_departure: chrono::DateTime<chrono::Local>,
    #[allow(dead_code)]
    minutes_until_departure: i64,
    #[allow(dead_code)]
    status: crate::train_schedule::ScheduleStatus,
}
