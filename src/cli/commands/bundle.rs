use anyhow::Result;
use crate::train_schedule::TrainSchedule;
use crate::bundling::BundleManager;

pub struct BundleCommand {
    pub force: bool,
    pub dry_run: bool,
    pub verbose: bool,
}

impl BundleCommand {
    pub fn new(force: bool, dry_run: bool, verbose: bool) -> Self {
        Self {
            force,
            dry_run,
            verbose,
        }
    }

    pub async fn execute(&self) -> Result<()> {
        if self.dry_run {
            println!("🚄 CLAMBAKE BUNDLE - Create PR from queued branches (DRY RUN)");
        } else {
            println!("🚄 CLAMBAKE BUNDLE - Create PR from queued branches");
        }
        println!("==========================================");
        println!();

        // Check if we're at a departure time (unless forced)
        if !self.force && !TrainSchedule::is_departure_time() {
            let schedule = TrainSchedule::calculate_next_schedule();
            println!("⏰ Not at departure time yet.");
            println!("{}", schedule.format_schedule_display(&[]));
            println!();
            println!("💡 Use --force to bundle outside schedule, or wait for departure time");
            return Ok(());
        }

        // Get queued branches
        print!("🔍 Scanning for queued branches... ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        
        let queued_branches = TrainSchedule::get_queued_branches().await
            .map_err(|e| anyhow::anyhow!("Failed to get queued branches: {}", e))?;
        println!("found {}", queued_branches.len());

        if queued_branches.is_empty() {
            println!("📦 No branches ready for bundling");
            return Ok(());
        }

        if self.verbose {
            println!("\n📦 Queued branches:");
            for branch in &queued_branches {
                println!("   • {} (Issue #{}: {})", branch.branch_name, branch.issue_number, branch.description);
            }
            println!();
        }

        // Initialize bundle manager
        let mut bundle_manager = BundleManager::new()?;

        // Perform bundling
        if self.dry_run {
            println!("🔧 DRY RUN: Would create bundle PR with {} branches", queued_branches.len());
            let bundle_branch = bundle_manager.generate_bundle_branch_name(&queued_branches);
            println!("   Bundle branch: {}", bundle_branch);
            println!("   Issues: {}", queued_branches.iter()
                .map(|b| format!("#{}", b.issue_number))
                .collect::<Vec<_>>()
                .join(", "));
        } else {
            println!("🚄 Creating bundle PR...");
            let result = bundle_manager.create_bundle(&queued_branches).await?;
            
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
                    println!("❌ Bundle creation failed: {}", error);
                    return Err(error.into());
                }
            }
        }

        Ok(())
    }
}