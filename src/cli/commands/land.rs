use anyhow::{Result, anyhow};
use crate::github::GitHubClient;
use crate::train_schedule::{TrainSchedule, QueuedBranch};
use crate::agents::AgentCoordinator;
use git2::Repository;
use std::process::Command;

pub struct LandCommand {
    pub include_closed: bool,
    pub days: u32,
    pub dry_run: bool,
    pub verbose: bool,
    pub ci_mode: bool,
}

impl LandCommand {
    pub fn new(include_closed: bool, days: u32, dry_run: bool, verbose: bool) -> Self {
        Self {
            include_closed,
            days,
            dry_run,
            verbose,
            ci_mode: false,
        }
    }

    pub fn with_ci_mode(mut self, ci_mode: bool) -> Self {
        self.ci_mode = ci_mode;
        self
    }

    pub async fn execute(&self) -> Result<()> {
        if self.dry_run {
            println!("🚀 MY LITTLE SODA LAND - Mark Work Ready for Review (DRY RUN)");
        } else {
            println!("🚀 MY LITTLE SODA LAND - Mark Work Ready for Review");
        }
        println!("==========================================");
        println!();
        
        // Get current branch and parse agent/issue info
        let current_branch = self.get_current_branch()?;
        let (agent_id, issue_number) = self.parse_agent_branch(&current_branch)?;
        
        // Validate ready to land (unless dry run - we want to show what would happen)
        if !self.dry_run {
            self.validate_ready_to_land(&current_branch)?;
        }
        
        if self.verbose {
            println!("🔧 Configuration:");
            println!("   🌿 Current branch: {}", current_branch);
            println!("   🤖 Agent ID: {}", agent_id);
            println!("   📋 Issue number: {}", issue_number);
            println!("   🔍 Dry run mode: {}", if self.dry_run { "Yes" } else { "No" });
            println!();
        }
        
        // Initialize GitHub client and agent coordinator
        let client = GitHubClient::new()
            .map_err(|e| anyhow!("Failed to initialize GitHub client: {}", e))?;
        let coordinator = AgentCoordinator::new().await
            .map_err(|e| anyhow!("Failed to initialize agent coordinator: {}", e))?;
        
        println!("🔍 Processing agent work for issue #{}...", issue_number);
        
        // Step 1: Push current branch to remote if needed
        if !self.dry_run {
            print!("📤 Ensuring branch is pushed to remote... ");
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            
            self.push_current_branch(&current_branch)?;
            println!("✅");
        } else {
            println!("📤 [DRY RUN] Would push branch to remote: {}", current_branch);
        }
        
        // Step 2: Remove route:ready label (if present) to transition from ready to review
        if !self.dry_run {
            print!("🏷️  Removing route:ready label from issue #{}... ", issue_number);
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            
            // Note: It's OK if the label doesn't exist - GitHub API handles this gracefully
            if let Err(e) = client.remove_label_from_issue(issue_number, "route:ready").await {
                // Log warning but don't fail the entire operation
                eprintln!("⚠️  Warning: Could not remove route:ready label: {}", e);
                eprintln!("   This is usually safe - the label may not exist or may already be removed");
            }
            println!("✅");
        } else {
            println!("🏷️  [DRY RUN] Would remove route:ready label from issue #{}", issue_number);
        }

        // Step 3: Add route:review label to mark as ready for bundling
        if !self.dry_run {
            print!("🏷️  Adding route:review label to issue #{}... ", issue_number);
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            
            client.add_label_to_issue(issue_number, "route:review").await
                .map_err(|e| anyhow!("Failed to add route:review label: {}", e))?;
            println!("✅");
        } else {
            println!("🏷️  [DRY RUN] Would add route:review label to issue #{}", issue_number);
        }
        
        // Step 4: Trigger state machine transition to complete work
        if !self.dry_run {
            print!("⚙️  Completing work in state machine... ");
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            
            coordinator.complete_work(&agent_id).await
                .map_err(|e| anyhow!("Failed to complete work in state machine: {}", e))?;
            println!("✅");
        } else {
            println!("⚙️  [DRY RUN] Would complete work in state machine for agent {}", agent_id);
        }
        
        // Step 5: Remove agent label to free the agent
        if !self.dry_run {
            print!("🤖 Freeing agent by removing {} label... ", agent_id);
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            
            self.remove_agent_label(&client, issue_number, &agent_id).await?;
            
            // Reset agent to idle state after completing the workflow
            coordinator.abandon_work(&agent_id).await
                .map_err(|e| anyhow!("Failed to reset agent to idle state: {}", e))?;
            
            println!("✅");
        } else {
            println!("🤖 [DRY RUN] Would remove {} label from issue #{}", agent_id, issue_number);
            println!("🤖 [DRY RUN] Would reset agent {} to idle state", agent_id);
        }
        
        println!();
        println!("✅ Bottle complete:");
        println!("   🌿 Branch {} is ready for bundling", current_branch);
        println!("   🏷️  Issue #{} label transition: route:ready → route:review", issue_number);
        println!("   🤖 Agent {} is now free for new work", agent_id);
        println!();
        println!("🎯 Next steps:");
        println!("   → Use 'my-little-soda pop' to get your next task");
        println!("   → Branch will be bundled into PR during next bundle cycle");
        
        // Check if we're at departure time and trigger bundling if needed
        if TrainSchedule::is_departure_time() {
            println!();
            println!("🚄 Departure time detected - triggering automatic bundling...");
            
            if let Err(e) = self.trigger_bundling().await {
                eprintln!("⚠️  Automatic bundling failed: {}", e);
                eprintln!("   Bundling will be retried on next departure window");
                eprintln!("   Work is still properly landed and ready for bundling");
            }
        }
        
        Ok(())
    }
    
