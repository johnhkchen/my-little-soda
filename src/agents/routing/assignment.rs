use crate::github::{GitHubClient, GitHubError};
use crate::agents::{Agent, AgentCoordinator};
use crate::git::{GitOperations, Git2Operations};
use octocrab::models::issues::Issue;
use std::process::Command;

#[derive(Debug)]
pub struct AssignmentOperations;

impl AssignmentOperations {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_branch_name(&self, agent_id: &str, issue_number: u64, issue_title: &str) -> String {
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

    pub async fn create_agent_branch(&self, github_client: &GitHubClient, agent_id: &str, issue_number: u64, issue_title: &str) -> Result<String, GitHubError> {
        let branch_name = self.generate_branch_name(agent_id, issue_number, issue_title);
        
        match github_client.create_branch(&branch_name, "main").await {
            Ok(()) => {
                println!("✅ Branch '{}' created successfully", branch_name);
                Ok(branch_name)
            }
            Err(e) => {
                tracing::warn!("Branch creation failed for {}: {:?}", branch_name, e);
                println!("⚠️  Branch creation failed for '{}', continuing with task assignment", branch_name);
                Ok(branch_name)
            }
        }
    }

    pub async fn assign_agent_to_issue(&self, coordinator: &AgentCoordinator, agent_id: &str, issue_number: u64) -> Result<(), GitHubError> {
        coordinator.assign_agent_to_issue(agent_id, issue_number).await
    }

    pub fn is_agent_branch_completed(&self, issue_number: u64, agent_labels: &[&str]) -> bool {
        if let Some(agent_label) = agent_labels.iter().find(|label| label.starts_with("agent")) {
            let agent_id = agent_label;
            
            let old_branch_name = format!("{}/{}", agent_id, issue_number);
            if self.branch_has_commits_ahead_of_main(&old_branch_name) {
                return true;
            }
            
            return self.check_agent_branch_with_slug(agent_id, issue_number);
        }
        
        self.check_any_agent_branch_completed(issue_number)
    }

    fn check_agent_branch_with_slug(&self, agent_id: &str, issue_number: u64) -> bool {
        let pattern = format!("{}/{}-", agent_id, issue_number);
        
        if let Ok(output) = std::process::Command::new("git")
            .args(&["branch", "-a"])
            .output()
        {
            if output.status.success() {
                let branches = String::from_utf8_lossy(&output.stdout);
                for line in branches.lines() {
                    let branch_name = line.trim().trim_start_matches("* ").trim_start_matches("remotes/origin/");
                    if branch_name.starts_with(&pattern) {
                        if self.branch_has_commits_ahead_of_main(branch_name) {
                            return true;
                        }
                    }
                }
            }
        }
        
        false
    }

    fn check_any_agent_branch_completed(&self, issue_number: u64) -> bool {
        let common_agents = ["agent001", "agent002", "agent003", "agent004", "agent005"];
        
        for agent_id in &common_agents {
            let old_branch_name = format!("{}/{}", agent_id, issue_number);
            if self.branch_has_commits_ahead_of_main(&old_branch_name) {
                return true;
            }
            
            if self.check_agent_branch_with_slug(agent_id, issue_number) {
                return true;
            }
        }
        
        false
    }

    fn branch_has_commits_ahead_of_main(&self, branch_name: &str) -> bool {
        let git_ops = match Git2Operations::new(".") {
            Ok(ops) => ops,
            Err(_) => return false,
        };
        
        let branch_exists = git_ops.branch_exists(branch_name).unwrap_or(false);
        
        if !branch_exists {
            let remote_branch_exists = git_ops.remote_branch_exists("origin", branch_name).unwrap_or(false);
            
            if !remote_branch_exists {
                return false;
            }
            
            let _ = git_ops.fetch("origin");
        }
        
        match git_ops.get_commits(Some("main"), Some(branch_name)) {
            Ok(commits) => !commits.is_empty(),
            Err(_) => false,
        }
    }
}