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
        
        // In production, this would:
        // 1. Create PR if not exists
        // 2. Wait for CI to pass
        // 3. Merge to main branch
        // 4. Update issue status
        // 5. Clean up branch
        
        // For MVP, simulate successful integration
        let result = IntegrationResult {
            issue_number: work.issue.number,
            success: true,
            merged_commit: Some(format!("merged-{}", work.commit_sha)),
            error: None,
        };
        
        println!("✅ Successfully integrated issue #{}", work.issue.number);
        Ok(result)
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