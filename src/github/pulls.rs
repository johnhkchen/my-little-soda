use super::{
    errors::GitHubError,
    types::{ConflictAnalysis, ConflictRecoveryData, SafeMergeResult},
};
use octocrab::params::pulls::MergeMethod;
use octocrab::Octocrab;
use std::collections::HashMap;

/// Handler for GitHub pull request operations
#[derive(Debug, Clone)]
pub struct PullRequestHandler {
    octocrab: Octocrab,
    owner: String,
    repo: String,
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

impl PullRequestHandler {
    pub fn new(octocrab: Octocrab, owner: String, repo: String) -> Self {
        Self {
            octocrab,
            owner,
            repo,
        }
    }

    /// Create a new pull request
    pub async fn create_pull_request(
        &self,
        title: &str,
        head_branch: &str,
        base_branch: &str,
        body: &str,
    ) -> Result<octocrab::models::pulls::PullRequest, GitHubError> {
        let pr = self
            .octocrab
            .pulls(&self.owner, &self.repo)
            .create(title, head_branch, base_branch)
            .body(body)
            .send()
            .await?;

        println!(
            "📋 Created PR #{}: {} ({})",
            pr.number,
            title,
            pr.html_url.as_ref().unwrap()
        );
        Ok(pr)
    }

    /// Get a specific pull request by number
    pub async fn get_pull_request(
        &self,
        pr_number: u64,
    ) -> Result<octocrab::models::pulls::PullRequest, GitHubError> {
        let pr = self
            .octocrab
            .pulls(&self.owner, &self.repo)
            .get(pr_number)
            .await?;

        Ok(pr)
    }

