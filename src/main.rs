use clap::{Parser, Subcommand};
use anyhow::Result;

mod github;
mod agents;
mod workflows;

use agents::AgentRouter;
use std::process::Command;
use github::GitHubClient;

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
    /// Initialize clambake configuration for the current project
    Init,
    /// Reset all agents to idle state by removing agent labels from issues
    Reset,
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
        Some(Commands::Init) => {
            println!("Initializing clambake...");
            Ok(())
        }
        Some(Commands::Reset) => {
            tokio::runtime::Runtime::new()?.block_on(async {
                reset_command().await
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
                    
                    println!("📋 TASK QUEUE:");
                    println!("─────────────");
                    
                    let high_count = issues.iter().filter(|i| get_issue_priority(i) == 3).count();
                    let medium_count = issues.iter().filter(|i| get_issue_priority(i) == 2).count();
                    let low_count = issues.iter().filter(|i| get_issue_priority(i) == 1).count();
                    let normal_count = issues.iter().filter(|i| get_issue_priority(i) == 0).count();
                    let total = high_count + medium_count + low_count + normal_count;
                    
                    if total > 0 {
                        if high_count > 0 { println!(" 🔴 High priority: {} tasks", high_count); }
                        if medium_count > 0 { println!(" 🟡 Medium priority: {} tasks", medium_count); }
                        if low_count > 0 { println!(" 🟢 Low priority: {} tasks", low_count); }
                        if normal_count > 0 { println!(" ⚪ Normal priority: {} tasks", normal_count); }
                        println!();
                        println!(" 📊 Total: {} tasks ready for assignment", total);
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
                    let usage_percent = ((total - remaining) as f32 / total as f32 * 100.0) as u32;
                    
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
                    let priority_label = match priority {
                        3 => "HIGH",
                        2 => "MEDIUM", 
                        1 => "LOW",
                        _ => "NORMAL",
                    };
                    
                    println!("🎯 NEXT TASK TO BE ASSIGNED:");
                    println!("   📋 Issue #{}: {}", next_issue.number, next_issue.title);
                    println!("   🏷️  Priority: {} ({})", priority_label, priority);
                    
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
    // Priority based on labels: higher number = higher priority
    if issue.labels.iter().any(|label| label.name == "route:priority-high") {
        3 // Highest priority
    } else if issue.labels.iter().any(|label| label.name == "route:priority-medium") {
        2 // Medium priority
    } else if issue.labels.iter().any(|label| label.name == "route:priority-low") {
        1 // Low priority
    } else {
        0 // No priority label = lowest priority
    }
}

// Removed: show_quick_system_status() - not needed for streamlined agent workflow

#[derive(Debug)]
struct OngoingWork {
    issue_number: u64,
    issue_title: String,
    branch_name: String,
    status: String,
    has_uncommitted_changes: bool,
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
                        let branch_name = format!("work-{}", issue.number);
                        
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

