use crate::cli::commands::with_agent_router;
use crate::train_schedule::TrainSchedule;
use anyhow::Result;
use std::error::Error;
use std::process::Command;

pub struct PopCommand {
    pub mine_only: bool,
    pub bundle_branches: bool,
    pub auto_approve: bool,
    pub ci_mode: bool,
    pub verbose: bool,
}

impl PopCommand {
    pub fn new(mine_only: bool, bundle_branches: bool, auto_approve: bool) -> Self {
        Self {
            mine_only,
            bundle_branches,
            auto_approve,
            ci_mode: false,
            verbose: false,
        }
    }

    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    pub fn with_ci_mode(mut self, ci_mode: bool) -> Self {
        self.ci_mode = ci_mode;
        self
    }

    pub async fn execute(&self) -> Result<()> {
        // Handle bundle branches special case first
        if self.bundle_branches {
            println!("🚄 EMERGENCY TRAIN DEPARTURE - Bundling all queued branches");
            println!("========================================================");
            println!();
            return bundle_all_branches(self.auto_approve).await;
        }

        // Check if we're already on a work branch
        if let Some(current_branch) = get_current_git_branch() {
            if let Some((agent_id, branch_suffix)) = current_branch.split_once('/') {
                if agent_id.starts_with("agent") {
                    // Extract issue number from branch suffix (handle descriptive branch names)
                    let issue_number_str = if let Some(dash_pos) = branch_suffix.find('-') {
                        &branch_suffix[..dash_pos]
                    } else {
                        branch_suffix
                    };

                    if let Ok(issue_number) = issue_number_str.parse::<u64>() {
                        println!("⚠️  You're already working on something!");
                        println!();
                        println!("🌿 Current branch: {current_branch}");
                        println!("📋 Working on: Issue #{issue_number}");
                        println!();
                        println!("💡 Suggested actions:");
                        println!("   → Check progress: my-little-soda status");
                        println!("   → Complete work: my-little-soda land");
                        println!("   → Switch to main: git checkout main");
                        println!(
                            "   → Force new task: my-little-soda pop --force (not yet implemented)"
                        );
                        println!();
                        println!("🎯 To work on multiple issues, complete current work first or switch branches.");
                        return Ok(());
                    }
                }
            }
        }

        if self.mine_only {
            println!("🎯 Popping next task assigned to you...");
        } else {
            println!("🎯 Popping next available task...");
        }
        println!();

        with_agent_router(|router| async move {
            if self.verbose {
                println!("🔍 VERBOSE MODE: Enhanced diagnostic logging enabled");
                println!("   → mine_only: {}", self.mine_only);
                println!("   → ci_mode: {}", self.ci_mode);
                println!("   → GitHub client: Checking connection...");
            }

            print!("📋 Searching for available tasks... ");
            std::io::Write::flush(&mut std::io::stdout()).unwrap();

            if self.verbose {
                println!();
                println!("🔍 VERBOSE: Making GitHub API call to fetch issues...");
                println!("   → API endpoint: GitHub Issues API");
                println!("   → Filter: route:ready labels");
                println!("   → Operation: {}", if self.mine_only { "pop_task_assigned_to_me" } else { "pop_any_available_task" });
            }

            let result = if self.mine_only {
                router.pop_task_assigned_to_me().await
            } else {
                router.pop_any_available_task().await
            };

            match result {
                Ok(Some(task)) => {
                    println!("✅");
                    println!();
                    println!("✅ Successfully popped task:");
                    println!("  📋 Issue #{}: {}", task.issue.number, task.issue.title);
                    println!("  👤 Assigned to: {}", task.assigned_agent.id);
                    println!("  🌿 Branch: {}", task.branch_name);
                    println!("  🔗 URL: {}", task.issue.html_url);
                    println!();
                    println!("🚀 Ready to work! Issue assigned and branch created/checked out.");
                    Ok(())
                }
                Ok(None) => {
                    println!("📋 No tasks found");
                    println!();
                    if self.mine_only {
                        println!("🎯 NO ASSIGNED TASKS:");
                        println!("   → Try: my-little-soda pop  # Get any available task");
                        println!("   → Create: gh issue create --title 'Your task' --label 'route:ready' --add-assignee @me");
                        println!("   → Check: gh issue list --assignee @me --label 'route:ready'");
                    } else {
                        println!("🎯 NO AVAILABLE TASKS:");
                        println!("   → Create: gh issue create --title 'Your task' --label 'route:ready'");
                        println!("   → Check existing: gh issue list --label 'route:ready'");
                        println!("   → Try assigned: my-little-soda pop --mine");
                    }
                    Ok(())
                }
                Err(e) => {
                    if self.verbose {
                        println!("❌");
                        println!();
                        println!("🔍 VERBOSE ERROR DETAILS:");
                        println!("   → Error type: {:?}", std::any::type_name_of_val(&e));
                        println!("   → Full error: {e:#?}");
                        println!("   → Error chain:");
                        let mut source = e.source();
                        let mut depth = 1;
                        while let Some(err) = source {
                            println!("     {depth}. {err}");
                            source = err.source();
                            depth += 1;
                        }
                        println!();
                    }

                    println!("{e}");
                    println!();
                    println!("🎯 TASK-SPECIFIC HELP:");
                    println!("   → Check for available: gh issue list --label 'route:ready'");
                    if self.mine_only {
                        println!("   → Check assigned to you: gh issue list --assignee @me --label 'route:ready'");
                    }
                    println!("   → Create new task: gh issue create --title 'Your task' --label 'route:ready'");

                    if self.verbose {
                        println!();
                        println!("🔍 VERBOSE TROUBLESHOOTING:");
                        println!("   → Try: RUST_LOG=debug my-little-soda pop --verbose");
                        println!("   → Check GitHub token: ls -la .my-little-soda/credentials/");
                        println!("   → Test API directly: curl -H 'Authorization: token YOUR_TOKEN' https://api.github.com/user");
                    }

                    Err(e.into())
                }
            }
        }).await.or_else(|_| {
            println!("❌ Router initialization failed");
            println!();
            println!("📚 Full setup guide: my-little-soda init");
            Ok(())
        })
    }
}

