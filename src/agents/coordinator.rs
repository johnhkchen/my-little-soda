// Agent State Management - GitHub-native coordination
// Following VERBOTEN rules: GitHub is source of truth, no local state files

use crate::github::{GitHubClient, GitHubError};
use crate::telemetry::{generate_correlation_id, create_coordination_span};
use crate::metrics::MetricsTracker;
use std::collections::HashMap;
use std::time::Instant;
use tokio::sync::Mutex;
use std::sync::Arc;
use tracing::Instrument;

#[derive(Debug, Clone)]
pub enum AgentState {
    Available,
    Assigned(String), // GitHub issue URL
    Working(String),  // GitHub issue URL
    Completed(String), // GitHub issue URL
    UnderReview(String), // GitHub issue URL - CodeRabbit reviewing
    AwaitingApproval(String), // GitHub issue URL - Human approval needed
    ReadyToLand(String), // GitHub issue URL - Ready for final merge
}

#[derive(Debug, Clone)]
pub struct Agent {
    pub id: String,
    pub capacity: u32,
    pub state: AgentState,
}

#[derive(Debug)]
pub struct AgentCoordinator {
    github_client: GitHubClient,
    // Safe coordination state - prevents race conditions
    assignment_lock: Arc<Mutex<HashMap<u64, String>>>, // issue_number -> agent_id
    agent_capacity: Arc<Mutex<HashMap<String, (u32, u32)>>>, // agent_id -> (current, max)
    metrics_tracker: MetricsTracker,
}

impl AgentCoordinator {
    pub async fn new() -> Result<Self, GitHubError> {
        let github_client = GitHubClient::new()?;
        let metrics_tracker = MetricsTracker::new();
        
        // Agent capacity: Start with agent001, expandable to agent002-agent008
        let mut agent_capacity = HashMap::new();
        agent_capacity.insert("agent001".to_string(), (0, 1)); // 0 current, 1 max per agent
        
        Ok(Self { 
            github_client,
            assignment_lock: Arc::new(Mutex::new(HashMap::new())),
            agent_capacity: Arc::new(Mutex::new(agent_capacity)),
            metrics_tracker,
        })
    }

    pub async fn get_available_agents(&self) -> Result<Vec<Agent>, GitHubError> {
        // Check which agents have work by looking at agent labels (agent001, agent002, etc.)
        let issues = self.github_client.fetch_issues().await?;
        let capacities = self.agent_capacity.lock().await;
        let mut agents = Vec::new();
        
        // For each configured agent, check if they have work
        for (agent_id, (_current, max_capacity)) in capacities.iter() {
            let assigned_count = issues
                .iter()
                .filter(|issue| {
                    issue.state == octocrab::models::IssueState::Open
                        && issue.labels.iter().any(|label| label.name == *agent_id)
                        && issue.labels.iter().any(|label| label.name == "route:ready")
                })
                .count();
            
            let agent_state = if assigned_count > 0 {
                AgentState::Working(format!("Working on {} task(s)", assigned_count))
            } else {
                AgentState::Available
            };
            
            // Agent is available if under capacity
            if assigned_count < *max_capacity as usize {
                agents.push(Agent {
                    id: agent_id.clone(),
                    capacity: *max_capacity,
                    state: agent_state,
                });
            }
        }
        
        let available_count = agents.len();
        let total_agents = capacities.len();
        println!("📊 Available agents: {} of {} total", available_count, total_agents);
        Ok(agents)
    }

    /// Atomic assignment operation with conflict detection and capacity management
    pub async fn assign_agent_to_issue(&self, agent_id: &str, issue_number: u64) -> Result<(), GitHubError> {
        // Fetch issue to get title for descriptive branch name
        let issue = match self.github_client.fetch_issue(issue_number).await {
            Ok(issue) => issue,
            Err(_) => {
                // Fallback to simple assignment without descriptive branch if issue fetch fails
                return self.assign_agent_to_issue_simple(agent_id, issue_number).await;
            }
        };
        
        self.assign_agent_to_issue_with_title(agent_id, issue_number, &issue.title).await
    }
    
