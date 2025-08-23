use octocrab::{Octocrab, Error as OctocrabError};
use std::fs;
use std::path::Path;
use async_trait::async_trait;
use super::{
    issues::IssueHandler, 
    pulls::{PullRequestHandler, PullRequestStatus}, 
    branches::BranchHandler, 
    comments::CommentHandler,
    actions::ActionsHandler,
    types::{ConflictAnalysis, ConflictRecoveryData, SafeMergeResult},
    errors::GitHubError
};
use crate::github::retry::GitHubRetryHandler;


/// Trait for GitHub operations to enable testing with mocks
#[async_trait]
pub trait GitHubOps {
    async fn fetch_issues(&self) -> Result<Vec<octocrab::models::issues::Issue>, GitHubError>;
    async fn fetch_issues_with_state(&self, state: Option<octocrab::params::State>) -> Result<Vec<octocrab::models::issues::Issue>, GitHubError>;
    async fn assign_issue(&self, issue_number: u64, assignee: &str) -> Result<(), GitHubError>;
    async fn add_label_to_issue(&self, issue_number: u64, label: &str) -> Result<(), GitHubError>;
    async fn create_issue(&self, title: &str, body: &str, labels: Vec<String>) -> Result<octocrab::models::issues::Issue, GitHubError>;
    async fn create_branch(&self, branch_name: &str, from_branch: &str) -> Result<(), GitHubError>;
    async fn issue_has_blocking_pr(&self, issue_number: u64) -> Result<bool, GitHubError>;
    fn owner(&self) -> &str;
    fn repo(&self) -> &str;
}



#[derive(Debug, Clone)]
pub struct GitHubClient {
    pub issues: IssueHandler,
    pub pulls: PullRequestHandler,
    pub branches: BranchHandler,
    pub comments: CommentHandler,
    pub actions: ActionsHandler,
    owner: String,
    repo: String,
    retry_handler: GitHubRetryHandler,
}

impl GitHubClient {
    pub fn new() -> Result<Self, GitHubError> {
        let token = Self::read_token()?;
        let (owner, repo) = Self::read_config()?;
        
        let octocrab = Octocrab::builder()
            .personal_token(token)
            .build()?;

        Ok(GitHubClient {
            issues: IssueHandler::new(octocrab.clone(), owner.clone(), repo.clone()),
            pulls: PullRequestHandler::new(octocrab.clone(), owner.clone(), repo.clone()),
            branches: BranchHandler::new(octocrab.clone(), owner.clone(), repo.clone()),
            comments: CommentHandler::new(octocrab.clone(), owner.clone(), repo.clone()),
            actions: ActionsHandler::new(octocrab.clone(), owner.clone(), repo.clone()),
            owner,
            repo,
            retry_handler: GitHubRetryHandler::default(),
        })
    }

    fn read_token() -> Result<String, GitHubError> {
        // First try environment variable (set by flox)
        if let Ok(token) = std::env::var("MY_LITTLE_SODA_GITHUB_TOKEN") {
            if token != "YOUR_GITHUB_TOKEN_HERE" && !token.is_empty() {
                return Ok(token);
            }
        }
        
        // Fall back to file-based configuration
        let token_path = ".clambake/credentials/github_token";
        if !Path::new(token_path).exists() {
            return Err(GitHubError::TokenNotFound(format!(
                "GitHub token not found. Please set MY_LITTLE_SODA_GITHUB_TOKEN environment variable or create {} with your GitHub personal access token.",
                token_path
            )));
        }
        
        let token = fs::read_to_string(token_path)?
            .trim()
            .to_string();
            
        if token == "YOUR_GITHUB_TOKEN_HERE" || token.is_empty() {
            return Err(GitHubError::TokenNotFound(
                "Please replace YOUR_GITHUB_TOKEN_HERE with your actual GitHub token in the credential file".to_string()
            ));
        }
        
        Ok(token)
    }

