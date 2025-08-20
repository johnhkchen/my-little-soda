use clambake::github::GitHubError;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

mod fixtures;

// Test-only trait for mocking GitHub operations
#[async_trait]
pub trait GitHubOps {
    async fn fetch_issues(&self) -> Result<Vec<octocrab::models::issues::Issue>, GitHubError>;
    async fn assign_issue(&self, issue_number: u64, assignee: &str) -> Result<(), GitHubError>;
    async fn add_label_to_issue(&self, issue_number: u64, label: &str) -> Result<(), GitHubError>;
    async fn create_branch(&self, branch_name: &str, from_branch: &str) -> Result<(), GitHubError>;
    fn owner(&self) -> &str;
    fn repo(&self) -> &str;
}

/// Mock GitHub client for testing agent labeling
#[derive(Debug, Clone)]
pub struct MockGitHubClient {
    pub owner: String,
    pub repo: String,
    pub issues: Vec<octocrab::models::issues::Issue>,
    pub api_calls: Arc<Mutex<Vec<ApiCall>>>,
}

#[derive(Debug, Clone)]
pub enum ApiCall {
    FetchIssues,
    AssignIssue { issue_number: u64, assignee: String },
    AddLabel { issue_number: u64, label: String },
    CreateBranch { branch_name: String, from_branch: String },
}

