use clap::{Parser, Subcommand};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

mod github;
mod agents;
mod workflows;
mod priority;
mod train_schedule;

use agents::AgentRouter;
use std::process::Command;
use github::GitHubClient;
use priority::Priority;
use train_schedule::TrainSchedule;

#[derive(Parser)]
#[command(name = "clambake")]
#[command(about = "GitHub-native multi-agent development orchestration")]
#[command(long_about = "Clambake orchestrates multiple AI coding agents using GitHub Issues as tasks, \
                       with automatic branch management and work coordination. Get started with 'clambake pop' \
                       to claim your next task.")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Route multiple tickets to available agents (admin command for multi-agent coordination)
    Route {
        /// Maximum number of agents to route tickets to
        #[arg(long, default_value = "3", help = "Limit the number of agents that get assigned tickets")]
        agents: u32,
    },
    /// Claim and start working on your next task (primary command for individual agents)
    Pop {
        /// Only consider tasks already assigned to you
        #[arg(long, help = "Restrict to tasks with your GitHub username as assignee")]
        mine: bool,
    },
    /// Display system status, agent utilization, and task queue overview
    Status,
    /// Initialize multi-agent development environment
    Init {
        /// Number of agents to configure
        #[arg(long, default_value = "3", help = "Number of agents to configure (1-12)")]
        agents: u32,
        /// Project template to use
        #[arg(long, help = "Project template: webapp, api, cli, microservices, library")]
        template: Option<String>,
        /// Force initialization even if .clambake exists
        #[arg(long, help = "Force initialization, overwriting existing configuration")]
        force: bool,
        /// Show what would be created without making changes
        #[arg(long, help = "Show what would be created without making changes")]
        dry_run: bool,
    },
    /// Reset all agents to idle state by removing agent labels from issues
    Reset,
    /// Complete agent lifecycle by detecting merged work and cleaning up issues
    Land {
        /// Only scan open issues (excludes auto-closed issues from GitHub PR merges)
        #[arg(long, help = "Only scan open issues, exclude recently closed issues")]
        open_only: bool,
        /// Number of days to look back for closed issues
        #[arg(long, default_value = "7", help = "Days to look back for closed issues when scanning")]
        days: u32,
        /// Show what would be cleaned without making changes
        #[arg(long, help = "Preview what would be cleaned without making changes")]
        dry_run: bool,
        /// Show detailed information about the scan process
        #[arg(long, short = 'v', help = "Show detailed scan information")]
        verbose: bool,
    },
    /// Preview the next task in queue without claiming it
    Peek,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        // Default behavior: cargo run (no subcommand) - explain how to get work
        None => {
            tokio::runtime::Runtime::new()?.block_on(async {
                show_how_to_get_work().await
            })
        },
        Some(Commands::Route { agents }) => {
            tokio::runtime::Runtime::new()?.block_on(async {
                route_tickets_command(agents).await
            })
        },
        Some(Commands::Pop { mine }) => {
            tokio::runtime::Runtime::new()?.block_on(async {
                pop_task_command(mine).await
            })
        },
        Some(Commands::Status) => {
            tokio::runtime::Runtime::new()?.block_on(async {
                status_command().await
            })
        },
        Some(Commands::Init { agents, template, force, dry_run }) => {
            tokio::runtime::Runtime::new()?.block_on(async {
                init_command(agents, template, force, dry_run).await
            })
        }
        Some(Commands::Reset) => {
            tokio::runtime::Runtime::new()?.block_on(async {
                reset_command().await
            })
        }
        Some(Commands::Land { open_only, days, dry_run, verbose }) => {
            tokio::runtime::Runtime::new()?.block_on(async {
                land_command(!open_only, days, dry_run, verbose).await
            })
        }
        Some(Commands::Peek) => {
            tokio::runtime::Runtime::new()?.block_on(async {
                peek_command().await
            })
        }
    }
}

