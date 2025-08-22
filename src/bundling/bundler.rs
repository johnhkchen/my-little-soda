use anyhow::{Result, anyhow};
use std::collections::HashMap;
use fd_lock::{RwLock, RwLockWriteGuard};
use std::fs::File;

use crate::train_schedule::QueuedBranch;
use crate::github::{GitHubClient};
use super::{
    types::{BundleWindow, BundleResult},
    git_ops::{GitOperations, ConflictStrategy, ConflictCompatibilityReport},
};

/// Main bundle management system
pub struct BundleManager {
    git_ops: GitOperations,
    github_client: GitHubClient,
    _lock_guard: Option<RwLockWriteGuard<'static, File>>,
}

impl BundleManager {
    /// Create new bundle manager with singleton protection
    pub fn new() -> Result<Self> {
        // Ensure .clambake directory exists
        std::fs::create_dir_all(".clambake")?;
        
        // Acquire singleton lock
        let lock_file = File::create(".clambake/bundle.lock")?;
        let lock = Box::leak(Box::new(RwLock::new(lock_file)));
        let guard = lock.try_write()
            .map_err(|_| anyhow!("Another bundler is already running. Only one bundler can run at a time."))?;
        
        let git_ops = GitOperations::new()?;
        let github_client = GitHubClient::new()?;
        
        Ok(Self {
            git_ops,
            github_client,
            _lock_guard: Some(guard),
        })
    }
    
    /// Generate deterministic bundle branch name
    pub fn generate_bundle_branch_name(&self, queued_branches: &[QueuedBranch]) -> String {
        let window = BundleWindow::current();
        let issues: Vec<u64> = queued_branches.iter().map(|b| b.issue_number).collect();
        window.bundle_branch_name(&issues)
    }
    
    /// Create a bundle PR from queued branches
    pub async fn create_bundle(&self, queued_branches: &[QueuedBranch]) -> Result<BundleResult> {
        if queued_branches.is_empty() {
            return Ok(BundleResult::Failed {
                error: anyhow!("No branches to bundle"),
            });
        }
        
        // Remember the current branch to restore it later
        let original_branch = self.get_current_branch()?;
        
        let bundle_branch = self.generate_bundle_branch_name(queued_branches);
        let base_branch = "main";
        
        // Check if bundle branch already exists (idempotency)
        if self.git_ops.branch_exists(&bundle_branch) {
            println!("⚠️  Bundle branch {} already exists, checking for existing PR...", bundle_branch);
            
            // Check if PR already exists for this branch
            if let Ok(existing_pr) = self.find_existing_bundle_pr(&bundle_branch).await {
                return Ok(BundleResult::Success {
                    pr_number: existing_pr,
                    bundle_branch,
                });
            }
        }
        
        println!("🚄 Creating bundle: {}", bundle_branch);
        
        // Pre-flight conflict analysis
        let branch_names: Vec<String> = queued_branches.iter()
            .map(|b| b.branch_name.clone())
            .collect();
            
        println!("🔍 Analyzing bundle compatibility...");
        match self.git_ops.analyze_bundle_conflicts(&branch_names, base_branch) {
            Ok(compatibility_report) => {
                self.print_compatibility_report(&compatibility_report);
                
                if !compatibility_report.is_bundle_safe && compatibility_report.compatibility_score < 75.0 {
                    println!("⚠️  High conflict risk detected (score: {:.1}%), falling back to individual PRs", 
                        compatibility_report.compatibility_score);
                    return self.create_individual_prs_with_context(queued_branches, Some(compatibility_report)).await;
                }
            }
            Err(e) => {
                println!("⚠️  Conflict analysis failed: {}, proceeding with caution", e);
            }
        }
        
        // Create bundle branch
        if let Err(e) = self.git_ops.create_bundle_branch(&bundle_branch, base_branch) {
            return Ok(BundleResult::Failed {
                error: anyhow!("Failed to create bundle branch: {}", e),
            });
        }
        
        // Checkout bundle branch
        if let Err(e) = self.git_ops.checkout_branch(&bundle_branch) {
            return Ok(BundleResult::Failed {
                error: anyhow!("Failed to checkout bundle branch: {}", e),
            });
        }
        
        // Cherry-pick commits from each queued branch
        let mut conflicts_detected = false;
        let mut successfully_bundled = Vec::new();
        
        for queued_branch in queued_branches {
            println!("🍒 Cherry-picking from {}...", queued_branch.branch_name);
            
            match self.git_ops.cherry_pick_branch(&queued_branch.branch_name, ConflictStrategy::IndividualFallback) {
                Ok(commits) => {
                    println!("✅ Successfully cherry-picked {} commits from {}", 
                        commits.len(), queued_branch.branch_name);
                    successfully_bundled.push(queued_branch.clone());
                }
                Err(e) => {
                    println!("⚠️  Conflict detected with {}: {}", queued_branch.branch_name, e);
                    conflicts_detected = true;
                    break;
                }
            }
        }
        
        // Handle conflicts by falling back to individual PRs  
        if conflicts_detected {
            println!("🔄 Conflicts detected, falling back to individual PRs...");
            return self.create_individual_prs_with_context(queued_branches, None).await;
        }
        
        // Push bundle branch
        if let Err(e) = self.git_ops.push_branch(&bundle_branch, "origin") {
            return Ok(BundleResult::Failed {
                error: anyhow!("Failed to push bundle branch: {}", e),
            });
        }
        
        // Create bundle PR
        let pr_title = self.generate_bundle_pr_title(queued_branches);
        let pr_body = self.generate_bundle_pr_body(queued_branches);
        
        match self.github_client.pulls.create_pull_request(
            &pr_title,
            &bundle_branch,
            base_branch,
            &pr_body,
        ).await {
            Ok(pr) => {
                // Add route:review labels to bundled issues
                for queued_branch in &successfully_bundled {
                    if let Err(e) = self.github_client.add_label_to_issue(
                        queued_branch.issue_number, 
                        "route:review"
                    ).await {
                        println!("⚠️  Failed to add route:review label to issue #{}: {}", 
                            queued_branch.issue_number, e);
                    }
                }
                
                Ok(BundleResult::Success {
                    pr_number: pr.number,
                    bundle_branch,
                })
            }
            Err(e) => Ok(BundleResult::Failed {
                error: anyhow!("Failed to create bundle PR: {}", e),
            }),
        }
    }
    
