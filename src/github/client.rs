use super::{
    actions::ActionsHandler,
    branches::BranchHandler,
    comments::CommentHandler,
    errors::GitHubError,
    issues::IssueHandler,
    pulls::{PullRequestHandler, PullRequestStatus},
    types::{ConflictAnalysis, ConflictRecoveryData, SafeMergeResult},
};
use crate::github::retry::GitHubRetryHandler;
use async_trait::async_trait;
use octocrab::Octocrab;
use std::fs;
use std::path::Path;

/// Trait for GitHub operations to enable testing with mocks
#[async_trait]
#[allow(dead_code)] // Trait methods for future GitHub API abstraction and testing
pub trait GitHubOps {
    async fn fetch_issues(&self) -> Result<Vec<octocrab::models::issues::Issue>, GitHubError>;
    async fn fetch_issues_with_state(
        &self,
        state: Option<octocrab::params::State>,
    ) -> Result<Vec<octocrab::models::issues::Issue>, GitHubError>;
    async fn assign_issue(&self, issue_number: u64, assignee: &str) -> Result<(), GitHubError>;
    async fn add_label_to_issue(&self, issue_number: u64, label: &str) -> Result<(), GitHubError>;
    async fn create_issue(
        &self,
        title: &str,
        body: &str,
        labels: Vec<String>,
    ) -> Result<octocrab::models::issues::Issue, GitHubError>;
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
    #[allow(dead_code)]
    pub comments: CommentHandler,
    pub actions: ActionsHandler,
    owner: String,
    repo: String,
    #[allow(dead_code)]
    retry_handler: GitHubRetryHandler,
}

