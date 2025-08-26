use crate::agents::AgentRouter;
use anyhow::Result;
use octocrab::models::issues::Issue;

pub struct StatusCommand {
    pub ci_mode: bool,
}

impl StatusCommand {
    pub fn new() -> Self {
        Self { ci_mode: false }
    }

    pub fn with_ci_mode(mut self, ci_mode: bool) -> Self {
        self.ci_mode = ci_mode;
        self
    }

    pub async fn execute(&self) -> Result<()> {
        // Get repository context
        let repo_name = self
            .get_repo_name()
            .unwrap_or_else(|| "unknown".to_string());
        let current_branch = self
            .get_current_branch()
            .unwrap_or_else(|| "unknown".to_string());

        println!("🤖 MY LITTLE SODA STATUS - Repository: {repo_name}");
        println!("==========================================");
        println!();

        print!("🔄 Gathering system information... ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        // Initialize components
        match AgentRouter::new().await {
            Ok(router) => {
                println!("✅");
                println!();

                // Display agent status
                println!("🔧 AGENT STATUS:");
                println!("────────────────");

                // Get agent status
                match router.get_agent_status().await {
                    Ok(utilization) => {
                        let agent_available =
                            utilization.values().any(|(current, max)| current < max);

                        if agent_available {
                            println!("✅ Available - Ready for next task");
                        } else {
                            println!("🔴 Busy - Currently working on assigned task");
                        }

                        println!("📍 Current branch: {current_branch}");
                        println!("🚀 Mode: Manual (use 'my-little-soda spawn --autonomous' for unattended)");
                        println!();
                    }
                    Err(e) => {
                        println!("❌ Failed to get agent status: {e}");
                        println!();
                    }
                }

                // Display task queue with detailed issue information
                match router.fetch_routable_issues().await {
                    Ok(issues) => {
                        if issues.is_empty() {
                            println!("📋 ISSUE QUEUE:");
                            println!("────────────────────────────");
                            println!("📭 No tasks waiting");
                            println!("   💡 Create tasks with: gh issue create --title 'Your task' --label 'route:ready'");
                            println!();
                        } else {
                            println!("📋 ISSUE QUEUE ({} waiting):", issues.len());
                            println!("────────────────────────────");

                            // Sort issues by priority (highest first)
                            let mut sorted_issues = issues.clone();
                            sorted_issues.sort_by(|a, b| {
                                use crate::priority::Priority;
                                let priority_a = Priority::from_labels(
                                    &a.labels.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(),
                                );
                                let priority_b = Priority::from_labels(
                                    &b.labels.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(),
                                );
                                priority_b.cmp(&priority_a) // Higher priority first
                            });

                            // Show top 5 issues with details
                            for (idx, issue) in sorted_issues.iter().take(5).enumerate() {
                                let priority_info = self.get_priority_display(issue);
                                let labels = issue
                                    .labels
                                    .iter()
                                    .map(|l| l.name.as_str())
                                    .filter(|name| {
                                        !name.starts_with("route:") && !name.starts_with("agent")
                                    })
                                    .collect::<Vec<_>>()
                                    .join(",");

                                println!("{} #{} {}", priority_info.0, issue.number, issue.title);
                                println!(
                                    "   📝 {} | Labels: {}",
                                    priority_info.1,
                                    if labels.is_empty() {
                                        "none".to_string()
                                    } else {
                                        labels
                                    }
                                );

                                if idx < sorted_issues.len().min(5) - 1 {
                                    println!();
                                }
                            }

                            if sorted_issues.len() > 5 {
                                println!();
                                println!("   ... and {} more tasks", sorted_issues.len() - 5);
                            }

                            println!();
                        }
                    }
                    Err(e) => {
                        println!("❌ Failed to check task queue: {e}");
                        println!();
                    }
                }

                // Show next actions
                println!("🎯 NEXT ACTIONS:");
                println!("   → my-little-soda pop       # Get highest priority task");
                println!("   → my-little-soda peek      # Preview task details");
                println!("   → my-little-soda spawn --autonomous  # Start unattended mode");

                Ok(())
            }
            Err(e) => {
                println!("❌ System initialization failed: {e}");
                println!();
                println!("📚 Setup help: my-little-soda init");
                println!("🔧 Check GitHub auth: gh auth status");
                Err(e.into())
            }
        }
    }

    fn get_repo_name(&self) -> Option<String> {
        use std::process::Command;
        let output = Command::new("git")
            .args(["remote", "get-url", "origin"])
            .output()
            .ok()?;

        let url = String::from_utf8(output.stdout).ok()?;
        let repo_name = url
            .trim()
            .strip_suffix(".git")?
            .rsplit('/')
            .next()?
            .to_string();
        Some(repo_name)
    }

    fn get_current_branch(&self) -> Option<String> {
        use std::process::Command;
        let output = Command::new("git")
            .args(["branch", "--show-current"])
            .output()
            .ok()?;

        String::from_utf8(output.stdout)
            .ok()
            .map(|s| s.trim().to_string())
    }

    fn get_priority_display(&self, issue: &Issue) -> (String, String) {
        use crate::priority::Priority;

        let priority = Priority::from_labels(
            &issue
                .labels
                .iter()
                .map(|l| l.name.as_str())
                .collect::<Vec<_>>(),
        );

        match priority {
            Priority::Unblocker => ("🚨".to_string(), "Priority: Unblocker".to_string()),
            Priority::MergeReady => ("🔴".to_string(), "Priority: High (Merge Ready)".to_string()),
            Priority::VeryHigh => ("🔴".to_string(), "Priority: Very High".to_string()),
            Priority::High => ("🔴".to_string(), "Priority: High".to_string()),
            Priority::Medium => ("🟡".to_string(), "Priority: Medium".to_string()),
            Priority::Low => ("🟢".to_string(), "Priority: Low".to_string()),
            Priority::Normal => ("🟢".to_string(), "Priority: Normal".to_string()),
        }
    }
}

#[allow(dead_code)]
fn get_issue_priority_name(issue: &Issue) -> String {
    use crate::priority::Priority;

    let priority = Priority::from_labels(
        &issue
            .labels
            .iter()
            .map(|l| l.name.as_str())
            .collect::<Vec<_>>(),
    );

    match priority {
        Priority::Unblocker => "Priority: Critical (Unblocker)".to_string(),
        Priority::MergeReady => "Priority: High (Merge Ready)".to_string(),
        Priority::VeryHigh => "Priority: Very High".to_string(),
        Priority::High => "Priority: High".to_string(),
        Priority::Medium => "Priority: Medium".to_string(),
        Priority::Low => "Priority: Low".to_string(),
        Priority::Normal => "Priority: Normal".to_string(),
    }
}

#[allow(dead_code)]
fn get_priority_emoji(priority: &str) -> &'static str {
    if priority.contains("Critical") || priority.contains("Unblocker") {
        "🚨"
    } else if priority.contains("High") {
        "🔴"
    } else if priority.contains("Medium") {
        "🟡"
    } else {
        "🟢"
    }
}