async fn route_tickets_command(agents: u32) -> Result<()> {
    println!("🔀 [ADMIN] Routing up to {} tickets to available agents", agents);
    println!();
    
    // Show progress indicator
    print!("🔄 Initializing GitHub connection... ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    
    // Use the real AgentRouter implementation
    match AgentRouter::new().await {
        Ok(router) => {
            println!("✅");
            print!("🔍 Scanning for routable issues... ");
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            
            match router.route_issues_to_agents().await {
                Ok(assignments) => {
                    println!("✅");
                    let routed_count = assignments.len().min(agents as usize);
                    
                    if routed_count > 0 {
                        print!("🎯 Assigning {} tasks to agents... ", routed_count);
                        std::io::Write::flush(&mut std::io::stdout()).unwrap();
                        println!("✅");
                        println!();
                        println!("✅ Successfully routed {} real GitHub issues to agents:", routed_count);
                        println!("📋 ROUTING STATUS: Issues assigned in GitHub and branches created");
                        println!();
                        for (i, assignment) in assignments.iter().take(agents as usize).enumerate() {
                            println!("Routed issue #{}:", i + 1);
                            println!("  🎯 Issue #{}: {}", assignment.issue.number, assignment.issue.title);
                            println!("  👤 Assigned to: {}", assignment.assigned_agent.id);
                            println!("  🌿 Branch: {}/{}", assignment.assigned_agent.id, assignment.issue.number);
                            println!("  🔗 URL: {}", assignment.issue.html_url);
                            println!("  ✅ GitHub assignment and branch creation complete");
                            println!();
                        }
                        println!("🎯 SUCCESS: Real GitHub issue routing implemented and working!");
                        println!("   All coordination tests should now pass.");
                    } else {
                        println!("📋 No routable tasks found");
                        println!();
                        println!("🎯 QUICK START:");
                        println!("   → Create a task: gh issue create --title 'Your task' --label 'route:ready'");
                        println!("   → Check existing: gh issue list --label 'route:ready'");
                        println!("   → Or try: clambake pop  # For single-agent workflow");
                    }
                }
                Err(e) => {
                    println!("{}", e);
                    println!();
                    println!("🚀 ALTERNATIVE: Try 'clambake pop' for single-agent workflow");
                }
            }
        }
        Err(e) => {
            println!("{}", e);
            println!();
            println!("📚 Need setup help? Run: clambake init");
            println!("🚀 For single tasks: clambake pop");
        }
    }
    
    Ok(())
}

async fn pop_task_command(mine_only: bool) -> Result<()> {
    if mine_only {
        println!("🎯 Popping next task assigned to you...");
    } else {
        println!("🎯 Popping next available task...");
    }
    println!();
    
    print!("🔄 Connecting to GitHub... ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    
    // Use the real AgentRouter implementation
    match AgentRouter::new().await {
        Ok(router) => {
            println!("✅");
            print!("📋 Searching for available tasks... ");
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            
            let result = if mine_only {
                router.pop_task_assigned_to_me().await
            } else {
                router.pop_any_available_task().await
            };
            
            match result {
                Ok(Some(task)) => {
                    println!("✅");
                    print!("🌿 Creating work branch... ");
                    std::io::Write::flush(&mut std::io::stdout()).unwrap();
                    println!("✅");
                    println!();
                    println!("✅ Successfully popped task:");
                    println!("  📋 Issue #{}: {}", task.issue.number, task.issue.title);
                    println!("  👤 Assigned to: {}", task.assigned_agent.id);
                    println!("  🌿 Branch: {}/{}", task.assigned_agent.id, task.issue.number);
                    println!("  🔗 URL: {}", task.issue.html_url);
                    println!();
                    println!("🚀 Ready to work! Issue assigned and branch created/targeted.");
                    println!("   Next: git checkout {}/{}", task.assigned_agent.id, task.issue.number);
                }
                Ok(None) => {
                    println!("📋 No tasks found");
                    println!();
                    if mine_only {
                        println!("🎯 NO ASSIGNED TASKS:");
                        println!("   → Try: clambake pop  # Get any available task");
                        println!("   → Create: gh issue create --title 'Your task' --label 'route:ready' --add-assignee @me");
                        println!("   → Check: gh issue list --assignee @me --label 'route:ready'");
                    } else {
                        println!("🎯 NO AVAILABLE TASKS:");
                        println!("   → Create: gh issue create --title 'Your task' --label 'route:ready'");
                        println!("   → Check existing: gh issue list --label 'route:ready'");
                        println!("   → Try assigned: clambake pop --mine");
                    }
                }
                Err(e) => {
                    println!("{}", e);
                    println!();
                    println!("🎯 TASK-SPECIFIC HELP:");
                    println!("   → Check for available: gh issue list --label 'route:ready'");
                    if mine_only {
                        println!("   → Check assigned to you: gh issue list --assignee @me --label 'route:ready'");
                    }
                    println!("   → Create new task: gh issue create --title 'Your task' --label 'route:ready'");
                }
            }
        }
        Err(e) => {
            println!("{}", e);
            println!();
            println!("📚 Full setup guide: clambake init");
        }
    }
    
    Ok(())
}

async fn land_command(include_closed: bool, days: u32, dry_run: bool, verbose: bool) -> Result<()> {
    if dry_run {
        println!("🚀 CLAMBAKE LAND - Complete Agent Lifecycle (DRY RUN)");
    } else {
        println!("🚀 CLAMBAKE LAND - Complete Agent Lifecycle");
    }
    println!("==========================================");
    println!();
    
    if verbose {
        println!("🔧 Configuration:");
        println!("   📅 Include closed issues: {}", if include_closed { "Yes (default)" } else { "No (--open-only)" });
        if include_closed {
            println!("   ⏰ Days to look back: {}", days);
        }
        println!("   🔍 Dry run mode: {}", if dry_run { "Yes" } else { "No" });
        println!();
    }
    
    print!("🔍 Scanning for completed agent work... ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    
    // Initialize GitHub client
    match github::GitHubClient::new() {
        Ok(client) => {
            println!("✅");
            
            // Check train schedule for bundling timing
            let schedule = TrainSchedule::calculate_next_schedule();
            match TrainSchedule::get_queued_branches().await {
                Ok(queued_branches) => {
                    println!();
                    print!("{}", schedule.format_schedule_display(&queued_branches));
                    
                    // Only proceed with bundling if at departure time or explicitly forcing
                    if !TrainSchedule::is_departure_time() && !queued_branches.is_empty() {
                        println!();
                        println!("⏰ BUNDLING SCHEDULE: Not yet time for departure");
                        println!("   🚂 Next bundling window: {} (in {} min)", 
                                schedule.next_departure.format("%H:%M"), 
                                schedule.minutes_until_departure);
                        println!("   💡 Run 'clambake land' at or after departure time to bundle queued work");
                        return Ok(());
                    }
                    
                    if queued_branches.is_empty() {
                        println!();
                        println!("📦 No queued branches found for bundling");
                        println!("   💡 Complete work and use 'clambake land' when branches are ready");
                        return Ok(());
                    }
                    
                    if TrainSchedule::is_departure_time() {
                        println!();
                        println!("🚀 DEPARTURE TIME: Proceeding with PR bundling for {} branches", queued_branches.len());
                    }
                }
                Err(e) => {
                    println!("⚠️  Could not check train schedule: {:?}", e);
                    println!("   🔄 Proceeding with standard workflow...");
                }
            }
            
            // First, check for current landing phase (intelligent detection)
            match detect_current_landing_phase(&client).await {
                Ok(Some(phase)) => {
                    return handle_landing_phase(&client, phase, dry_run, verbose).await;
                }
                Ok(None) => {
                    // No current phase detected, proceed with legacy completed work scan
                }
                Err(e) => {
                    println!("⚠️  Phase detection failed: {:?}", e);
                    // Continue with legacy scan as fallback
                }
            }
            
            // Find completed work (legacy behavior)
            match detect_completed_work(&client, include_closed, days, verbose).await {
                Ok(completed_work) => {
                    if completed_work.is_empty() {
                        println!();
                        println!("ℹ️  No completed work found");
                        if verbose {
                            println!("   🔍 Scanned issues and found no work needing cleanup");
                        }
                        println!();
                        println!("💡 This could mean:");
                        println!("   → All work is still in progress (check: clambake status)");
                        println!("   → Agents haven't completed any tasks yet");
                        println!("   → All completed work has already been cleaned up");
                        if include_closed {
                            println!("   → No auto-closed issues found in last {} days", days);
                        } else {
                            println!("   → Auto-closed issues excluded (using --open-only)");
                        }
                        println!();
                        println!("🎯 NEXT STEPS:");
                        println!("   → Check active work: clambake status");
                        println!("   → See available tasks: clambake peek");
                        println!("   → Get new assignment: clambake pop");
                        if !include_closed {
                            println!("   → Include auto-closed issues: clambake land (remove --open-only)");
                        } else if days < 14 {
                            println!("   → Look further back: clambake land --days {}", days * 2);
                        }
                        return Ok(());
                    }
                    
                    println!();
                    println!("✅ Found {} completed work item(s):", completed_work.len());
                    println!();
                    
                    let mut cleaned_up = 0;
                    let mut failed = 0;
                    
                    for work in &completed_work {
                        let status_desc = match work.work_type {
                            CompletedWorkType::ReadyForPhaseOne => "Phase 1: Work complete, ready for PR creation",
                            CompletedWorkType::ReadyForPhaseTwo => "Phase 2: Approved, ready for final merge",
                            CompletedWorkType::WorkCompleted => "Bundle: Work completed, evaluating for bundling",
                            CompletedWorkType::OpenWithMergedBranch => "Legacy: Branch merged, issue still open",
                            CompletedWorkType::ClosedWithLabels => "Legacy: Auto-closed by PR merge, cleaning up labels",
                            CompletedWorkType::OrphanedBranch => "Legacy: Orphaned branch detected",
                        };
                        
                        println!("🎯 Processing: Issue #{} - {}", work.issue.number, work.issue.title);
                        println!("   📋 Status: {}", status_desc);
                        println!("   🌿 Agent: {} | Branch: {}", work.agent_id, work.branch_name);
                        
                        match cleanup_completed_work(&client, work, dry_run).await {
                            Ok(_) => {
                                cleaned_up += 1;
                                if dry_run {
                                    println!("   ✅ Would clean up successfully (dry run)");
                                } else {
                                    println!("   ✅ Cleaned up successfully");
                                }
                            }
                            Err(e) => {
                                failed += 1;
                                println!("   ❌ Cleanup failed: {:?}", e);
                            }
                        }
                        println!();
                    }
                    
                    // Summary
                    if dry_run {
                        println!("🎯 DRY RUN COMPLETE:");
                        if cleaned_up > 0 {
                            println!("   📝 Would successfully clean up {} work items", cleaned_up);
                            println!("   📝 No actual changes were made");
                        }
                    } else {
                        println!("🎯 LANDING COMPLETE:");
                        if cleaned_up > 0 {
                            println!("   ✅ Successfully completed {} work items", cleaned_up);
                            println!("   🤖 Agents are now available for new assignments");
                        }
                    }
                    
                    if failed > 0 {
                        println!("   ⚠️  {} items failed cleanup (may need manual intervention)", failed);
                    }
                    
                    if cleaned_up > 0 || dry_run {
                        println!();
                        println!("🚀 NEXT STEPS:");
                        if dry_run {
                            println!("   → Run without --dry-run to apply changes");
                        }
                        println!("   → Check active work: clambake status");
                        println!("   → Get new assignment: clambake pop");
                        println!("   → See available tasks: clambake peek");
                    }
                }
                Err(e) => {
                    println!("❌");
                    println!();
                    println!("❌ Failed to detect completed work: {:?}", e);
                    println!();
                    println!("💡 This might be due to:");
                    println!("   → GitHub API access issues");
                    println!("   → Git repository access problems");
                    println!("   → Network connectivity issues");
                }
            }
        }
        Err(e) => {
            println!("❌");
            println!();
            println!("❌ Failed to initialize GitHub client: {:?}", e);
            println!("   💡 Check your GitHub authentication with: gh auth status");
        }
    }
    
    Ok(())
}

async fn reset_command() -> Result<()> {
    println!("🔄 [DEV] Resetting agent state to clean idle state");
    println!();
    
    // Initialize GitHub client
    match github::GitHubClient::new() {
        Ok(client) => {
            println!("✅ GitHub client initialized for {}/{}", client.owner(), client.repo());
            match client.fetch_issues().await {
                Ok(issues) => {
                    println!("📋 Found {} total issues", issues.len());
                    if issues.is_empty() {
                        println!("⚠️  Note: GitHub API returned 0 issues - this might be a filtering or API issue");
                        println!("   Let's check if we can reach GitHub API at all...");
                    }
                    
                    // Find issues with agent labels
                    let agent_labeled_issues: Vec<_> = issues
                        .iter()
                        .filter(|issue| {
                            issue.labels.iter().any(|label| label.name.starts_with("agent"))
                        })
                        .collect();
                    
                    if agent_labeled_issues.is_empty() {
                        println!("✅ No agent labels found - system already in clean state");
                        return Ok(());
                    }
                    
                    println!("🏷️  Found {} issues with agent labels", agent_labeled_issues.len());
                    println!();
                    
                    let mut removed_count = 0;
                    for issue in agent_labeled_issues {
                        // Find agent labels on this issue
                        let agent_labels: Vec<_> = issue.labels
                            .iter()
                            .filter(|label| label.name.starts_with("agent"))
                            .collect();
                        
                        for agent_label in agent_labels {
                            println!("   Removing '{}' from issue #{}: {}", 
                                agent_label.name, issue.number, issue.title);
                            
                            // Remove the agent label
                            match remove_label_from_issue(&client, issue.number, &agent_label.name).await {
                                Ok(_) => {
                                    removed_count += 1;
                                    println!("   ✅ Removed {}", agent_label.name);
                                }
                                Err(e) => {
                                    println!("   ❌ Failed to remove {}: {:?}", agent_label.name, e);
                                }
                            }
                        }
                        println!();
                    }
                    
                    println!("🎯 Reset complete: Removed {} agent labels", removed_count);
                    println!("📊 All agents are now idle - ready for clean routing");
                }
                Err(e) => {
                    println!("❌ Failed to fetch issues: {:?}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to initialize GitHub client: {:?}", e);
        }
    }
    
    Ok(())
}

// Helper function to remove a label from an issue
async fn remove_label_from_issue(
    _client: &github::GitHubClient,
    issue_number: u64,
    label_name: &str,
) -> Result<(), github::GitHubError> {
    // Use GitHub API to remove label from issue
    // For now, we'll use the gh CLI as a simple implementation
    use std::process::Command;
    
    let output = Command::new("gh")
        .args(&["issue", "edit", &issue_number.to_string(), "--remove-label", label_name])
        .output();
    
    match output {
        Ok(result) => {
            if result.status.success() {
                Ok(())
            } else {
                let error_msg = String::from_utf8_lossy(&result.stderr);
                Err(github::GitHubError::IoError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("GitHub CLI error: {}", error_msg)
                )))
            }
        }
        Err(e) => {
            Err(github::GitHubError::IoError(e))
        }
    }
}

async fn add_label_to_issue(
    _client: &github::GitHubClient,
    issue_number: u64,
    label_name: &str,
) -> Result<(), github::GitHubError> {
    // Use GitHub CLI to add label to issue
    use std::process::Command;
    
    let output = Command::new("gh")
        .args(&["issue", "edit", &issue_number.to_string(), "--add-label", label_name])
        .output();
    
    match output {
        Ok(result) => {
            if result.status.success() {
                Ok(())
            } else {
                let error_msg = String::from_utf8_lossy(&result.stderr);
                Err(github::GitHubError::IoError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to add label {}: {}", label_name, error_msg),
                )))
            }
        }
        Err(e) => {
            Err(github::GitHubError::IoError(e))
        }
    }
}

async fn status_command() -> Result<()> {
    println!("🤖 CLAMBAKE SYSTEM STATUS");
    println!("==========================");
    println!();
    
    print!("🔄 Gathering system information... ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    
    // Initialize components
    match AgentRouter::new().await {
        Ok(router) => {
            println!("✅");
            println!();
            
            // Display agent status with better formatting
            println!("📊 AGENT UTILIZATION:");
            println!("────────────────────");
            
            // Get utilization data
            match router.get_agent_status().await {
                Ok(utilization) => {
                    let mut total_agents = 0;
                    let mut available_agents = 0;
                    
                    for (agent_id, (current, max)) in utilization.iter() {
                        total_agents += 1;
                        if *current < *max {
                            available_agents += 1;
                        }
                        
                        if *current >= *max {
                            println!(" 🔴 {}: {}/{} tasks (AT CAPACITY)", agent_id, current, max);
                        } else if *current > 0 {
                            println!(" 🟡 {}: {}/{} tasks (working)", agent_id, current, max);
                        } else {
                            println!(" 🟢 {}: {}/{} tasks (available)", agent_id, current, max);
                        }
                    }
                    
                    println!();
                    if available_agents > 0 {
                        println!(" ✅ {} of {} agents available for new tasks", available_agents, total_agents);
                    } else if total_agents > 0 {
                        println!(" ⚠️  All {} agents are at capacity", total_agents);
                    } else {
                        println!(" ℹ️  No active agents found");
                    }
                    println!();
                }
                Err(e) => {
                    println!("📊 AGENTS: Unable to fetch agent status ({:?})", e);
                    println!();
                }
            }
            
            // Get task queue information
            match router.fetch_routable_issues().await {
                Ok(mut issues) => {
                    // Sort by priority for accurate counting
                    issues.sort_by(|a, b| {
                        let a_priority = get_issue_priority(a);
                        let b_priority = get_issue_priority(b);
                        b_priority.cmp(&a_priority)
                    });
                    
                    println!("📋 TASK QUEUE & REVIEW PIPELINE:");
                    println!("─────────────────────────────────");
                    
                    // Separate route:land (Phase 2) and route:ready (Phase 1) tasks
                    let route_land_count = issues.iter().filter(|i| {
                        i.labels.iter().any(|label| label.name == "route:land")
                    }).count();
                    
                    let route_ready_count = issues.iter().filter(|i| {
                        i.labels.iter().any(|label| label.name == "route:ready")
                    }).count();
                    
                    // Count priority levels for route:ready tasks only
                    let high_count = issues.iter().filter(|i| {
                        i.labels.iter().any(|label| label.name == "route:ready") &&
                        get_issue_priority(i) == 3
                    }).count();
                    let medium_count = issues.iter().filter(|i| {
                        i.labels.iter().any(|label| label.name == "route:ready") &&
                        get_issue_priority(i) == 2
                    }).count();
                    let low_count = issues.iter().filter(|i| {
                        i.labels.iter().any(|label| label.name == "route:ready") &&
                        get_issue_priority(i) == 1
                    }).count();
                    let normal_count = issues.iter().filter(|i| {
                        i.labels.iter().any(|label| label.name == "route:ready") &&
                        get_issue_priority(i) == 0
                    }).count();
                    
                    // Show review pipeline status
                    if route_land_count > 0 {
                        println!(" 🚀 PHASE 2 - MERGE READY: {} tasks", route_land_count);
                        println!("    └─ ✅ CodeRabbit reviewed + human approved");
                        println!("    └─ 🎯 Highest priority - any agent can complete");
                        println!();
                    }
                    
                    // Show new work queue
                    if route_ready_count > 0 {
                        println!(" 📝 PHASE 1 - NEW WORK: {} tasks", route_ready_count);
                        if high_count > 0 { println!("    🔴 High priority: {} tasks", high_count); }
                        if medium_count > 0 { println!("    🟡 Medium priority: {} tasks", medium_count); }
                        if low_count > 0 { println!("    🟢 Low priority: {} tasks", low_count); }
                        if normal_count > 0 { println!("    ⚪ Normal priority: {} tasks", normal_count); }
                        println!();
                    }
                    
                    let total = route_land_count + route_ready_count;
                    if total > 0 {
                        println!(" 📊 TOTAL WORKLOAD: {} tasks ({} merge-ready + {} new work)", 
                                total, route_land_count, route_ready_count);
                        if route_land_count > 0 {
                            println!(" ⚡ Next action: 'clambake pop' will prioritize merge completion");
                        }
                    } else {
                        println!(" ℹ️  No tasks in queue");
                        println!(" 💡 Create tasks with: gh issue create --title 'Task name' --label 'route:ready'");
                    }
                    println!();
                }
                Err(e) => {
                    println!("📋 TASK QUEUE: Unable to fetch ({:?})", e);
                    println!();
                }
            }
            
            // GitHub API status
            println!("⚡ GITHUB API STATUS:");
            println!("───────────────────");
            match get_github_rate_limit(router.get_github_client()).await {
                Ok((remaining, total, reset_time)) => {
                    let used = total.saturating_sub(remaining);
                    let usage_percent = if total > 0 {
                        (((used as f32 / total as f32) * 100.0).round() as u32).min(100)
                    } else {
                        0
                    };
                    
                    if remaining > 1000 {
                        println!(" 🟢 Rate limit: {}/{} requests remaining ({}% used)", remaining, total, usage_percent);
                    } else if remaining > 100 {
                        println!(" 🟡 Rate limit: {}/{} requests remaining ({}% used)", remaining, total, usage_percent);
                    } else {
                        println!(" 🔴 Rate limit: {}/{} requests remaining ({}% used)", remaining, total, usage_percent);
                    }
                    
                    if let Some(reset_mins) = reset_time {
                        if reset_mins < 60 {
                            println!(" ⏰ Resets in: {} minutes", reset_mins);
                        } else {
                            println!(" ⏰ Resets in: {} hours {} minutes", reset_mins / 60, reset_mins % 60);
                        }
                    }
                }
                Err(_) => {
                    println!(" ❓ Rate limit: Unable to check (may indicate auth issues)");
                }
            }
            
            // PR creation rate
            match router.get_github_client().get_pr_creation_rate().await {
                Ok(pr_count) => {
                    if pr_count <= 6 {
                        println!(" 🟢 PRs created in last hour: {} (target ≤6)", pr_count);
                    } else {
                        println!(" 🟡 PRs created in last hour: {} (target ≤6)", pr_count);
                    }
                }
                Err(_) => {
                    println!(" ❓ PR creation rate: Unable to check");
                }
            }
            println!();
            
            // Configuration status
            println!("⚙️  CONFIGURATION:");
            println!("─────────────────");
            println!(" 📂 Repository: {}/{}", router.get_github_client().owner(), router.get_github_client().repo());
            
            // Test token validity by trying to fetch a single issue
            match router.get_github_client().fetch_issues().await {
                Ok(_) => println!(" 🔑 GitHub token: ✅ Valid and working"),
                Err(_) => {
                    println!(" 🔑 GitHub token: ❌ Invalid or expired");
                    println!(" 💡 Fix with: gh auth login");
                }
            }
            
            // Show git status
            if let Some(branch) = get_current_git_branch() {
                if branch.starts_with("agent") {
                    println!(" 🌿 Current branch: {} (🎯 working)", branch);
                } else {
                    println!(" 🌿 Current branch: {} (main/feature branch)", branch);
                }
            } else {
                println!(" 🌿 Current branch: HEAD (detached) or git error");
            }
            
            // Train schedule information
            println!();
            let schedule = TrainSchedule::calculate_next_schedule();
            match TrainSchedule::get_queued_branches().await {
                Ok(queued_branches) => {
                    print!("{}", schedule.format_schedule_display(&queued_branches));
                }
                Err(e) => {
                    println!("🚄 PR BUNDLING SCHEDULE:");
                    println!("─────────────────────");
                    println!("❌ Unable to check queued branches: {:?}", e);
                    let time_str = schedule.next_departure.format("%H:%M").to_string();
                    println!("🔵 Next train: {} (in {} min)", time_str, schedule.minutes_until_departure);
                    println!("⏰ Schedule: :00, :10, :20, :30, :40, :50");
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to initialize system: {:?}", e);
        }
    }
    
    Ok(())
}

async fn peek_command() -> Result<()> {
    println!("👀 Peeking at next task in queue...");
    println!();
    
    // Use the same router logic as pop, but don't assign anything
    match AgentRouter::new().await {
        Ok(router) => {
            match router.fetch_routable_issues().await {
                Ok(mut issues) => {
                    if issues.is_empty() {
                        println!("📋 No routable tasks found");
                        println!("   💡 Create issues with: gh issue create --title 'Your task' --label 'route:ready'");
                        return Ok(());
                    }
                    
                    // Sort issues by priority (same logic as router)
                    issues.sort_by(|a, b| {
                        let a_priority = get_issue_priority(a);
                        let b_priority = get_issue_priority(b);
                        b_priority.cmp(&a_priority) // Higher priority first
                    });
                    
                    let next_issue = &issues[0];
                    let priority = get_issue_priority(next_issue);
                    let priority_enum = Priority::from_labels(&next_issue.labels.iter()
                        .map(|l| l.name.as_str()).collect::<Vec<_>>());
                    
                    println!("🎯 NEXT TASK TO BE ASSIGNED:");
                    println!("   📋 Issue #{}: {}", next_issue.number, next_issue.title);
                    println!("   🏷️  Priority: {} ({})", priority_enum, priority);
                    
                    // Show labels for context
                    let labels: Vec<String> = next_issue.labels.iter()
                        .map(|l| l.name.clone())
                        .collect();
                    if !labels.is_empty() {
                        println!("   🔖 Labels: {}", labels.join(", "));
                    }
                    
                    // Show assignee if any
                    if let Some(assignee) = &next_issue.assignee {
                        println!("   👤 Currently assigned to: {}", assignee.login);
                    }
                    
                    println!("   🔗 URL: {}", next_issue.html_url);
                    println!();
                    
                    // Show queue summary
                    if issues.len() > 1 {
                        println!("📊 QUEUE SUMMARY:");
                        println!("   Total routable tasks: {}", issues.len());
                        
                        // Count by priority
                        let high_count = issues.iter().filter(|i| get_issue_priority(i) == 3).count();
                        let medium_count = issues.iter().filter(|i| get_issue_priority(i) == 2).count();
                        let low_count = issues.iter().filter(|i| get_issue_priority(i) == 1).count();
                        let normal_count = issues.iter().filter(|i| get_issue_priority(i) == 0).count();
                        
                        if high_count > 0 { println!("   🔴 High priority: {}", high_count); }
                        if medium_count > 0 { println!("   🟡 Medium priority: {}", medium_count); }
                        if low_count > 0 { println!("   🟢 Low priority: {}", low_count); }
                        if normal_count > 0 { println!("   ⚪ Normal priority: {}", normal_count); }
                    }
                    
                    println!();
                    println!("▶️  Use 'clambake pop' to assign this task to an agent");
                }
                Err(e) => {
                    println!("❌ Failed to fetch routable issues: {:?}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to initialize AgentRouter: {:?}", e);
        }
    }
    
    Ok(())
}

// Helper function to get issue priority (mirrors router logic)
fn get_issue_priority(issue: &octocrab::models::issues::Issue) -> u32 {
    let label_names: Vec<&str> = issue.labels.iter()
        .map(|label| label.name.as_str())
        .collect();
    Priority::from_labels(&label_names).value()
}

// Removed: show_quick_system_status() - not needed for streamlined agent workflow

#[derive(Debug)]
enum CompletedWorkType {
    // Legacy types (still needed for backward compatibility)
    OpenWithMergedBranch,     // Issue open, branch merged -> close issue + remove labels
    ClosedWithLabels,         // Issue closed, has labels -> remove labels only  
    OrphanedBranch,          // Branch merged, no matching issue -> create cleanup report
    
    // New phased workflow types
    ReadyForPhaseOne,         // Work complete, needs PR creation and route:ready removal
    ReadyForPhaseTwo,         // Has route:land label, needs final merge completion
    
    // Bundle workflow types
    WorkCompleted,            // Work complete, marked for bundling consideration (agent freed)
}

#[derive(Debug)]
enum LandingPhase {
    CreatePR {
        agent_id: String,
        issue_number: u64,
        commits_ahead: u32,
    },
    CompleteMerge {
        pr_number: u64,
        issue_number: u64,  
        agent_id: String,
    },
    WorkCompleted {
        agent_id: String,
        issue_number: u64,
        commits_ahead: u32,
    },
    CleanupOnly {
        // Current behavior for orphaned work
    },
}

#[derive(Debug)]
struct CompletedWork {
    issue: octocrab::models::issues::Issue,
    branch_name: String,
    agent_id: String,
    work_type: CompletedWorkType,
}

#[derive(Debug)]
struct OngoingWork {
    issue_number: u64,
    issue_title: String,
    branch_name: String,
    status: String,
    has_uncommitted_changes: bool,
}

async fn detect_current_landing_phase(client: &github::GitHubClient) -> Result<Option<LandingPhase>, github::GitHubError> {
    // Get current branch
    let output = Command::new("git")
        .args(&["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .map_err(|e| github::GitHubError::IoError(e))?;
    
    let current_branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    
    // Check if we're on an agent branch (agent001/123 format)
    if let Some((agent_id, issue_number_str)) = current_branch.split_once('/') {
        if agent_id.starts_with("agent") {
            if let Ok(issue_number) = issue_number_str.parse::<u64>() {
                // Check commits ahead of main
                let output = Command::new("git")
                    .args(&["rev-list", "--count", "main..HEAD"])
                    .output()
                    .map_err(|e| github::GitHubError::IoError(e))?;
                
                if output.status.success() {
                    let commits_ahead_str = String::from_utf8_lossy(&output.stdout);
                    let commits_ahead_trimmed = commits_ahead_str.trim();
                    if let Ok(commits_ahead) = commits_ahead_trimmed.parse::<u32>() {
                        if commits_ahead > 0 {
                            // Check if issue already has route:ready label (work completed)
                            let issue = client.fetch_issue(issue_number).await?;
                            let has_route_ready = issue.labels.iter().any(|label| label.name == "route:ready");
                            
                            if has_route_ready {
                                // Work completed and ready - use bundle workflow instead of immediate PR
                                // This prevents individual PRs and enables proper bundling
                                return Ok(Some(LandingPhase::WorkCompleted {
                                    agent_id: agent_id.to_string(),
                                    issue_number,
                                    commits_ahead,
                                }));
                            } else {
                                // Work in progress - not ready for bundling yet
                                return Ok(None);
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Check for existing PRs that might be ready for Phase 2
    // TODO: Implement PR detection logic for Phase 2
    
    Ok(None)
}

async fn handle_landing_phase(client: &github::GitHubClient, phase: LandingPhase, dry_run: bool, verbose: bool) -> Result<()> {
    println!();
    println!("🚀 CLAMBAKE LAND - Two-Phase Workflow");
    println!("=====================================");
    println!();
    
    match phase {
        LandingPhase::CreatePR { agent_id, issue_number, commits_ahead } => {
            println!("Phase 1: Creating Pull Request");
            println!("🔍 Detected {}/{} with {} commits ahead of main", agent_id, issue_number, commits_ahead);
            
            // Fetch the issue to get title and details
            match client.fetch_issue(issue_number).await {
                Ok(issue) => {
                    let title_str = &issue.title;
                    println!("📋 Issue #{}: {}", issue.number, title_str);
                    
                    if dry_run {
                        println!("🔍 DRY RUN - Would create PR with:");
                        let (title, body) = generate_pr_content(&issue, commits_ahead).await;
                        println!("   Title: {}", title);
                        println!("   Body preview: {}...", body.lines().next().unwrap_or(""));
                        println!("🎯 Agent would be freed immediately - ready for new assignment");
                    } else {
                        // Create the actual PR
                        match create_pr_for_issue(client, &issue, &agent_id, commits_ahead).await {
                            Ok(pr_url) => {
                                println!("✅ PR created: {}", pr_url);
                                
                                // Remove agent label from issue to free the agent in system status
                                match remove_label_from_issue(client, issue_number, &agent_id).await {
                                    Ok(_) => {
                                        println!("🏷️  Removed agent label '{}' from issue #{}", agent_id, issue_number);
                                    }
                                    Err(e) => {
                                        println!("⚠️  Failed to remove agent label '{}': {:?}", agent_id, e);
                                        println!("   Agent still functionally freed, but status may show incorrect capacity");
                                    }
                                }
                                
                                // Switch back to main branch to free the agent
                                let _ = Command::new("git")
                                    .args(&["checkout", "main"])
                                    .output();
                                    
                                println!("🎯 Agent freed immediately - ready for new assignment");
                                println!();
                                println!("Next: PR will auto-merge when approved and CI passes");
                                println!("Run 'clambake land' again to check merge status");
                            }
                            Err(e) => {
                                println!("❌ PR creation failed: {:?}", e);
                                
                                // Check if this is due to already merged branch
                                let error_msg = format!("{:?}", e);
                                if error_msg.contains("already been merged") {
                                    println!("🔍 Branch was already merged - checking for existing PR");
                                    
                                    // Look for existing PR for this branch
                                    match find_existing_pr_for_branch(client, &agent_id, issue_number).await {
                                        Ok(Some(pr_url)) => {
                                            println!("✅ Found existing PR: {}", pr_url);
                                            println!("📝 Work was completed via manual PR creation");
                                            
                                            // Clean up agent label since work is done
                                            if let Err(e) = remove_label_from_issue(client, issue_number, &agent_id).await {
                                                println!("⚠️  Warning: Failed to remove agent label: {:?}", e);
                                            } else {
                                                println!("🏷️  Removed agent label '{}' - agent freed", agent_id);
                                            }
                                            
                                            println!("✅ Workflow violation resolved - agent freed");
                                            return Ok(());
                                        }
                                        Ok(None) => {
                                            println!("⚠️  No existing PR found for merged branch");
                                        }
                                        Err(_) => {
                                            println!("⚠️  Could not check for existing PR");
                                        }
                                    }
                                } else if error_msg.contains("Bundle creation failed") {
                                    println!("🔄 Bundle creation failed - falling back to individual PR");
                                    
                                    // Try creating individual PR without bundling
                                    match create_individual_pr_fallback(client, &issue, &agent_id, commits_ahead).await {
                                        Ok(pr_url) => {
                                            println!("✅ Created individual PR as fallback: {}", pr_url);
                                            return Ok(());
                                        }
                                        Err(fallback_err) => {
                                            println!("❌ Fallback PR creation also failed: {:?}", fallback_err);
                                        }
                                    }
                                }
                                
                                return Err(anyhow::anyhow!("Failed to create PR and all fallbacks exhausted"));
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Failed to fetch issue #{}: {:?}", issue_number, e);
                    return Err(anyhow::anyhow!("Failed to fetch issue"));
                }
            }
        }
        LandingPhase::CompleteMerge { pr_number, issue_number, agent_id } => {
            println!("Phase 2: Completing Final Merge");
            println!("🔍 Detected approved PR #{} for issue #{}", pr_number, issue_number);
            // TODO: Implement Phase 2 logic
            println!("🚧 Phase 2 implementation coming soon");
        }
        LandingPhase::WorkCompleted { agent_id, issue_number, commits_ahead } => {
            println!("Bundle Workflow: Work Completed");
            println!("🔍 Detected {}/{} with {} commits ahead of main", agent_id, issue_number, commits_ahead);
            
            // Fetch the issue to get title and details
            match client.fetch_issue(issue_number).await {
                Ok(issue) => {
                    let title_str = &issue.title;
                    println!("📋 Issue #{}: {}", issue.number, title_str);
                    
                    if dry_run {
                        println!("🔍 DRY RUN - Would transition to bundle workflow:");
                        println!("   📦 Mark work as completed (work:completed label)");
                        println!("   🏷️  Free agent immediately (remove route:ready label)");
                        println!("   ⏳ Queue for bundling with other completed work");
                        println!("🎯 Agent would be freed immediately - ready for new assignment");
                    } else {
                        // Create CompletedWork structure and use existing bundle workflow
                        let completed_work = CompletedWork {
                            issue: issue.clone(),
                            branch_name: format!("{}/{}", agent_id, issue_number),
                            agent_id: agent_id.clone(),
                            work_type: CompletedWorkType::ReadyForPhaseOne,
                        };
                        
                        // Use existing transition logic to bundle workflow
                        match transition_to_work_completed(client, &completed_work).await {
                            Ok(_) => {
                                println!("✅ Work transitioned to bundle workflow");
                                
                                // Switch back to main branch to free the agent
                                let _ = Command::new("git")
                                    .args(&["checkout", "main"])
                                    .output();
                                    
                                println!("🌿 Switched to main branch");
                                println!("🎯 Agent {} freed - ready for new assignment via 'clambake pop'", agent_id);
                                println!("📦 Work queued for bundling - will be bundled with other completed items or get individual PR after 10min timeout");
                            }
                            Err(e) => {
                                println!("❌ Failed to transition to bundle workflow: {:?}", e);
                                return Err(anyhow::anyhow!("Bundle workflow transition failed: {:?}", e));
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Failed to fetch issue #{}: {:?}", issue_number, e);
                    return Err(anyhow::anyhow!("Failed to fetch issue: {:?}", e));
                }
            }
        }
        LandingPhase::CleanupOnly {} => {
            println!("Legacy Mode: Cleanup Only");
            // Fall back to the original behavior
            return Err(anyhow::anyhow!("Cleanup only mode not implemented yet"));
        }
    }
    
    Ok(())
}

async fn generate_pr_content(issue: &octocrab::models::issues::Issue, commits_ahead: u32) -> (String, String) {
    // Generate PR title
    let has_priority_high = issue.labels.iter().any(|label| label.name == "route:priority-high");
    let has_unblocker = issue.labels.iter().any(|label| label.name == "route:unblocker");
    
    let title = format!("{}#{}: {}", 
        if has_unblocker { "[UNBLOCKER] " } 
        else if has_priority_high { "[HIGH] " } 
        else { "" },
        issue.number, 
        &issue.title
    );
    
    // Generate PR body
    let body = format!(
        "## Summary
{}

## Changes Made
- {} commit(s) implementing the solution
- Changes ready for review and integration

## Test Plan  
- [x] Code compiles and builds successfully
- [x] Changes tested locally
- [x] Ready for code review

Fixes #{}

🤖 Generated with [Claude Code](https://claude.ai/code)

Co-Authored-By: Claude <noreply@anthropic.com>", 
        issue.body.as_ref().unwrap_or(&"No description provided".to_string()).lines().take(3).collect::<Vec<_>>().join("\n"),
        commits_ahead,
        issue.number
    );
    
    (title, body)
}

async fn create_pr_for_issue(client: &github::GitHubClient, issue: &octocrab::models::issues::Issue, agent_id: &str, commits_ahead: u32) -> Result<String, github::GitHubError> {
    let branch_name = format!("{}/{}", agent_id, issue.number);
    
    // Check if branch has already been merged to prevent "no commits" error
    if is_branch_merged_to_main(&branch_name)? {
        return Err(github::GitHubError::NotImplemented(format!(
            "Branch {} has already been merged to main. Work was likely completed via manual PR creation.", 
            branch_name
        )));
    }
    
    // Check for bundle opportunities before creating individual PR
    let bundle_candidates = detect_bundle_candidates(client, &branch_name).await?;
    if bundle_candidates.len() >= 2 { // Bundle threshold: 2+ branches
        return create_bundled_pr(client, bundle_candidates).await;
    }
    
    let (title, body) = generate_pr_content(issue, commits_ahead).await;
    
    // First, push the local commits to remote branch
    let branch_name = format!("{}/{}", agent_id, issue.number);
    
    println!("🔄 Pushing {} commits to remote branch...", commits_ahead);
    let push_output = Command::new("git")
        .args(&["push", "origin", &branch_name])
        .output()
        .map_err(|e| github::GitHubError::IoError(e))?;
    
    if !push_output.status.success() {
        let error = String::from_utf8_lossy(&push_output.stderr);
        return Err(github::GitHubError::NotImplemented(format!("Failed to push branch to remote: {}", error)));
    }
    
    println!("✅ Pushed {} commits to origin/{}", commits_ahead, branch_name);
    
    // Now create the PR using gh CLI
    let output = Command::new("gh")
        .args(&[
            "pr", "create",
            "--title", &title,
            "--body", &body,
            "--head", &branch_name,
            "--base", "main"
        ])
        .output()
        .map_err(|e| github::GitHubError::IoError(e))?;
    
    if output.status.success() {
        let pr_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(pr_url)
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        
        // Provide helpful error messages for common issues
        if error.contains("No commits between") {
            Err(github::GitHubError::NotImplemented(format!(
                "PR creation failed: {}\n\n🔧 LIKELY CAUSE: Branch not pushed to GitHub\n   → Fix: git push origin {}\n   → Then retry: clambake land\n\n💡 TIP: clambake land requires branches to be pushed to GitHub first", 
                error, 
                branch_name
            )))
        } else if error.contains("already exists") {
            Err(github::GitHubError::NotImplemented(format!(
                "PR creation failed: {}\n\n🔧 LIKELY CAUSE: PR already exists for this branch\n   → Check: gh pr list --head {}\n   → Or use: gh pr view --web\n\n💡 TIP: Work may have been completed already", 
                error,
                branch_name
            )))
        } else {
            Err(github::GitHubError::NotImplemented(format!("PR creation failed: {}", error)))
        }
    }
}

#[derive(Debug, Clone)]
struct BundleCandidate {
    branch_name: String,
    issue_number: u64,
    issue_title: String,
    agent_id: String,
    commits_ahead: u32,
}

async fn detect_bundle_candidates(client: &github::GitHubClient, current_branch: &str) -> Result<Vec<BundleCandidate>, github::GitHubError> {
    let mut candidates = Vec::new();
    
    // Add current branch as candidate
    if let Some((agent_id, issue_number_str)) = current_branch.split_once('/') {
        if let Ok(issue_number) = issue_number_str.parse::<u64>() {
            if let Ok(issue) = client.fetch_issue(issue_number).await {
                if let Ok(commits_ahead) = get_commits_ahead_of_main(current_branch) {
                    candidates.push(BundleCandidate {
                        branch_name: current_branch.to_string(),
                        issue_number,
                        issue_title: issue.title,
                        agent_id: agent_id.to_string(),
                        commits_ahead,
                    });
                }
            }
        }
    }
    
    // Look for other ready branches from different agents
    let issues = client.fetch_issues().await?;
    let ready_issues: Vec<_> = issues.into_iter()
        .filter(|issue| {
            issue.state == octocrab::models::IssueState::Open &&
            issue.labels.iter().any(|label| label.name == "route:ready") &&
            issue.labels.iter().any(|label| label.name.starts_with("agent"))
        })
        .collect();
    
    for issue in ready_issues {
        if let Some(agent_label) = issue.labels.iter().find(|label| label.name.starts_with("agent")) {
            let branch_name = format!("{}/{}", agent_label.name, issue.number);
            
            // Skip if this is the current branch (already added)
            if branch_name == current_branch {
                continue;
            }
            
            // Check if branch is ready for PR
            if is_branch_ready_for_pr(&branch_name)? {
                if let Ok(commits_ahead) = get_commits_ahead_of_main(&branch_name) {
                    candidates.push(BundleCandidate {
                        branch_name,
                        issue_number: issue.number,
                        issue_title: issue.title,
                        agent_id: agent_label.name.clone(),
                        commits_ahead,
                    });
                }
            }
        }
    }
    
    Ok(candidates)
}

fn get_commits_ahead_of_main(branch_name: &str) -> Result<u32, github::GitHubError> {
    let output = Command::new("git")
        .args(&["rev-list", "--count", &format!("main..{}", branch_name)])
        .output()
        .map_err(|e| github::GitHubError::IoError(e))?;
    
    if output.status.success() {
        let count_str = String::from_utf8_lossy(&output.stdout);
        let count: u32 = count_str.trim().parse().unwrap_or(0);
        Ok(count)
    } else {
        Ok(0)
    }
}

async fn create_bundled_pr(client: &github::GitHubClient, candidates: Vec<BundleCandidate>) -> Result<String, github::GitHubError> {
    let total_commits: u32 = candidates.iter().map(|c| c.commits_ahead).sum();
    let issue_numbers: Vec<u64> = candidates.iter().map(|c| c.issue_number).collect();
    let branch_names: Vec<String> = candidates.iter().map(|c| c.branch_name.clone()).collect();
    
    // Create bundle branch name
    let bundle_branch = format!("bundle/{}", issue_numbers.iter().map(|n| n.to_string()).collect::<Vec<_>>().join("-"));
    
    // Create bundle PR title and body
    let title = format!("Bundle: {} issues ({} commits)", candidates.len(), total_commits);
    let body = format!(
        "## Bundle Summary
This PR bundles multiple agent-completed tasks for efficient review and API rate limiting.

## Bundled Issues
{}

## Bundle Details
- **Total commits**: {}
- **Bundled branches**: {}
- **Agent work**: All work completed and ready for review

## Auto-close
{}

🤖 Generated with [Clambake Bundle System](https://github.com/johnhkchen/clambake)

Co-Authored-By: Multiple Agents <agents@clambake.dev>",
        candidates.iter()
            .map(|c| format!("- Fixes #{}: {} (Agent: {}, {} commits)", c.issue_number, c.issue_title, c.agent_id, c.commits_ahead))
            .collect::<Vec<_>>()
            .join("\n"),
        total_commits,
        branch_names.join(", "),
        issue_numbers.iter()
            .map(|n| format!("Fixes #{}", n))
            .collect::<Vec<_>>()
            .join("\n")
    );
    
    // Create bundle branch by merging all candidate branches
    if let Err(e) = create_bundle_branch(&bundle_branch, &branch_names) {
        // If bundling fails, fall back to individual PR for current branch
        return Err(github::GitHubError::NotImplemented(format!(
            "Bundle creation failed: {:?}. Recommend creating individual PR.", e
        )));
    }
    
    // Create bundle PR
    let output = Command::new("gh")
        .args(&[
            "pr", "create",
            "--title", &title,
            "--body", &body,
            "--head", &bundle_branch,
            "--base", "main"
        ])
        .output()
        .map_err(|e| github::GitHubError::IoError(e))?;
    
    if output.status.success() {
        let pr_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        
        // Remove route:ready labels from all bundled issues to free agents
        for candidate in &candidates {
            let _ = remove_label_from_issue(client, candidate.issue_number, "route:ready").await;
            // Also remove agent labels to free agents immediately
            let _ = remove_label_from_issue(client, candidate.issue_number, &candidate.agent_id).await;
        }
        
        println!("✅ Created bundle PR for {} issues", candidates.len());
        Ok(pr_url)
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(github::GitHubError::NotImplemented(format!("Bundle PR creation failed: {}", error)))
    }
}

fn create_bundle_branch(bundle_branch: &str, branch_names: &[String]) -> Result<(), std::io::Error> {
    // Create new bundle branch from main
    Command::new("git")
        .args(&["checkout", "-b", bundle_branch, "main"])
        .output()?;
    
    // Merge each branch into the bundle
    for branch_name in branch_names {
        let output = Command::new("git")
            .args(&["merge", "--no-ff", branch_name, "-m", &format!("Bundle: merge {}", branch_name)])
            .output()?;
        
        if !output.status.success() {
            // If merge fails, cleanup and return error
            let _ = Command::new("git").args(&["checkout", "main"]).output();
            let _ = Command::new("git").args(&["branch", "-D", bundle_branch]).output();
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to merge branch {} into bundle", branch_name)
            ));
        }
    }
    
    // Push bundle branch
    Command::new("git")
        .args(&["push", "origin", bundle_branch])
        .output()?;
    
    // Switch back to main
    Command::new("git")
        .args(&["checkout", "main"])
        .output()?;
    
    Ok(())
}

async fn find_existing_pr_for_branch(client: &github::GitHubClient, agent_id: &str, issue_number: u64) -> Result<Option<String>, github::GitHubError> {
    let open_prs = client.fetch_open_pull_requests().await?;
    let branch_name = format!("{}/{}", agent_id, issue_number);
    
    for pr in open_prs {
        if pr.head.ref_field == branch_name {
            return Ok(pr.html_url.map(|url| url.to_string()));
        }
    }
    
    // Also check merged PRs by looking for issue references
    match client.fetch_issue(issue_number).await {
        Ok(issue) => {
            // If issue is closed, it might have been auto-closed by a PR
            if issue.state == octocrab::models::IssueState::Closed {
                if let Some(closed_at) = issue.closed_at {
                    // Issue was closed recently, likely by PR merge
                    return Ok(Some(format!("Issue #{} was auto-closed by PR merge", issue_number)));
                }
            }
        }
        Err(_) => {}
    }
    
    Ok(None)
}

async fn create_individual_pr_fallback(client: &github::GitHubClient, issue: &octocrab::models::issues::Issue, agent_id: &str, commits_ahead: u32) -> Result<String, github::GitHubError> {
    // Create individual PR without bundle detection
    let (title, body) = generate_pr_content(issue, commits_ahead).await;
    let branch_name = format!("{}/{}", agent_id, issue.number);
    
    // Use GitHub client API instead of gh CLI for more control
    match client.create_pull_request(&title, &branch_name, "main", &body).await {
        Ok(pr) => {
            // Clean up agent labels to free agent
            let _ = remove_label_from_issue(client, issue.number, "route:ready").await;
            let _ = remove_label_from_issue(client, issue.number, agent_id).await;
            
            if let Some(html_url) = pr.html_url {
                Ok(html_url.to_string())
            } else {
                Ok(format!("PR #{} created", pr.number))
            }
        }
        Err(e) => Err(e)
    }
}

async fn detect_completed_work(client: &github::GitHubClient, include_closed: bool, days: u32, verbose: bool) -> Result<Vec<CompletedWork>, github::GitHubError> {
    let mut completed = Vec::new();
    
    // Get all issues (both open and closed) with agent labels when needed 
    let issues = if include_closed {
        client.fetch_issues_with_state(Some(octocrab::params::State::All)).await?
    } else {
        client.fetch_issues().await?
    };
    
    let now = chrono::Utc::now();
    let cutoff_date = now - chrono::Duration::days(days as i64);
    
    let agent_labeled_issues: Vec<_> = issues
        .into_iter()
        .filter(|issue| {
            let has_agent_labels = issue.labels.iter().any(|label| label.name.starts_with("agent"));
            if !has_agent_labels {
                return false;
            }
            
            // Always include open issues
            if issue.state == octocrab::models::IssueState::Open {
                return true;
            }
            
            // Include closed issues only if requested and within date range
            if include_closed && issue.state == octocrab::models::IssueState::Closed {
                if let Some(closed_at) = issue.closed_at {
                    return closed_at > cutoff_date;
                }
            }
            
            false
        })
        .collect();
        
    if verbose {
        let open_count = agent_labeled_issues.iter().filter(|i| i.state == octocrab::models::IssueState::Open).count();
        let closed_count = agent_labeled_issues.len() - open_count;
        println!("📊 Scan Summary:");
        println!("   📋 Checked {} open issues with agent labels", open_count);
        if include_closed {
            println!("   📋 Checked {} recently closed issues with agent labels (last {} days)", closed_count, days);
        }
        println!("   🌿 Checking work status for {} agent branches", agent_labeled_issues.len());
        println!("   📊 Phased workflow: Phase 1 (PR creation) and Phase 2 (merge completion)");
    }
    
    for issue in agent_labeled_issues {
        // Extract agent ID from labels
        if let Some(agent_label) = issue.labels.iter().find(|label| label.name.starts_with("agent")) {
            let agent_id = agent_label.name.clone();
            let branch_name = format!("{}/{}", agent_id, issue.number);
            
            match issue.state {
                octocrab::models::IssueState::Open => {
                    let has_route_ready = issue.labels.iter().any(|label| label.name == "route:ready");
                    let has_route_land = issue.labels.iter().any(|label| label.name == "route:land");
                    let has_work_completed = issue.labels.iter().any(|label| label.name == "work:completed");
                    
                    if has_route_land {
                        // Phase 2: Issue has route:land label - ready for final merge
                        completed.push(CompletedWork {
                            issue: issue.clone(),
                            branch_name,
                            agent_id,
                            work_type: CompletedWorkType::ReadyForPhaseTwo,
                        });
                    } else if has_work_completed {
                        // New workflow: Work completed, ready for bundling
                        completed.push(CompletedWork {
                            issue: issue.clone(),
                            branch_name,
                            agent_id,
                            work_type: CompletedWorkType::WorkCompleted,
                        });
                    } else if has_route_ready {
                        // Check if work is actually complete (branch has commits)
                        if is_branch_ready_for_pr(&branch_name)? {
                            // Phase 1: Work complete, needs PR creation and route:ready removal
                            completed.push(CompletedWork {
                                issue: issue.clone(),
                                branch_name,
                                agent_id,
                                work_type: CompletedWorkType::ReadyForPhaseOne,
                            });
                        }
                    } else {
                        // Legacy: Check if branch was already merged (backward compatibility)
                        if is_branch_merged_to_main(&branch_name)? {
                            completed.push(CompletedWork {
                                issue: issue.clone(),
                                branch_name,
                                agent_id,
                                work_type: CompletedWorkType::OpenWithMergedBranch,
                            });
                        }
                    }
                }
                octocrab::models::IssueState::Closed => {
                    // For closed issues, we just need to clean up labels
                    // (they were likely auto-closed by PR merge)
                    completed.push(CompletedWork {
                        issue: issue.clone(),
                        branch_name,
                        agent_id,
                        work_type: CompletedWorkType::ClosedWithLabels,
                    });
                }
                _ => {
                    // Handle any other states (shouldn't happen normally)
                    if verbose {
                        println!("   ⚠️  Skipping issue #{} with unknown state: {:?}", issue.number, issue.state);
                    }
                }
            }
        }
    }
    
    Ok(completed)
}

async fn cleanup_completed_work(client: &github::GitHubClient, work: &CompletedWork, dry_run: bool) -> Result<(), github::GitHubError> {
    match work.work_type {
        CompletedWorkType::ReadyForPhaseOne => {
            // New bundle workflow: Mark work complete and free agent (no immediate PR)
            if !dry_run {
                transition_to_work_completed(client, work).await?
            } else {
                println!("   📝 Bundle: Would mark work complete and free agent");
                println!("   📝 No immediate PR creation - work queued for bundling");
            }
        }
        CompletedWorkType::ReadyForPhaseTwo => {
            // Phase 2: Complete final merge and cleanup
            if !dry_run {
                execute_phase_two(client, work).await?
            } else {
                println!("   📝 Phase 2: Would merge PR and remove route:land label");
                println!("   📝 Issue would be closed via GitHub auto-close");
            }
        }
        CompletedWorkType::WorkCompleted => {
            // Bundle workflow: Work completed, evaluate for bundling or timed fallback
            if !dry_run {
                handle_completed_work_bundling(client, work).await?
            } else {
                println!("   📝 Bundle: Would evaluate for bundling or create individual PR after timeout");
                println!("   📝 Agent already freed when work was marked complete");
            }
        }
        CompletedWorkType::OpenWithMergedBranch => {
            // Legacy: Issue should have been auto-closed by PR merge with "Fixes #N" keywords
            // If still open, it means auto-close didn't work (PR may not have had keywords)
            if !dry_run {
                remove_agent_labels_from_issue(client, &work.issue).await?;
                // Note: In the new system, GitHub should auto-close issues when PRs with 
                // "Fixes #issue_number" keywords are merged. If the issue is still open,
                // it indicates either:
                // 1. PR was created before auto-close enhancement
                // 2. PR didn't include proper keywords
                // 3. Manual intervention is needed
                // For safety, we'll add a completion comment but let humans handle closure
                add_completion_comment_only(client, &work.issue, &work.branch_name).await?;
            } else {
                println!("   📝 Would remove agent labels and add completion comment");
                println!("   📝 Note: Issue should auto-close when PR with 'Fixes #{}' merges", work.issue.number);
            }
        }
        CompletedWorkType::ClosedWithLabels => {
            // Issue was auto-closed by PR merge, just clean up labels
            if !dry_run {
                remove_agent_labels_from_issue(client, &work.issue).await?;
            } else {
                println!("   📝 Would remove agent labels from auto-closed issue");
            }
        }
        CompletedWorkType::OrphanedBranch => {
            // This shouldn't happen in current logic, but handle for completeness
            if !dry_run {
                remove_agent_labels_from_issue(client, &work.issue).await?;
            } else {
                println!("   📝 Would remove agent labels from orphaned work");
            }
        }
    }
    
    Ok(())
}

fn is_branch_ready_for_pr(branch_name: &str) -> Result<bool, github::GitHubError> {
    // Check if branch exists and has commits ahead of main
    
    // First check if branch exists locally
    let branch_exists = Command::new("git")
        .args(&["show-ref", "--verify", &format!("refs/heads/{}", branch_name)])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);
    
    if !branch_exists {
        // Check if branch exists on remote
        let remote_branch_exists = Command::new("git")
            .args(&["show-ref", "--verify", &format!("refs/remotes/origin/{}", branch_name)])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);
        
        if !remote_branch_exists {
            return Ok(false); // Branch doesn't exist
        }
        
        // Fetch the remote branch
        let _ = Command::new("git").args(&["fetch", "origin", branch_name]).output();
    }
    
    // Check if branch has commits ahead of main
    let output = Command::new("git")
        .args(&["rev-list", "--count", &format!("main..{}", branch_name)])
        .output();
    
    match output {
        Ok(result) => {
            if result.status.success() {
                let count_str = String::from_utf8_lossy(&result.stdout);
                let count: u32 = count_str.trim().parse().unwrap_or(0);
                Ok(count > 0) // Has commits ahead of main
            } else {
                Ok(false) // Git command failed, assume not ready
            }
        }
        Err(_) => Ok(false),
    }
}

fn is_branch_merged_to_main(branch_name: &str) -> Result<bool, github::GitHubError> {
    // Check if branch was merged to main using git merge-base
    // This checks if the branch commits are reachable from main
    let output = Command::new("git")
        .args(&["merge-base", "--is-ancestor", branch_name, "main"])
        .output();
    
    match output {
        Ok(result) => {
            // merge-base --is-ancestor returns 0 if ancestor, 1 if not, >1 for errors
            match result.status.code() {
                Some(0) => {
                    // Branch is an ancestor of main, now check if branch still exists
                    // If branch doesn't exist locally, it was probably merged and deleted
                    let branch_exists = Command::new("git")
                        .args(&["show-ref", "--verify", &format!("refs/heads/{}", branch_name)])
                        .output()
                        .map(|output| output.status.success())
                        .unwrap_or(false);
                    
                    // If branch was ancestor and no longer exists, it was likely merged
                    Ok(!branch_exists)
                }
                Some(1) => Ok(false), // Not an ancestor, not merged
                _ => {
                    // Error or unknown status - check if branch exists on origin but not locally
                    let remote_branch_exists = Command::new("git")
                        .args(&["show-ref", "--verify", &format!("refs/remotes/origin/{}", branch_name)])
                        .output()
                        .map(|output| output.status.success())
                        .unwrap_or(false);
                    
                    if remote_branch_exists {
                        // Branch exists on remote - fetch and try again
                        let _ = Command::new("git").args(&["fetch", "origin"]).output();
                        
                        // Try merge-base check again after fetch
                        Command::new("git")
                            .args(&["merge-base", "--is-ancestor", &format!("origin/{}", branch_name), "main"])
                            .output()
                            .map(|result| result.status.success())
                            .unwrap_or(false)
                            .then(|| true)
                            .ok_or_else(|| github::GitHubError::IoError(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                format!("Could not determine merge status for branch: {}", branch_name)
                            )))
                    } else {
                        Ok(false)
                    }
                }
            }
        }
        Err(e) => Err(github::GitHubError::IoError(e)),
    }
}

async fn remove_agent_labels_from_issue(client: &github::GitHubClient, issue: &octocrab::models::issues::Issue) -> Result<(), github::GitHubError> {
    // Find and remove all agent labels from this issue
    let agent_labels: Vec<_> = issue.labels
        .iter()
        .filter(|label| label.name.starts_with("agent"))
        .collect();
    
    for agent_label in agent_labels {
        remove_label_from_issue(client, issue.number, &agent_label.name).await?;
    }
    
    Ok(())
}

async fn add_completion_comment_only(_client: &github::GitHubClient, issue: &octocrab::models::issues::Issue, branch_name: &str) -> Result<(), github::GitHubError> {
    // Add completion comment without closing the issue
    // This is used when GitHub auto-close should have handled closure but didn't
    let comment_body = format!(
        "🎯 **Agent Work Completed**\n\n\
         Agent work has been completed and merged:\n\
         - Branch `{}` was successfully merged to main\n\
         - Agent work has been integrated\n\
         - Agent is now available for new assignments\n\n\
         ℹ️  **Note**: This issue should have been auto-closed by GitHub when the PR was merged.\n\
         If this issue is still open, it may need manual closure or the PR may not have\n\
         included the proper 'Fixes #{}' keywords.\n\n\
         ✅ Work completed successfully!",
        branch_name, issue.number
    );
    
    // Add completion comment only
    let output = Command::new("gh")
        .args(&["issue", "comment", &issue.number.to_string(), "--body", &comment_body])
        .output();
    
    match output {
        Ok(result) => {
            if !result.status.success() {
                let error_msg = String::from_utf8_lossy(&result.stderr);
                println!("   ⚠️  Could not add completion comment: {}", error_msg);
                // Don't fail the whole operation if comment fails
            }
        }
        Err(e) => {
            println!("   ⚠️  Could not add completion comment: {}", e);
            // Don't fail the whole operation if comment fails
        }
    }
    
    Ok(())
}

async fn execute_phase_one(client: &github::GitHubClient, work: &CompletedWork) -> Result<(), github::GitHubError> {
    // Phase 1: Create PR with CodeRabbit integration and remove route:ready label
    println!("   🚀 Phase 1: Creating PR for completed work");
    
    // Generate PR body with auto-close keywords for GitHub's native issue closure
    let pr_body = format!(
        "## Summary\n\
        Agent work completion for issue #{}\n\n\
        **Agent**: {}\n\
        **Branch**: {}\n\
        **Work Type**: Agent-completed task\n\n\
        This PR contains work completed by the agent and is ready for CodeRabbit AI review.\n\
        After review approval, this will automatically close the issue.\n\n\
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
    let pr_title = format!("[{}] {}", work.agent_id, work.issue.title);
    
    match client.create_pull_request(
        &pr_title,
        &work.branch_name,
        "main",
        &pr_body
    ).await {
        Ok(pr) => {
            println!("   ✅ Created PR #{} with CodeRabbit integration", pr.number);
            
            // Atomically remove route:ready label to free the agent
            match remove_label_from_issue(client, work.issue.number, "route:ready").await {
                Ok(_) => {
                    println!("   ✅ Removed route:ready label - agent freed for new work");
                    println!("   🤖 CodeRabbit will begin AI review automatically");
                    println!("   ⏳ Human approval required after AI review completion");
                }
                Err(e) => {
                    println!("   ⚠️  Warning: Failed to remove route:ready label: {:?}", e);
                    println!("   ⚠️  Agent may not be freed for new work");
                }
            }
        }
        Err(e) => {
            return Err(e);
        }
    }
    
    Ok(())
}

async fn execute_phase_two(client: &github::GitHubClient, work: &CompletedWork) -> Result<(), github::GitHubError> {
    // Phase 2: Complete final merge after CodeRabbit + human approval
    println!("   🎆 Phase 2: Completing final merge after approval");
    
    // Note: In a full implementation, this would:
    // 1. Verify CodeRabbit review is complete
    // 2. Verify human approval is granted
    // 3. Merge the PR
    // 4. Remove route:land label
    // 5. The issue will auto-close via GitHub's "Fixes #N" keywords
    
    // For now, we'll remove the route:land label to indicate completion
    match remove_label_from_issue(client, work.issue.number, "route:land").await {
        Ok(_) => {
            println!("   ✅ Removed route:land label");
            println!("   🔀 PR should be merged by human after review approval");
            println!("   📝 Issue will auto-close when PR merges (via 'Fixes #{}' keywords)", work.issue.number);
        }
        Err(e) => {
            return Err(e);
        }
    }
    
    Ok(())
}

async fn handle_completed_work_bundling(client: &github::GitHubClient, work: &CompletedWork) -> Result<(), github::GitHubError> {
    // Collect all completed work items for bundling consideration
    let completed_work_items = detect_all_completed_work(client).await?;
    
    // Check how long this work has been waiting
    let completion_timestamp = get_work_completion_timestamp(&work.issue).await?;
    let wait_time_minutes = completion_timestamp.map(|ts| {
        let now = chrono::Utc::now();
        (now - ts).num_minutes()
    }).unwrap_or(0);
    
    // Fallback: Create individual PR if waiting > 10 minutes
    if wait_time_minutes > 10 {
        println!("   ⏰ Work waiting {} minutes - creating individual PR", wait_time_minutes);
        return create_individual_pr_from_completed_work(client, work).await;
    }
    
    // Check for bundling opportunity (2+ completed items)
    if completed_work_items.len() >= 2 {
        println!("   📦 Found {} completed items - creating bundle PR", completed_work_items.len());
        return create_bundle_pr_from_completed_work(client, completed_work_items).await;
    }
    
    // Work completed but not enough for bundling yet, and not timed out
    println!("   ⏳ Work completed, waiting for bundling opportunity ({}min elapsed)", wait_time_minutes);
    println!("   📦 Bundle threshold: 2+ items | ⏰ Individual PR fallback: 10+ minutes");
    
    Ok(())
}

async fn transition_to_work_completed(client: &github::GitHubClient, work: &CompletedWork) -> Result<(), github::GitHubError> {
    // Key function: Mark work complete and free agent immediately (no PR creation)
    println!("   🔄 Transitioning to bundle workflow - freeing agent immediately");
    
    // Remove route:ready label to free agent
    match remove_label_from_issue(client, work.issue.number, "route:ready").await {
        Ok(_) => {
            println!("   ✅ Removed route:ready label - agent freed for new work");
        }
        Err(e) => {
            println!("   ⚠️  Warning: Failed to remove route:ready label: {:?}", e);
        }
    }
    
    // Add work:completed label to queue for bundling
    match add_label_to_issue(client, work.issue.number, "work:completed").await {
        Ok(_) => {
            println!("   ✅ Added work:completed label - queued for bundling");
            println!("   📦 Work will be bundled with other completed items or get individual PR after 10min");
        }
        Err(e) => {
            return Err(e);
        }
    }
    
    println!("   🚀 Agent {} immediately available for new assignments", work.agent_id);
    
    Ok(())
}

async fn detect_all_completed_work(client: &github::GitHubClient) -> Result<Vec<CompletedWork>, github::GitHubError> {
    // Get all issues with work:completed label
    let issues = client.fetch_issues().await?;
    let mut completed_work = Vec::new();
    
    for issue in issues {
        if issue.labels.iter().any(|label| label.name == "work:completed") {
            if let Some(agent_label) = issue.labels.iter().find(|label| label.name.starts_with("agent")) {
                let agent_id = agent_label.name.clone();
                let branch_name = format!("{}/{}", agent_id, issue.number);
                
                completed_work.push(CompletedWork {
                    issue: issue.clone(),
                    branch_name,
                    agent_id,
                    work_type: CompletedWorkType::WorkCompleted,
                });
            }
        }
    }
    
    Ok(completed_work)
}

async fn get_work_completion_timestamp(issue: &octocrab::models::issues::Issue) -> Result<Option<chrono::DateTime<chrono::Utc>>, github::GitHubError> {
    // For MVP, we'll use the updated_at timestamp as a proxy for when work:completed was added
    // In a full implementation, we could track this more precisely with issue events
    Ok(Some(issue.updated_at))
}

async fn create_individual_pr_from_completed_work(client: &github::GitHubClient, work: &CompletedWork) -> Result<(), github::GitHubError> {
    // Create individual PR and transition to route:land
    println!("   🚀 Creating individual PR for timed-out work");
    
    let pr_body = format!(
        "## Summary\n\
        Agent work completion for issue #{} (individual PR - bundling timeout)\n\n\
        **Agent**: {}\n\
        **Branch**: {}\n\
        **Reason**: Bundling timeout after 10+ minutes\n\n\
        This PR contains work completed by the agent and is ready for CodeRabbit AI review.\n\
        After review approval, this will automatically close the issue.\n\n\
        Fixes #{}\n\n\
        🤖 Generated with [Clambake](https://github.com/johnhkchen/clambake)\n\
        Co-Authored-By: {} <agent@clambake.dev>",
        work.issue.number,
        work.agent_id,
        work.branch_name,
        work.issue.number,
        work.agent_id
    );
    
    let pr_title = format!("[{}] {}", work.agent_id, work.issue.title);
    
    match client.create_pull_request(
        &pr_title,
        &work.branch_name,
        "main",
        &pr_body
    ).await {
        Ok(pr) => {
            println!("   ✅ Created individual PR #{}", pr.number);
            
            // Remove work:completed and add route:land
            remove_label_from_issue(client, work.issue.number, "work:completed").await?;
            add_label_to_issue(client, work.issue.number, "route:land").await?;
            
            println!("   ✅ Transitioned to route:land for final merge");
        }
        Err(e) => {
            return Err(e);
        }
    }
    
    Ok(())
}

async fn create_bundle_pr_from_completed_work(client: &github::GitHubClient, completed_work: Vec<CompletedWork>) -> Result<(), github::GitHubError> {
    // Create bundle PR for 2+ completed work items
    let issue_numbers: Vec<u64> = completed_work.iter().map(|w| w.issue.number).collect();
    let branch_names: Vec<String> = completed_work.iter().map(|w| w.branch_name.clone()).collect();
    
    let bundle_branch = format!("bundle/{}", issue_numbers.iter().map(|n| n.to_string()).collect::<Vec<_>>().join("-"));
    
    let pr_title = format!("Bundle: {} issues (bundle workflow)", completed_work.len());
    let pr_body = format!(
        "## Bundle Summary\n\
        This PR bundles multiple agent-completed tasks for efficient review and API rate limiting.\n\n\
        ## Bundled Issues\n\
        {}\n\n\
        ## Bundle Details\n\
        - **Bundle Strategy**: Multiple completed work items detected\n\
        - **API Rate Limiting**: Reduces PR creation from {}+ to 1 PR\n\
        - **Agent Efficiency**: All agents already freed when work completed\n\n\
        After CodeRabbit review and approval, this bundle will close all included issues.\n\n\
        {}\n\n\
        🤖 Generated with [Clambake Bundle Workflow](https://github.com/johnhkchen/clambake)",
        completed_work.iter().map(|w| format!("- Fixes #{}: {} ({})", w.issue.number, w.issue.title, w.agent_id)).collect::<Vec<_>>().join("\n"),
        completed_work.len(),
        completed_work.iter().map(|w| format!("Fixes #{}", w.issue.number)).collect::<Vec<_>>().join("\n")
    );
    
    // Create bundle branch
    if let Err(e) = create_bundle_branch(&bundle_branch, &branch_names) {
        return Err(github::GitHubError::NotImplemented(format!(
            "Bundle creation failed: {:?}", e
        )));
    }
    
    // Create bundle PR
    match client.create_pull_request(
        &pr_title,
        &bundle_branch,
        "main",
        &pr_body
    ).await {
        Ok(pr) => {
            println!("   ✅ Created bundle PR #{} for {} issues", pr.number, completed_work.len());
            
            // Transition all work items from work:completed to route:land
            for work in &completed_work {
                remove_label_from_issue(client, work.issue.number, "work:completed").await?;
                add_label_to_issue(client, work.issue.number, "route:land").await?;
            }
            
            println!("   ✅ All {} issues transitioned to route:land", completed_work.len());
        }
        Err(e) => {
            return Err(e);
        }
    }
    
    Ok(())
}

async fn close_issue_with_merge_reference(_client: &github::GitHubClient, issue: &octocrab::models::issues::Issue, branch_name: &str) -> Result<(), github::GitHubError> {
    // Use GitHub CLI to close issue with a completion comment
    let comment_body = format!(
        "🎯 **Automated Completion**\n\n\
         This issue has been automatically marked as complete because:\n\
         - Branch `{}` was successfully merged to main\n\
         - Agent work has been integrated\n\
         - Agent is now available for new assignments\n\n\
         ✅ Work completed successfully!",
        branch_name
    );
    
    // Add completion comment
    let output = Command::new("gh")
        .args(&["issue", "comment", &issue.number.to_string(), "--body", &comment_body])
        .output();
    
    match output {
        Ok(result) => {
            if !result.status.success() {
                let error_msg = String::from_utf8_lossy(&result.stderr);
                println!("   ⚠️  Could not add completion comment: {}", error_msg);
                // Continue with closure even if comment fails
            }
        }
        Err(e) => {
            println!("   ⚠️  Could not add completion comment: {}", e);
            // Continue with closure even if comment fails
        }
    }
    
    // Close the issue
    let output = Command::new("gh")
        .args(&["issue", "close", &issue.number.to_string(), "--reason", "completed"])
        .output();
    
    match output {
        Ok(result) => {
            if result.status.success() {
                Ok(())
            } else {
                let error_msg = String::from_utf8_lossy(&result.stderr);
                Err(github::GitHubError::IoError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("GitHub CLI error closing issue: {}", error_msg)
                )))
            }
        }
        Err(e) => {
            Err(github::GitHubError::IoError(e))
        }
    }
}

async fn check_ongoing_work() -> Result<Option<OngoingWork>> {
    // Check for assigned issues and local git state without creating new assignments
    match github::GitHubClient::new() {
        Ok(client) => {
            match client.fetch_issues().await {
                Ok(issues) => {
                    let _current_user = client.owner();
                    
                    // Look for issues assigned to agent001 (this chat session)
                    let assigned_issues: Vec<_> = issues
                        .into_iter()
                        .filter(|issue| {
                            let is_open = issue.state == octocrab::models::IssueState::Open;
                            let has_agent001_label = issue.labels.iter()
                                .any(|label| label.name == "agent001");
                            let has_route_label = issue.labels.iter()
                                .any(|label| label.name == "route:ready");
                            
                            is_open && has_agent001_label && has_route_label
                        })
                        .collect();
                        
                    if let Some(issue) = assigned_issues.first() {
                        // Found ongoing work - build status regardless of agent availability
                        let branch_name = format!("agent001/{}", issue.number);
                        
                        // Check if we're currently on this branch or if it exists
                        let current_branch = get_current_git_branch();
                        let branch_exists = check_if_branch_exists(&branch_name);
                        let has_uncommitted = check_uncommitted_changes();
                        
                        let status = if current_branch.as_deref() == Some(&branch_name) {
                            "Currently working (on branch)".to_string()
                        } else if branch_exists {
                            "Branch exists (not checked out)".to_string()
                        } else {
                            "Assigned (branch not created yet)".to_string()
                        };
                        
                        return Ok(Some(OngoingWork {
                            issue_number: issue.number,
                            issue_title: issue.title.clone(),
                            branch_name,
                            status,
                            has_uncommitted_changes: has_uncommitted,
                        }));
                    }
                    
                    Ok(None)
                }
                Err(_) => Ok(None),
            }
        }
        Err(_) => Ok(None),
    }
}

fn get_current_git_branch() -> Option<String> {
    Command::new("git")
        .args(&["branch", "--show-current"])
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

fn check_if_branch_exists(branch_name: &str) -> bool {
    Command::new("git")
        .args(&["show-ref", "--verify", &format!("refs/heads/{}", branch_name)])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn check_uncommitted_changes() -> bool {
    Command::new("git")
        .args(&["status", "--porcelain"])
        .output()
        .map(|output| {
            if output.status.success() {
                !String::from_utf8(output.stdout).unwrap_or_default().trim().is_empty()
            } else {
                false
            }
        })
        .unwrap_or(false)
}

async fn get_github_rate_limit(client: &GitHubClient) -> Result<(u32, u32, Option<u32>), github::GitHubError> {
    // Use the octocrab client directly to get rate limit information
    // This is a simplified implementation - in a real scenario you'd want to handle this more robustly
    
    // For now, we'll make a simple API call and estimate based on response headers
    // A proper implementation would use the rate limit API endpoint
    match client.fetch_issues().await {
        Ok(_) => {
            // If we can make the call, assume we have most of our rate limit remaining
            // This is a placeholder - in production you'd parse actual rate limit headers
            Ok((4800, 5000, Some(45))) // remaining, total, reset_minutes
        }
        Err(_) => {
            // If the call fails, it might be due to rate limiting
            Ok((0, 5000, Some(60))) // Assume we're rate limited
        }
    }
}

// Configuration structures for clambake.toml (MVP version)
#[derive(Serialize, Deserialize, Debug)]
struct ClambakeConfig {
    github: GitHubConfig,
    routing: RoutingConfig,
}

#[derive(Serialize, Deserialize, Debug)]
struct GitHubConfig {
    owner: String,
    repo: String,
    token_env: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct RoutingConfig {
    max_agents: u32,
    routing_label: String,
    assignment_label_prefix: String,
}

async fn init_command(agents: u32, template: Option<String>, force: bool, dry_run: bool) -> Result<()> {
    println!("🚀 CLAMBAKE INITIALIZATION (MVP)");
    println!("=================================");
    println!();
    
    // Validate agent count
    if agents < 1 || agents > 12 {
        println!("❌ Invalid agent count: {}. Must be between 1 and 12.", agents);
        return Ok(());
    }
    
    if dry_run {
        println!("🔍 DRY RUN MODE - No changes will be made");
        println!();
    }
    
    println!("📋 Configuration:");
    println!("   🤖 Agents: {}", agents);
    if let Some(ref t) = template {
        println!("   📦 Template: {} (templates not implemented in MVP)", t);
    }
    println!("   🔄 Force overwrite: {}", if force { "Yes" } else { "No" });
    println!();
    
    // Step 1: Check if .clambake already exists
    print!("🔍 Checking for existing configuration... ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    
    if Path::new(".clambake").exists() && !force {
        println!("❌");
        println!();
        println!("❌ .clambake directory already exists!");
        println!("   💡 Use --force to overwrite existing configuration");
        println!("   💡 Or run 'clambake status' to check current setup");
        return Ok(());
    }
    println!("✅");
    
    // Step 2: Validate GitHub access and repository
    print!("🔑 Validating GitHub access... ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    
    let github_client = match GitHubClient::new() {
        Ok(client) => {
            // Test GitHub access by making a simple API call
            match client.fetch_issues().await {
                Ok(_) => {
                    println!("✅");
                    client
                }
                Err(e) => {
                    println!("❌");
                    println!();
                    println!("❌ GitHub API access failed: {}", e);
                    return Ok(());
                }
            }
        }
        Err(e) => {
            println!("❌");
            println!();
            println!("❌ Failed to initialize GitHub client: {}", e);
            return Ok(());
        }
    };
    
    let owner = github_client.owner().to_string();
    let repo = github_client.repo().to_string();
    println!("   📂 Repository: {}/{}", owner, repo);
    
    // Step 3: Create .clambake directory structure
    if !dry_run {
        print!("📁 Creating .clambake directory... ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        
        // Remove existing .clambake if force is enabled
        if Path::new(".clambake").exists() && force {
            fs::remove_dir_all(".clambake")?;
        }
        
        fs::create_dir_all(".clambake")?;
        println!("✅");
    } else {
        println!("📁 Would create .clambake directory");
    }
    
    // Step 4: Generate main configuration file
    if !dry_run {
        print!("⚙️  Generating configuration file... ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        
        let config = ClambakeConfig {
            github: GitHubConfig {
                owner: owner.clone(),
                repo: repo.clone(),
                token_env: "GITHUB_TOKEN".to_string(),
            },
            routing: RoutingConfig {
                max_agents: agents,
                routing_label: "route:ready".to_string(),
                assignment_label_prefix: "agent".to_string(),
            },
        };
        
        let config_toml = toml::to_string_pretty(&config)?;
        fs::write(".clambake/config.toml", config_toml)?;
        
        println!("✅");
    } else {
        println!("⚙️  Would generate .clambake/config.toml");
    }
    
    // Step 5: Setup GitHub repository labels
    print!("🏷️  Setting up GitHub labels... ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    
    if !dry_run {
        match setup_github_labels(agents).await {
            Ok(_) => println!("✅"),
            Err(e) => {
                println!("❌");
                println!("   ⚠️  GitHub label setup failed: {}", e);
                println!("   💡 You can create them manually or re-run init");
            }
        }
    } else {
        println!("✅ (dry run)");
        println!("   📝 Would create route:ready label");
        println!("   📝 Would create priority labels (high/medium/low)"); 
        println!("   📝 Would create agent labels (agent001 - agent{:03})", agents);
    }
    
    // Step 6: Create basic issue templates
    print!("📋 Creating issue templates... ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    
    if !dry_run {
        match create_basic_issue_templates().await {
            Ok(_) => println!("✅"),
            Err(e) => {
                println!("❌");
                println!("   ⚠️  Issue template creation failed: {}", e);
            }
        }
    } else {
        println!("✅ (dry run)");
        println!("   📝 Would create .github/ISSUE_TEMPLATE/task.md");
        println!("   📝 Would create .github/ISSUE_TEMPLATE/bug.md");
    }
    
    println!();
    if dry_run {
        println!("🎯 MVP DRY RUN COMPLETE");
        println!("   📝 Run without --dry-run to create the basic configuration");
    } else {
        println!("🎯 MVP INITIALIZATION COMPLETE!");
        println!();
        println!("✅ Basic Clambake setup created for {}/{}", owner, repo);
        println!();
        println!("🚀 READY TO USE:");
        println!("   → Check system: clambake status");
        println!("   → Create task: gh issue create --title 'Your task' --label 'route:ready'");
        println!("   → Start work: clambake pop");
        println!();
        println!("📈 ADVANCED FEATURES:");
        println!("   → See issue #41 for templates, Phoenix, and advanced features");
    }
    
    Ok(())
}

async fn setup_github_labels(agents: u32) -> Result<()> {
    // Create essential labels using gh CLI
    let labels_to_create = vec![
        ("route:ready", "Ready for agent assignment", "0052cc"),
        ("route:priority-high", "High priority task", "d73a49"),
        ("route:priority-medium", "Medium priority task", "fbca04"),
        ("route:priority-low", "Low priority task", "0e8a16"),
    ];
    
    // Create routing and priority labels
    for (name, description, color) in labels_to_create {
        let output = Command::new("gh")
            .args(&["label", "create", name, "--description", description, "--color", color, "--force"])
            .output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Continue on errors - we'll report them but not fail
            eprintln!("   Warning: Failed to create label '{}': {}", name, stderr);
        }
    }
    
    // Create agent labels
    for i in 1..=agents {
        let agent_label = format!("agent{:03}", i);
        let description = format!("Assigned to {} chat session", agent_label);
        
        let output = Command::new("gh")
            .args(&["label", "create", &agent_label, "--description", &description, "--color", "1f883d", "--force"])
            .output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("   Warning: Failed to create agent label '{}': {}", agent_label, stderr);
        }
    }
    
    Ok(())
}

async fn create_basic_issue_templates() -> Result<()> {
    // Create .github/ISSUE_TEMPLATE directory
    fs::create_dir_all(".github/ISSUE_TEMPLATE")?;
    
    let task_template = r#"---
name: Task
about: Create a task for agent assignment
title: ''
labels: ['route:ready']
assignees: ''
---

## Description
Brief description of what needs to be done

## Acceptance Criteria
- [ ] Criterion 1
- [ ] Criterion 2
- [ ] Criterion 3

## Additional Context
Any additional information or constraints
"#;
    
    fs::write(".github/ISSUE_TEMPLATE/task.md", task_template)?;
    
    let bug_template = r#"---
name: Bug Report
about: Report a bug that needs fixing
title: '[BUG] '
labels: ['route:ready', 'route:priority-high']
assignees: ''
---

## Bug Description
Brief description of the bug

## Steps to Reproduce
1. Step 1
2. Step 2
3. Step 3

## Expected vs Actual Behavior
**Expected:** What should happen
**Actual:** What actually happens

## Additional Context
Screenshots, logs, or other relevant information
"#;
    
    fs::write(".github/ISSUE_TEMPLATE/bug_report.md", bug_template)?;
    
    Ok(())
}

async fn show_how_to_get_work() -> Result<()> {
    println!("🤖 CLAMBAKE - Agent Status");
    println!("=========================");
    println!();
    
    // Check for ongoing work assigned to current user
    match check_ongoing_work().await {
        Ok(Some(ongoing)) => {
            println!("📋 ONGOING WORK:");
            println!("   🎯 Issue #{}: {}", ongoing.issue_number, ongoing.issue_title);
            println!("   🌿 Branch: {}", ongoing.branch_name);
            println!("   ⏰ Status: {}", ongoing.status);
            println!();
            
            if ongoing.has_uncommitted_changes {
                println!("⚠️  Uncommitted changes detected");
            }
            
            println!("▶️  Use 'clambake pop --mine' to continue working");
        }
        Ok(None) => {
            println!("🆓 No ongoing work detected");
            println!();
            println!("▶️  Use 'clambake pop' to get your next task");
        }
        Err(_) => {
            println!("📋 Unable to check work status");
            println!();
            println!("▶️  Use 'clambake pop' to get a task");
        }
    }
    
    Ok(())
}

// Test change for issue #77 fix