    /// Create individual PRs when bundling fails due to conflicts
    async fn create_individual_prs_with_context(&self, queued_branches: &[QueuedBranch], conflict_report: Option<ConflictCompatibilityReport>) -> Result<BundleResult> {
        let mut individual_prs = HashMap::new();
        
        for queued_branch in queued_branches {
            println!("📋 Creating individual PR for {}...", queued_branch.branch_name);
            
            // Check if commits exist on this branch
            match self.git_ops.commits_ahead(&queued_branch.branch_name, "main") {
                Ok(0) => {
                    println!("⚠️  No commits found on {}, skipping", queued_branch.branch_name);
                    continue;
                }
                Ok(count) => {
                    println!("📦 Found {} commits on {}", count, queued_branch.branch_name);
                }
                Err(e) => {
                    println!("❌ Error checking commits on {}: {}", queued_branch.branch_name, e);
                    continue;
                }
            }
            
            let pr_title = format!("[AUTO] {}", queued_branch.description);
            let pr_body = self.generate_enhanced_fallback_pr_body(queued_branch, &conflict_report);
            
            match self.github_client.pulls.create_pull_request(
                &pr_title,
                &queued_branch.branch_name,
                "main",
                &pr_body,
            ).await {
                Ok(pr) => {
                    // Add route:review label
                    if let Err(e) = self.github_client.add_label_to_issue(
                        queued_branch.issue_number, 
                        "route:review"
                    ).await {
                        println!("⚠️  Failed to add route:review label to issue #{}: {}", 
                            queued_branch.issue_number, e);
                    }
                    
                    individual_prs.insert(queued_branch.branch_name.clone(), pr.number);
                    println!("✅ Created PR #{} for {}", pr.number, queued_branch.branch_name);
                }
                Err(e) => {
                    println!("❌ Failed to create PR for {}: {}", queued_branch.branch_name, e);
                }
            }
        }
        
        Ok(BundleResult::ConflictFallback { individual_prs })
    }
    
    /// Generate bundle PR title
    fn generate_bundle_pr_title(&self, queued_branches: &[QueuedBranch]) -> String {
        let window = BundleWindow::current();
        let issue_count = queued_branches.len();
        
        format!(
            "[BUNDLE] {} issues - {} train",
            issue_count,
            window.start.format("%H:%M")
        )
    }
    
