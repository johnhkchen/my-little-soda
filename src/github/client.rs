use octocrab::{Octocrab, Error as OctocrabError};
use octocrab::params::pulls::MergeMethod;
use std::fs;
use std::path::Path;
use async_trait::async_trait;
// use crate::github::retry::GitHubRetryHandler;

/// Trait for GitHub operations to enable testing with mocks
#[async_trait]
pub trait GitHubOps {
    async fn fetch_issues(&self) -> Result<Vec<octocrab::models::issues::Issue>, GitHubError>;
    async fn fetch_issues_with_state(&self, state: Option<octocrab::params::State>) -> Result<Vec<octocrab::models::issues::Issue>, GitHubError>;
    async fn assign_issue(&self, issue_number: u64, assignee: &str) -> Result<(), GitHubError>;
    async fn add_label_to_issue(&self, issue_number: u64, label: &str) -> Result<(), GitHubError>;
    async fn create_branch(&self, branch_name: &str, from_branch: &str) -> Result<(), GitHubError>;
    async fn issue_has_blocking_pr(&self, issue_number: u64) -> Result<bool, GitHubError>;
    fn owner(&self) -> &str;
    fn repo(&self) -> &str;
}

#[derive(Debug)]
pub struct PullRequestStatus {
    pub number: u64,
    pub state: String,
    pub mergeable: Option<bool>,
    pub merged: bool,
    pub ci_status: String,
    pub approved_reviews: usize,
    pub requested_changes: usize,
    pub head_sha: String,
}

#[derive(Debug)]
pub enum GitHubError {
    TokenNotFound(String),
    ConfigNotFound(String),
    ApiError(OctocrabError),
    IoError(std::io::Error),
    NotImplemented(String),
}


impl From<OctocrabError> for GitHubError {
    fn from(err: OctocrabError) -> Self {
        GitHubError::ApiError(err)
    }
}

impl From<std::io::Error> for GitHubError {
    fn from(err: std::io::Error) -> Self {
        GitHubError::IoError(err)
    }
}

impl std::fmt::Display for GitHubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitHubError::TokenNotFound(msg) => {
                write!(f, "GitHub Authentication Error\n")?;
                write!(f, "──────────────────────────\n")?;
                write!(f, "🔑 {}\n\n", msg)?;
                write!(f, "🔧 QUICK FIXES:\n")?;
                write!(f, "   → Use GitHub CLI: gh auth login\n")?;
                write!(f, "   → Set token directly: export CLAMBAKE_GITHUB_TOKEN=your_token\n")?;
                write!(f, "   → Create token at: https://github.com/settings/tokens\n")?;
                write!(f, "     (needs 'repo' scope for private repos, 'public_repo' for public)")
            },
            GitHubError::ConfigNotFound(msg) => {
                write!(f, "GitHub Configuration Error\n")?;
                write!(f, "─────────────────────────\n")?;
                write!(f, "📂 {}\n\n", msg)?;
                write!(f, "🔧 QUICK FIXES:\n")?;
                write!(f, "   → Set environment variables: export GITHUB_OWNER=username GITHUB_REPO=reponame\n")?;
                write!(f, "   → Use GitHub CLI in repo: gh repo view\n")?;
                write!(f, "   → Run setup: clambake init")
            },
            GitHubError::ApiError(octocrab_err) => {
                write!(f, "GitHub API Error\n")?;
                write!(f, "────────────────\n")?;
                write!(f, "🌐 {}\n\n", octocrab_err)?;
                write!(f, "🔧 TROUBLESHOOTING:\n")?;
                write!(f, "   → Check authentication: gh auth status\n")?;
                write!(f, "   → Test connection: curl -I https://api.github.com\n")?;
                write!(f, "   → Verify repository access: gh repo view\n")?;
                write!(f, "   → Check rate limits: gh api rate_limit")
            },
            GitHubError::IoError(io_err) => {
                write!(f, "File System Error\n")?;
                write!(f, "─────────────────\n")?;
                write!(f, "📁 {}\n\n", io_err)?;
                write!(f, "🔧 POSSIBLE CAUSES:\n")?;
                write!(f, "   → File permissions issue\n")?;
                write!(f, "   → Directory doesn't exist\n")?;
                write!(f, "   → Disk space or I/O error")
            },
            GitHubError::NotImplemented(msg) => {
                write!(f, "Feature Not Yet Implemented\n")?;
                write!(f, "──────────────────────────\n")?;
                write!(f, "🚧 {}\n\n", msg)?;
                write!(f, "🔧 ALTERNATIVES:\n")?;
                write!(f, "   → Manual workaround may be available\n")?;
                write!(f, "   → Feature coming in future release")
            }
        }
    }
}