    /// Get the current git branch name
    fn get_current_branch(&self) -> Result<String> {
        let repo = Repository::open(".")?;
        let head = repo.head()?;
        if let Some(branch_name) = head.shorthand() {
            Ok(branch_name.to_string())
        } else {
            Err(anyhow!("Could not determine current branch name"))
        }
    }
    
    /// Parse agent ID and issue number from branch name (e.g., "agent001/159" or "agent001/159-description" -> ("agent001", 159))
    fn parse_agent_branch(&self, branch_name: &str) -> Result<(String, u64)> {
        let parts: Vec<&str> = branch_name.split('/').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Branch '{}' is not an agent branch. Expected format: agent001/123 or agent001/123-description", branch_name));
        }
        
        let agent_id = parts[0];
        let issue_part = parts[1];
        
        // Extract issue number - handle both formats: "123" and "123-description"
        let issue_number = if let Some(dash_pos) = issue_part.find('-') {
            // New format: "123-description" -> extract "123"
            issue_part[..dash_pos].parse::<u64>()
                .map_err(|_| anyhow!("Invalid issue number in branch '{}'. Expected format: agent001/123 or agent001/123-description", branch_name))?
        } else {
            // Legacy format: "123" -> parse directly
            issue_part.parse::<u64>()
                .map_err(|_| anyhow!("Invalid issue number in branch '{}'. Expected format: agent001/123 or agent001/123-description", branch_name))?
        };
            
        if !agent_id.starts_with("agent") {
            return Err(anyhow!("Branch '{}' is not an agent branch. Expected format: agent001/123 or agent001/123-description", branch_name));
        }
        
