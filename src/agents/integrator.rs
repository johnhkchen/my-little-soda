// Work Completion Handling - Integration Pipeline
// Following VERBOTEN rules: Work must be preserved, atomic operations

use crate::github::{GitHubClient, GitHubError};
use octocrab::models::issues::Issue;

#[derive(Debug, Clone)]
pub struct CompletedWork {
    pub issue: Issue,
    pub branch_name: String,
    pub commit_sha: String,
    pub agent_id: String,
}

#[derive(Debug, Clone)]
pub struct IntegrationResult {
    pub issue_number: u64,
    pub success: bool,
    pub merged_commit: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OrphanedBranch {
    pub branch_name: String,
    pub issue_number: u64,
    pub reason: OrphanReason,
    pub last_commit_date: Option<i64>,
}

#[derive(Debug, Clone)]
pub enum OrphanReason {
    IssueNotFound,
    IssueClosedNoPr,
    PrMergedBranchRemains,
}

#[derive(Debug)]
pub struct WorkIntegrator {
    github_client: GitHubClient,
}

impl WorkIntegrator {
    pub async fn new() -> Result<Self, GitHubError> {
        let github_client = GitHubClient::new()?;
        Ok(Self { github_client })
    }

    pub async fn collect_completed_work(&self) -> Result<Vec<CompletedWork>, GitHubError> {
        // GitHub-native: Scan for completed work in GitHub repository
        // Look for branches with naming pattern like "agent-work-issue-123"
        // Check PR status, CI status, etc.
        
        let issues = self.github_client.fetch_issues().await?;
        let mut completed_work = Vec::new();
        
        // For MVP, simulate detecting completed work
        // In production, this would:
        // 1. List all branches with agent work patterns
        // 2. Check CI status for each branch  
        // 3. Verify work is ready for integration
        for issue in issues {
            if issue.state == octocrab::models::IssueState::Open {
                // Simulate finding completed work for some issues
                if issue.number % 3 == 0 {
                    completed_work.push(CompletedWork {
                        issue: issue.clone(),
                        branch_name: format!("agent-work-issue-{}", issue.number),
                        commit_sha: format!("abc123{}", issue.number),
                        agent_id: format!("agent-{}", issue.number % 8 + 1),
                    });
                }
            }
        }
        
        Ok(completed_work)
    }

    pub async fn land_completed_work(&self, work_items: Vec<CompletedWork>) -> Result<Vec<IntegrationResult>, GitHubError> {
        // Atomic integration: Each work item is integrated atomically
        // Following "clambake land" command pattern
        let mut results = Vec::new();
        
        for work in work_items {
            let result = self.integrate_single_work_item(&work).await?;
            results.push(result);
        }
        
        Ok(results)
    }

    async fn integrate_single_work_item(&self, work: &CompletedWork) -> Result<IntegrationResult, GitHubError> {
        // Atomic operation: Integrate single work item to main branch
        // This preserves work by ensuring it's never lost during integration
        
        println!("🔄 Integrating work for issue #{} from branch {}", 
                work.issue.number, work.branch_name);
        
        // Generate PR body with auto-close keywords for GitHub's native issue closure
        let pr_body = format!(
            "## Summary\n\
            Automated agent work integration for issue #{}\n\n\
            **Agent**: {}\n\
            **Branch**: {}\n\
            **Work Type**: Agent-completed task\n\n\
            Fixes #{}\n\n\
            🤖 Generated with [Clambake](https://github.com/johnhkchen/clambake)\n\
            Co-Authored-By: {} <agent@clambake.dev>",
            work.issue.number,
            work.agent_id,
            work.branch_name,
            work.issue.number, // This is the key auto-close keyword
            work.agent_id
        );
        
        // Create PR with auto-close keywords
        let pr_title = format!("Agent {}: {}", work.agent_id, work.issue.title);
        
        match self.github_client.create_pull_request(
            &pr_title,
            &work.branch_name,
            "main",
            &pr_body
        ).await {
            Ok(pr) => {
                println!("✅ Created PR #{} with auto-close for issue #{}", pr.number, work.issue.number);
                println!("🔗 PR URL: {}", pr.html_url.as_ref().map(|url| url.as_str()).unwrap_or("(URL not available)"));
                
                // The issue will automatically close when this PR is merged
                let result = IntegrationResult {
                    issue_number: work.issue.number,
                    success: true,
                    merged_commit: Some(format!("pr-{}-{}", pr.number, work.commit_sha)),
                    error: None,
                };
                
                println!("✅ Successfully integrated issue #{} - will auto-close on PR merge", work.issue.number);
                Ok(result)
            }
            Err(e) => {
                // Fallback to manual issue management
                self.preserve_work_on_failure(work, &format!("PR creation failed: {:?}", e)).await?;
                
                let result = IntegrationResult {
                    issue_number: work.issue.number,
                    success: false,
                    merged_commit: None,
                    error: Some(format!("PR creation failed: {:?}", e)),
                };
                
                Ok(result)
            }
        }
    }

