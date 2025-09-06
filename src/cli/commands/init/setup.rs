/// Agent setup and final verification
///
/// Contains all logic for setting up agents and performing final verification
/// to ensure the My Little Soda system is properly initialized.

use crate::github::client::GitHubClient;
use anyhow::{anyhow, Result};
use std::io::Write;

use super::core::InitCommand;
use super::validation;

/// Setup agent working directories and configuration
pub async fn setup_agents(init_command: &InitCommand) -> Result<()> {
    if init_command.dry_run {
        println!("Would configure 1 agent with capacity settings (single-agent mode)");
        println!("Would create agent state tracking");
        return Ok(());
    }

    print!("🤖 Configuring agent capacity (1 agent)... ");
    std::io::stdout().flush().unwrap();

    // Create agent working directories
    init_command
        .fs_ops()
        .create_dir_all(".my-little-soda/agents")
        .await
        .map_err(|e| anyhow!("Failed to create agent directories: {}", e))?;

    println!("✅");

    Ok(())
}

/// Verify the complete setup is working correctly
pub async fn verify_setup(init_command: &InitCommand) -> Result<()> {
    if init_command.dry_run {
        println!("Would test GitHub API connectivity");
        println!("Would verify all labels were created");
        println!("Would confirm configuration is loadable");
        return Ok(());
    }

    // Check if this is a fresh project
    let is_fresh_project = validation::detect_fresh_project(init_command).await;

    if is_fresh_project {
        println!("⏭️  Skipping GitHub API connectivity test for fresh project");
    } else {
        // Test GitHub API connectivity for existing repositories
        print!("✅ Testing GitHub API connectivity... ");
        std::io::stdout().flush().unwrap();

        let github_client = GitHubClient::with_verbose(init_command.verbose)
            .map_err(|e| anyhow!("Failed to create GitHub client: {}", e))?;

        // Try to fetch a few issues to test API access
        match github_client.fetch_issues().await {
            Ok(_) => println!("✅"),
            Err(e) => return Err(anyhow!("GitHub API test failed: {}", e)),
        }
    }

    if is_fresh_project {
        println!("⏭️  Skipping configuration validation for fresh project");
        println!("   Configuration uses placeholder values - update my-little-soda.toml with real GitHub info");
    } else {
        // Verify configuration is loadable for existing repositories
        print!("✅ Verifying configuration is loadable... ");
        std::io::stdout().flush().unwrap();

        let _config = crate::config::MyLittleSodaConfig::load()
            .map_err(|e| anyhow!("Generated configuration is invalid: {}", e))?;
        println!("✅");
    }

    if is_fresh_project {
        println!("⏭️  Skipping routing system test for fresh project");
    } else {
        // Basic routing test for existing repositories
        print!("✅ Running basic routing test... ");
        std::io::stdout().flush().unwrap();

        // This is a simple test - just verify we can create an agent router
        match crate::agents::AgentRouter::new().await {
            Ok(_) => println!("✅"),
            Err(e) => return Err(anyhow!("Routing system test failed: {}", e)),
        }
    }

    Ok(())
}