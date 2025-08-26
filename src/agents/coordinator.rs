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
pub enum AgentState {
    Available,
    Assigned(String),         // GitHub issue URL
    Working(String),          // GitHub issue URL
    Completed(String),        // GitHub issue URL
    UnderReview(String),      // GitHub issue URL - CodeRabbit reviewing
    AwaitingApproval(String), // GitHub issue URL - Human approval needed
    ReadyToLand(String),      // GitHub issue URL - Ready for final merge
}

#[derive(Debug, Clone)]
pub struct Agent {
    pub id: String,
    pub capacity: u32,
    pub state: AgentState,
}

pub struct AgentCoordinator {
    github_client: GitHubClient,
    // Safe coordination state - prevents race conditions
    assignment_lock: Arc<Mutex<HashMap<u64, String>>>, // issue_number -> agent_id
    agent_capacity: Arc<Mutex<HashMap<String, (u32, u32)>>>, // agent_id -> (current, max)
    #[cfg(feature = "metrics")]
    metrics_tracker: MetricsTracker,
    // State machine for agent lifecycle management
    agent_state_machines: Arc<Mutex<HashMap<String, StateMachine<AgentStateMachine>>>>,
    // Work continuity manager for persistent state across restarts
    #[cfg(feature = "autonomous")]
    work_continuity: Arc<Mutex<Option<WorkContinuityManager>>>,
}

impl AgentCoordinator {
    pub async fn new() -> Result<Self, GitHubError> {
        let github_client = GitHubClient::new()?;
        #[cfg(feature = "metrics")]
        let metrics_tracker = MetricsTracker::new();

        // Agent capacity: Solo agent system - agent001 only
        let mut agent_capacity = HashMap::new();
        agent_capacity.insert("agent001".to_string(), (0, 1)); // 0 current, 1 max per agent

        // Initialize state machines for each agent
        let mut agent_state_machines = HashMap::new();
        let agent001_sm = AgentStateMachine::new("agent001".to_string()).state_machine();
        agent_state_machines.insert("agent001".to_string(), agent001_sm);

        Ok(Self {
            github_client,
            assignment_lock: Arc::new(Mutex::new(HashMap::new())),
            agent_capacity: Arc::new(Mutex::new(agent_capacity)),
            #[cfg(feature = "metrics")]
            metrics_tracker,
            agent_state_machines: Arc::new(Mutex::new(agent_state_machines)),
            #[cfg(feature = "autonomous")]
            work_continuity: Arc::new(Mutex::new(None)),
        })
    }

