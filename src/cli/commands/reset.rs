use crate::github::GitHubClient;
use anyhow::Result;

pub struct ResetCommand {
    pub ci_mode: bool,
}

impl ResetCommand {
    pub fn new() -> Self {
        Self { ci_mode: false }
    }

    pub fn with_ci_mode(mut self, ci_mode: bool) -> Self {
        self.ci_mode = ci_mode;
        self
    }

    pub async fn execute(&self) -> Result<()> {
        println!("🔄 [DEV] Resetting agent state to clean idle state");
        println!();

        // Initialize GitHub client
        match GitHubClient::new() {
            Ok(client) => {
                println!(
                    "✅ GitHub client initialized for {}/{}",
                    client.owner(),
                    client.repo()
                );
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
                                issue
                                    .labels
                                    .iter()
                                    .any(|label| label.name.starts_with("agent"))
                            })
                            .collect();

                        if agent_labeled_issues.is_empty() {
                            println!("✅ No agent labels found - system already in clean state");
                            return Ok(());
                        }

                        println!(
                            "🏷️  Found {} issues with agent labels",
                            agent_labeled_issues.len()
                        );
                        println!();

                        // Remove agent labels from each issue
                        let mut reset_count = 0;
                        for issue in agent_labeled_issues {
                            let agent_labels: Vec<_> = issue
                                .labels
                                .iter()
                                .filter(|label| label.name.starts_with("agent"))
                                .collect();

                            for agent_label in agent_labels {
                                print!(
                                    "🧹 Removing {} from Issue #{}... ",
                                    agent_label.name, issue.number
                                );
                                std::io::Write::flush(&mut std::io::stdout()).unwrap();

                                match remove_label_from_issue(
                                    &client,
                                    issue.number,
                                    &agent_label.name,
                                )
                                .await
                                {
                                    Ok(_) => {
                                        println!("✅");
                                        reset_count += 1;
                                    }
                                    Err(e) => {
                                        println!("❌ Error: {e}");
                                    }
                                }
                            }
                        }

                        println!();
                        println!("✅ Reset complete:");
                        println!("   🧹 {reset_count} agent labels removed");
                        println!("   🤖 All agents now available for new work");
                        println!();
                        println!("🎯 Ready for fresh start! Use 'my-little-soda route' or 'my-little-soda pop' to begin new work.");

                        Ok(())
                    }
                    Err(e) => {
                        println!("❌ Failed to fetch issues: {e}");
                        Err(e.into())
                    }
                }
            }
            Err(e) => {
                println!("❌ Failed to initialize GitHub client: {e}");
                println!();
                println!("🔧 Troubleshooting:");
                println!("   → Check GitHub authentication: gh auth status");
                println!("   → Verify repository access");
                println!("   → Ensure GITHUB_TOKEN is set correctly");
                Err(e.into())
            }
        }
    }
}

// Helper function to remove a label from an issue
async fn remove_label_from_issue(
    client: &GitHubClient,
    issue_number: u64,
    label_name: &str,
) -> Result<(), crate::github::GitHubError> {
    // Use GitHub API to remove label from issue
    // For now, we'll use the gh CLI as a simple implementation
    use std::process::Command;

    let repo = format!("{}/{}", client.owner(), client.repo());
    let output = Command::new("gh")
        .args([
            "issue",
            "edit",
            &issue_number.to_string(),
            "-R",
            &repo,
            "--remove-label",
            label_name,
        ])
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                Ok(())
            } else {
                let error_msg = String::from_utf8_lossy(&result.stderr);
                Err(crate::github::GitHubError::IoError(std::io::Error::other(
                    format!("GitHub CLI error: {error_msg}"),
                )))
            }
        }
        Err(e) => Err(crate::github::GitHubError::IoError(e)),
    }
}