#[allow(dead_code)] // Many methods are architectural for future GitHub API features
impl GitHubClient {
    pub fn new() -> Result<Self, GitHubError> {
        let token = Self::read_token()?;
        let (owner, repo) = Self::read_config()?;

        let octocrab = Octocrab::builder().personal_token(token).build()?;

        let client = Self::create_client(octocrab, owner, repo);
        
        // Validate API connectivity before returning
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(client.validate_api_connectivity())
        })?;

        Ok(client)
    }

    /// Pre-flight validation to ensure API connectivity and authentication
    async fn validate_api_connectivity(&self) -> Result<(), GitHubError> {
        let octocrab = self.issues.octocrab();
        
        // Detect CI/CD environment for enhanced reporting
        let is_ci = std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok();
        let environment_context = if is_ci { "CI/CD" } else { "local development" };
        
        // Test basic authentication first
        match octocrab.current().user().await {
            Ok(user) => {
                eprintln!("✅ GitHub authentication successful");
                eprintln!("   🧑‍💼 Authenticated as: {}", user.login);
                eprintln!("   🌍 Environment: {}", environment_context);
                
                // Validate token scopes by attempting repository access
                self.validate_token_scopes().await?;
                Ok(())
            },
            Err(octocrab_err) => {
                // Enhanced error reporting with environment context
                match &octocrab_err {
                    octocrab::Error::GitHub { source, .. } if source.status_code.as_u16() == 401 => {
                        let mut error_msg = format!(
                            "GitHub API authentication failed (HTTP 401) in {} environment.\nToken may be invalid or expired.",
                            environment_context
                        );
                        
                        if is_ci {
                            error_msg.push_str("\n\nCI/CD Environment Troubleshooting:");
                            error_msg.push_str("\n  → Check GITHUB_TOKEN secret is set correctly");
                            error_msg.push_str("\n  → Verify token hasn't expired");
                            error_msg.push_str("\n  → Ensure workflow has appropriate permissions");
                        } else {
                            error_msg.push_str("\n\nLocal Development Troubleshooting:");
                            error_msg.push_str("\n  → Run 'gh auth login' to refresh authentication");
                            error_msg.push_str("\n  → Or set valid MY_LITTLE_SODA_GITHUB_TOKEN environment variable");
                            error_msg.push_str("\n  → Check token at: https://github.com/settings/tokens");
                        }
                        
                        Err(GitHubError::TokenNotFound(error_msg))
                    },
                    octocrab::Error::GitHub { source, .. } if source.status_code.as_u16() == 403 => {
                        Err(GitHubError::ApiError(octocrab_err))
                    },
                    octocrab::Error::Http { .. } => {
                        let mut error_msg = format!(
                            "Unable to connect to GitHub API from {} environment.",
                            environment_context
                        );
                        
                        if is_ci {
                            error_msg.push_str("\n\nCI/CD Network Troubleshooting:");
                            error_msg.push_str("\n  → GitHub Actions should have internet access by default");
                            error_msg.push_str("\n  → Check for custom network configurations");
                            error_msg.push_str("\n  → Verify GitHub status: https://status.github.com");
                        } else {
                            error_msg.push_str("\n\nLocal Network Troubleshooting:");
                            error_msg.push_str("\n  → Check internet connectivity");
                            error_msg.push_str("\n  → Test: curl -I https://api.github.com");
                            error_msg.push_str("\n  → Check firewall/proxy settings");
                        }
                        
                        Err(GitHubError::NetworkError(error_msg))
                    },
                    _ => Err(GitHubError::ApiError(octocrab_err))
                }
            }
        }
    }

    /// Validate token has required scopes for repository operations
    async fn validate_token_scopes(&self) -> Result<(), GitHubError> {
        let octocrab = self.issues.octocrab();
        
        // Test repository access to validate scopes
        match octocrab.repos(&self.owner, &self.repo).get().await {
            Ok(repo) => {
                eprintln!("   📦 Repository access confirmed: {}/{}", self.owner, self.repo);
                if repo.private.unwrap_or(false) {
                    eprintln!("   🔒 Token has access to private repository");
                } else {
                    eprintln!("   🌍 Token has access to public repository");
                }
                
                // Test issue access permission
                match octocrab.issues(&self.owner, &self.repo).list().per_page(1).send().await {
                    Ok(_) => {
                        eprintln!("   ✍️  Token has issue read access");
                        Ok(())
                    },
                    Err(_) => {
                        eprintln!("   ⚠️  Token may lack issue write permissions");
                        Ok(()) // Don't fail - just warn
                    }
                }
            },
            Err(octocrab_err) => {
                match &octocrab_err {
                    octocrab::Error::GitHub { source, .. } if source.status_code.as_u16() == 403 => {
                        Err(GitHubError::ApiError(octocrab_err))
                    },
                    octocrab::Error::GitHub { source, .. } if source.status_code.as_u16() == 404 => {
                        Err(GitHubError::ConfigNotFound(
                            format!("Repository '{}/{}' not found or not accessible. Check GITHUB_OWNER and GITHUB_REPO configuration.", 
                                   &self.owner, &self.repo)
                        ))
                    },
                    _ => Err(GitHubError::ApiError(octocrab_err))
                }
            }
        }
    }

    fn read_token() -> Result<String, GitHubError> {
        // First try environment variable (set by flox)
        if let Ok(token) = std::env::var("MY_LITTLE_SODA_GITHUB_TOKEN") {
            if token != "YOUR_GITHUB_TOKEN_HERE" && !token.is_empty() {
                return Ok(token);
            }
        }

        // Try file-based configuration
        let token_path = ".my-little-soda/credentials/github_token";
        if Path::new(token_path).exists() {
            let token = fs::read_to_string(token_path)?.trim().to_string();
            if token != "YOUR_GITHUB_TOKEN_HERE" && !token.is_empty() {
                return Ok(token);
            }
        }

        // Fall back to GitHub CLI authentication
        if let Ok(gh_token) = Self::try_github_cli_token() {
            return Ok(gh_token);
        }

        // All authentication methods failed
        Err(GitHubError::TokenNotFound(
            "No valid GitHub authentication found. Please set up authentication using one of these methods:\n  1. Set MY_LITTLE_SODA_GITHUB_TOKEN environment variable\n  2. Run 'gh auth login' (GitHub CLI)\n  3. Create .my-little-soda/credentials/github_token file with your token".to_string()
        ))
    }

    fn try_github_cli_token() -> Result<String, GitHubError> {
        use std::process::Command;
        
        // First check if gh CLI is available and authenticated
        let auth_status = Command::new("gh")
            .args(["auth", "status"])
            .output()
            .map_err(|e| GitHubError::TokenNotFound(
                format!("GitHub CLI (gh) not available: {e}. Install from https://cli.github.com/")
            ))?;

        if !auth_status.status.success() {
            return Err(GitHubError::TokenNotFound(
                "GitHub CLI not authenticated. Run 'gh auth login' first.".to_string()
            ));
        }

        // Get the token from gh CLI
        let token_output = Command::new("gh")
            .args(["auth", "token"])
            .output()
            .map_err(|e| GitHubError::TokenNotFound(
                format!("Failed to get token from GitHub CLI: {e}")
            ))?;

        if !token_output.status.success() {
            return Err(GitHubError::TokenNotFound(
                "Failed to retrieve token from GitHub CLI".to_string()
            ));
        }

        let token = String::from_utf8(token_output.stdout)
            .map_err(|e| GitHubError::TokenNotFound(
                format!("Invalid UTF-8 in GitHub CLI token: {e}")
            ))?
            .trim()
            .to_string();

        if token.is_empty() {
            return Err(GitHubError::TokenNotFound(
                "GitHub CLI returned empty token".to_string()
            ));
        }

        Ok(token)
    }

    fn read_config() -> Result<(String, String), GitHubError> {
        // First try environment variables (set by flox)
        let env_owner = std::env::var("GITHUB_OWNER").unwrap_or_default();
        let env_repo = std::env::var("GITHUB_REPO").unwrap_or_default();

        if !env_owner.is_empty()
            && !env_repo.is_empty()
            && env_owner != "your-github-username"
            && env_repo != "your-repo-name"
        {
            return Ok((env_owner, env_repo));
        }

        // Fall back to file-based configuration
        let owner_path = ".my-little-soda/credentials/github_owner";
        let repo_path = ".my-little-soda/credentials/github_repo";

        if !Path::new(owner_path).exists() {
            return Err(GitHubError::ConfigNotFound(format!(
                "GitHub config not found. Please set GITHUB_OWNER and GITHUB_REPO environment variables or create {owner_path} with your GitHub username/organization."
            )));
        }

        if !Path::new(repo_path).exists() {
            return Err(GitHubError::ConfigNotFound(format!(
                "GitHub repo not found at {repo_path}. Please create this file with your repository name."
            )));
        }

        let owner = fs::read_to_string(owner_path)?.trim().to_string();
        let repo = fs::read_to_string(repo_path)?.trim().to_string();

        if owner.is_empty()
            || repo.is_empty()
            || owner == "your-github-username"
            || repo == "your-repo-name"
        {
            return Err(GitHubError::ConfigNotFound(
                "GitHub owner and repo must be set to actual values, not placeholders".to_string(),
            ));
        }

        Ok((owner, repo))
    }

    /// Common error handling utility for GitHub API calls
    pub async fn handle_api_result<T>(
        &self,
        result: Result<T, octocrab::Error>,
    ) -> Result<T, GitHubError> {
        result.map_err(GitHubError::ApiError)
    }

    /// Standard retry wrapper for GitHub operations
    pub async fn with_retry<F, T>(
        &self,
        operation_name: &str,
        operation: F,
    ) -> Result<T, GitHubError>
    where
        F: std::future::Future<Output = Result<T, octocrab::Error>>,
    {
        // For now, just execute once - retry logic can be added later
        tracing::info!("Executing GitHub operation: {}", operation_name);
        operation.await.map_err(GitHubError::ApiError)
    }

    /// Common pattern for issue operations with consistent error handling
    pub async fn execute_issue_operation<F, T>(
        &self,
        operation_name: &str,
        issue_number: u64,
        operation: F,
    ) -> Result<T, GitHubError>
    where
        F: std::future::Future<Output = Result<T, octocrab::Error>>,
    {
        tracing::debug!(
            "GitHub issue operation: {} on issue #{}",
            operation_name,
            issue_number
        );
        operation.await.map_err(GitHubError::ApiError)
    }

    /// Common pattern for PR operations with consistent error handling
    pub async fn execute_pr_operation<F, T>(
        &self,
        operation_name: &str,
        pr_number: u64,
        operation: F,
    ) -> Result<T, GitHubError>
    where
        F: std::future::Future<Output = Result<T, octocrab::Error>>,
    {
        tracing::debug!(
            "GitHub PR operation: {} on PR #{}",
            operation_name,
            pr_number
        );
        operation.await.map_err(GitHubError::ApiError)
    }

    /// Factory method to reduce constructor duplication
    fn create_client(octocrab: Octocrab, owner: String, repo: String) -> Self {
        GitHubClient {
            issues: IssueHandler::new(octocrab.clone(), owner.clone(), repo.clone()),
            pulls: PullRequestHandler::new(octocrab.clone(), owner.clone(), repo.clone()),
            branches: BranchHandler::new(octocrab.clone(), owner.clone(), repo.clone()),
            #[allow(dead_code)]
            comments: CommentHandler::new(octocrab.clone(), owner.clone(), repo.clone()),
            actions: ActionsHandler::new(octocrab.clone(), owner.clone(), repo.clone()),
            owner,
            repo,
            #[allow(dead_code)]
            retry_handler: GitHubRetryHandler::default(),
        }
    }

    pub async fn fetch_issues(&self) -> Result<Vec<octocrab::models::issues::Issue>, GitHubError> {
        self.issues.fetch_issues().await
    }

    pub async fn fetch_issues_with_state(
        &self,
        state: Option<octocrab::params::State>,
    ) -> Result<Vec<octocrab::models::issues::Issue>, GitHubError> {
        self.issues.fetch_issues_with_state(state).await
    }

    pub async fn fetch_issue(
        &self,
        issue_number: u64,
    ) -> Result<octocrab::models::issues::Issue, GitHubError> {
        self.issues.fetch_issue(issue_number).await
    }

    pub async fn assign_issue(
        &self,
        issue_number: u64,
        assignee: &str,
    ) -> Result<octocrab::models::issues::Issue, GitHubError> {
        self.issues.assign_issue(issue_number, assignee).await
    }

    pub async fn create_branch(
        &self,
        branch_name: &str,
        from_branch: &str,
    ) -> Result<(), GitHubError> {
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
        self.pulls
            .create_pull_request(title, head_branch, base_branch, body)
            .await
    }

    pub async fn get_pull_request(
        &self,
        pr_number: u64,
    ) -> Result<octocrab::models::pulls::PullRequest, GitHubError> {
        self.pulls.get_pull_request(pr_number).await
    }

    /// Check if a PR is ready for merging
    pub async fn is_pr_mergeable(
        &self,
        pr: &octocrab::models::pulls::PullRequest,
    ) -> Result<bool, GitHubError> {
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

    pub async fn add_label_to_issue(
        &self,
        issue_number: u64,
        label: &str,
    ) -> Result<(), GitHubError> {
        self.issues.add_label_to_issue(issue_number, label).await
    }

    pub async fn remove_label_from_issue(
        &self,
        issue_number: u64,
        label: &str,
    ) -> Result<(), GitHubError> {
        self.issues.remove_label(issue_number, label).await
    }

    pub async fn create_issue(
        &self,
        title: &str,
        body: &str,
        labels: Vec<String>,
    ) -> Result<octocrab::models::issues::Issue, GitHubError> {
        self.issues.create_issue(title, body, labels).await
    }

    pub async fn fetch_open_pull_requests(
        &self,
    ) -> Result<Vec<octocrab::models::pulls::PullRequest>, GitHubError> {
        self.pulls.fetch_open_pull_requests().await
    }

    /// Check if an issue has an open PR that references it
    /// Returns true if the issue has an open PR WITHOUT route:ready_to_merge label
    pub async fn issue_has_blocking_pr(&self, issue_number: u64) -> Result<bool, GitHubError> {
        let open_prs = self.fetch_open_pull_requests().await?;
        self.issues
            .issue_has_blocking_pr(issue_number, &open_prs)
            .await
    }

    /// Get the number of PRs created in the last hour
    pub async fn get_pr_creation_rate(&self) -> Result<u32, GitHubError> {
        self.pulls.get_pr_creation_rate().await
    }

    /// Enhanced merge conflict detection with detailed diagnostics
    pub async fn detect_merge_conflicts(
        &self,
        pr_number: u64,
    ) -> Result<ConflictAnalysis, GitHubError> {
        self.pulls.detect_merge_conflicts(pr_number).await
    }

    /// Create a recovery PR for conflicted work with human review request
    pub async fn create_conflict_recovery_pr(
        &self,
        original_pr: u64,
        work_data: ConflictRecoveryData,
    ) -> Result<octocrab::models::pulls::PullRequest, GitHubError> {
        // Use pulls handler but pass issue handler for label operations
        let pr = self
            .pulls
            .create_conflict_recovery_pr(original_pr, work_data, &self.issues)
            .await?;
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
        self.pulls
            .safe_merge_pull_request(
                pr_number,
                agent_id,
                issue_number,
                merge_method,
                &self.issues,
            )
            .await
    }
}

// Implement the trait for GitHubClient
#[async_trait]
impl GitHubOps for GitHubClient {
    async fn fetch_issues(&self) -> Result<Vec<octocrab::models::issues::Issue>, GitHubError> {
        self.fetch_issues().await
    }

    async fn fetch_issues_with_state(
        &self,
        state: Option<octocrab::params::State>,
    ) -> Result<Vec<octocrab::models::issues::Issue>, GitHubError> {
        self.fetch_issues_with_state(state).await
    }

    async fn assign_issue(&self, issue_number: u64, assignee: &str) -> Result<(), GitHubError> {
        self.assign_issue(issue_number, assignee).await?;
        Ok(())
    }

    async fn add_label_to_issue(&self, issue_number: u64, label: &str) -> Result<(), GitHubError> {
        self.add_label_to_issue(issue_number, label).await
    }

    async fn create_issue(
        &self,
        title: &str,
        body: &str,
        labels: Vec<String>,
    ) -> Result<octocrab::models::issues::Issue, GitHubError> {
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
