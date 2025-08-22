use anyhow::Result;
use crate::agents::AgentRouter;
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
                                println!("🔴 {} - BUSY ({}/{})", agent_id, current, max);
                            } else {
                                println!("🟢 {} - Available ({}/{})", agent_id, current, max);
                            }
                        }
                        
                        println!();
                        println!("💼 CAPACITY OVERVIEW:");
                        println!("   🎯 Total agents: {}", total_agents);
                        println!("   ✅ Available: {}", available_agents);
                        println!("   🔴 Busy: {}", total_agents - available_agents);
                        
                        if available_agents > 0 {
                            println!("   🚀 Ready for new work!");
                        } else {
                            println!("   ⏳ All agents busy - work will queue");
                        }
                        
                        println!();
                    }
                    Err(e) => {
                        println!("❌ Failed to get agent status: {}", e);
                        println!();
                    }
                }
                
                // Display state machine information
                println!("🔧 STATE MACHINE STATUS:");
                println!("───────────────────────");
                
                match router.get_agent_state_machine_status().await {
                    Ok(states) => {
                        if states.is_empty() {
                            println!("⚠️  No state machines initialized");
                        } else {
                            for (agent_id, status) in states {
                                println!("🤖 {}", status);
                            }
                        }
                        println!();
                    }
                    Err(e) => {
                        println!("❌ Failed to get state machine status: {}", e);
                        println!();
                    }
                }
                
                // Display task queue
                println!("📋 TASK QUEUE:");
                println!("──────────────");
                
                match router.fetch_routable_issues().await {
                    Ok(issues) => {
                        if issues.is_empty() {
                            println!("📭 No tasks in queue");
                            println!("   💡 Create tasks with: gh issue create --title 'Your task' --label 'route:ready'");
                        } else {
                            println!("📊 {} tasks waiting for assignment", issues.len());
                            
                            // Show priority breakdown
                            let mut priority_counts = std::collections::HashMap::new();
                            for issue in &issues {
                                let priority = get_issue_priority_name(issue);
                                *priority_counts.entry(priority).or_insert(0) += 1;
                            }
                            
                            for (priority, count) in priority_counts {
                                println!("   {} {} {} tasks", get_priority_emoji(&priority), count, priority);
                            }
                        }
                        
                        println!();
                    }
                    Err(e) => {
                        println!("❌ Failed to check task queue: {}", e);
                        println!();
                    }
                }
                
                // Show helpful commands
                println!("🎯 QUICK ACTIONS:");
                println!("   → clambake pop      # Claim next task");
                println!("   → clambake peek     # Preview next task");
                println!("   → clambake route    # Route tasks to agents");
                println!("   → clambake land     # Complete lifecycle");
                
                Ok(())
            }
            Err(e) => {
                println!("❌ System initialization failed: {}", e);
                println!();
                println!("📚 Setup help: clambake init");
                println!("🔧 Check GitHub auth: gh auth status");
                Err(e.into())
            }
        }
    }
}

fn get_issue_priority_name(issue: &Issue) -> String {
    use crate::priority::Priority;
    
    let priority = Priority::from_labels(&issue.labels.iter()
        .map(|l| l.name.as_str()).collect::<Vec<_>>());
    
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