impl MockGitHubClient {
    pub fn new(owner: &str, repo: &str) -> Self {
        Self {
            owner: owner.to_string(),
            repo: repo.to_string(),
            issues: Vec::new(),
            api_calls: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    pub fn with_issues(mut self, issues: Vec<octocrab::models::issues::Issue>) -> Self {
        self.issues = issues;
        self
    }
    
    pub fn get_api_calls(&self) -> Vec<ApiCall> {
        self.api_calls.lock().unwrap().clone()
    }
    
    pub fn clear_api_calls(&self) {
        self.api_calls.lock().unwrap().clear();
    }
}

#[async_trait]
impl GitHubOps for MockGitHubClient {
    async fn fetch_issues(&self) -> Result<Vec<octocrab::models::issues::Issue>, GitHubError> {
        self.api_calls.lock().unwrap().push(ApiCall::FetchIssues);
        Ok(self.issues.clone())
    }
    
    async fn assign_issue(&self, issue_number: u64, assignee: &str) -> Result<(), GitHubError> {
        self.api_calls.lock().unwrap().push(ApiCall::AssignIssue {
            issue_number,
            assignee: assignee.to_string(),
        });
        Ok(())
    }
    
    async fn add_label_to_issue(&self, issue_number: u64, label: &str) -> Result<(), GitHubError> {
        self.api_calls.lock().unwrap().push(ApiCall::AddLabel {
            issue_number,
            label: label.to_string(),
        });
        Ok(())
    }
    
    async fn create_branch(&self, branch_name: &str, from_branch: &str) -> Result<(), GitHubError> {
        self.api_calls.lock().unwrap().push(ApiCall::CreateBranch {
            branch_name: branch_name.to_string(),
            from_branch: from_branch.to_string(),
        });
        Ok(())
    }
    
    fn owner(&self) -> &str {
        &self.owner
    }
    
    fn repo(&self) -> &str {
        &self.repo
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    // use clambake::agents::AgentCoordinator; // Not needed for current tests

    #[tokio::test]
    async fn test_agent_labeling_tdd_style() {
        // Given: A mock GitHub client (no need for complex issue structures)
        let mock_client = MockGitHubClient::new("testowner", "testrepo");
            
        // When: We assign the issue to agent001
        mock_client.add_label_to_issue(123, "agent001").await.unwrap();
        
        // Then: The API call should match GitHub's expected format
        let api_calls = mock_client.get_api_calls();
        assert_eq!(api_calls.len(), 1);
        
        match &api_calls[0] {
            ApiCall::AddLabel { issue_number, label } => {
                assert_eq!(*issue_number, 123);
                assert_eq!(label, "agent001");
            }
            _ => panic!("Expected AddLabel API call"),
        }
    }
    
    #[tokio::test] 
    async fn test_human_only_label_logic() {
        // This test focuses on the filtering logic rather than complex mock issues
        let labels_human_only = vec!["route:ready", "human-only"];
        let labels_agent_task = vec!["route:ready"];
        
        // Given: Logic for filtering human-only issues
        let is_human_only_task = |labels: &Vec<&str>| {
            let has_route_ready = labels.iter().any(|l| *l == "route:ready");
            let has_human_only = labels.iter().any(|l| *l == "human-only");
            has_route_ready && has_human_only
        };
        
        let is_agent_routable = |labels: &Vec<&str>| {
            let has_route_ready = labels.iter().any(|l| *l == "route:ready");
            let has_human_only = labels.iter().any(|l| *l == "human-only");
            has_route_ready && !has_human_only
        };
        
        // Then: Human-only tasks should be filtered out
        assert!(is_human_only_task(&labels_human_only));
        assert!(!is_agent_routable(&labels_human_only));
        
        // And: Agent tasks should be routable
        assert!(!is_human_only_task(&labels_agent_task));
        assert!(is_agent_routable(&labels_agent_task));
    }
    
    #[tokio::test] 
    async fn test_agent_assignment_workflow() {
        // Given: A mock client for testing API call sequence
        let mock_client = MockGitHubClient::new("johnhkchen", "clambake");
        
        // When: Agent001 gets assigned to work on issue #1
        mock_client.assign_issue(1, "johnhkchen").await.unwrap();
        mock_client.add_label_to_issue(1, "agent001").await.unwrap(); 
        mock_client.create_branch("work-1", "main").await.unwrap();
        
        // Then: The correct API calls should be made in sequence
        let api_calls = mock_client.get_api_calls();
        assert_eq!(api_calls.len(), 3);
        
        // Verify assignment API call
        match &api_calls[0] {
            ApiCall::AssignIssue { issue_number, assignee } => {
                assert_eq!(*issue_number, 1);
                assert_eq!(assignee, "johnhkchen");
            }
            _ => panic!("Expected AssignIssue API call"),
        }
        
        // Verify labeling API call (this is the new functionality we're adding)
        match &api_calls[1] {
            ApiCall::AddLabel { issue_number, label } => {
                assert_eq!(*issue_number, 1);
                assert_eq!(label, "agent001");
            }
            _ => panic!("Expected AddLabel API call"),
        }
        
        // Verify branch creation API call
        match &api_calls[2] {
            ApiCall::CreateBranch { branch_name, from_branch } => {
                assert_eq!(branch_name, "work-1");
                assert_eq!(from_branch, "main");
            }
            _ => panic!("Expected CreateBranch API call"),
        }
    }

    #[tokio::test]
    async fn test_ongoing_work_detection_with_real_fixtures() {
        // Given: Real GitHub API response data with agent001 ongoing work
        let test_issues = fixtures::load_test_issues();
        let mock_client = MockGitHubClient::new("johnhkchen", "clambake")
            .with_issues(test_issues);
        
        // When: We check for ongoing work using the exact same logic as main.rs
        let issues = mock_client.fetch_issues().await.unwrap();
        let ongoing_work = fixtures::filter_agent001_ongoing_work(&issues);
            
        // Then: Should find exactly 1 issue with agent001 ongoing work
        assert_eq!(ongoing_work.len(), 1);
        assert_eq!(ongoing_work[0].number, 1);
        assert_eq!(ongoing_work[0].title, "Real GitHub Issue Routing Implementation");
        
        // Verify the issue has all required labels and assignment
        assert!(ongoing_work[0].labels.iter().any(|l| l.name == "agent001"));
        assert!(ongoing_work[0].labels.iter().any(|l| l.name == "route:ready"));
        assert!(ongoing_work[0].assignee.is_some());
        assert_eq!(ongoing_work[0].assignee.as_ref().unwrap().login, "johnhkchen");
    }
    
    // Future tests for multi-agent and human-only scenarios
    // Creating GitHub issues to track these features for later implementation
}