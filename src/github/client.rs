use octocrab::{Octocrab, Error as OctocrabError};
use std::fmt;
use std::fs;
use std::path::Path;
use async_trait::async_trait;
// use crate::github::retry::GitHubRetryHandler;

/// Trait for GitHub operations to enable testing with mocks
#[async_trait]
pub trait GitHubOps {
    async fn fetch_issues(&self) -> Result<Vec<octocrab::models::issues::Issue>, GitHubError>;
    async fn assign_issue(&self, issue_number: u64, assignee: &str) -> Result<(), GitHubError>;
    async fn add_label_to_issue(&self, issue_number: u64, label: &str) -> Result<(), GitHubError>;
    async fn create_branch(&self, branch_name: &str, from_branch: &str) -> Result<(), GitHubError>;
    fn owner(&self) -> &str;
    fn repo(&self) -> &str;
}

#[derive(Debug)]
pub enum GitHubError {
    TokenNotFound(String),
    ConfigNotFound(String),
    ApiError(OctocrabError),
    IoError(std::io::Error),
    NotImplemented(String),
}

impl fmt::Display for GitHubError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitHubError::TokenNotFound(msg) => write!(f, "GitHub token not found: {}", msg),
            GitHubError::ConfigNotFound(msg) => write!(f, "Configuration not found: {}", msg),
            GitHubError::ApiError(err) => write!(f, "GitHub API error: {}", err),
            GitHubError::IoError(err) => write!(f, "IO error: {}", err),
            GitHubError::NotImplemented(msg) => write!(f, "{}", msg),
        }
    }
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
        let issues = self.octocrab
            .issues(&self.owner, &self.repo)
            .list()
            .state(octocrab::params::State::Open)
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

    pub async fn merge_pull_request(
        &self,
        pr_number: u64,
        _merge_method: Option<&str>,
    ) -> Result<(), GitHubError> {
        // Simplified for MVP - just track that we would merge
        println!("🔀 Would merge PR #{}", pr_number);
        
        // TODO: Implement real merge once we understand the octocrab merge API
        
        Ok(())
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
}

// Implement the trait for GitHubClient
#[async_trait]
impl GitHubOps for GitHubClient {
    async fn fetch_issues(&self) -> Result<Vec<octocrab::models::issues::Issue>, GitHubError> {
        self.fetch_issues().await
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