    pub async fn get_available_agents(&self) -> Result<Vec<Agent>, GitHubError> {
        // Solo agent system: Check if agent001 is actively working on a branch
        let capacities = self.agent_capacity.lock().await;
        let mut agents = Vec::new();

        // Check current git branch to see if agent is actively working
        let current_branch = self.get_current_git_branch();
        let is_agent_working = current_branch
            .as_ref()
            .map(|branch| branch.starts_with("agent001/"))
            .unwrap_or(false);

        // Check bundling status for additional context
        let bundling_status = self.get_bundling_status().await;

        // For each configured agent, check availability
        for (agent_id, (_current, max_capacity)) in capacities.iter() {
            let agent_state = if is_agent_working && agent_id == "agent001" {
                AgentState::Working(format!(
                    "Active on branch: {}",
                    current_branch.as_ref().unwrap()
                ))
            } else {
                AgentState::Available
            };

            // In solo mode, agent is available unless actively working on a branch
            let is_available = !(is_agent_working && agent_id == "agent001");

            if is_available {
                agents.push(Agent {
                    id: agent_id.clone(),
                    capacity: *max_capacity,
                    state: agent_state,
                });
            }
        }

        let available_count = agents.len();
        let total_agents = capacities.len();
        println!("📊 Available agents: {available_count} of {total_agents} total");

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

    /// Atomic assignment operation with conflict detection and capacity management
    pub async fn assign_agent_to_issue(
        &self,
        agent_id: &str,
        issue_number: u64,
    ) -> Result<(), GitHubError> {
        // Fetch issue to get title for descriptive branch name
        let issue = match self.github_client.fetch_issue(issue_number).await {
            Ok(issue) => issue,
            Err(_) => {
                // Fallback to simple assignment without descriptive branch if issue fetch fails
                return self
                    .assign_agent_to_issue_simple(agent_id, issue_number)
                    .await;
            }
        };

        self.assign_agent_to_issue_with_title(agent_id, issue_number, &issue.title)
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
            let mut state_machines = self.agent_state_machines.lock().await;
            if let Some(sm) = state_machines.get_mut(agent_id) {
                // Check if agent is available before attempting assignment
                if !sm.inner().is_available() {
                    // Track failed coordination decision
                    #[cfg(feature = "metrics")]
                    #[cfg(feature = "metrics")]
        let _ = self.metrics_tracker.track_coordination_decision(
                        correlation_id.clone(),
                        "assign_agent_to_issue",
                        Some(agent_id),
                        Some(issue_number),
                        &format!("Agent {} not available for assignment (current state: available={})", agent_id, sm.inner().is_available()),
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
                sm.handle(&AgentEvent::Assign {
                    agent_id: agent_id.to_string(),
                    issue: issue_number,
                    branch: branch_name.clone(),
                });

                // Verify the transition succeeded
                if sm.inner().current_issue() != Some(issue_number) {
                    // Track failed coordination decision
                    #[cfg(feature = "metrics")]
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
            } else {
                return Err(GitHubError::IoError(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("State machine not found for agent: {agent_id}")
                )));
            }
        }

        // ATOMIC OPERATION: All-or-nothing assignment with conflict detection
        {
            let mut assignments = self.assignment_lock.lock().await;
            let mut capacities = self.agent_capacity.lock().await;

            // CONFLICT DETECTION: Check if issue already assigned
            if assignments.contains_key(&issue_number) {
                let existing_agent = assignments.get(&issue_number).unwrap();

                // Track failed coordination decision
                #[cfg(feature = "metrics")]
        let _ = self.metrics_tracker.track_coordination_decision(
                    correlation_id.clone(),
                    "assign_agent_to_issue",
                    Some(agent_id),
                    Some(issue_number),
                    &format!("Assignment conflict: Issue #{issue_number} already assigned to agent {existing_agent}"),
                    execution_start,
                    false,
                    HashMap::new(),
                ).await;

                return Err(GitHubError::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Issue #{issue_number} already assigned to agent {existing_agent}")
                )));
            }

            // CAPACITY MANAGEMENT: Check agent capacity
            let (current, max) = *capacities.get(agent_id)
                .ok_or_else(|| GitHubError::IoError(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Unknown agent: {agent_id}")
                )))?;

            if current >= max {
                // Track failed coordination decision
                let mut metadata = HashMap::new();
                metadata.insert("current_capacity".to_string(), current.to_string());
                metadata.insert("max_capacity".to_string(), max.to_string());

                #[cfg(feature = "metrics")]
        let _ = self.metrics_tracker.track_coordination_decision(
                    correlation_id.clone(),
                    "assign_agent_to_issue",
                    Some(agent_id),
                    Some(issue_number),
                    &format!("Capacity exceeded: Agent {agent_id} at capacity ({current}/{max})"),
                    execution_start,
                    false,
                    metadata,
                ).await;

                return Err(GitHubError::IoError(std::io::Error::new(
                    std::io::ErrorKind::ResourceBusy,
                    format!("Agent {agent_id} at capacity ({current}/{max})")
                )));
            }

            // RESERVATION: Reserve the assignment before GitHub operations
            assignments.insert(issue_number, agent_id.to_string());
            capacities.insert(agent_id.to_string(), (current + 1, max));

            println!("✅ Reserved assignment: agent {} -> issue #{} (capacity: {}/{})",
                    agent_id, issue_number, current + 1, max);
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

    /// Simple assignment without descriptive branch name (fallback)
    async fn assign_agent_to_issue_simple(
        &self,
        agent_id: &str,
        issue_number: u64,
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
                "Starting atomic agent assignment"
            );

            println!("🤖 Attempting atomic assignment: agent {agent_id} -> issue #{issue_number}");

        // ATOMIC OPERATION: All-or-nothing assignment with conflict detection
        {
            let mut assignments = self.assignment_lock.lock().await;
            let mut capacities = self.agent_capacity.lock().await;

            // CONFLICT DETECTION: Check if issue already assigned
            if assignments.contains_key(&issue_number) {
                let existing_agent = assignments.get(&issue_number).unwrap();

                // Track failed coordination decision
                #[cfg(feature = "metrics")]
        let _ = self.metrics_tracker.track_coordination_decision(
                    correlation_id.clone(),
                    "assign_agent_to_issue",
                    Some(agent_id),
                    Some(issue_number),
                    &format!("Assignment conflict: Issue #{issue_number} already assigned to agent {existing_agent}"),
                    execution_start,
                    false,
                    HashMap::new(),
                ).await;

                return Err(GitHubError::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Issue #{issue_number} already assigned to agent {existing_agent}")
                )));
            }

