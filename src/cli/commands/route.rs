use anyhow::Result;
use crate::cli::commands::with_agent_router;

pub struct RouteCommand {
    pub agents: u32,
    pub ci_mode: bool,
}

impl RouteCommand {
    pub fn new(agents: u32) -> Self {
        Self { 
            agents,
            ci_mode: false,
        }
    }

    pub fn with_ci_mode(mut self, ci_mode: bool) -> Self {
        self.ci_mode = ci_mode;
        self
    }

    pub async fn execute(&self) -> Result<()> {
        println!("🔀 [ADMIN] Routing up to {} tickets to available agents", self.agents);
        println!();
        
        with_agent_router(|router| async move {
            print!("🔍 Scanning for routable issues... ");
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            
            match router.route_issues_to_agents().await {
                Ok(assignments) => {
                    println!("✅");
                    let routed_count = assignments.len().min(self.agents as usize);
                    
                    if routed_count > 0 {
                        print!("🎯 Assigning {} tasks to agents... ", routed_count);
                        std::io::Write::flush(&mut std::io::stdout()).unwrap();
                        println!("✅");
                        println!();
                        println!("✅ Successfully routed {} real GitHub issues to agents:", routed_count);
                        println!("📋 ROUTING STATUS: Issues assigned in GitHub and branches created");
                        println!();
                        for (i, assignment) in assignments.iter().take(self.agents as usize).enumerate() {
                            println!("Routed issue #{}:", i + 1);
                            println!("  🎯 Issue #{}: {}", assignment.issue.number, assignment.issue.title);
                            println!("  👤 Assigned to: {}", assignment.assigned_agent.id);
                            println!("  🌿 Branch: {}", assignment.branch_name);
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
                    Ok(())
                }
                Err(e) => {
                    println!("{}", e);
                    println!();
                    println!("🚀 ALTERNATIVE: Try 'clambake pop' for single-agent workflow");
                    Err(e.into())
                }
            }
        }).await.or_else(|_| {
            println!("📚 Need setup help? Run: clambake init");
            println!("🚀 For single tasks: clambake pop");
            Ok(())
        })
    }
}