    /// Assignment with descriptive branch name
    async fn assign_agent_to_issue_with_title(&self, agent_id: &str, issue_number: u64, issue_title: &str) -> Result<(), GitHubError> {
        let execution_start = Instant::now();
        let correlation_id = generate_correlation_id();
        let span = create_coordination_span(
            "assign_agent_to_issue", 
            Some(agent_id), 
            Some(issue_number), 
            Some(&correlation_id)
        );
        
        async move {
            tracing::info!(
                agent_id = %agent_id,
                issue_number = issue_number,
                correlation_id = %correlation_id,
                "Starting atomic agent assignment with descriptive branch"
            );
            
            println!("🤖 Attempting atomic assignment: agent {} -> issue #{}", agent_id, issue_number);
        
        // Generate descriptive branch name
        let branch_name = self.generate_descriptive_branch_name(agent_id, issue_number, issue_title);
        
        // ATOMIC OPERATION: All-or-nothing assignment with conflict detection
        {
            let mut assignments = self.assignment_lock.lock().await;
            let mut capacities = self.agent_capacity.lock().await;
            
            // CONFLICT DETECTION: Check if issue already assigned
            if assignments.contains_key(&issue_number) {
                let existing_agent = assignments.get(&issue_number).unwrap();
                
                // Track failed coordination decision
                let _ = self.metrics_tracker.track_coordination_decision(
                    correlation_id.clone(),
                    "assign_agent_to_issue",
                    Some(agent_id),
                    Some(issue_number),
                    &format!("Assignment conflict: Issue #{} already assigned to agent {}", issue_number, existing_agent),
                    execution_start,
                    false,
                    HashMap::new(),
                ).await;
                
                return Err(GitHubError::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Issue #{} already assigned to agent {}", issue_number, existing_agent)
                )));
            }
            
            // CAPACITY MANAGEMENT: Check agent capacity
            let (current, max) = capacities.get(agent_id)
                .ok_or_else(|| GitHubError::IoError(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Unknown agent: {}", agent_id)
                )))?.clone();
            
            if current >= max {
                // Track failed coordination decision
                let mut metadata = HashMap::new();
                metadata.insert("current_capacity".to_string(), current.to_string());
                metadata.insert("max_capacity".to_string(), max.to_string());
                
                let _ = self.metrics_tracker.track_coordination_decision(
                    correlation_id.clone(),
                    "assign_agent_to_issue",
                    Some(agent_id),
                    Some(issue_number),
                    &format!("Capacity exceeded: Agent {} at capacity ({}/{})", agent_id, current, max),
                    execution_start,
                    false,
                    metadata,
                ).await;
                
                return Err(GitHubError::IoError(std::io::Error::new(
                    std::io::ErrorKind::ResourceBusy,
                    format!("Agent {} at capacity ({}/{})", agent_id, current, max)
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
                println!("✅ Issue #{} assigned to GitHub user: {}", issue_number, github_user);
            },
            Err(e) => {
                // ROLLBACK: Remove reservation on failure
                self.rollback_assignment(agent_id, issue_number).await;
                println!("❌ Failed to assign issue #{}: {:?}", issue_number, e);
                
                // Track failed coordination decision
                let mut metadata = HashMap::new();
                metadata.insert("error_type".to_string(), "github_assignment_failed".to_string());
                metadata.insert("error_message".to_string(), format!("{:?}", e));
                
                let _ = self.metrics_tracker.track_coordination_decision(
                    correlation_id.clone(),
                    "assign_agent_to_issue",
                    Some(agent_id),
                    Some(issue_number),
                    &format!("GitHub assignment failed: {:?}", e),
                    execution_start,
                    false,
                    metadata,
                ).await;
                
                return Err(e);
            }
        }
        
        // Step 2: Add agent label to track which agent is working on this
        println!("🏷️  Adding agent label: {}", agent_id);
        match self.github_client.add_label_to_issue(issue_number, agent_id).await {
            Ok(_) => {
                println!("✅ Added agent label: {}", agent_id);
            },
            Err(e) => {
                println!("⚠️  Agent labeling failed but assignment succeeded: {:?}", e);
            }
        }

        // Step 3: Create agent branch using descriptive naming scheme
        println!("🌿 Creating agent branch: {}", branch_name);
        
        match self.github_client.create_branch(&branch_name, "main").await {
            Ok(_) => {
                println!("✅ Branch '{}' created successfully", branch_name);
            },
            Err(e) => {
                println!("⚠️  Branch creation failed: {}", e);
                println!("   📝 Note: Branch may already exist, or you can create it manually");
                // Don't rollback - the issue assignment is the important part
                // Agent can still work without automatic branch creation
            }
        }
        
        println!("🎯 ATOMIC ASSIGNMENT COMPLETE: agent {} -> issue #{}", agent_id, issue_number);
        tracing::info!(
            agent_id = %agent_id,
            issue_number = issue_number,
            "Successfully completed atomic agent assignment"
        );
        
        // Track coordination decision
        let mut metadata = HashMap::new();
        metadata.insert("branch_name".to_string(), branch_name.clone());
        metadata.insert("github_user".to_string(), github_user.to_string());
        
        let _ = self.metrics_tracker.track_coordination_decision(
            correlation_id.clone(),
            "assign_agent_to_issue",
            Some(agent_id),
            Some(issue_number),
            &format!("Successfully assigned agent {} to issue #{}", agent_id, issue_number),
            execution_start,
            true,
            metadata,
        ).await;
        
        Ok(())
        }.instrument(span).await
    }
    
