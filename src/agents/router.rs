// GitHub Issues → Agent Assignment Router
// Following VERBOTEN rules: GitHub is source of truth, atomic operations

use crate::github::{GitHubClient, GitHubError};
use crate::agents::{Agent, AgentCoordinator};
use octocrab::models::issues::Issue;
use std::collections::HashMap;

#[derive(Debug)]
pub struct AgentRouter {
    github_client: GitHubClient,
    coordinator: AgentCoordinator,
}

#[derive(Debug)]
pub struct RoutingAssignment {
    pub issue: Issue,
    pub assigned_agent: Agent,
}

impl AgentRouter {
    pub async fn new() -> Result<Self, GitHubError> {
        let github_client = GitHubClient::new()?;
        let coordinator = AgentCoordinator::new().await?;
        
        Ok(Self {
            github_client,
            coordinator,
        })
    }

    pub async fn fetch_routable_issues(&self) -> Result<Vec<Issue>, GitHubError> {
        // GitHub-native: Only route issues that exist in GitHub
        let all_issues = self.github_client.fetch_issues().await?;
        
        // Filter for issues that can be routed to agents
        // Must have route:ready label and NOT have agent labels (agents not yet assigned)
        let routable_issues: Vec<Issue> = all_issues
            .into_iter()
            .filter(|issue| {
                // Must be open and have route:ready label
                let is_open = issue.state == octocrab::models::IssueState::Open;
                let has_route_label = issue.labels.iter()
                    .any(|label| label.name == "route:ready");
                
                // Key fix: Allow human-assigned issues, but NOT agent-assigned issues
                let has_agent_label = issue.labels.iter()
                    .any(|label| label.name.starts_with("agent"));
                
                // Human-only filtering: Exclude issues marked for human-only assignment
                let is_human_only = issue.labels.iter()
                    .any(|label| label.name == "route:human-only");
                
                is_open && has_route_label && !has_agent_label && !is_human_only
            })
            .collect();
            
        Ok(routable_issues)
    }

    fn get_issue_priority(&self, issue: &Issue) -> u32 {
        // Priority based on labels: higher number = higher priority
        if issue.labels.iter().any(|label| label.name == "route:priority-high") {
            3 // Highest priority
        } else if issue.labels.iter().any(|label| label.name == "route:priority-medium") {
            2 // Medium priority
        } else if issue.labels.iter().any(|label| label.name == "route:priority-low") {
            1 // Low priority
        } else {
            0 // No priority label = lowest priority
        }
    }

    pub async fn route_issues_to_agents(&self) -> Result<Vec<RoutingAssignment>, GitHubError> {
        let mut issues = self.fetch_routable_issues().await?;
        let available_agents = self.coordinator.get_available_agents().await?;
        
        // Sort issues by priority: high priority first
        issues.sort_by(|a, b| {
            let a_priority = self.get_issue_priority(a);
            let b_priority = self.get_issue_priority(b);
            b_priority.cmp(&a_priority) // Reverse order: high priority first
        });
        
        let mut assignments = Vec::new();
        
        // Simplified: Single agent model - assign one task if available
        if let Some(agent) = available_agents.first() {
            if let Some(issue) = issues.first() {
                let assignment = RoutingAssignment {
                    issue: issue.clone(),
                    assigned_agent: agent.clone(),
                };
                
                // Assign to GitHub user (human) and create work branch  
                self.coordinator.assign_agent_to_issue(&agent.id, issue.number).await?;
                
                assignments.push(assignment);
            }
        }
        
        Ok(assignments)
    }