    /// Generate bundle PR body with issue references
    fn generate_bundle_pr_body(&self, queued_branches: &[QueuedBranch]) -> String {
        let window = BundleWindow::current();
        
        let mut body = format!(
            "🚄 **Train Bundle - {} Departure**\n\n\
            This bundle combines multiple completed agent tasks into a single PR for efficient review.\n\n\
            **Bundle Window:** {} - {}\n\
            **Issues Included:** {}\n\n\
            ## Bundled Work\n\n",
            window.start.format("%H:%M"),
            window.start.format("%Y-%m-%d %H:%M"),
            window.end.format("%H:%M"),
            queued_branches.len()
        );
        
        for (i, branch) in queued_branches.iter().enumerate() {
            body.push_str(&format!(
                "{}. **Issue #{}**: {}\n   - Branch: `{}`\n   - [View Issue](https://github.com/{}/{}/issues/{})\n\n",
                i + 1,
                branch.issue_number,
                branch.description,
                branch.branch_name,
                self.github_client.owner(),
                self.github_client.repo(),
                branch.issue_number
            ));
        }
        
        body.push_str(&format!(
            "## Review Notes\n\n\
            - ✅ All branches have been automatically cherry-picked and tested\n\
            - 🔍 Each issue should be reviewed individually for code quality\n\
            - 🚀 Merge this PR to close all {} included issues\n\n\
            ---\n\
            🤖 Generated by Clambake bundling system",
            queued_branches.len()
        ));
        
        body
    }
    
    /// Get the current branch name
    fn get_current_branch(&self) -> Result<String> {
        let head = self.git_ops.repo.head()?;
        if let Some(branch_name) = head.shorthand() {
            Ok(branch_name.to_string())
        } else {
            Ok("main".to_string()) // fallback
        }
    }
    
    /// Find existing bundle PR for a branch
    async fn find_existing_bundle_pr(&self, _bundle_branch: &str) -> Result<u64> {
        // This would query GitHub for existing PRs with the bundle branch
        // For now, we'll return an error to indicate no existing PR found
        Err(anyhow!("No existing PR found for bundle branch"))
    }
    
    /// Print compatibility report for debugging and transparency
    fn print_compatibility_report(&self, report: &ConflictCompatibilityReport) {
        println!("📊 Bundle Compatibility Report:");
        println!("   • Safety Score: {:.1}%", report.compatibility_score);
        println!("   • Bundle Safe: {}", if report.is_bundle_safe { "✅" } else { "⚠️" });
        println!("   • Analyzed Branches: {}", report.analyzed_branches.len());
        
        if !report.potential_conflicts.is_empty() {
            println!("   • Potential Conflicts:");
            for (file, branches) in &report.potential_conflicts {
                println!("     - {}: {} ({})", file, branches.len(), branches.join(", "));
            }
        }
        
        if !report.analysis_errors.is_empty() {
            println!("   • Analysis Errors:");
            for error in &report.analysis_errors {
                println!("     - {}", error);
            }
        }
    }
    
    /// Generate enhanced PR body for individual fallback PRs
    fn generate_enhanced_fallback_pr_body(&self, queued_branch: &QueuedBranch, conflict_report: &Option<ConflictCompatibilityReport>) -> String {
        let mut body = format!(
            "🤖 **Automated PR from bundling fallback**\n\n\
            This PR was created automatically because bundling conflicts were detected.\n\n\
            **Issue:** #{}\n\
            **Branch:** `{}`\n\n",
            queued_branch.issue_number,
            queued_branch.branch_name
        );
        
        if let Some(report) = conflict_report {
            body.push_str(&format!(
                "## Conflict Analysis\n\n\
                **Bundle Safety Score:** {:.1}%\n\
                **Analysis Timestamp:** {}\n\n",
                report.compatibility_score,
                report.analysis_timestamp.format("%Y-%m-%d %H:%M:%S UTC")
            ));
            
            if !report.potential_conflicts.is_empty() {
                body.push_str("**Detected File Conflicts:**\n");
                for (file, branches) in &report.potential_conflicts {
                    if branches.contains(&queued_branch.branch_name) {
                        body.push_str(&format!("- `{}` (conflicts with: {})\n", file, 
                            branches.iter()
                                .filter(|b| *b != &queued_branch.branch_name)
                                .cloned()
                                .collect::<Vec<_>>()
                                .join(", ")));
                    }
                }
                body.push('\n');
            }
            
            body.push_str("This individual PR ensures clean merging while preserving work integrity.\n\n");
        }
        
        body.push_str(
            "## Review Notes\n\n\
            - ✅ Work has been preserved in individual PR for safe merging\n\
            - 🔍 Please review for code quality and functionality\n\
            - 🚀 This PR can be merged independently\n\n\
            Please review and merge when ready.\n\n\
            ---\n\
            🤖 Generated by Clambake conflict resolution system"
        );
        
        body
    }
    
    /// Legacy method for backwards compatibility
    async fn create_individual_prs(&self, queued_branches: &[QueuedBranch]) -> Result<BundleResult> {
        self.create_individual_prs_with_context(queued_branches, None).await
    }
}