    /// Simple assignment without descriptive branch name (fallback)
    async fn assign_agent_to_issue_simple(&self, agent_id: &str, issue_number: u64) -> Result<(), GitHubError> {
        let execution_start = Instant::now();
        let correlation_id = generate_correlation_id();
        let span = create_coordination_span(
            "assign_agent_to_issue", 
            Some(agent_id), 
            Some(issue_number), 
            Some(&correlation_id)
        );
        
        async move {
            tracing::info!(
                agent_id = %agent_id,
                issue_number = issue_number,
                correlation_id = %correlation_id,
                "Starting atomic agent assignment"
            );
            
            println!("🤖 Attempting atomic assignment: agent {} -> issue #{}", agent_id, issue_number);
        
        // ATOMIC OPERATION: All-or-nothing assignment with conflict detection
        {
            let mut assignments = self.assignment_lock.lock().await;
            let mut capacities = self.agent_capacity.lock().await;
            
            // CONFLICT DETECTION: Check if issue already assigned
            if assignments.contains_key(&issue_number) {
                let existing_agent = assignments.get(&issue_number).unwrap();
                
                // Track failed coordination decision
                let _ = self.metrics_tracker.track_coordination_decision(
                    correlation_id.clone(),
                    "assign_agent_to_issue",
                    Some(agent_id),
                    Some(issue_number),
                    &format!("Assignment conflict: Issue #{} already assigned to agent {}", issue_number, existing_agent),
                    execution_start,
                    false,
                    HashMap::new(),
                ).await;
                
                return Err(GitHubError::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Issue #{} already assigned to agent {}", issue_number, existing_agent)
                )));
            }
            
            // CAPACITY MANAGEMENT: Check agent capacity
            let (current, max) = capacities.get(agent_id)
                .ok_or_else(|| GitHubError::IoError(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Unknown agent: {}", agent_id)
                )))?.clone();
            
