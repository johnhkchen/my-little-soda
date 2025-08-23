use anyhow::Result;
use crate::cli::commands::with_agent_router;
use crate::priority::Priority;
use octocrab::models::issues::Issue;

pub struct PeekCommand {
    pub ci_mode: bool,
}

impl PeekCommand {
    pub fn new() -> Self {
        Self { ci_mode: false }
    }

    pub fn with_ci_mode(mut self, ci_mode: bool) -> Self {
        self.ci_mode = ci_mode;
        self
    }

    pub async fn execute(&self) -> Result<()> {
        println!("👀 Peeking at next task in queue...");
        println!();
        
        with_agent_router(|router| async move {
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
                            println!("   🏷️  Labels: {}", labels.join(", "));
                        }
                        
                        // Show description if available
                        if let Some(body) = &next_issue.body {
                            if !body.is_empty() {
                                let preview = if body.len() > 200 {
                                    format!("{}...", &body[..200])
                                } else {
                                    body.clone()
                                };
                                println!("   📄 Description: {}", preview);
                            }
                        }
                        
                        println!("   🔗 URL: {}", next_issue.html_url);
                        println!();
                        
                        if issues.len() > 1 {
                            println!("📈 QUEUE DEPTH: {} total routable tasks available", issues.len());
                        }
                        
                        println!("💡 Run 'my-little-soda pop' to claim this task");
                        Ok(())
                    }
                    Err(e) => {
                        println!("❌ Failed to fetch tasks: {}", e);
                        println!();
                        println!("🎯 TROUBLESHOOTING:");
                        println!("   → Check GitHub authentication: gh auth status");
                        println!("   → Check for routable issues: gh issue list --label 'route:ready'");
                        println!("   → Create new task: gh issue create --title 'Your task' --label 'route:ready'");
                        Err(e.into())
                    }
                }
        }).await.or_else(|_| {
            println!("❌ Router initialization failed");
            println!();
            println!("📚 Full setup guide: clambake init");
            Ok(())
        })
    }
}

fn get_issue_priority(issue: &Issue) -> u32 {
    Priority::from_labels(&issue.labels.iter()
        .map(|l| l.name.as_str()).collect::<Vec<_>>()).value()
}