            // CAPACITY MANAGEMENT: Check agent capacity
            let (current, max) = *capacities.get(agent_id)
                .ok_or_else(|| GitHubError::IoError(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Unknown agent: {agent_id}")
                )))?;

            if current >= max {
                // Track failed coordination decision
                let mut metadata = HashMap::new();
                metadata.insert("current_capacity".to_string(), current.to_string());
                metadata.insert("max_capacity".to_string(), max.to_string());

                #[cfg(feature = "metrics")]
        let _ = self.metrics_tracker.track_coordination_decision(
                    correlation_id.clone(),
                    "assign_agent_to_issue",
                    Some(agent_id),
                    Some(issue_number),
                    &format!("Capacity exceeded: Agent {agent_id} at capacity ({current}/{max})"),
                    execution_start,
                    false,
                    metadata,
                ).await;

                return Err(GitHubError::IoError(std::io::Error::new(
                    std::io::ErrorKind::ResourceBusy,
                    format!("Agent {agent_id} at capacity ({current}/{max})")
                )));
            }

            // RESERVATION: Reserve the assignment before GitHub operations
            assignments.insert(issue_number, agent_id.to_string());
            capacities.insert(agent_id.to_string(), (current + 1, max));

            println!("✅ Reserved assignment: agent {} -> issue #{} (capacity: {}/{})",
                    agent_id, issue_number, current + 1, max);
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

        // Step 3: Create agent branch using proper naming scheme
        let branch_name = format!("{agent_id}/{issue_number}");
        println!("🌿 Creating agent branch: {branch_name}");

        match self.github_client.create_branch(&branch_name, "main").await {
            Ok(_) => {
                // Branch creation succeeded - the success message is already printed by the client
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
        metadata.insert("branch_name".to_string(), format!("{agent_id}/{issue_number}"));
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
        let slug = issue_title
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-')
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<&str>>()
            .join("-")
            .chars()
            .take(30)
            .collect::<String>();

        format!("{agent_id}/{issue_number}-{slug}")
    }

    /// Rollback assignment reservation on failure
    async fn rollback_assignment(&self, agent_id: &str, issue_number: u64) {
        let mut assignments = self.assignment_lock.lock().await;
        let mut capacities = self.agent_capacity.lock().await;

        assignments.remove(&issue_number);

        if let Some((current, max)) = capacities.get(agent_id).cloned() {
            if current > 0 {
                capacities.insert(agent_id.to_string(), (current - 1, max));
                println!(
                    "🔄 Rolled back assignment: agent {} (capacity: {}/{})",
                    agent_id,
                    current - 1,
                    max
                );
            }
        }
    }

    /// Get agent utilization for load balancing
    pub async fn get_agent_utilization(&self) -> HashMap<String, (u32, u32)> {
        // Get real-time utilization by checking GitHub for agent assignments
        let issues = match self.github_client.fetch_issues().await {
            Ok(issues) => issues,
            Err(_) => {
                // Fall back to cached state if GitHub is unavailable
                let capacities = self.agent_capacity.lock().await;
                return capacities.clone();
            }
        };

        let capacities = self.agent_capacity.lock().await;
        let mut utilization = HashMap::new();

        // For each configured agent, count their actual assignments from GitHub
        for (agent_id, (_cached_current, max_capacity)) in capacities.iter() {
            let actual_count = issues
                .iter()
                .filter(|issue| {
                    issue.state == octocrab::models::IssueState::Open
                        && issue.labels.iter().any(|label| label.name == *agent_id)
                        && issue.labels.iter().any(|label| label.name == "route:ready")
                })
                .count() as u32;

            utilization.insert(agent_id.clone(), (actual_count, *max_capacity));
        }

        utilization
    }

    /// Validate system consistency - no conflicts or over-assignments
    pub async fn validate_consistency(&self) -> Result<bool, GitHubError> {
        let assignments = self.assignment_lock.lock().await;
        let capacities = self.agent_capacity.lock().await;

        // Check that assignment counts match capacity tracking
        let mut agent_counts = HashMap::new();
        for agent_id in assignments.values() {
            *agent_counts.entry(agent_id.clone()).or_insert(0) += 1;
        }

        for (agent_id, (current, max)) in capacities.iter() {
            let actual_count = agent_counts.get(agent_id).unwrap_or(&0);
            if actual_count != current {
                println!("❌ CONSISTENCY ERROR: Agent {agent_id} capacity tracking mismatch: tracked={current}, actual={actual_count}");
                return Ok(false);
            }

            if current > max {
                println!("❌ CONSISTENCY ERROR: Agent {agent_id} over-capacity: {current}/{max}");
                return Ok(false);
            }
        }

        println!(
            "✅ Consistency check passed: {} assignments across {} agents",
            assignments.len(),
            capacities.len()
        );
        Ok(true)
    }

    pub async fn update_agent_state(
        &self,
        agent_id: &str,
        new_state: AgentState,
    ) -> Result<(), GitHubError> {
        // GitHub-native: State changes reflected in GitHub repository
        // This would update issue status, labels, or branch state
        println!("🔄 Updating agent {agent_id} state to {new_state:?}");
        Ok(())
    }

    /// Handle agent starting work - triggers state machine transition to working state
    pub async fn start_work(&self, agent_id: &str, commits_ahead: u32) -> Result<(), GitHubError> {
        let mut state_machines = self.agent_state_machines.lock().await;
        if let Some(sm) = state_machines.get_mut(agent_id) {
            sm.handle(&AgentEvent::StartWork { commits_ahead });

            tracing::info!(
                agent_id = %agent_id,
                commits_ahead = %commits_ahead,
                "Agent started work via state machine"
            );

            Ok(())
        } else {
            Err(GitHubError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("State machine not found for agent: {agent_id}"),
            )))
        }
    }

    /// Handle agent completing work - triggers state machine transition to landed state
    pub async fn complete_work(&self, agent_id: &str) -> Result<(), GitHubError> {
        let mut state_machines = self.agent_state_machines.lock().await;
        if let Some(sm) = state_machines.get_mut(agent_id) {
            sm.handle(&AgentEvent::CompleteWork);

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
        } else {
            Err(GitHubError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("State machine not found for agent: {agent_id}"),
            )))
        }
    }

    /// Handle agent abandoning work - triggers state machine transition back to idle
    pub async fn abandon_work(&self, agent_id: &str) -> Result<(), GitHubError> {
        let mut state_machines = self.agent_state_machines.lock().await;
        if let Some(sm) = state_machines.get_mut(agent_id) {
            sm.handle(&AgentEvent::Abandon);

            // Also clear internal state tracking
            {
                let mut assignments = self.assignment_lock.lock().await;
                let mut capacities = self.agent_capacity.lock().await;

                // Find and remove the assignment for this agent
                let issue_to_remove = assignments.iter().find_map(|(issue, assigned_agent)| {
                    if assigned_agent == agent_id {
                        Some(*issue)
                    } else {
                        None
                    }
                });

                if let Some(issue_number) = issue_to_remove {
                    assignments.remove(&issue_number);
                }

                // Reduce agent capacity
                if let Some((current, max)) = capacities.get(agent_id).cloned() {
                    if current > 0 {
                        capacities.insert(agent_id.to_string(), (current - 1, max));
                    }
                }
            }

            tracing::info!(
                agent_id = %agent_id,
                "Agent abandoned work via state machine"
            );

            Ok(())
        } else {
            Err(GitHubError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("State machine not found for agent: {agent_id}"),
            )))
        }
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

    /// Manually trigger GitHub Actions bundling workflow with options
    /// This provides direct API access for CLI commands and external triggers
    pub async fn trigger_bundling_workflow(
        &self,
        force: bool,
        dry_run: bool,
        verbose: bool,
    ) -> Result<(), GitHubError> {
        self.trigger_bundling_workflow_with_ci_mode(force, dry_run, verbose, false)
            .await
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

    /// Get current state machine state for debugging and status reporting
    pub async fn get_agent_state_machine_info(&self, agent_id: &str) -> Option<String> {
        let state_machines = self.agent_state_machines.lock().await;
        if let Some(sm) = state_machines.get(agent_id) {
            let inner = sm.inner();
            let status = if inner.is_available() {
                "AVAILABLE"
            } else if inner.is_assigned() {
                "ASSIGNED"
            } else if inner.is_working() {
                "WORKING"
            } else {
                "OTHER"
            };

            Some(format!(
                "Agent: {} | Status: {} | Issue: {:?} | Branch: {:?} | Commits: {}",
                agent_id,
                status,
                inner.current_issue(),
                inner.current_branch(),
                inner.commits_ahead()
            ))
        } else {
            None
        }
    }

    /// Get detailed state machine status for all agents
    pub async fn get_all_agent_states(&self) -> Vec<(String, String)> {
        let state_machines = self.agent_state_machines.lock().await;
        let mut states = Vec::new();

        for (agent_id, sm) in state_machines.iter() {
            let inner = sm.inner();
            let status = if inner.is_available() {
                "AVAILABLE".to_string()
            } else if inner.is_assigned() {
                format!("ASSIGNED(issue: {})", inner.current_issue().unwrap_or(0))
            } else if inner.is_working() {
                format!(
                    "WORKING(issue: {}, commits: {})",
                    inner.current_issue().unwrap_or(0),
                    inner.commits_ahead()
                )
            } else {
                "OTHER".to_string()
            };

            states.push((agent_id.clone(), status));
        }

        states
    }

    /// Get current bundling status for operational visibility
    async fn get_bundling_status(&self) -> Option<BundlingStatus> {
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

        let state_machines = self.agent_state_machines.lock().await;
        let agent_state = match state_machines.get(agent_id) {
            Some(sm) => sm.inner(),
            None => {
                warn!("No state machine found for agent {}", agent_id);
                return Ok(());
            }
        };

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

        let mut state_machines = self.agent_state_machines.lock().await;
        let agent_state = match state_machines.get_mut(agent_id) {
            Some(sm) => unsafe { sm.inner_mut() },
            None => {
                warn!("No state machine found for agent {}", agent_id);
                return Ok(());
            }
        };

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
            .field("assignment_lock", &"Arc<Mutex<HashMap<u64, String>>>")
            .field("agent_capacity", &"Arc<Mutex<HashMap<String, (u32, u32)>>>");
        
        #[cfg(feature = "metrics")]
        debug_struct.field("metrics_tracker", &"MetricsTracker");
        
        debug_struct
            .field(
                "agent_state_machines",
                &"Arc<Mutex<HashMap<String, StateMachine<AgentStateMachine>>>>",
            )
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
