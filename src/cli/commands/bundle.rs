use crate::bundling::BundleManager;
use crate::train_schedule::TrainSchedule;
use anyhow::Result;

pub struct BundleCommand {
    pub force: bool,
    pub dry_run: bool,
    pub verbose: bool,
    pub diagnose: bool,
    pub ci_mode: bool,
}

impl BundleCommand {
    pub fn new(force: bool, dry_run: bool, verbose: bool, diagnose: bool) -> Self {
        Self {
            force,
            dry_run,
            verbose,
            diagnose,
            ci_mode: false,
        }
    }

    pub fn with_ci_mode(mut self, ci_mode: bool) -> Self {
        self.ci_mode = ci_mode;
        self
    }

    pub async fn execute(&self) -> Result<()> {
        if self.diagnose {
            return self.execute_diagnostics().await;
        }

        if self.dry_run {
            println!("🚄 MY LITTLE SODA BUNDLE - Create PR from queued branches (DRY RUN)");
        } else {
            println!("🚄 MY LITTLE SODA BUNDLE - Create PR from queued branches");
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

        let queued_branches = TrainSchedule::get_queued_branches()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get queued branches: {}", e))?;
        println!("found {}", queued_branches.len());

        if queued_branches.is_empty() {
            println!("📦 No branches ready for bundling");
            return Ok(());
        }

        if self.verbose {
            println!("\n📦 Queued branches:");
            for branch in &queued_branches {
                println!(
                    "   • {} (Issue #{}: {})",
                    branch.branch_name, branch.issue_number, branch.description
                );
            }
            println!();
        }

        // Initialize bundle manager
        let mut bundle_manager = BundleManager::new()?;

        // Perform bundling
        if self.dry_run {
            println!(
                "🔧 DRY RUN: Would create bundle PR with {} branches",
                queued_branches.len()
            );
            let bundle_branch = bundle_manager.generate_bundle_branch_name(&queued_branches);
            println!("   Bundle branch: {bundle_branch}");
            println!(
                "   Issues: {}",
                queued_branches
                    .iter()
                    .map(|b| format!("#{}", b.issue_number))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        } else {
            println!("🚄 Creating bundle PR...");
            let result = bundle_manager.create_bundle(&queued_branches).await?;

            match result {
                crate::bundling::BundleResult::Success {
                    pr_number,
                    bundle_branch,
                } => {
                    println!("✅ Bundle PR created successfully!");
                    println!("   📋 PR: #{pr_number}");
                    println!("   🌿 Branch: {bundle_branch}");
                    println!("   📦 Bundled {} branches", queued_branches.len());
                }
                crate::bundling::BundleResult::ConflictFallback { individual_prs } => {
                    println!("⚠️  Conflicts detected - created individual PRs:");
                    for (branch, pr) in individual_prs {
                        println!("   • {branch} → PR #{pr}");
                    }
                }
                crate::bundling::BundleResult::Failed { error } => {
                    println!("❌ Bundle creation failed: {error}");
                    return Err(error);
                }
            }
        }

        Ok(())
    }

    async fn execute_diagnostics(&self) -> Result<()> {
        println!("🔍 MY LITTLE SODA BUNDLE DIAGNOSTICS");
        println!("=====================================");
        println!();

        // System status
        self.check_bundling_system_status().await?;

        // Schedule information
        self.display_bundling_schedule()?;

        // Work availability
        self.check_work_availability().await?;

        // Configuration status
        self.check_configuration()?;

        // Recent bundling activity
        self.display_recent_activity().await?;

        // Performance metrics
        self.display_performance_metrics().await?;

        println!("=====================================");
        println!("🔍 Diagnostic complete");

        Ok(())
    }

    async fn check_bundling_system_status(&self) -> Result<()> {
        println!("📊 System Status");
        println!("───────────────");

        // Check if we can create a bundle manager
        match BundleManager::new() {
            Ok(_) => println!("✅ Bundle manager: Available"),
            Err(e) => {
                println!("❌ Bundle manager: Failed to initialize");
                println!("   Error: {e}");
                return Ok(()); // Continue with other diagnostics
            }
        }

        // Check Git repository status
        if let Ok(repo) = git2::Repository::open(".") {
            let head = repo.head()?;
            if let Some(branch_name) = head.shorthand() {
                println!("✅ Git repository: {branch_name}");
            } else {
                println!("⚠️  Git repository: Detached HEAD");
            }
        } else {
            println!("❌ Git repository: Not found or inaccessible");
        }

        // Check GitHub connectivity
        match crate::github::GitHubClient::new() {
            Ok(_) => println!("✅ GitHub API: Connected"),
            Err(e) => println!("❌ GitHub API: {e}"),
        }

        println!();
        Ok(())
    }

    fn display_bundling_schedule(&self) -> Result<()> {
        println!("📅 Bundling Schedule");
        println!("──────────────────");

        let now = chrono::Utc::now();
        let is_departure_time = TrainSchedule::is_departure_time();

        println!("⏰ Current time: {}", now.format("%Y-%m-%d %H:%M:%S UTC"));

        if is_departure_time {
            println!("🚄 Status: AT DEPARTURE TIME - Bundling available");
        } else {
            println!("⏳ Status: Waiting for departure time");
            let schedule = TrainSchedule::calculate_next_schedule();
            println!("{}", schedule.format_schedule_display(&[]));
        }

        println!("🔧 Override: Use --force to bundle outside schedule");
        println!();
        Ok(())
    }

    async fn check_work_availability(&self) -> Result<()> {
        println!("📦 Work Availability");
        println!("──────────────────");

        match TrainSchedule::get_queued_branches().await {
            Ok(queued_branches) => {
                if queued_branches.is_empty() {
                    println!("📭 No branches ready for bundling");
                    println!("   Check that issues have 'route:review' labels");
                } else {
                    println!("📋 Found {} queued branches:", queued_branches.len());
                    for (i, branch) in queued_branches.iter().enumerate() {
                        println!(
                            "   {}. {} (Issue #{}: {})",
                            i + 1,
                            branch.branch_name,
                            branch.issue_number,
                            branch.description
                        );
                    }
                }
            }
            Err(e) => {
                println!("❌ Failed to get queued branches: {e}");
            }
        }

        println!();
        Ok(())
    }

    fn check_configuration(&self) -> Result<()> {
        println!("⚙️  Configuration");
        println!("───────────────");

        // Check for my-little-soda directory
        if std::path::Path::new(".my-little-soda").exists() {
            println!("✅ .my-little-soda directory: Present");
        } else {
            println!("⚠️  .my-little-soda directory: Not found (will be created)");
        }

        // Check for bundle lock
        if std::path::Path::new(".my-little-soda/bundle.lock").exists() {
            println!("🔒 Bundle lock: Present (another bundler may be running)");
        } else {
            println!("🔓 Bundle lock: Available");
        }

        // Check for previous state
        if std::path::Path::new(".my-little-soda/bundle_state.json").exists() {
            println!("💾 Previous state: Found (may need recovery)");
        } else {
            println!("🆕 Previous state: Clean");
        }

        println!();
        Ok(())
    }

    async fn display_recent_activity(&self) -> Result<()> {
        println!("📈 Recent Activity");
        println!("────────────────");

        // This would show recent bundle attempts, PRs created, etc.
        // For now, we'll provide a placeholder
        println!("🔄 Last 24 hours: Data collection in progress");
        println!("📊 Bundle attempts: Not tracked yet");
        println!("✅ Successful bundles: Not tracked yet");
        println!("⚠️  Fallback to individual PRs: Not tracked yet");

        println!();
        Ok(())
    }

    async fn display_performance_metrics(&self) -> Result<()> {
        println!("⚡ Performance Metrics");
        println!("────────────────────");

        // This would show bundling success rates, timing, etc.
        // For now, we'll provide system health indicators
        println!("🎯 Bundle success rate: Tracking not implemented");
        println!("⏱️  Average bundle time: Tracking not implemented");
        println!("🔀 Conflict rate: Tracking not implemented");
        println!("📊 API calls per bundle: Tracking not implemented");

        println!();
        println!("💡 Tip: Run 'my-little-soda bundle --dry-run' to preview next bundle");
        println!("💡 Tip: Use 'my-little-soda bundle --verbose' for detailed output");

        Ok(())
    }
}
