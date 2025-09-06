/// Configuration generation and repository detection
///
/// Contains all logic for generating the my-little-soda.toml configuration file
/// and detecting repository information from git remotes.

use crate::config::{
    AgentConfig, AgentProcessConfig, BundleConfig, CIModeConfig, DatabaseConfig, GitHubConfig,
    MyLittleSodaConfig, ObservabilityConfig, RateLimitConfig, WorkContinuityConfig,
};
use crate::git::{Git2Operations, GitHubRepoInfo};
use anyhow::{anyhow, Result};
use std::io::Write;

use super::core::InitCommand;

/// Generate the my-little-soda.toml configuration file
pub async fn generate_configuration(init_command: &InitCommand) -> Result<()> {
    let config_path = "my-little-soda.toml";

    if init_command.fs_ops().exists(config_path) && !init_command.force {
        return Err(anyhow!(
            "Configuration file {} already exists. Use --force to overwrite.",
            config_path
        ));
    }

    if init_command.dry_run {
        println!("Would create configuration file: {config_path}");
        println!("Would create directory: .my-little-soda/");
        return Ok(());
    }

    // Create .my-little-soda directory
    print!("📁 Creating .my-little-soda directory... ");
    std::io::stdout().flush().unwrap();

    init_command
        .fs_ops()
        .create_dir_all(".my-little-soda/credentials")
        .await
        .map_err(|e| anyhow!("Failed to create .my-little-soda directory: {}", e))?;
    println!("✅");

    // Detect repository information
    let (owner, repo) = detect_repository_info(init_command).await?;

    // Generate configuration
    print!("⚙️  Generating my-little-soda.toml... ");
    std::io::stdout().flush().unwrap();

    let config = MyLittleSodaConfig {
        github: GitHubConfig {
            token: None, // Will be read from env var
            owner,
            repo,
            rate_limit: RateLimitConfig {
                requests_per_hour: 5000,
                burst_capacity: 100,
            },
        },
        observability: ObservabilityConfig {
            tracing_enabled: true,
            otlp_endpoint: None,
            log_level: "info".to_string(),
            metrics_enabled: true,
        },
        agents: AgentConfig {
            coordination_timeout_seconds: 300,
            bundle_processing: BundleConfig {
                max_queue_size: 50,
                processing_timeout_seconds: 1800,
            },
            process_management: AgentProcessConfig {
                claude_code_path: "claude-code".to_string(),
                timeout_minutes: 30,
                cleanup_on_failure: true,
                work_dir_prefix: ".my-little-soda/agents".to_string(),
                enable_real_agents: false,
            },
            ci_mode: CIModeConfig {
                enabled: init_command.ci_mode,
                artifact_handling: "standard".to_string(),
                github_token_strategy: "standard".to_string(),
                workflow_state_persistence: true,
                ci_timeout_adjustment: 300,
                enhanced_error_reporting: true,
            },
            work_continuity: WorkContinuityConfig::default(),
        },
        database: Some(DatabaseConfig {
            url: ".my-little-soda/my-little-soda.db".to_string(),
            max_connections: 10,
            auto_migrate: true,
        }),
    };

    config
        .save_to_file(config_path)
        .map_err(|e| anyhow!("Failed to save configuration: {}", e))?;
    println!("✅");

    Ok(())
}

/// Detect repository owner and name from git remote
pub async fn detect_repository_info(init_command: &InitCommand) -> Result<(String, String)> {
    let output = init_command
        .fs_ops()
        .execute_command(
            "git",
            &[
                "remote".to_string(),
                "get-url".to_string(),
                "origin".to_string(),
            ],
        )
        .await
        .map_err(|e| anyhow!("Failed to get git remote URL: {}", e))?;

    if !output.status.success() {
        // For fresh projects (with or without --force), provide enhanced guidance
        println!("⚠️  No git remote found in this repository");
        println!("   To set up a GitHub remote, run:");
        println!("   git remote add origin https://github.com/YOUR-USERNAME/YOUR-REPO.git");
        println!("   Using placeholder values for now - update my-little-soda.toml after setting up remote");
        return Ok((
            "your-github-username".to_string(),
            "your-repo-name".to_string(),
        ));
    }

    let remote_url = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Use the improved URL parsing from git operations
    match Git2Operations::parse_github_url(&remote_url) {
        Ok(Some(GitHubRepoInfo { owner, repo })) => {
            Ok((owner, repo))
        }
        Ok(None) => {
            Err(anyhow!(
                "Could not parse GitHub repository from remote URL: {}. Only GitHub repositories are supported. Expected format: git@github.com:owner/repo.git or https://github.com/owner/repo.git",
                remote_url
            ))
        }
        Err(e) => {
            Err(anyhow!(
                "Error parsing GitHub repository URL '{}': {}. Make sure this is a valid GitHub remote URL",
                remote_url, e
            ))
        }
    }
}