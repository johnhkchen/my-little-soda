use octocrab::Octocrab;
use super::errors::GitHubError;

/// Handler for GitHub issue operations
#[derive(Debug)]
pub struct IssueHandler {
    octocrab: Octocrab,
    owner: String,
    repo: String,
}

impl IssueHandler {
    pub fn new(octocrab: Octocrab, owner: String, repo: String) -> Self {
        Self {
            octocrab,
            owner,
            repo,
        }
    }

    /// Fetch all open issues
    pub async fn fetch_issues(&self) -> Result<Vec<octocrab::models::issues::Issue>, GitHubError> {
        self.fetch_issues_with_state(None).await
    }

    /// Fetch issues with specific state
    pub async fn fetch_issues_with_state(&self, state: Option<octocrab::params::State>) -> Result<Vec<octocrab::models::issues::Issue>, GitHubError> {
        let issues = self.octocrab
            .issues(&self.owner, &self.repo)
            .list()
            .state(state.unwrap_or(octocrab::params::State::Open))
            .send()
            .await?;
            
        Ok(issues.items)
    }

    /// Fetch a specific issue by number
    pub async fn fetch_issue(&self, issue_number: u64) -> Result<octocrab::models::issues::Issue, GitHubError> {
        let issue = self.octocrab
            .issues(&self.owner, &self.repo)
            .get(issue_number)
            .await?;
            
        Ok(issue)
    }

    /// Assign an issue to a user
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

    /// Add a label to an issue
    pub async fn add_label_to_issue(&self, issue_number: u64, label: &str) -> Result<(), GitHubError> {
        self.octocrab
            .issues(&self.owner, &self.repo)
            .add_labels(issue_number, &[label.to_string()])
            .await
            .map_err(GitHubError::ApiError)?;
        Ok(())
    }

    /// Create a new issue
    pub async fn create_issue(&self, title: &str, body: &str, labels: Vec<String>) -> Result<octocrab::models::issues::Issue, GitHubError> {
        let issue = self.octocrab
            .issues(&self.owner, &self.repo)
            .create(title)
            .body(body)
            .labels(labels)
            .send()
            .await
            .map_err(GitHubError::ApiError)?;
        
        println!("✅ Created issue #{}: {}", issue.number, title);
        Ok(issue)
    }

    /// Check if an issue has an open PR that references it
    /// Returns true if the issue has an open PR WITHOUT route:land label
    pub async fn issue_has_blocking_pr(&self, issue_number: u64, open_prs: &[octocrab::models::pulls::PullRequest]) -> Result<bool, GitHubError> {
        use regex::Regex;
        use std::sync::OnceLock;

        /// Compiled regex patterns for issue references, cached using OnceLock
        static ISSUE_REFERENCE_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();

        /// Get compiled regex patterns for matching issue references in PR bodies
        fn get_issue_reference_patterns() -> &'static Vec<Regex> {
            ISSUE_REFERENCE_PATTERNS.get_or_init(|| {
                // Compile the regex patterns once and cache them
                let patterns = [
                    r"(?i)fixes\s+#(\d+)",
                    r"(?i)closes\s+#(\d+)", 
                    r"(?i)resolves\s+#(\d+)",
                    r"(?i)fix\s+#(\d+)",
                    r"(?i)close\s+#(\d+)",
                    r"(?i)resolve\s+#(\d+)",
                    r"#(\d+)", // Simple reference
                ];
                
                patterns
                    .iter()
                    .filter_map(|pattern| Regex::new(pattern).ok())
                    .collect()
            })
        }

        /// Check if a PR body references a specific issue number using optimized regex patterns
        fn pr_references_issue(body: &str, issue_number: u64) -> bool {
            let patterns = get_issue_reference_patterns();
            let issue_str = issue_number.to_string();
            
            for pattern in patterns {
                if let Some(captures) = pattern.captures(body) {
                    if let Some(captured_number) = captures.get(1) {
                        if captured_number.as_str() == issue_str {
                            return true;
                        }
                    }
                }
            }
            false
        }
        
        for pr in open_prs {
            // Check if this PR references the issue number using optimized regex patterns
            if let Some(body) = &pr.body {
                if pr_references_issue(body, issue_number) {
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
}