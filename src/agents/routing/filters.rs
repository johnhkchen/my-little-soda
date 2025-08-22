use crate::github::{GitHubClient, GitHubError};
use crate::agents::routing::assignment::AssignmentOperations;
use octocrab::models::issues::Issue;

#[derive(Debug)]
pub struct IssueFilter {
    assignment_ops: AssignmentOperations,
}

impl IssueFilter {
    pub fn new(assignment_ops: AssignmentOperations) -> Self {
        Self {
            assignment_ops,
        }
    }

    pub async fn fetch_routable_issues(&self, github_client: &GitHubClient) -> Result<Vec<Issue>, GitHubError> {
        let all_issues = github_client.fetch_issues().await?;
        
        let mut routable_issues = Vec::new();
        
        for issue in all_issues {
            let is_open = issue.state == octocrab::models::IssueState::Open;
            
            let has_route_ready = issue.labels.iter()
                .any(|label| label.name == "route:ready");
            let has_route_land = issue.labels.iter()
                .any(|label| label.name == "route:ready_to_merge");
            let has_route_unblocker = issue.labels.iter()
                .any(|label| label.name == "route:unblocker");
            let has_route_review = issue.labels.iter()
                .any(|label| label.name == "route:review");
            
            let has_agent_label = issue.labels.iter()
                .any(|label| label.name.starts_with("agent"));
            
            let is_human_only = issue.labels.iter()
                .any(|label| label.name == "route:human-only");
            
            let is_routable = if has_route_review {
                false
            } else if has_route_unblocker {
                if has_agent_label {
                    let agent_labels: Vec<&str> = issue.labels.iter()
                        .filter(|label| label.name.starts_with("agent"))
                        .map(|label| label.name.as_str())
                        .collect();
                    !self.assignment_ops.is_agent_branch_completed(issue.number, &agent_labels)
                } else {
                    true
                }
            } else if has_route_land {
                true
            } else if has_route_ready {
                let agent_labels: Vec<&str> = issue.labels.iter()
                    .filter(|label| label.name.starts_with("agent"))
                    .map(|label| label.name.as_str())
                    .collect();
                !self.assignment_ops.is_agent_branch_completed(issue.number, &agent_labels)
            } else {
                false
            };
            
            if is_open && is_routable && !is_human_only {
                match github_client.issue_has_blocking_pr(issue.number).await {
                    Ok(has_blocking_pr) => {
                        if !has_blocking_pr {
                            routable_issues.push(issue);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to check PR status for issue #{}: {:?}", issue.number, e);
                        routable_issues.push(issue);
                    }
                }
            }
        }
            
        Ok(routable_issues)
    }

    pub fn filter_available_issues(&self, all_issues: &[Issue], current_user: &str) -> Vec<Issue> {
        let mut available_issues = Vec::new();
        
        for issue in all_issues {
            let is_open = issue.state == octocrab::models::IssueState::Open;
            let has_route_ready = issue.labels.iter()
                .any(|label| label.name == "route:ready");
            let has_route_land = issue.labels.iter()
                .any(|label| label.name == "route:ready_to_merge");
            let has_route_unblocker = issue.labels.iter()
                .any(|label| label.name == "route:unblocker");
            let has_route_review = issue.labels.iter()
                .any(|label| label.name == "route:review");
            
            if !is_open || (!has_route_ready && !has_route_land && !has_route_unblocker) {
                continue;
            }
            
            if has_route_review {
                continue;
            }
            
            let is_human_only = issue.labels.iter()
                .any(|label| label.name == "route:human-only");
            
            let is_acceptable = match &issue.assignee {
                None => !is_human_only,
                Some(assignee) => assignee.login == current_user
            };
            
            if is_acceptable {
                if has_route_ready || has_route_unblocker {
                    let agent_labels: Vec<&str> = issue.labels.iter()
                        .filter(|label| label.name.starts_with("agent"))
                        .map(|label| label.name.as_str())
                        .collect();
                    
                    if self.assignment_ops.is_agent_branch_completed(issue.number, &agent_labels) {
                        continue;
                    }
                }

                available_issues.push(issue.clone());
            }
        }
        
        available_issues
    }

    pub fn filter_assigned_issues(&self, all_issues: &[Issue], current_user: &str) -> Vec<Issue> {
        all_issues
            .iter()
            .filter(|issue| {
                let is_open = issue.state == octocrab::models::IssueState::Open;
                let is_assigned_to_me = issue.assignee.as_ref()
                    .map(|assignee| assignee.login == current_user)
                    .unwrap_or(false);
                let has_route_label = issue.labels.iter()
                    .any(|label| label.name == "route:ready");
                let has_route_review = issue.labels.iter()
                    .any(|label| label.name == "route:review");
                
                let basic_filter = is_open && is_assigned_to_me && has_route_label && !has_route_review;
                
                if basic_filter {
                    let agent_labels: Vec<&str> = issue.labels.iter()
                        .filter(|label| label.name.starts_with("agent"))
                        .map(|label| label.name.as_str())
                        .collect();
                    
                    !self.assignment_ops.is_agent_branch_completed(issue.number, &agent_labels)
                } else {
                    false
                }
            })
            .cloned()
            .collect()
    }
}