            if current >= max {
                // Track failed coordination decision
                let mut metadata = HashMap::new();
                metadata.insert("current_capacity".to_string(), current.to_string());
                metadata.insert("max_capacity".to_string(), max.to_string());
                
                let _ = self.metrics_tracker.track_coordination_decision(
                    correlation_id.clone(),
                    "assign_agent_to_issue",
                    Some(agent_id),
                    Some(issue_number),
                    &format!("Capacity exceeded: Agent {} at capacity ({}/{})", agent_id, current, max),
                    execution_start,
                    false,
                    metadata,
                ).await;
                
                return Err(GitHubError::IoError(std::io::Error::new(
                    std::io::ErrorKind::ResourceBusy,
                    format!("Agent {} at capacity ({}/{})", agent_id, current, max)
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
                println!("✅ Issue #{} assigned to GitHub user: {}", issue_number, github_user);
            },
            Err(e) => {
                // ROLLBACK: Remove reservation on failure
                self.rollback_assignment(agent_id, issue_number).await;
                println!("❌ Failed to assign issue #{}: {:?}", issue_number, e);
                
                // Track failed coordination decision
                let mut metadata = HashMap::new();
                metadata.insert("error_type".to_string(), "github_assignment_failed".to_string());
                metadata.insert("error_message".to_string(), format!("{:?}", e));
                
                let _ = self.metrics_tracker.track_coordination_decision(
                    correlation_id.clone(),
                    "assign_agent_to_issue",
                    Some(agent_id),
                    Some(issue_number),
                    &format!("GitHub assignment failed: {:?}", e),
                    execution_start,
                    false,
                    metadata,
                ).await;
                
                return Err(e);
            }
        }
        
        // Step 2: Add agent label to track which agent is working on this
        println!("🏷️  Adding agent label: {}", agent_id);
        match self.github_client.add_label_to_issue(issue_number, agent_id).await {
            Ok(_) => {
                println!("✅ Added agent label: {}", agent_id);
            },
            Err(e) => {
                println!("⚠️  Agent labeling failed but assignment succeeded: {:?}", e);
            }
        }

        // Step 3: Create agent branch using proper naming scheme
        let branch_name = format!("{}/{}", agent_id, issue_number);
        println!("🌿 Creating agent branch: {}", branch_name);
        
        match self.github_client.create_branch(&branch_name, "main").await {
            Ok(_) => {
                // Branch creation succeeded - the success message is already printed by the client
            },
            Err(e) => {
                println!("⚠️  Branch creation failed: {}", e);
                println!("   📝 Note: Branch may already exist, or you can create it manually");
                // Don't rollback - the issue assignment is the important part
                // Agent can still work without automatic branch creation
            }
        }
        
        println!("🎯 ATOMIC ASSIGNMENT COMPLETE: agent {} -> issue #{}", agent_id, issue_number);
        tracing::info!(
            agent_id = %agent_id,
            issue_number = issue_number,
            "Successfully completed atomic agent assignment"
        );
        
        // Track coordination decision
        let mut metadata = HashMap::new();
        metadata.insert("branch_name".to_string(), format!("{}/{}", agent_id, issue_number));
        metadata.insert("github_user".to_string(), github_user.to_string());
        
        let _ = self.metrics_tracker.track_coordination_decision(
            correlation_id.clone(),
            "assign_agent_to_issue",
            Some(agent_id),
            Some(issue_number),
            &format!("Successfully assigned agent {} to issue #{}", agent_id, issue_number),
            execution_start,
            true,
            metadata,
        ).await;
        
        Ok(())
        }.instrument(span).await
    }
    
    /// Generate descriptive branch name from issue title
    fn generate_descriptive_branch_name(&self, agent_id: &str, issue_number: u64, issue_title: &str) -> String {
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
        
        format!("{}/{}-{}", agent_id, issue_number, slug)
    }
    
    /// Rollback assignment reservation on failure
    async fn rollback_assignment(&self, agent_id: &str, issue_number: u64) {
        let mut assignments = self.assignment_lock.lock().await;
        let mut capacities = self.agent_capacity.lock().await;
        
        assignments.remove(&issue_number);
        
        if let Some((current, max)) = capacities.get(agent_id).cloned() {
            if current > 0 {
                capacities.insert(agent_id.to_string(), (current - 1, max));
                println!("🔄 Rolled back assignment: agent {} (capacity: {}/{})", 
                        agent_id, current - 1, max);
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
                println!("❌ CONSISTENCY ERROR: Agent {} capacity tracking mismatch: tracked={}, actual={}", 
                        agent_id, current, actual_count);
                return Ok(false);
            }
            
            if current > max {
                println!("❌ CONSISTENCY ERROR: Agent {} over-capacity: {}/{}", 
                        agent_id, current, max);
                return Ok(false);
            }
        }
        
        println!("✅ Consistency check passed: {} assignments across {} agents", 
                assignments.len(), capacities.len());
        Ok(true)
    }

    pub async fn update_agent_state(&self, agent_id: &str, new_state: AgentState) -> Result<(), GitHubError> {
        // GitHub-native: State changes reflected in GitHub repository
        // This would update issue status, labels, or branch state
        println!("🔄 Updating agent {} state to {:?}", agent_id, new_state);
        Ok(())
    }
}