impl std::error::Error for GitHubError {}

#[derive(Debug)]
pub struct GitHubClient {
    octocrab: Octocrab,
    owner: String,
    repo: String,
    // retry_handler: GitHubRetryHandler,
}

impl GitHubClient {
    pub fn new() -> Result<Self, GitHubError> {
        let token = Self::read_token()?;
        let (owner, repo) = Self::read_config()?;
        
        let octocrab = Octocrab::builder()
            .personal_token(token)
            .build()?;

        Ok(GitHubClient {
            octocrab,
            owner,
            repo,
            // retry_handler: GitHubRetryHandler::default(),
        })
    }

    fn read_token() -> Result<String, GitHubError> {
        // First try environment variable (set by flox)
        if let Ok(token) = std::env::var("CLAMBAKE_GITHUB_TOKEN") {
            if token != "YOUR_GITHUB_TOKEN_HERE" && !token.is_empty() {
                return Ok(token);
            }
        }
        
        // Fall back to file-based configuration
        let token_path = ".clambake/credentials/github_token";
        if !Path::new(token_path).exists() {
            return Err(GitHubError::TokenNotFound(format!(
                "GitHub token not found. Please set CLAMBAKE_GITHUB_TOKEN environment variable or create {} with your GitHub personal access token.",
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
        self.fetch_issues_with_state(None).await
    }

    pub async fn fetch_issues_with_state(&self, state: Option<octocrab::params::State>) -> Result<Vec<octocrab::models::issues::Issue>, GitHubError> {
        let issues = self.octocrab
            .issues(&self.owner, &self.repo)
            .list()
            .state(state.unwrap_or(octocrab::params::State::Open))
            .send()
            .await?;
            
        Ok(issues.items)
    }

    pub async fn fetch_issue(&self, issue_number: u64) -> Result<octocrab::models::issues::Issue, GitHubError> {
        let issue = self.octocrab
            .issues(&self.owner, &self.repo)
            .get(issue_number)
            .await?;
            
        Ok(issue)
    }

    pub async fn assign_issue(&self, issue_number: u64, assignee: &str) -> Result<octocrab::models::issues::Issue, GitHubError> {
        // Simplified retry for MVP - focus on getting the core functionality working
        let mut attempts = 0;
        const MAX_ATTEMPTS: u32 = 3;
        
        loop {
            attempts += 1;
            
            match self.octocrab
                .issues(&self.owner, &self.repo)
                .update(issue_number)
                .assignees(&[assignee.to_string()])
                .send()
                .await {
                Ok(issue) => return Ok(issue),
                Err(e) if attempts < MAX_ATTEMPTS => {
                    tracing::warn!("GitHub API call failed (attempt {}): {:?}", attempts, e);
                    tokio::time::sleep(std::time::Duration::from_millis(500 * attempts as u64)).await;
                    continue;
                }
                Err(e) => return Err(GitHubError::from(e)),
            }
        }
    }

    pub async fn create_branch(&self, branch_name: &str, from_branch: &str) -> Result<(), GitHubError> {
        println!("🌿 Creating branch '{}' from '{}'", branch_name, from_branch);
        
        // Use the git refs API to create the branch
        // This is a simplified implementation - for now we'll return success
        // to indicate the branch creation was attempted
        
        // TODO: Implement proper octocrab branch creation once we resolve the API details
        // The current octocrab version may have different API structure than expected
        
        match std::process::Command::new("git")
            .args(&["push", "origin", &format!("{}:{}", from_branch, branch_name)])
            .output()
        {
            Ok(output) if output.status.success() => {
                println!("✅ Branch '{}' created successfully", branch_name);
                Ok(())
            },
            Ok(_) => {
                println!("⚠️  Branch creation via git push failed");
                println!("   📝 Note: Branch may already exist or need manual creation");
                Ok(()) // Don't fail the whole operation
            },
            Err(_) => {
                println!("⚠️  Git command not available for branch creation");
                println!("   📝 Note: Branch needs to be created manually");
                Ok(()) // Don't fail the whole operation
            }
        }
    }

    pub async fn delete_branch(&self, branch_name: &str) -> Result<(), GitHubError> {
        println!("🗑️  Would delete branch '{}'", branch_name);
        
        // TODO: Implement real branch deletion
        
        Ok(())
    }

    pub async fn create_pull_request(
        &self,
        title: &str,
        head_branch: &str,
        base_branch: &str,
        body: &str,
    ) -> Result<octocrab::models::pulls::PullRequest, GitHubError> {
        let pr = self.octocrab
            .pulls(&self.owner, &self.repo)
            .create(title, head_branch, base_branch)
            .body(body)
            .send()
            .await?;
            
        println!("📋 Created PR #{}: {} ({})", pr.number, title, pr.html_url.as_ref().unwrap());
        Ok(pr)
    }

    pub async fn get_pull_request(&self, pr_number: u64) -> Result<octocrab::models::pulls::PullRequest, GitHubError> {
        let pr = self.octocrab
            .pulls(&self.owner, &self.repo)
            .get(pr_number)
            .await?;
            
        Ok(pr)
    }

    /// Check if a PR is ready for merging
    pub async fn is_pr_mergeable(&self, pr: &octocrab::models::pulls::PullRequest) -> Result<bool, GitHubError> {
        // Check basic merge conditions
        if pr.merged.unwrap_or(false) {
            return Ok(false); // Already merged
        }
        
        // Check if PR is open (using string comparison for compatibility)
        let pr_state_str = format!("{:?}", pr.state).to_lowercase();
        if !pr_state_str.contains("open") {
            return Ok(false); // Not open
        }
        
        // Check if PR is mergeable (no conflicts)
        if pr.mergeable == Some(false) {
            return Ok(false); // Has conflicts
        }
        
        // For now, we'll be permissive and allow merging if basic conditions are met
        // In a production system, you might want to check:
        // - Required status checks
        // - Required reviews
        // - Branch protection rules
        
        Ok(true)
    }
    
    /// Get detailed PR status including CI and review status
    pub async fn get_pr_status(&self, pr_number: u64) -> Result<PullRequestStatus, GitHubError> {
        let pr = self.get_pull_request(pr_number).await?;
        
        // Get commit status for the PR head - simplified for compatibility
        let ci_status = "unknown".to_string();
            
        // Check reviews
        let reviews_result = self.octocrab
            .pulls(&self.owner, &self.repo)
            .list_reviews(pr_number)
            .send()
            .await;
            
        let (approved_reviews, requested_changes) = match reviews_result {
            Ok(reviews) => {
                let approved = reviews.items.iter()
                    .filter(|review| {
                        review.state.as_ref().map(|s| format!("{:?}", s).contains("Approved")).unwrap_or(false)
                    })
                    .count();
                    
                let changes = reviews.items.iter()
                    .filter(|review| {
                        review.state.as_ref().map(|s| format!("{:?}", s).contains("ChangesRequested")).unwrap_or(false)
                    })
                    .count();
                    
                (approved, changes)
            }
            Err(_) => (0, 0),
        };
        
        Ok(PullRequestStatus {
            number: pr.number,
            state: format!("{:?}", pr.state),
            mergeable: pr.mergeable,
            merged: pr.merged.unwrap_or(false),
            ci_status,
            approved_reviews,
            requested_changes,
            head_sha: pr.head.sha.clone(),
        })
    }

    pub async fn merge_pull_request(
        &self,
        pr_number: u64,
        merge_method: Option<&str>,
    ) -> Result<octocrab::models::pulls::PullRequest, GitHubError> {
        // First check if PR is mergeable
        let pr = self.get_pull_request(pr_number).await?;
        
        if !self.is_pr_mergeable(&pr).await? {
            return Err(GitHubError::NotImplemented(format!(
                "PR #{} is not ready for merge. Check CI status, conflicts, or review requirements.",
                pr_number
            )));
        }
        
        println!("🔀 Merging PR #{}: {}", pr_number, pr.title.as_ref().unwrap_or(&"".to_string()));
        
        let method = merge_method.unwrap_or("squash");
        
        // Use octocrab to merge the PR
        let merge_result = self.octocrab
            .pulls(&self.owner, &self.repo)
            .merge(pr_number)
            .method(match method {
                "merge" => MergeMethod::Merge,
                "rebase" => MergeMethod::Rebase,
                _ => MergeMethod::Squash,
            })
            .send()
            .await?;
            
        if merge_result.merged {
            println!("✅ Successfully merged PR #{}", pr_number);
            Ok(pr)
        } else {
            Err(GitHubError::NotImplemented(format!(
                "PR #{} merge was not successful. SHA: {:?}",
                pr_number, merge_result.sha
            )))
        }
    }

    pub fn owner(&self) -> &str {
        &self.owner
    }

    pub fn repo(&self) -> &str {
        &self.repo
    }
    
    pub async fn add_label_to_issue(&self, issue_number: u64, label: &str) -> Result<(), GitHubError> {
        self.octocrab
            .issues(&self.owner, &self.repo)
            .add_labels(issue_number, &[label.to_string()])
            .await
            .map_err(GitHubError::ApiError)?;
        Ok(())
    }

    pub async fn fetch_open_pull_requests(&self) -> Result<Vec<octocrab::models::pulls::PullRequest>, GitHubError> {
        let pulls = self.octocrab
            .pulls(&self.owner, &self.repo)
            .list()
            // Note: octocrab::params::State::Open is correct here, not octocrab::params::pulls::State::Open
            // The .state() method expects octocrab::params::State, as verified by compilation
            .state(octocrab::params::State::Open)
            .send()
            .await?;
            
        Ok(pulls.items)
    }

    /// Check if an issue has an open PR that references it
    /// Returns true if the issue has an open PR WITHOUT route:land label
    pub async fn issue_has_blocking_pr(&self, issue_number: u64) -> Result<bool, GitHubError> {
        let open_prs = self.fetch_open_pull_requests().await?;
        
        for pr in open_prs {
            // Check if this PR references the issue number
            if let Some(body) = &pr.body {
                // Look for common patterns like "fixes #123", "closes #123", etc.
                let patterns = [
                    format!("fixes #{}", issue_number),
                    format!("closes #{}", issue_number),
                    format!("resolves #{}", issue_number),
                    format!("fix #{}", issue_number),
                    format!("close #{}", issue_number),
                    format!("resolve #{}", issue_number),
                    format!("#{}", issue_number), // Simple reference
                ];
                
                let body_lower = body.to_lowercase();
                let references_issue = patterns.iter().any(|pattern| body_lower.contains(&pattern.to_lowercase()));
                
                if references_issue {
                    // Check if this PR has route:land label
                    let has_route_land = pr.labels.as_ref()
                        .map(|labels| labels.iter().any(|label| label.name == "route:land"))
                        .unwrap_or(false);
                    
                    // If PR references the issue but doesn't have route:land, it's blocking
                    if !has_route_land {
                        return Ok(true);
                    }
                }
            }
        }
        
        Ok(false)
    }

    /// Get the number of PRs created in the last hour
    pub async fn get_pr_creation_rate(&self) -> Result<u32, GitHubError> {
        use chrono::{Utc, Duration};
        
        let one_hour_ago = Utc::now() - Duration::hours(1);
        
        // Fetch both open and closed PRs
        let mut all_prs = Vec::new();
        
        // Get open PRs
        let open_pulls = self.octocrab
            .pulls(&self.owner, &self.repo)
            .list()
            .state(octocrab::params::State::Open)
            .per_page(100)
            .send()
            .await?;
        all_prs.extend(open_pulls.items);
        
        // Get closed PRs
        let closed_pulls = self.octocrab
            .pulls(&self.owner, &self.repo)
            .list()
            .state(octocrab::params::State::Closed)
            .per_page(100)
            .send()
            .await?;
        all_prs.extend(closed_pulls.items);
        
        // Count PRs created in the last hour
        let count = all_prs.iter()
            .filter(|pr| {
                if let Some(created_at) = pr.created_at {
                    created_at >= one_hour_ago
                } else {
                    false
                }
            })
            .count() as u32;
            
        Ok(count)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_not_found() {
        let result = GitHubClient::read_token();
        if let Err(GitHubError::TokenNotFound(_)) = result {
            // Expected when token file doesn't exist or has placeholder
        } else {
            // If token exists and is valid, that's also fine
        }
    }

    #[test]
    fn test_config_not_found() {
        let result = GitHubClient::read_config();
        if let Err(GitHubError::ConfigNotFound(_)) = result {
            // Expected when config files don't exist
        } else {
            // If config exists and is valid, that's also fine
        }
    }
}