    fn read_config() -> Result<(String, String), GitHubError> {
        // First try environment variables (set by flox)
        let env_owner = std::env::var("GITHUB_OWNER").unwrap_or_default();
        let env_repo = std::env::var("GITHUB_REPO").unwrap_or_default();
        
        if !env_owner.is_empty() && !env_repo.is_empty() 
            && env_owner != "your-github-username" 
            && env_repo != "your-repo-name" {
            return Ok((env_owner, env_repo));
        }
        
        // Fall back to file-based configuration
        let owner_path = ".clambake/credentials/github_owner";
        let repo_path = ".clambake/credentials/github_repo";
        
        if !Path::new(owner_path).exists() {
            return Err(GitHubError::ConfigNotFound(format!(
                "GitHub config not found. Please set GITHUB_OWNER and GITHUB_REPO environment variables or create {} with your GitHub username/organization.",
                owner_path
            )));
        }
        
        if !Path::new(repo_path).exists() {
            return Err(GitHubError::ConfigNotFound(format!(
                "GitHub repo not found at {}. Please create this file with your repository name.",
                repo_path
            )));
        }
        
        let owner = fs::read_to_string(owner_path)?.trim().to_string();
        let repo = fs::read_to_string(repo_path)?.trim().to_string();
        
        if owner.is_empty() || repo.is_empty() 
            || owner == "your-github-username" 
            || repo == "your-repo-name" {
            return Err(GitHubError::ConfigNotFound(
                "GitHub owner and repo must be set to actual values, not placeholders".to_string()
            ));
        }
        
        Ok((owner, repo))
    }

    pub async fn fetch_issues(&self) -> Result<Vec<octocrab::models::issues::Issue>, GitHubError> {
        self.issues.fetch_issues().await
    }

    pub async fn fetch_issues_with_state(&self, state: Option<octocrab::params::State>) -> Result<Vec<octocrab::models::issues::Issue>, GitHubError> {
        self.issues.fetch_issues_with_state(state).await
    }

    pub async fn fetch_issue(&self, issue_number: u64) -> Result<octocrab::models::issues::Issue, GitHubError> {
        self.issues.fetch_issue(issue_number).await
    }

    pub async fn assign_issue(&self, issue_number: u64, assignee: &str) -> Result<octocrab::models::issues::Issue, GitHubError> {
        self.issues.assign_issue(issue_number, assignee).await
    }

    pub async fn create_branch(&self, branch_name: &str, from_branch: &str) -> Result<(), GitHubError> {
        self.branches.create_branch(branch_name, from_branch).await
    }

    pub async fn delete_branch(&self, branch_name: &str) -> Result<(), GitHubError> {
        self.branches.delete_branch(branch_name).await
    }

    pub async fn branch_exists(&self, branch_name: &str) -> Result<bool, GitHubError> {
        self.branches.branch_exists(branch_name).await
    }

    pub async fn create_pull_request(
        &self,
        title: &str,
        head_branch: &str,
        base_branch: &str,
        body: &str,
    ) -> Result<octocrab::models::pulls::PullRequest, GitHubError> {
        self.pulls.create_pull_request(title, head_branch, base_branch, body).await
    }

    pub async fn get_pull_request(&self, pr_number: u64) -> Result<octocrab::models::pulls::PullRequest, GitHubError> {
        self.pulls.get_pull_request(pr_number).await
    }

    /// Check if a PR is ready for merging
    pub async fn is_pr_mergeable(&self, pr: &octocrab::models::pulls::PullRequest) -> Result<bool, GitHubError> {
        self.pulls.is_pr_mergeable(pr).await
    }
    
    /// Get detailed PR status including CI and review status
    pub async fn get_pr_status(&self, pr_number: u64) -> Result<PullRequestStatus, GitHubError> {
        self.pulls.get_pr_status(pr_number).await
    }

    pub async fn merge_pull_request(
        &self,
        pr_number: u64,
        merge_method: Option<&str>,
    ) -> Result<octocrab::models::pulls::PullRequest, GitHubError> {
        self.pulls.merge_pull_request(pr_number, merge_method).await
    }

    pub fn owner(&self) -> &str {
        &self.owner
    }

    pub fn repo(&self) -> &str {
        &self.repo
    }
    