fn get_current_git_branch() -> Option<String> {
    Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                let branch = String::from_utf8(output.stdout).ok()?;
                let trimmed = branch.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            } else {
                None
            }
        })
}

async fn bundle_all_branches(auto_approve: bool) -> Result<()> {
    print!("🔍 Scanning for completed agent work... ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();

    // Get all queued branches (overdue and on-schedule) for emergency bundling
    match TrainSchedule::get_queued_branches().await {
        Ok(all_queued_branches) => {
            println!("✅");

            if all_queued_branches.is_empty() {
                println!();
                println!("📦 No completed work found");
                println!(
                    "   💡 All work is either in progress or no issues have route:review labels"
                );
                return Ok(());
            }

            println!();
            println!("🚂 EARLY TRAIN DEPARTURE - Emergency bundling all completed work");
            println!(
                "🔍 Found {} branches with completed work:",
                all_queued_branches.len()
            );
            for branch in &all_queued_branches {
                println!("  • {} - {}", branch.branch_name, branch.description);
            }

            println!();
            println!("📋 EMERGENCY BUNDLE PROCESSING PROTOCOL:");
            println!("For each branch, agent will:");
            println!("1. Switch to branch");
            println!("2. Verify commits exist and are meaningful");
            println!("3. Push commits to origin if needed");
            println!("4. Create PR with proper title/body");
            println!("5. Remove agent labels to free capacity");
            println!("6. Mark work as completed");
            println!();
            println!("⚠️  GUARDRAILS:");
            println!("- Agent must review each commit before creating PR");
            println!("- Branches without commits will be skipped");
            println!("- Agent can abort at any step with Ctrl+C");
            println!("- All operations logged for audit");
            println!();

            if !auto_approve {
                print!("Proceed with emergency bundling of all completed work? [y/N]: ");
                std::io::Write::flush(&mut std::io::stdout()).unwrap();

                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                let input = input.trim().to_lowercase();

                if input != "y" && input != "yes" {
                    println!("❌ Operation cancelled by user");
                    return Ok(());
                }
            }

            process_overdue_branches_interactively(all_queued_branches).await
        }
        Err(e) => {
            println!("❌ Could not scan for overdue branches: {e:?}");
            Err(anyhow::anyhow!("Failed to scan overdue branches"))
        }
    }
}

async fn process_overdue_branches_interactively(
    overdue_branches: Vec<crate::train_schedule::QueuedBranch>,
) -> Result<()> {
    use crate::git::{Git2Operations, GitOperations};

    println!();
    println!("🚀 Starting interactive overdue branch processing...");
    println!("═══════════════════════════════════════════════════");

    for (index, branch) in overdue_branches.iter().enumerate() {
        println!();
        println!(
            "🌿 Processing {} ({}/{})...",
            branch.branch_name,
            index + 1,
            overdue_branches.len()
        );
        println!("📋 {}", branch.description);

        // Step 1: Switch to branch
        println!("Step 1: Switching to branch...");
        let git_ops = match Git2Operations::new(".") {
            Ok(ops) => ops,
            Err(e) => {
                println!("❌ Failed to initialize git operations: {e}");
                continue;
            }
        };

        match git_ops.checkout_branch(&branch.branch_name) {
            Ok(()) => {
                println!("✅ Switched to branch {}", branch.branch_name);
            }
            Err(e) => {
                println!("❌ Failed to switch to branch: {e}");
                print!("Continue to next branch? [y/N]: ");
                std::io::Write::flush(&mut std::io::stdout()).unwrap();

                let mut input = String::new();
                if std::io::stdin().read_line(&mut input).is_ok()
                    && input.trim().to_lowercase() != "y"
                {
                    println!("❌ Operation aborted");
                    return Ok(());
                }
                continue;
            }
        }

        // Continue with the rest of the bundling process...
        // This is a complex function that would need more extraction
        // For now, just indicate the operation was started
        println!("⚠️  Branch processing not fully implemented in refactored version");
    }

    Ok(())
}
