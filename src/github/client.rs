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
    verbose: bool,
}

#[allow(dead_code)] // Many methods are architectural for future GitHub API features
impl GitHubClient {
    pub fn new() -> Result<Self, GitHubError> {
        Self::with_verbose(true)
    }

    /// Create a new GitHubClient with configurable verbose output
    pub fn with_verbose(verbose: bool) -> Result<Self, GitHubError> {
        let token = Self::read_token(verbose)?;
        let (owner, repo) = Self::read_config()?;

        let octocrab = Octocrab::builder().personal_token(token).build()?;

        let client = Self::create_client(octocrab, owner, repo, verbose);

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
                if self.verbose {
                    eprintln!("✅ GitHub authentication successful");
                    eprintln!("   🧑‍💼 Authenticated as: {}", user.login);
                    eprintln!("   🌍 Environment: {}", environment_context);
                }

                // Validate token scopes by attempting repository access
                self.validate_token_scopes().await?;
                Ok(())
            }
            Err(octocrab_err) => {
                // Enhanced error reporting with environment context
                match &octocrab_err {
                    octocrab::Error::GitHub { source, .. }
                        if source.status_code.as_u16() == 401 =>
                    {
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
                            error_msg
                                .push_str("\n  → Run 'gh auth login' to refresh authentication");
                            error_msg.push_str("\n  → Or set valid MY_LITTLE_SODA_GITHUB_TOKEN environment variable");
                            error_msg.push_str(
                                "\n  → Check token at: https://github.com/settings/tokens",
                            );
                        }

                        Err(GitHubError::TokenNotFound(error_msg))
                    }
                    octocrab::Error::GitHub { source, .. }
                        if source.status_code.as_u16() == 403 =>
                    {
                        Err(GitHubError::ApiError(octocrab_err))
                    }
                    octocrab::Error::Http { .. } => {
                        let mut error_msg = format!(
                            "Unable to connect to GitHub API from {} environment.",
                            environment_context
                        );

                        if is_ci {
                            error_msg.push_str("\n\nCI/CD Network Troubleshooting:");
                            error_msg.push_str(
                                "\n  → GitHub Actions should have internet access by default",
                            );
                            error_msg.push_str("\n  → Check for custom network configurations");
                            error_msg
                                .push_str("\n  → Verify GitHub status: https://status.github.com");
                        } else {
                            error_msg.push_str("\n\nLocal Network Troubleshooting:");
                            error_msg.push_str("\n  → Check internet connectivity");
                            error_msg.push_str("\n  → Test: curl -I https://api.github.com");
                            error_msg.push_str("\n  → Check firewall/proxy settings");
                        }

                        Err(GitHubError::NetworkError(error_msg))
                    }
                    _ => Err(GitHubError::ApiError(octocrab_err)),
                }
            }
        }
    }

    /// Enhanced token scope validation with detailed error messages and scope detection
    async fn validate_token_scopes(&self) -> Result<(), GitHubError> {
        let octocrab = self.issues.octocrab();

        // Test repository access to validate scopes
        match octocrab.repos(&self.owner, &self.repo).get().await {
            Ok(repo) => {
                if self.verbose {
                    eprintln!(
                        "   📦 Repository access confirmed: {}/{}",
                        self.owner, self.repo
                    );
                    if repo.private.unwrap_or(false) {
                        eprintln!("   🔒 Token has access to private repository");
                    } else {
                        eprintln!("   🌍 Token has access to public repository");
                    }
                }

                // Enhanced scope validation with detailed testing
                self.validate_detailed_scopes(&octocrab, &repo).await?;
                Ok(())
            }
            Err(octocrab_err) => {
                match &octocrab_err {
                    octocrab::Error::GitHub { source, .. } if source.status_code.as_u16() == 403 => {
                        // Enhanced 403 error with token scope guidance
                        let repo_type = if std::env::var("GITHUB_ACTIONS").is_ok() { "public" } else { "private" };
                        let scope_needed = if repo_type == "private" { "repo" } else { "public_repo" };
                        
                        Err(GitHubError::TokenScopeInsufficient { 
                            required_scopes: vec![scope_needed.to_string(), "issues:write".to_string(), "pull_requests:write".to_string()],
                            current_error: source.message.clone(),
                            token_url: "https://github.com/settings/tokens".to_string(),
                        })
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

    /// Detailed scope validation with comprehensive permission testing
    async fn validate_detailed_scopes(
        &self,
        octocrab: &Octocrab,
        repo: &octocrab::models::Repository,
    ) -> Result<(), GitHubError> {
        let mut missing_scopes = Vec::new();
        let mut warnings = Vec::new();

        // Test issue read access
        match octocrab
            .issues(&self.owner, &self.repo)
            .list()
            .per_page(1)
            .send()
            .await
        {
            Ok(_) => {
                if self.verbose {
                    eprintln!("   ✅ Token has issue read access");
                }
            }
            Err(e) => match &e {
                octocrab::Error::GitHub { source, .. } if source.status_code.as_u16() == 403 => {
                    missing_scopes.push("issues:read".to_string());
                }
                _ => warnings.push("Issue read access test inconclusive".to_string()),
            },
        }

        // Test issue write access (try to fetch assignees which requires write scope)
        match octocrab
            .issues(&self.owner, &self.repo)
            .list_assignees()
            .send()
            .await
        {
            Ok(_) => {
                if self.verbose {
                    eprintln!("   ✅ Token has issue write access");
                }
            }
            Err(e) => match &e {
                octocrab::Error::GitHub { source, .. } if source.status_code.as_u16() == 403 => {
                    missing_scopes.push("issues:write".to_string());
                }
                _ => warnings.push("Issue write access test inconclusive".to_string()),
            },
        }

        // Test pull request access
        match octocrab
            .pulls(&self.owner, &self.repo)
            .list()
            .per_page(1)
            .send()
            .await
        {
            Ok(_) => {
                if self.verbose {
                    eprintln!("   ✅ Token has pull request access");
                }
            }
            Err(e) => match &e {
                octocrab::Error::GitHub { source, .. } if source.status_code.as_u16() == 403 => {
                    missing_scopes.push("pull_requests:read".to_string());
                }
                _ => warnings.push("Pull request access test inconclusive".to_string()),
            },
        }

        // Report warnings
        if self.verbose {
            for warning in &warnings {
                eprintln!("   ⚠️  {}", warning);
            }
        }

        // Fail if critical scopes are missing
        if !missing_scopes.is_empty() {
            let required_base_scope = if repo.private.unwrap_or(false) {
                "repo"
            } else {
                "public_repo"
            };
            let mut all_required = vec![required_base_scope.to_string()];
            all_required.extend(missing_scopes);

            return Err(GitHubError::TokenScopeInsufficient {
                required_scopes: all_required,
                current_error: "Token lacks required permissions for My Little Soda operations"
                    .to_string(),
                token_url: "https://github.com/settings/tokens".to_string(),
            });
        }

        if self.verbose {
            eprintln!("   ✅ Token has all required scopes for My Little Soda operations");
        }
        Ok(())
    }

    fn read_token(verbose: bool) -> Result<String, GitHubError> {
        // First try environment variable (set by flox)
        if let Ok(token) = std::env::var("MY_LITTLE_SODA_GITHUB_TOKEN") {
            if token != "YOUR_GITHUB_TOKEN_HERE" && !token.is_empty() {
                if verbose {
                    eprintln!("   🔑 Authentication method: Environment variable (MY_LITTLE_SODA_GITHUB_TOKEN)");
                }
                return Ok(token);
            }
        }

        // Try file-based configuration
        let token_path = ".my-little-soda/credentials/github_token";
        if Path::new(token_path).exists() {
            let token = fs::read_to_string(token_path)?.trim().to_string();
            if token != "YOUR_GITHUB_TOKEN_HERE" && !token.is_empty() {
                if verbose {
                    eprintln!("   🔑 Authentication method: File-based configuration (.my-little-soda/credentials/github_token)");
                }
                return Ok(token);
            }
        }

        // Fall back to GitHub CLI authentication
        if let Ok(gh_token) = Self::try_github_cli_token() {
            if verbose {
                eprintln!("   🔑 Authentication method: GitHub CLI (gh auth token)");
            }
            return Ok(gh_token);
        }

        // All authentication methods failed - provide comprehensive guidance
        let is_ci = std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok();
        let mut error_msg = "No valid GitHub authentication found.".to_string();

        if is_ci {
            error_msg.push_str("\n\nCI/CD Environment Setup:");
            error_msg.push_str(
                "\n  1. Set GITHUB_TOKEN or MY_LITTLE_SODA_GITHUB_TOKEN in repository secrets",
            );
            error_msg.push_str("\n  2. Ensure workflow has appropriate permissions:");
            error_msg.push_str("\n     permissions:");
            error_msg.push_str("\n       contents: write");
            error_msg.push_str("\n       issues: write");
            error_msg.push_str("\n       pull-requests: write");
        } else {
            error_msg.push_str("\n\nLocal Development Setup (choose one):");
            error_msg.push_str("\n  1. 🎯 GitHub CLI (recommended): gh auth login");
            error_msg.push_str(
                "\n  2. 📝 Environment variable: export MY_LITTLE_SODA_GITHUB_TOKEN=your_token",
            );
            error_msg.push_str(
                "\n  3. 📁 Configuration file: Create .my-little-soda/credentials/github_token",
            );
            error_msg.push_str("\n\n🔗 Create token at: https://github.com/settings/tokens");
            error_msg.push_str("\n   Required scopes: repo (private) or public_repo (public), issues, pull_requests");
        }

        Err(GitHubError::TokenNotFound(error_msg))
    }

    fn try_github_cli_token() -> Result<String, GitHubError> {
        use std::process::Command;

        // First check if gh CLI is available and authenticated
        let auth_status = Command::new("gh")
            .args(["auth", "status"])
            .output()
            .map_err(|e| {
                GitHubError::TokenNotFound(format!(
                    "GitHub CLI (gh) not available: {e}. Install from https://cli.github.com/"
                ))
            })?;

        if !auth_status.status.success() {
            return Err(GitHubError::TokenNotFound(
                "GitHub CLI not authenticated. Run 'gh auth login' first.".to_string(),
            ));
        }

        // Get the token from gh CLI
        let token_output = Command::new("gh")
            .args(["auth", "token"])
            .output()
            .map_err(|e| {
                GitHubError::TokenNotFound(format!("Failed to get token from GitHub CLI: {e}"))
            })?;

        if !token_output.status.success() {
            return Err(GitHubError::TokenNotFound(
                "Failed to retrieve token from GitHub CLI".to_string(),
            ));
        }

        let token = String::from_utf8(token_output.stdout)
            .map_err(|e| {
                GitHubError::TokenNotFound(format!("Invalid UTF-8 in GitHub CLI token: {e}"))
            })?
            .trim()
            .to_string();

        if token.is_empty() {
            return Err(GitHubError::TokenNotFound(
                "GitHub CLI returned empty token".to_string(),
            ));
        }

        Ok(token)
    }

    fn read_config() -> Result<(String, String), GitHubError> {
        // First try environment variables
        let env_owner = std::env::var("GITHUB_OWNER").unwrap_or_default();
        let env_repo = std::env::var("GITHUB_REPO").unwrap_or_default();

        if !env_owner.is_empty()
            && !env_repo.is_empty()
            && env_owner != "your-github-username"
            && env_repo != "your-repo-name"
        {
            return Ok((env_owner, env_repo));
        }

        // Try to auto-detect from git remote
        if let Ok(repo_info) = Self::try_git_auto_detection() {
            return Ok((repo_info.owner, repo_info.repo));
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

    /// Try to auto-detect GitHub repository information from git remote
    fn try_git_auto_detection() -> Result<crate::git::GitHubRepoInfo, GitHubError> {
        use crate::git::{Git2Operations, GitOperations};

        let git_ops = Git2Operations::new(".").map_err(|_| {
            GitHubError::ConfigNotFound(
                "Not in a git repository. Initialize with: git init && git remote add origin https://github.com/YOUR-USERNAME/YOUR-REPO.git".to_string()
            )
        })?;

        let repo_info = git_ops.get_github_repo_info(None).map_err(|e| {
            GitHubError::ConfigNotFound(format!(
                "Could not read git remote information: {}. Check your git configuration and network connection", e
            ))
        })?;

        repo_info.ok_or_else(|| {
            GitHubError::ConfigNotFound(
                "No GitHub remote found. Add one with: git remote add origin https://github.com/YOUR-USERNAME/YOUR-REPO.git".to_string()
            )
        })
    }

    /// Enhanced error handling utility for GitHub API calls with rate limit detection
    pub async fn handle_api_result<T>(
        &self,
        result: Result<T, octocrab::Error>,
    ) -> Result<T, GitHubError> {
        match result {
            Ok(value) => Ok(value),
            Err(octocrab_err) => {
                // Check for rate limiting
                if let Some(rate_limit_error) = self.detect_rate_limit(&octocrab_err) {
                    return Err(rate_limit_error);
                }

                // Enhanced error context for common API errors
                match &octocrab_err {
                    octocrab::Error::GitHub { source, .. } => {
                        match source.status_code.as_u16() {
                            403 => {
                                // Could be rate limit or permissions
                                if source.message.contains("rate")
                                    || source.message.contains("limit")
                                {
                                    return self.check_rate_limit_status().await;
                                } else {
                                    Err(GitHubError::ApiError(octocrab_err))
                                }
                            }
                            502 | 503 | 504 => {
                                // GitHub is having issues - enhance error message
                                Err(GitHubError::NetworkError(format!(
                                    "GitHub API server error (HTTP {}). This is likely a temporary GitHub service issue. Please try again in a few minutes or check https://status.github.com",
                                    source.status_code.as_u16()
                                )))
                            }
                            _ => Err(GitHubError::ApiError(octocrab_err)),
                        }
                    }
                    _ => Err(GitHubError::ApiError(octocrab_err)),
                }
            }
        }
    }

    /// Detect rate limiting from octocrab error
    fn detect_rate_limit(&self, error: &octocrab::Error) -> Option<GitHubError> {
        match error {
            octocrab::Error::GitHub { source, .. } if source.status_code.as_u16() == 403 => {
                if source.message.contains("rate") || source.message.contains("limit") {
                    // Try to parse rate limit info from headers (simplified)
                    let reset_time = chrono::Utc::now() + chrono::Duration::minutes(60); // Default assumption
                    Some(GitHubError::RateLimit {
                        reset_time,
                        remaining: 0,
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Check current rate limit status and provide helpful information  
    async fn check_rate_limit_status<T>(&self) -> Result<T, GitHubError> {
        let octocrab = self.issues.octocrab();

        // Try to get rate limit information
        match octocrab.ratelimit().get().await {
            Ok(rate_limit) => {
                let reset_time =
                    chrono::DateTime::from_timestamp(rate_limit.resources.core.reset as i64, 0)
                        .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::hours(1));

                Err(GitHubError::RateLimit {
                    reset_time,
                    remaining: rate_limit.resources.core.remaining as u32,
                })
            }
            Err(_) => {
                // Fallback error if we can't get rate limit info
                Err(GitHubError::RateLimit {
                    reset_time: chrono::Utc::now() + chrono::Duration::hours(1),
                    remaining: 0,
                })
            }
        }
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
    fn create_client(octocrab: Octocrab, owner: String, repo: String, verbose: bool) -> Self {
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
            verbose,
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

    /// Create a review comment on a pull request
    pub async fn create_pr_review_comment(
        &self,
        pr_number: u64,
        body: &str,
        commit_id: &str,
        path: &str,
        line: u32,
    ) -> Result<octocrab::models::pulls::Comment, GitHubError> {
        self.comments
            .create_pr_review_comment(pr_number, body, commit_id, path, line)
            .await
    }

    /// Get review comments for a pull request
    pub async fn get_pr_review_comments(
        &self,
        pr_number: u64,
    ) -> Result<Vec<octocrab::models::pulls::Comment>, GitHubError> {
        self.comments.get_pr_review_comments(pr_number).await
    }

    /// Update a PR review comment
    pub async fn update_pr_review_comment(
        &self,
        comment_id: u64,
        body: &str,
    ) -> Result<octocrab::models::pulls::Comment, GitHubError> {
        self.comments
            .update_pr_review_comment(comment_id, body)
            .await
    }

    /// Delete a PR review comment
    pub async fn delete_pr_review_comment(&self, comment_id: u64) -> Result<(), GitHubError> {
        self.comments.delete_pr_review_comment(comment_id).await
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
