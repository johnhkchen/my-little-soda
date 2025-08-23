use anyhow::Result;
use crate::agents::AgentCoordinator;
use crate::github::{GitHubClient, GitHubActions, WorkflowStatus};
use tracing::{info, warn};

pub struct ActionsCommand {
    pub trigger_bundle: bool,
    pub status: bool,
    pub logs: bool,
    pub run_id: Option<u64>,
    pub force: bool,
    pub dry_run: bool,
    pub verbose: bool,
    pub ci_mode: bool,
}

impl ActionsCommand {
    pub fn new(
        trigger_bundle: bool,
        status: bool,
        logs: bool,
        run_id: Option<u64>,
        force: bool,
        dry_run: bool,
        verbose: bool,
    ) -> Self {
        Self {
            trigger_bundle,
            status,
            logs,
            run_id,
            force,
            dry_run,
            verbose,
            ci_mode: false,
        }
    }

    pub fn with_ci_mode(mut self, ci_mode: bool) -> Self {
        self.ci_mode = ci_mode;
        self
    }

    pub async fn execute(&self) -> Result<()> {
        if self.trigger_bundle {
            self.trigger_bundling_workflow().await
        } else if self.status {
            self.show_workflow_status().await
        } else if self.logs {
            self.show_workflow_logs().await
        } else {
            println!("🔧 MY LITTLE SODA ACTIONS - GitHub Actions Integration");
            println!("===============================================");
            println!();
            self.show_actions_help();
            Ok(())
        }
    }

    async fn trigger_bundling_workflow(&self) -> Result<()> {
        println!("🚀 MY LITTLE SODA ACTIONS - Trigger Bundling Workflow");
        println!("===============================================");
        println!();

        if self.verbose {
            println!("🔧 Configuration:");
            println!("   🚀 Force bundle: {}", self.force);
            println!("   🔍 Dry run: {}", self.dry_run);
            println!("   📢 Verbose: {}", self.verbose);
            println!("   🤖 CI mode: {}", self.ci_mode);
            println!();
        }

        let coordinator = AgentCoordinator::new().await?;

        print!("🎯 Triggering GitHub Actions bundling workflow... ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        match coordinator.trigger_bundling_workflow_with_ci_mode(self.force, self.dry_run, self.verbose, self.ci_mode).await {
            Ok(_) => {
                println!("✅");
                println!();
                println!("✅ Successfully triggered GitHub Actions bundling workflow");
                println!("💡 Check the Actions tab in your GitHub repository to monitor progress");
                if !self.force {
                    println!("⏰ Workflow will respect train schedule unless forced");
                }
            },
            Err(e) => {
                println!("❌");
                println!();
                println!("❌ Failed to trigger workflow: {}", e);
                return Err(e.into());
            }
        }

        Ok(())
    }

    async fn show_workflow_status(&self) -> Result<()> {
        println!("📊 MY LITTLE SODA ACTIONS - Workflow Status");
        println!("====================================");
        println!();

        let client = GitHubClient::new()?;

        print!("🔍 Fetching recent workflow runs... ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        match client.actions.get_workflow_runs("clambake-bundling.yml", Some(5)).await {
            Ok(runs) => {
                println!("✅");
                println!();

                if runs.is_empty() {
                    println!("📭 No recent workflow runs found");
                    return Ok(());
                }

                println!("📋 Recent bundling workflow runs:");
                println!();

                for (i, run) in runs.iter().enumerate() {
                    let status_icon = match run.status {
                        WorkflowStatus::Completed => {
                            match run.conclusion.as_deref() {
                                Some("success") => "✅",
                                Some("failure") => "❌",
                                Some("cancelled") => "🚫",
                                Some("skipped") => "⏭️",
                                _ => "❓",
                            }
                        },
                        WorkflowStatus::InProgress => "🔄",
                        WorkflowStatus::Queued => "⏳",
                        WorkflowStatus::Failed => "❌",
                        WorkflowStatus::Cancelled => "🚫",
                        WorkflowStatus::Skipped => "⏭️",
                        WorkflowStatus::Unknown(_) => "❓",
                    };

                    println!("{}. {} {} (ID: {})", 
                             i + 1, 
                             status_icon, 
                             run.workflow_name,
                             run.id);
                    println!("   📅 Created: {}", run.created_at.format("%Y-%m-%d %H:%M:%S UTC"));
                    println!("   📅 Updated: {}", run.updated_at.format("%Y-%m-%d %H:%M:%S UTC"));
                    println!("   🔗 URL: {}", run.html_url);
                    
                    if let Some(conclusion) = &run.conclusion {
                        println!("   🎯 Conclusion: {}", conclusion);
                    }
                    
                    println!();
                }

                println!("💡 Use --logs --run-id <ID> to view logs for a specific run");
            },
            Err(e) => {
                println!("❌");
                println!();
                println!("❌ Failed to fetch workflow runs: {}", e);
                return Err(e.into());
            }
        }

        Ok(())
    }

    async fn show_workflow_logs(&self) -> Result<()> {
        let run_id = self.run_id.ok_or_else(|| {
            anyhow::anyhow!("Run ID is required for viewing logs. Use --run-id <ID>")
        })?;

        println!("📜 MY LITTLE SODA ACTIONS - Workflow Logs");
        println!("==================================");
        println!();

        let client = GitHubClient::new()?;

        print!("🔍 Fetching workflow run details... ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        match client.actions.get_workflow_run(run_id).await {
            Ok(run) => {
                println!("✅");
                println!();

                println!("📋 Workflow Run Details:");
                println!("   🆔 ID: {}", run.id);
                println!("   📛 Name: {}", run.workflow_name);
                println!("   📊 Status: {:?}", run.status);
                if let Some(conclusion) = &run.conclusion {
                    println!("   🎯 Conclusion: {}", conclusion);
                }
                println!("   📅 Created: {}", run.created_at.format("%Y-%m-%d %H:%M:%S UTC"));
                println!("   📅 Updated: {}", run.updated_at.format("%Y-%m-%d %H:%M:%S UTC"));
                println!("   🔗 URL: {}", run.html_url);
                println!();

                println!("💡 For detailed logs, visit the workflow URL above in your browser");
                println!("🔧 GitHub API doesn't provide direct log access, but the web interface shows full logs");
            },
            Err(e) => {
                println!("❌");
                println!();
                println!("❌ Failed to fetch workflow run: {}", e);
                return Err(e.into());
            }
        }

        Ok(())
    }

    fn show_actions_help(&self) {
        println!("GitHub Actions integration for CLambake bundling automation");
        println!();
        println!("Available subcommands:");
        println!("  --trigger-bundle     Manually trigger the bundling workflow");
        println!("  --status             Show recent workflow run status");
        println!("  --logs --run-id ID   Show details for a specific workflow run");
        println!();
        println!("Options for --trigger-bundle:");
        println!("  --force              Force bundling outside schedule");
        println!("  --dry-run            Perform dry run without creating PRs");
        println!("  --verbose            Enable verbose output");
        println!();
        println!("Examples:");
        println!("  clambake actions --trigger-bundle");
        println!("  clambake actions --trigger-bundle --force --verbose");
        println!("  clambake actions --status");
        println!("  clambake actions --logs --run-id 12345");
        println!();
        println!("💡 The bundling workflow runs automatically every 10 minutes");
        println!("🔗 View workflows: https://github.com/{}/clambake/actions", 
                 std::env::var("GITHUB_REPOSITORY_OWNER").unwrap_or_else(|_| "your-org".to_string()));
    }
}