    pub async fn add_label_to_issue(&self, issue_number: u64, label: &str) -> Result<(), GitHubError> {
        self.issues.add_label_to_issue(issue_number, label).await
    }

    pub async fn remove_label_from_issue(&self, issue_number: u64, label: &str) -> Result<(), GitHubError> {
        self.issues.remove_label(issue_number, label).await
    }

    pub async fn create_issue(&self, title: &str, body: &str, labels: Vec<String>) -> Result<octocrab::models::issues::Issue, GitHubError> {
        self.issues.create_issue(title, body, labels).await
    }

    pub async fn fetch_open_pull_requests(&self) -> Result<Vec<octocrab::models::pulls::PullRequest>, GitHubError> {
        self.pulls.fetch_open_pull_requests().await
    }

    /// Check if an issue has an open PR that references it
    /// Returns true if the issue has an open PR WITHOUT route:ready_to_merge label
    pub async fn issue_has_blocking_pr(&self, issue_number: u64) -> Result<bool, GitHubError> {
        let open_prs = self.fetch_open_pull_requests().await?;
        self.issues.issue_has_blocking_pr(issue_number, &open_prs).await
    }

    /// Get the number of PRs created in the last hour
    pub async fn get_pr_creation_rate(&self) -> Result<u32, GitHubError> {
        self.pulls.get_pr_creation_rate().await
    }

    /// Enhanced merge conflict detection with detailed diagnostics
    pub async fn detect_merge_conflicts(&self, pr_number: u64) -> Result<ConflictAnalysis, GitHubError> {
        self.pulls.detect_merge_conflicts(pr_number).await
    }

    /// Create a recovery PR for conflicted work with human review request
    pub async fn create_conflict_recovery_pr(&self, original_pr: u64, work_data: ConflictRecoveryData) -> Result<octocrab::models::pulls::PullRequest, GitHubError> {
        // Use pulls handler but pass issue handler for label operations
        let pr = self.pulls.create_conflict_recovery_pr(original_pr, work_data, &self.issues).await?;
        Ok(pr)
    }

    /// Enhanced merge attempt with conflict detection and automatic recovery
    pub async fn safe_merge_pull_request(
        &self,
        pr_number: u64,
        agent_id: &str,
        issue_number: u64,
        merge_method: Option<&str>,
    ) -> Result<SafeMergeResult, GitHubError> {
        self.pulls.safe_merge_pull_request(pr_number, agent_id, issue_number, merge_method, &self.issues).await
    }
}


// Implement the trait for GitHubClient
#[async_trait]
impl GitHubOps for GitHubClient {
    async fn fetch_issues(&self) -> Result<Vec<octocrab::models::issues::Issue>, GitHubError> {
        self.fetch_issues().await
    }
    
    async fn fetch_issues_with_state(&self, state: Option<octocrab::params::State>) -> Result<Vec<octocrab::models::issues::Issue>, GitHubError> {
        self.fetch_issues_with_state(state).await
    }
    
    async fn assign_issue(&self, issue_number: u64, assignee: &str) -> Result<(), GitHubError> {
        self.assign_issue(issue_number, assignee).await?;
        Ok(())
    }
    
    async fn add_label_to_issue(&self, issue_number: u64, label: &str) -> Result<(), GitHubError> {
        self.add_label_to_issue(issue_number, label).await
    }
    
    async fn create_issue(&self, title: &str, body: &str, labels: Vec<String>) -> Result<octocrab::models::issues::Issue, GitHubError> {
        self.create_issue(title, body, labels).await
    }
    
    async fn create_branch(&self, branch_name: &str, from_branch: &str) -> Result<(), GitHubError> {
        self.create_branch(branch_name, from_branch).await
    }
    
    async fn issue_has_blocking_pr(&self, issue_number: u64) -> Result<bool, GitHubError> {
        self.issue_has_blocking_pr(issue_number).await
    }
    
    fn owner(&self) -> &str {
        self.owner()
    }
    
    fn repo(&self) -> &str {
        self.repo()
    }
}

