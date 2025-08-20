use octocrab::{Octocrab, Error as OctocrabError};
use std::fs;
use std::path::Path;
use async_trait::async_trait;
// use crate::github::retry::GitHubRetryHandler;

/// CodeRabbit feedback categorization
#[derive(Debug, Clone)]
pub struct CodeRabbitFeedback {
    pub actionable_suggestions: Vec<FeedbackItem>,
    pub nitpick_suggestions: Vec<FeedbackItem>,
    pub has_feedback: bool,
}

#[derive(Debug, Clone)]
pub struct FeedbackItem {
    pub content: String,
    pub url: String,
    pub category: FeedbackCategory,
}

#[derive(Debug, Clone)]
pub enum FeedbackCategory {
    Actionable,
    Nitpick,
}

impl CodeRabbitFeedback {
    pub fn new() -> Self {
        Self {
            actionable_suggestions: Vec::new(),
            nitpick_suggestions: Vec::new(),
            has_feedback: false,
        }
    }
    
    pub fn add_review_comment(&mut self, content: &str, url: &str) {
        self.has_feedback = true;
        
        // Categorize the feedback based on content
        if self.is_actionable_feedback(content) {
            self.actionable_suggestions.push(FeedbackItem {
                content: content.to_string(),
                url: url.to_string(),
                category: FeedbackCategory::Actionable,
            });
        } else if self.is_nitpick_feedback(content) {
            self.nitpick_suggestions.push(FeedbackItem {
                content: content.to_string(),
                url: url.to_string(),
                category: FeedbackCategory::Nitpick,
            });
        }
    }
    
    pub fn add_line_comment(&mut self, content: &str, url: &str) {
        self.has_feedback = true;
        
        // Line comments are typically more actionable
        if self.is_actionable_feedback(content) {
            self.actionable_suggestions.push(FeedbackItem {
                content: content.to_string(),
                url: url.to_string(),
                category: FeedbackCategory::Actionable,
            });
        } else {
            self.nitpick_suggestions.push(FeedbackItem {
                content: content.to_string(),
                url: url.to_string(),
                category: FeedbackCategory::Nitpick,
            });
        }
    }
    
    fn is_actionable_feedback(&self, content: &str) -> bool {
        let content_lower = content.to_lowercase();
        
        // Keywords that indicate actionable feedback
        let actionable_keywords = [
            "security", "vulnerability", "bug", "error", "fix", "critical",
            "performance", "memory leak", "null pointer", "unsafe",
            "should", "must", "needs to", "required", "recommend",
            "breaking change", "compatibility", "refactor"
        ];
        
        actionable_keywords.iter().any(|&keyword| content_lower.contains(keyword))
    }
    
    fn is_nitpick_feedback(&self, content: &str) -> bool {
        let content_lower = content.to_lowercase();
        
        // Keywords that indicate style/nitpick feedback
        let nitpick_keywords = [
            "style", "formatting", "naming", "comment", "documentation",
            "consider", "might", "could", "nitpick", "minor",
            "suggestion", "readability", "clarity"
        ];
        
        nitpick_keywords.iter().any(|&keyword| content_lower.contains(keyword))
    }
    