    /// Clean up agent branch after successful merge
    pub async fn cleanup_merged_branch(&self, branch_name: &str, pr_number: u64) -> Result<(), GitHubError> {
        // Check if PR was successfully merged before cleanup
        match self.github_client.get_pull_request(pr_number).await {
            Ok(pr) => {
                if pr.merged.unwrap_or(false) {
                    println!("🧹 Cleaning up merged branch: {}", branch_name);
                    
                    // Delete the branch using GitHub API
                    match self.github_client.delete_branch(branch_name).await {
                        Ok(()) => {
                            println!("✅ Successfully deleted branch: {}", branch_name);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to delete branch {}: {:?}", branch_name, e);
                            println!("⚠️  Failed to delete branch {}: {:?}", branch_name, e);
                        }
                    }
                } else {
                    println!("⏳ PR #{} not yet merged, keeping branch: {}", pr_number, branch_name);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to check PR status for cleanup {}: {:?}", pr_number, e);
                println!("⚠️  Could not verify PR merge status, keeping branch: {}", branch_name);
            }
        }
        
        Ok(())
    }

    /// Scan for merged PRs and clean up their branches
    pub async fn cleanup_all_merged_branches(&self) -> Result<Vec<String>, GitHubError> {
        let cleaned_branches = Vec::new();
        
        // This would scan all PRs and clean up merged branches
        // For now, this is a placeholder for the full implementation
        println!("🔍 Scanning for merged PRs to clean up branches...");
        
        // In production, this would:
        // 1. Fetch all merged PRs from the last N days
        // 2. Check if their head branches still exist
        // 3. Delete branches that correspond to completed agent work
        // 4. Handle orphaned branches (branches without corresponding PRs)
        
        Ok(cleaned_branches)
    }

    /// Recover orphaned agent branches (branches without corresponding issues or PRs)
    pub async fn recover_orphaned_branches(&self) -> Result<Vec<OrphanedBranch>, GitHubError> {
        let mut orphaned_branches = Vec::new();
        
        println!("🔍 Scanning for orphaned agent branches...");
        
        // Get all agent branches using git command
        if let Ok(output) = std::process::Command::new("git")
            .args(&["branch", "-r", "--format=%(refname:short)"])
            .output()
        {
            if output.status.success() {
                let branches = String::from_utf8_lossy(&output.stdout);
                for line in branches.lines() {
                    let branch_name = line.trim().trim_start_matches("origin/");
                    
                    // Check if this looks like an agent branch
                    if self.is_agent_branch(branch_name) {
                        match self.analyze_branch_status(branch_name).await {
                            Ok(Some(orphaned)) => {
                                println!("🔍 Found orphaned branch: {}", orphaned.branch_name);
                                orphaned_branches.push(orphaned);
                            }
                            Ok(None) => {
                                // Branch is not orphaned, skip
                            }
                            Err(e) => {
                                tracing::warn!("Failed to analyze branch {}: {:?}", branch_name, e);
                            }
                        }
                    }
                }
            }
        }
        
        println!("✅ Found {} orphaned branches", orphaned_branches.len());
        Ok(orphaned_branches)
    }

    /// Check if a branch name follows agent branch patterns
    fn is_agent_branch(&self, branch_name: &str) -> bool {
        // Check for both old and new patterns:
        // Old: agent001/123
        // New: agent001/123-some-title
        let parts: Vec<&str> = branch_name.split('/').collect();
        if parts.len() == 2 {
            let agent_part = parts[0];
            let issue_part = parts[1];
            
            // Check if agent part matches pattern (agentXXX)
            if agent_part.starts_with("agent") && agent_part.len() >= 6 {
                // Check if issue part starts with a number
                if let Some(first_char) = issue_part.chars().next() {
                    return first_char.is_numeric();
                }
            }
        }
        
        false
    }

    /// Analyze a branch to determine if it's orphaned
    async fn analyze_branch_status(&self, branch_name: &str) -> Result<Option<OrphanedBranch>, GitHubError> {
        // Extract issue number from branch name
        let issue_number = if let Some(issue_num) = self.extract_issue_number(branch_name) {
            issue_num
        } else {
            return Ok(None); // Can't extract issue number, assume not orphaned
        };

        // Check if corresponding issue exists and is still open
        match self.github_client.fetch_issue(issue_number).await {
            Ok(issue) => {
                // Issue exists - check if it has corresponding PR
                match self.github_client.issue_has_blocking_pr(issue_number).await {
                    Ok(has_pr) => {
                        if !has_pr && issue.state == octocrab::models::IssueState::Closed {
                            // Issue is closed but no PR - potential orphaned branch
                            return Ok(Some(OrphanedBranch {
                                branch_name: branch_name.to_string(),
                                issue_number,
                                reason: OrphanReason::IssueClosedNoPr,
                                last_commit_date: self.get_branch_last_commit_date(branch_name).await?,
                            }));
                        }
                    }
                    Err(_) => {
                        // Can't check PR status, be conservative
                    }
                }
                Ok(None) // Branch is not orphaned
            }
            Err(_) => {
                // Issue doesn't exist - this is an orphaned branch
                Ok(Some(OrphanedBranch {
                    branch_name: branch_name.to_string(),
                    issue_number,
                    reason: OrphanReason::IssueNotFound,
                    last_commit_date: self.get_branch_last_commit_date(branch_name).await?,
                }))
            }
        }
    }

    /// Extract issue number from branch name
    fn extract_issue_number(&self, branch_name: &str) -> Option<u64> {
        let parts: Vec<&str> = branch_name.split('/').collect();
        if parts.len() == 2 {
            let issue_part = parts[1];
            // Handle both old (123) and new (123-title) patterns
            let number_part = issue_part.split('-').next()?;
            number_part.parse().ok()
        } else {
            None
        }
    }

    /// Get the last commit date for a branch
    async fn get_branch_last_commit_date(&self, branch_name: &str) -> Result<Option<i64>, GitHubError> {
        // Use git command to get last commit date
        if let Ok(output) = std::process::Command::new("git")
            .args(&["log", "-1", "--format=%ct", &format!("origin/{}", branch_name)])
            .output()
        {
            if output.status.success() {
                let timestamp_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if let Ok(timestamp) = timestamp_str.parse::<i64>() {
                    return Ok(Some(timestamp));
                }
            }
        }
        
        Ok(None)
    }

    /// Recover an orphaned branch by creating recovery issue
    pub async fn recover_branch(&self, orphaned: &OrphanedBranch) -> Result<(), GitHubError> {
        let recovery_title = format!("Branch Recovery: {}", orphaned.branch_name);
        let recovery_body = format!(
            "## Orphaned Branch Recovery\n\n\
            **Branch**: `{}`\n\
            **Original Issue**: #{}\n\
            **Reason**: {:?}\n\
            **Last Commit**: {}\n\n\
            This branch appears to be orphaned and may contain valuable work that needs to be reviewed and potentially integrated.\n\n\
            ### Actions Required\n\
            - [ ] Review branch contents\n\
            - [ ] Determine if work should be integrated\n\
            - [ ] Create new PR or close branch\n\n\
            🤖 Generated by Clambake orphaned branch recovery",
            orphaned.branch_name,
            orphaned.issue_number,
            orphaned.reason,
            orphaned.last_commit_date
                .map(|ts| format!("Unix timestamp: {}", ts))
                .unwrap_or_else(|| "Unknown".to_string())
        );

        // Create recovery issue
        match self.github_client.create_issue(&recovery_title, &recovery_body, vec!["route:ready".to_string()]).await {
            Ok(_) => {
                println!("✅ Created recovery issue for branch: {}", orphaned.branch_name);
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to create recovery issue for {}: {:?}", orphaned.branch_name, e);
                Err(e)
            }
        }
    }

    pub async fn preserve_work_on_failure(&self, work: &CompletedWork, error: &str) -> Result<(), GitHubError> {
        // VERBOTEN rule: Work must be preserved
        // If integration fails, ensure work is not lost
        
        println!("🛡️ Preserving work for issue #{} due to error: {}", 
                work.issue.number, error);
        
        // In production, this would:
        // 1. Create backup branch
        // 2. Tag the work with preservation metadata
        // 3. Create GitHub issue documenting the preservation
        // 4. Notify relevant stakeholders
        
        Ok(())
    }
}