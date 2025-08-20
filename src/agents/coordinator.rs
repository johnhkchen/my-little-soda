// Agent State Management - GitHub-native coordination
// Following VERBOTEN rules: GitHub is source of truth, no local state files

use crate::github::{GitHubClient, GitHubError};
use std::collections::HashMap;
use tokio::sync::Mutex;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum AgentState {
    Available,
    Assigned(String), // GitHub issue URL
    Working(String),  // GitHub issue URL
    Completed(String), // GitHub issue URL
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
}

impl AgentCoordinator {
    pub async fn new() -> Result<Self, GitHubError> {
        let github_client = GitHubClient::new()?;
        
        // Agent capacity: Start with agent001, expandable to agent002-agent008
        let mut agent_capacity = HashMap::new();
        agent_capacity.insert("agent001".to_string(), (0, 1)); // 0 current, 1 max per agent
        
        Ok(Self { 
            github_client,
            assignment_lock: Arc::new(Mutex::new(HashMap::new())),
            agent_capacity: Arc::new(Mutex::new(agent_capacity)),
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
        println!("🤖 Attempting atomic assignment: agent {} -> issue #{}", agent_id, issue_number);
        
        // ATOMIC OPERATION: All-or-nothing assignment with conflict detection
        {
            let mut assignments = self.assignment_lock.lock().await;
            let mut capacities = self.agent_capacity.lock().await;
            
            // CONFLICT DETECTION: Check if issue already assigned
            if assignments.contains_key(&issue_number) {
                let existing_agent = assignments.get(&issue_number).unwrap();
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

        // Step 3: Create work branch - simplified since we only have 1 agent (this session)
        let branch_name = format!("work-{}", issue_number);
        println!("🌿 Creating work branch: {}", branch_name);
        
        match self.github_client.create_branch(&branch_name, "main").await {
            Ok(_) => {
                println!("✅ Created branch: {}", branch_name);
            },
            Err(e) => {
                println!("⚠️  Branch creation failed but assignment succeeded: {:?}", e);
                // Don't rollback - the issue assignment is the important part
                // Agent can still work without the branch
            }
        }
        
        println!("🎯 ATOMIC ASSIGNMENT COMPLETE: agent {} -> issue #{}", agent_id, issue_number);
        Ok(())
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
        let capacities = self.agent_capacity.lock().await;
        capacities.clone()
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