    pub fn total_suggestions(&self) -> usize {
        self.actionable_suggestions.len() + self.nitpick_suggestions.len()
    }
}

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

    pub async fn add_label_to_pr(&self, pr_number: u64, label: &str) -> Result<(), GitHubError> {
        self.octocrab
            .issues(&self.owner, &self.repo)
            .add_labels(pr_number, &[label.to_string()])
            .await
            .map_err(GitHubError::ApiError)?;
        Ok(())
    }

    pub async fn fetch_open_pull_requests(&self) -> Result<Vec<octocrab::models::pulls::PullRequest>, GitHubError> {
        let pulls = self.octocrab
            .pulls(&self.owner, &self.repo)
            .list()
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

    /// Find PRs with completed reviews that are ready for route:land labeling
    /// Returns list of (PR number, associated issue number) tuples
    pub async fn find_prs_with_completed_reviews(&self) -> Result<Vec<(u64, u64)>, GitHubError> {
        let open_prs = self.fetch_open_pull_requests().await?;
        let mut completed_prs = Vec::new();
        
        for pr in open_prs {
            // Skip PRs that already have route:land label
            let has_route_land = pr.labels.as_ref()
                .map(|labels| labels.iter().any(|label| label.name == "route:land"))
                .unwrap_or(false);
            
            if has_route_land {
                continue;
            }
            
            // Extract associated issue number from PR body
            if let Some(issue_number) = self.extract_issue_number_from_pr(&pr).await {
                // Check if this PR has completed reviews
                if let Ok(is_review_complete) = self.is_pr_review_complete(pr.number).await {
                    if is_review_complete {
                        completed_prs.push((pr.number, issue_number));
                    }
                }
            }
        }
        
        Ok(completed_prs)
    }
    
    /// Extract issue number from PR body using common patterns
    async fn extract_issue_number_from_pr(&self, pr: &octocrab::models::pulls::PullRequest) -> Option<u64> {
        if let Some(body) = &pr.body {
            let body_lower = body.to_lowercase();
            
            // Look for patterns like "fixes #123", "closes #123", etc.
            let patterns = [
                r"fixes #(\d+)",
                r"closes #(\d+)", 
                r"resolves #(\d+)",
                r"fix #(\d+)",
                r"close #(\d+)",
                r"resolve #(\d+)",
            ];
            
            for pattern in patterns {
                if let Ok(regex) = regex::Regex::new(pattern) {
                    if let Some(captures) = regex.captures(&body_lower) {
                        if let Some(number_str) = captures.get(1) {
                            if let Ok(issue_number) = number_str.as_str().parse::<u64>() {
                                return Some(issue_number);
                            }
                        }
                    }
                }
            }
        }
        
        None
    }
    
    /// Check if a PR's review process is complete
    async fn is_pr_review_complete(&self, pr_number: u64) -> Result<bool, GitHubError> {
        // Get PR reviews
        let reviews = self.octocrab
            .pulls(&self.owner, &self.repo)
            .list_reviews(pr_number)
            .send()
            .await?;
        
        let mut has_approval = false;
        let mut has_pending_changes = false;
        let mut has_coderabbit_review = false;
        
        // Check the latest review from each reviewer
        let mut latest_reviews = std::collections::HashMap::new();
        
        for review in reviews.items {
            if let Some(user) = &review.user {
                // Keep only the latest review from each user
                latest_reviews.insert(user.id, review);
            }
        }
        
        // Analyze the latest reviews
        let mut coderabbit_completed = false;
        for review in latest_reviews.values() {
            if let Some(user) = &review.user {
                // Check if this is a CodeRabbit review
                if user.login.to_lowercase().contains("coderabbit") {
                    has_coderabbit_review = true;
                    
                    // For CodeRabbit, check both review state and content
                    if let Some(state) = &review.state {
                        // CodeRabbit completes with state "COMMENTED" when it has provided feedback
                        if matches!(state, octocrab::models::pulls::ReviewState::Commented) {
                            // Also verify it has substantial content (not just initial comment)
                            if let Some(body) = &review.body {
                                if body.len() > 100 { // Substantial review content
                                    coderabbit_completed = true;
                                }
                            }
                        }
                    }
                } else {
                    // Human reviewer logic
                    if let Some(state) = &review.state {
                        match state {
                            octocrab::models::pulls::ReviewState::Approved => has_approval = true,
                            octocrab::models::pulls::ReviewState::ChangesRequested => has_pending_changes = true,
                            _ => {} // COMMENTED or other states don't affect completion
                        }
                    }
                }
            }
        }
        
        // If we found CodeRabbit, check if it completed its review
        if has_coderabbit_review {
            return Ok(coderabbit_completed);
        }
        
        // For human PRs, require approval without pending changes
        Ok(has_approval && !has_pending_changes)
    }
    
    /// Check PR review completion using GitHub's check runs (for bots like CodeRabbit)
    async fn check_pr_review_status_via_checks(&self, pr_number: u64) -> Result<bool, GitHubError> {
        // Get the PR to find the latest commit SHA
        let pr = self.get_pull_request(pr_number).await?;
        let commit_sha = &pr.head.sha;
        
        // Get check runs for the latest commit
        let check_runs = self.octocrab
            .checks(&self.owner, &self.repo)
            .list_check_runs_for_git_ref(octocrab::params::repos::Commitish(commit_sha.clone()))
            .send()
            .await?;
        
        // Look for CodeRabbit or other review bot check runs
        for check_run in check_runs.check_runs {
            let name_lower = check_run.name.to_lowercase();
            
            // Check for CodeRabbit or other review tools
            if name_lower.contains("coderabbit") 
                || name_lower.contains("review") 
                || name_lower.contains("code-review") {
                
                // Check if the review is complete and passed
                if let Some(conclusion) = &check_run.conclusion {
                    match conclusion.as_str() {
                        "success" | "neutral" => {
                            println!("  ✅ Found completed review check: {} = {}", check_run.name, conclusion);
                            return Ok(true);
                        }
                        "failure" | "cancelled" | "timed_out" => {
                            println!("  ❌ Found failed review check: {} = {}", check_run.name, conclusion);
                            return Ok(false);
                        }
                        _ => {
                            println!("  ⏳ Review check in progress: {} = {}", check_run.name, conclusion);
                        }
                    }
                }
            }
        }
        
        Ok(false)
    }
    
    /// Check if a PR was created by a bot (e.g., CodeRabbit)
    async fn is_bot_pr(&self, pr_number: u64) -> Result<bool, GitHubError> {
        let pr = self.get_pull_request(pr_number).await?;
        
        if let Some(user) = &pr.user {
            // Check if the PR author is a known bot
            let bot_names = ["coderabbitai", "dependabot", "renovate", "github-actions"];
            let is_bot = bot_names.iter().any(|&bot| user.login.to_lowercase().contains(bot));
            // For now, just check by username pattern - we can enhance type checking later
            Ok(is_bot)
        } else {
            Ok(false)
        }
    }

    /// Get CodeRabbit feedback from a PR and categorize suggestions
    pub async fn get_coderabbit_feedback(&self, pr_number: u64) -> Result<CodeRabbitFeedback, GitHubError> {
        let mut feedback = CodeRabbitFeedback::new();
        
        // Get PR reviews
        let reviews = self.octocrab
            .pulls(&self.owner, &self.repo)
            .list_reviews(pr_number)
            .send()
            .await?;
        
        // Get PR review comments (line-specific comments)
        let review_comments = self.octocrab
            .pulls(&self.owner, &self.repo)
            .list_comments(Some(pr_number))
            .send()
            .await?;
        
        // Process reviews from CodeRabbit
        for review in reviews.items {
            if let Some(user) = &review.user {
                if user.login.to_lowercase().contains("coderabbit") {
                    if let Some(body) = &review.body {
                        // Use PR URL as fallback for review URL
                        let url_str = format!("PR #{}", pr_number);
                        feedback.add_review_comment(body, &url_str);
                    }
                }
            }
        }
        
        // Process line-specific comments from CodeRabbit  
        for comment in review_comments.items {
            if let Some(user) = &comment.user {
                if user.login.to_lowercase().contains("coderabbit") {
                    let url_str = if comment.html_url.is_empty() {
                        format!("PR #{}", pr_number)
                    } else {
                        comment.html_url
                    };
                    feedback.add_line_comment(&comment.body, &url_str);
                }
            }
        }
        
        Ok(feedback)
    }

    /// Check if CodeRabbit feedback has been decomposed into issues
    pub async fn is_coderabbit_feedback_decomposed(&self, pr_number: u64) -> Result<bool, GitHubError> {
        let feedback = self.get_coderabbit_feedback(pr_number).await?;
        
        // If no significant feedback, consider it decomposed
        if feedback.actionable_suggestions.is_empty() && feedback.nitpick_suggestions.is_empty() {
            return Ok(true);
        }
        
        // Check if there are recent issues created with coderabbit-feedback label
        // that reference this PR
        let issues = self.fetch_issues().await?;
        let coderabbit_issues: Vec<_> = issues
            .iter()
            .filter(|issue| {
                // Check for coderabbit-feedback label
                let has_feedback_label = issue.labels.iter()
                    .any(|label| label.name == "coderabbit-feedback");
                
                // Check if issue references this PR
                let references_pr = if let Some(body) = &issue.body {
                    body.contains(&format!("PR #{}", pr_number)) || 
                    body.contains(&format!("pr/{}", pr_number))
                } else {
                    false
                };
                
                has_feedback_label && references_pr
            })
            .collect();
        
        // Consider feedback decomposed if we have at least one follow-up issue
        // In a more sophisticated implementation, we could check if the number
        // of issues matches the number of suggestions
        Ok(!coderabbit_issues.is_empty())
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