        Ok((agent_id.to_string(), issue_number))
    }
    
    /// Push current branch to remote
    fn push_current_branch(&self, branch_name: &str) -> Result<()> {
        let output = Command::new("git")
            .args(&["push", "-u", "origin", branch_name])
            .output()?;
            
        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to push branch: {}", error_msg));
        }
        
        Ok(())
    }
    
    /// Remove agent label from issue using GitHub CLI
    async fn remove_agent_label(&self, client: &GitHubClient, issue_number: u64, agent_id: &str) -> Result<()> {
        let repo = format!("{}/{}", client.owner(), client.repo());
        let output = Command::new("gh")
            .args(&["issue", "edit", &issue_number.to_string(), "-R", &repo, "--remove-label", agent_id])
            .output()?;
        
        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to remove agent label: {}", error_msg));
        }
        
        Ok(())
    }
    
    /// Validate that the branch is ready to land
    fn validate_ready_to_land(&self, _branch_name: &str) -> Result<()> {
        // Check for uncommitted changes
        let status_output = Command::new("git")
            .args(&["status", "--porcelain"])
            .output()?;
        
        if !status_output.stdout.is_empty() {
            let uncommitted_files = String::from_utf8_lossy(&status_output.stdout);
            return Err(anyhow!(
                "⚠️  You have uncommitted changes. Please commit your work first:\n\n{}\nCommands to fix:\n   git add .\n   git commit -m \"Your commit message\"",
                uncommitted_files.trim()
            ));
        }
        
        // Check for commits ahead of main
        let commits_output = Command::new("git")
            .args(&["rev-list", "--count", "main..HEAD"])
            .output()?;
            
        let commits_ahead: u32 = String::from_utf8_lossy(&commits_output.stdout)
            .trim()
            .parse()
            .unwrap_or(0);
            
        if commits_ahead == 0 {
            return Err(anyhow!(
                "⚠️  No commits to land. Make sure you've committed your changes.\n\nCommands to fix:\n   git add .\n   git commit -m \"Your commit message\""
            ));
        }
        
        Ok(())
    }
    
    /// Trigger bundling of all queued branches
    async fn trigger_bundling(&self) -> Result<()> {
        print!("🔍 Scanning for completed agent work... ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        
        // Get all queued branches ready for bundling
        let queued_branches = TrainSchedule::get_queued_branches().await
            .map_err(|e| anyhow!("Failed to get queued branches: {}", e))?;
        
        if queued_branches.is_empty() {
            println!("none found");
            println!("   💡 No completed work ready for bundling at this time");
            return Ok(());
        }
        
        println!("found {}", queued_branches.len());
        println!();
        println!("🚂 AUTOMATIC TRAIN DEPARTURE - Bundling completed work");
        println!("🔍 Found {} branches with completed work:", queued_branches.len());
        for branch in &queued_branches {
            println!("  • {} - {}", branch.branch_name, branch.description);
        }
        
        // Process bundling using existing logic
        self.process_bundling_interactively(queued_branches).await
    }
    
    /// Process bundling for queued branches (adapted from pop.rs)
    async fn process_bundling_interactively(&self, queued_branches: Vec<QueuedBranch>) -> Result<()> {
        use crate::bundling::BundleManager;
        
        println!();
        println!("🚀 Starting automatic bundling process...");
        println!("═══════════════════════════════════════");
        
        // Initialize bundle manager
        let mut bundle_manager = BundleManager::new()
            .map_err(|e| anyhow!("Failed to initialize bundle manager: {}", e))?;
        
        // Create bundle using the existing bundling system
        println!("🚄 Creating bundle PR...");
        let result = bundle_manager.create_bundle(&queued_branches).await
            .map_err(|e| anyhow!("Bundle creation failed: {}", e))?;
        
        match result {
            crate::bundling::BundleResult::Success { pr_number, bundle_branch } => {
                println!("✅ Bundle PR created successfully!");
                println!("   📋 PR: #{}", pr_number);
                println!("   🌿 Branch: {}", bundle_branch);
                println!("   📦 Bundled {} branches", queued_branches.len());
            }
            crate::bundling::BundleResult::ConflictFallback { individual_prs } => {
                println!("⚠️  Conflicts detected - created individual PRs:");
                for (branch, pr) in individual_prs {
                    println!("   • {} → PR #{}", branch, pr);
                }
            }
            crate::bundling::BundleResult::Failed { error } => {
                return Err(anyhow!("Bundle creation failed: {}", error));
            }
        }
        
        println!();
        println!("🎯 Automatic bundling complete - system ready for next cycle");
        
        Ok(())
    }
}