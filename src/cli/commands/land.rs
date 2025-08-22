use anyhow::{Result, anyhow};
use crate::github::GitHubClient;
use git2::Repository;
use std::process::Command;

pub struct LandCommand {
    pub include_closed: bool,
    pub days: u32,
    pub dry_run: bool,
    pub verbose: bool,
}

impl LandCommand {
    pub fn new(include_closed: bool, days: u32, dry_run: bool, verbose: bool) -> Self {
        Self {
            include_closed,
            days,
            dry_run,
            verbose,
        }
    }

    pub async fn execute(&self) -> Result<()> {
        if self.dry_run {
            println!("🚀 CLAMBAKE LAND - Mark Work Ready for Review (DRY RUN)");
        } else {
            println!("🚀 CLAMBAKE LAND - Mark Work Ready for Review");
        }
        println!("==========================================");
        println!();
        
        // Get current branch and parse agent/issue info
        let current_branch = self.get_current_branch()?;
        let (agent_id, issue_number) = self.parse_agent_branch(&current_branch)?;
        
        if self.verbose {
            println!("🔧 Configuration:");
            println!("   🌿 Current branch: {}", current_branch);
            println!("   🤖 Agent ID: {}", agent_id);
            println!("   📋 Issue number: {}", issue_number);
            println!("   🔍 Dry run mode: {}", if self.dry_run { "Yes" } else { "No" });
            println!();
        }
        
        // Initialize GitHub client
        let client = GitHubClient::new()
            .map_err(|e| anyhow!("Failed to initialize GitHub client: {}", e))?;
        
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
        
        // Step 2: Add route:review label to mark as ready for bundling
        if !self.dry_run {
            print!("🏷️  Adding route:review label to issue #{}... ", issue_number);
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            
            client.add_label_to_issue(issue_number, "route:review").await
                .map_err(|e| anyhow!("Failed to add route:review label: {}", e))?;
            println!("✅");
        } else {
            println!("🏷️  [DRY RUN] Would add route:review label to issue #{}", issue_number);
        }
        
        // Step 3: Remove agent label to free the agent
        if !self.dry_run {
            print!("🤖 Freeing agent by removing {} label... ", agent_id);
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            
            self.remove_agent_label(&client, issue_number, &agent_id).await?;
            println!("✅");
        } else {
            println!("🤖 [DRY RUN] Would remove {} label from issue #{}", agent_id, issue_number);
        }
        
        println!();
        println!("✅ Land complete:");
        println!("   🌿 Branch {} is ready for bundling", current_branch);
        println!("   🏷️  Issue #{} marked with route:review", issue_number);
        println!("   🤖 Agent {} is now free for new work", agent_id);
        println!();
        println!("🎯 Next steps:");
        println!("   → Use 'clambake pop' to get your next task");
        println!("   → Branch will be bundled into PR during next bundle cycle");
        
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
    
    /// Parse agent ID and issue number from branch name (e.g., "agent001/159" -> ("agent001", 159))
    fn parse_agent_branch(&self, branch_name: &str) -> Result<(String, u64)> {
        let parts: Vec<&str> = branch_name.split('/').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Branch '{}' is not an agent branch. Expected format: agent001/123", branch_name));
        }
        
        let agent_id = parts[0];
        let issue_number = parts[1].parse::<u64>()
            .map_err(|_| anyhow!("Invalid issue number in branch '{}'. Expected format: agent001/123", branch_name))?;
            
        if !agent_id.starts_with("agent") {
            return Err(anyhow!("Branch '{}' is not an agent branch. Expected format: agent001/123", branch_name));
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
}