    pub async fn pop_task_assigned_to_me(&self) -> Result<Option<RoutingAssignment>, GitHubError> {
        // Get issues assigned to current user (repo owner) only
        let all_issues = self.github_client.fetch_issues().await?;
        let current_user = self.github_client.owner();
        
        // Filter for issues assigned to current user with route:ready label
        let mut my_issues: Vec<_> = all_issues
            .into_iter()
            .filter(|issue| {
                let is_open = issue.state == octocrab::models::IssueState::Open;
                let is_assigned_to_me = issue.assignee.as_ref()
                    .map(|assignee| assignee.login == current_user)
                    .unwrap_or(false);
                let has_route_label = issue.labels.iter()
                    .any(|label| label.name == "route:ready");
                
                is_open && is_assigned_to_me && has_route_label
            })
            .collect();
            
        if my_issues.is_empty() {
            return Ok(None);
        }
        
        // Sort by priority
        my_issues.sort_by(|a, b| {
            let a_priority = self.get_issue_priority(a);
            let b_priority = self.get_issue_priority(b);
            b_priority.cmp(&a_priority)
        });
        
        // Get the first available agent and the highest priority assigned issue
        let available_agents = self.coordinator.get_available_agents().await?;
        if let (Some(issue), Some(agent)) = (my_issues.first(), available_agents.first()) {
            // Create branch for the assigned issue (no need to assign again)
            let branch_name = format!("{}/{}", agent.id, issue.number);
            println!("🌿 Creating agent branch: {}", branch_name);
            let _ = self.github_client.create_branch(&branch_name, "main").await;
            
            // Return the assignment
            Ok(Some(RoutingAssignment {
                issue: issue.clone(),
                assigned_agent: agent.clone(),
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn pop_any_available_task(&self) -> Result<Option<RoutingAssignment>, GitHubError> {
        // Get any available task (unassigned OR assigned to me)
        let all_issues = self.github_client.fetch_issues().await?;
        let current_user = self.github_client.owner();
        
        // Filter for issues that are either unassigned or assigned to current user
        let mut available_issues: Vec<_> = all_issues
            .into_iter()
            .filter(|issue| {
                let is_open = issue.state == octocrab::models::IssueState::Open;
                let has_route_label = issue.labels.iter()
                    .any(|label| label.name == "route:ready");
                
                if !is_open || !has_route_label {
                    return false;
                }
                
                // Check if this is a human-only task
                let is_human_only = issue.labels.iter()
                    .any(|label| label.name == "route:human-only");
                
                // Accept based on assignment status and human-only filtering
                match &issue.assignee {
                    None => {
                        // Unassigned tasks: exclude human-only tasks (bots can't take them)
                        !is_human_only
                    },
                    Some(assignee) => {
                        // Tasks assigned to current user: allow regardless of human-only status
                        assignee.login == current_user
                    }
                }
            })
            .collect();
            
        if available_issues.is_empty() {
            return Ok(None);
        }
        
        // Sort by priority
        available_issues.sort_by(|a, b| {
            let a_priority = self.get_issue_priority(a);
            let b_priority = self.get_issue_priority(b);
            b_priority.cmp(&a_priority)
        });
        
        // Get the first available agent and the highest priority issue
        let available_agents = self.coordinator.get_available_agents().await?;
        if let (Some(issue), Some(agent)) = (available_issues.first(), available_agents.first()) {
            // If unassigned, assign it. If already assigned to me, just create branch
            if issue.assignee.is_none() {
                self.coordinator.assign_agent_to_issue(&agent.id, issue.number).await?;
            } else {
                // Already assigned to me, just create branch
                let branch_name = format!("{}/{}", agent.id, issue.number);
                println!("🌿 Creating agent branch: {}", branch_name);
                let _ = self.github_client.create_branch(&branch_name, "main").await;
            }
            
            // Return the assignment
            Ok(Some(RoutingAssignment {
                issue: issue.clone(),
                assigned_agent: agent.clone(),
            }))
        } else {
            Ok(None)
        }
    }

    // Legacy method - keeping for backward compatibility
    pub async fn pop_next_task(&self) -> Result<Option<RoutingAssignment>, GitHubError> {
        // Delegate to the broader "any available task" method
        self.pop_any_available_task().await
    }

    pub async fn route_specific_issue(&self, issue_number: u64) -> Result<Option<RoutingAssignment>, GitHubError> {
        let issue = self.github_client.fetch_issue(issue_number).await?;
        let available_agents = self.coordinator.get_available_agents().await?;
        
        if let Some(agent) = available_agents.first() {
            self.coordinator.assign_agent_to_issue(&agent.id, issue.number).await?;
            
            Ok(Some(RoutingAssignment {
                issue,
                assigned_agent: agent.clone(),
            }))
        } else {
            Ok(None)
        }
    }

    // Public access to coordinator functionality for status command
    pub async fn get_agent_status(&self) -> Result<HashMap<String, (u32, u32)>, GitHubError> {
        Ok(self.coordinator.get_agent_utilization().await)
    }

    pub fn get_github_client(&self) -> &GitHubClient {
        &self.github_client
    }
}