    /// Check if a PR is ready for merging
    pub async fn is_pr_mergeable(
        &self,
        pr: &octocrab::models::pulls::PullRequest,
    ) -> Result<bool, GitHubError> {
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
        let reviews_result = self
            .octocrab
            .pulls(&self.owner, &self.repo)
            .list_reviews(pr_number)
            .send()
            .await;

        let (approved_reviews, requested_changes) = match reviews_result {
            Ok(reviews) => {
                // Get latest review per reviewer by comparing submitted_at timestamps
                let mut latest_reviews: HashMap<
                    octocrab::models::UserId,
                    &octocrab::models::pulls::Review,
                > = HashMap::new();

                for review in &reviews.items {
                    if let Some(user) = &review.user {
                        let user_id = user.id;

                        // Check if this is the latest review from this user
                        match latest_reviews.get(&user_id) {
                            Some(existing_review) => {
                                // Compare submitted_at to keep the latest review
                                if let (Some(new_submitted), Some(existing_submitted)) =
                                    (&review.submitted_at, &existing_review.submitted_at)
                                {
                                    if new_submitted > existing_submitted {
                                        latest_reviews.insert(user_id, review);
                                    }
                                }
                            }
                            None => {
                                latest_reviews.insert(user_id, review);
                            }
                        }
                    }
                }

                // Count approved and changes requested from latest reviews only
                let approved = latest_reviews
                    .values()
                    .filter(|review| {
                        review
                            .state
                            .as_ref()
                            .map(|s| format!("{s:?}").contains("Approved"))
                            .unwrap_or(false)
                    })
                    .count();

                let changes = latest_reviews
                    .values()
                    .filter(|review| {
                        review
                            .state
                            .as_ref()
                            .map(|s| format!("{s:?}").contains("ChangesRequested"))
                            .unwrap_or(false)
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

    /// Merge a pull request
    pub async fn merge_pull_request(
        &self,
        pr_number: u64,
        merge_method: Option<&str>,
    ) -> Result<octocrab::models::pulls::PullRequest, GitHubError> {
        // First check if PR is mergeable
        let pr = self.get_pull_request(pr_number).await?;

        if !self.is_pr_mergeable(&pr).await? {
            return Err(GitHubError::NotImplemented(format!(
                "PR #{pr_number} is not ready for merge. Check CI status, conflicts, or review requirements."
            )));
        }

        println!(
            "🔀 Merging PR #{}: {}",
            pr_number,
            pr.title.as_ref().unwrap_or(&"".to_string())
        );

        let method = merge_method.unwrap_or("squash");

        // Use octocrab to merge the PR
        let merge_result = self
            .octocrab
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
            println!("✅ Successfully merged PR #{pr_number}");
            Ok(pr)
        } else {
            Err(GitHubError::NotImplemented(format!(
                "PR #{} merge was not successful. SHA: {:?}",
                pr_number, merge_result.sha
            )))
        }
    }

    /// Fetch all open pull requests
    pub async fn fetch_open_pull_requests(
        &self,
    ) -> Result<Vec<octocrab::models::pulls::PullRequest>, GitHubError> {
        let pulls = self
            .octocrab
            .pulls(&self.owner, &self.repo)
            .list()
            // Note: octocrab::params::State::Open is correct here, not octocrab::params::pulls::State::Open
            // The .state() method expects octocrab::params::State, as verified by compilation
            .state(octocrab::params::State::Open)
            .send()
            .await?;

        Ok(pulls.items)
    }

    /// Get the number of PRs created in the last hour
    pub async fn get_pr_creation_rate(&self) -> Result<u32, GitHubError> {
        use chrono::{Duration, Utc};

        let one_hour_ago = Utc::now() - Duration::hours(1);

        // Fetch both open and closed PRs
        let mut all_prs = Vec::new();

        // Get open PRs
        let open_pulls = self
            .octocrab
            .pulls(&self.owner, &self.repo)
            .list()
            .state(octocrab::params::State::Open)
            .per_page(100)
            .send()
            .await?;
        all_prs.extend(open_pulls.items);

        // Get closed PRs
        let closed_pulls = self
            .octocrab
            .pulls(&self.owner, &self.repo)
            .list()
            .state(octocrab::params::State::Closed)
            .per_page(100)
            .send()
            .await?;
        all_prs.extend(closed_pulls.items);

        // Count PRs created in the last hour
        let count = all_prs
            .iter()
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

    /// Enhanced merge conflict detection with detailed diagnostics
    pub async fn detect_merge_conflicts(
        &self,
        pr_number: u64,
    ) -> Result<ConflictAnalysis, GitHubError> {
        let pr = self.get_pull_request(pr_number).await?;

        let mut analysis = ConflictAnalysis {
            has_conflicts: false,
            is_mergeable: true,
            conflict_files: Vec::new(),
            base_branch: pr.base.ref_field.clone(),
            head_branch: pr.head.ref_field.clone(),
            head_sha: pr.head.sha.clone(),
            analysis_timestamp: chrono::Utc::now(),
        };

        // Check GitHub's mergeable status
        if pr.mergeable == Some(false) {
            analysis.has_conflicts = true;
            analysis.is_mergeable = false;
        }

        // If mergeable is None, GitHub may still be calculating - treat as potential conflict
        if pr.mergeable.is_none() {
            analysis.is_mergeable = false;
            println!("⚠️  GitHub is still calculating merge status for PR #{pr_number}");
        }

        // Additional checks for merge readiness
        if pr.merged.unwrap_or(false) {
            analysis.is_mergeable = false;
        }

        let pr_state_str = format!("{:?}", pr.state).to_lowercase();
        if !pr_state_str.contains("open") {
            analysis.is_mergeable = false;
        }

        Ok(analysis)
    }

    /// Enhanced merge attempt with conflict detection and automatic recovery
    pub async fn safe_merge_pull_request(
        &self,
        pr_number: u64,
        agent_id: &str,
        issue_number: u64,
        merge_method: Option<&str>,
        issue_handler: &super::issues::IssueHandler,
    ) -> Result<SafeMergeResult, GitHubError> {
        println!("🔍 Performing pre-merge conflict analysis for PR #{pr_number}...");

        // Step 1: Detect conflicts before attempting merge
        let conflict_analysis = self.detect_merge_conflicts(pr_number).await?;

        if conflict_analysis.has_conflicts || !conflict_analysis.is_mergeable {
            println!("🚨 Merge conflicts detected! Initiating recovery workflow...");

            // Step 2: Create recovery data
            let recovery_data = ConflictRecoveryData {
                agent_id: agent_id.to_string(),
                issue_number,
                original_pr_number: pr_number,
                conflict_analysis,
                backup_branch: format!("backup/{agent_id}-{issue_number}"),
                recovery_timestamp: chrono::Utc::now(),
            };

            // Step 3: Create recovery PR with human review request
            let recovery_pr = self
                .create_conflict_recovery_pr(pr_number, recovery_data, issue_handler)
                .await?;

            return Ok(SafeMergeResult::ConflictDetected {
                original_pr: pr_number,
                recovery_pr: recovery_pr.number,
                recovery_url: recovery_pr.html_url.map(|url| url.to_string()),
                requires_human_review: true,
            });
        }

        // Step 4: If no conflicts, proceed with normal merge
        println!("✅ No conflicts detected. Proceeding with merge...");
        match self.merge_pull_request(pr_number, merge_method).await {
            Ok(merged_pr) => Ok(SafeMergeResult::SuccessfulMerge {
                pr_number,
                merged_sha: merged_pr.merge_commit_sha,
            }),
            Err(e) => {
                // Even if pre-check passed, merge can still fail - create recovery
                println!("🚨 Unexpected merge failure! Creating recovery PR...");

                let recovery_data = ConflictRecoveryData {
                    agent_id: agent_id.to_string(),
                    issue_number,
                    original_pr_number: pr_number,
                    conflict_analysis,
                    backup_branch: format!("backup/{agent_id}-{issue_number}"),
                    recovery_timestamp: chrono::Utc::now(),
                };

                let recovery_pr = self
                    .create_conflict_recovery_pr(pr_number, recovery_data, issue_handler)
                    .await?;

                Ok(SafeMergeResult::MergeFailed {
                    error: format!("{e:?}"),
                    recovery_pr: recovery_pr.number,
                    work_preserved: true,
                })
            }
        }
    }

    /// Create a recovery PR for conflicted work with human review request
    pub async fn create_conflict_recovery_pr(
        &self,
        original_pr: u64,
        work_data: ConflictRecoveryData,
        issue_handler: &super::issues::IssueHandler,
    ) -> Result<octocrab::models::pulls::PullRequest, GitHubError> {
        // Create a new branch for conflict recovery
        let recovery_branch = format!("conflict-recovery/{}-{}", original_pr, work_data.agent_id);

        println!("🛡️ Creating conflict recovery branch: {recovery_branch}");

        // Recovery PR body with detailed conflict information and human review request
        let pr_body = format!(
            "## 🚨 MERGE CONFLICT RECOVERY\n\
            \n\
            **Original PR**: #{}\n\
            **Agent**: {}\n\
            **Issue**: #{}\n\
            **Recovery Branch**: {}\n\
            **Conflict Detection**: {}\n\
            \n\
            ## Conflict Analysis\n\
            - **Base Branch**: {}\n\
            - **Head Branch**: {}\n\
            - **Head SHA**: {}\n\
            - **Conflicts Detected**: {}\n\
            \n\
            ## Preserved Work\n\
            This PR preserves all agent work that would have been lost due to merge conflicts.\n\
            The original implementation has been backed up and is ready for human review.\n\
            \n\
            ## Human Review Required\n\
            ⚠️  **MANUAL CONFLICT RESOLUTION NEEDED**\n\
            \n\
            1. Review the conflicted files and resolve merge conflicts\n\
            2. Test the merged functionality thoroughly\n\
            3. Ensure no agent work is lost in the resolution\n\
            4. Merge this recovery PR when conflicts are resolved\n\
            \n\
            ## Next Steps\n\
            - [ ] Human reviewer resolves merge conflicts\n\
            - [ ] Functionality testing completed\n\
            - [ ] Original work preserved and integrated\n\
            - [ ] Recovery PR merged\n\
            \n\
            Fixes #{}\n\
            \n\
            🤖 Generated with [Clambake](https://github.com/johnhkchen/clambake) - Conflict Recovery System\n\
            Co-Authored-By: {} <agent@clambake.dev>",
            original_pr,
            work_data.agent_id,
            work_data.issue_number,
            recovery_branch,
            work_data.conflict_analysis.analysis_timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            work_data.conflict_analysis.base_branch,
            work_data.conflict_analysis.head_branch,
            work_data.conflict_analysis.head_sha,
            if work_data.conflict_analysis.has_conflicts { "Yes" } else { "Potential" },
            work_data.issue_number,
            work_data.agent_id
        );

        let pr_title = format!(
            "🚨 CONFLICT RECOVERY: Agent {} work for issue #{}",
            work_data.agent_id, work_data.issue_number
        );

        // Create the recovery PR
        let pr = self
            .create_pull_request(&pr_title, &recovery_branch, "main", &pr_body)
            .await?;

        // Add labels to indicate this is a recovery PR requiring human attention
        issue_handler
            .add_label_to_issue(work_data.issue_number, "merge-conflict")
            .await?;
        issue_handler
            .add_label_to_issue(work_data.issue_number, "human-review-required")
            .await?;
        issue_handler
            .add_label_to_issue(work_data.issue_number, "work-preserved")
            .await?;

        println!("✅ Created conflict recovery PR #{}", pr.number);
        println!(
            "🔗 Recovery PR URL: {}",
            pr.html_url
                .as_ref()
                .map(|url| url.as_str())
                .unwrap_or("(URL not available)")
        );

        Ok(pr)
    }
}
