use anyhow::{Result, anyhow};
use std::path::Path;
use std::fs;
use octocrab::Octocrab;
use crate::config::{MyLittleSodaConfig, GitHubConfig, ObservabilityConfig, AgentConfig, DatabaseConfig, RateLimitConfig, BundleConfig, AgentProcessConfig, CIModeConfig};
use crate::github::client::GitHubClient;
use crate::github::errors::GitHubError;

pub struct InitCommand {
    pub agents: u32,
    pub template: Option<String>,
    pub force: bool,
    pub dry_run: bool,
    pub ci_mode: bool,
}

#[derive(Debug)]
struct LabelSpec {
    name: String,
    color: String,
    description: String,
}

impl InitCommand {
    pub fn new(agents: u32, template: Option<String>, force: bool, dry_run: bool) -> Self {
        Self {
            agents,
            template,
            force,
            dry_run,
            ci_mode: false,
        }
    }

    pub fn with_ci_mode(mut self, ci_mode: bool) -> Self {
        self.ci_mode = ci_mode;
        self
    }

    pub async fn execute(&self) -> Result<()> {
        if self.dry_run {
            println!("🚀 CLAMBAKE INIT - Development Environment Setup (DRY RUN)");
        } else {
            println!("🚀 CLAMBAKE INIT - Development Environment Setup");
        }
        println!("====================================================");
        println!();
        
        println!("⚙️  Configuration:");
        println!("   🤖 Agents: {}", self.agents);
        if let Some(template) = &self.template {
            println!("   📋 Template: {}", template);
        }
        println!("   🔄 Force: {}", self.force);
        println!("   🔍 Dry run: {}", self.dry_run);
        println!();

        // Validate input parameters
        if self.agents == 0 || self.agents > 12 {
            return Err(anyhow!("Number of agents must be between 1 and 12, got: {}", self.agents));
        }

        // Phase 1: Validation
        println!("Phase 1: Validation");
        println!("─────────────────");
        self.validate_environment().await?;
        println!();

        // Phase 2: Label Setup  
        println!("Phase 2: Label Setup");
        println!("──────────────────");
        self.setup_labels().await?;
        println!();

        // Phase 3: Configuration
        println!("Phase 3: Configuration");
        println!("─────────────────────");
        self.generate_configuration().await?;
        println!();

        // Phase 4: Agent Setup
        println!("Phase 4: Agent Setup");
        println!("───────────────────");
        self.setup_agents().await?;
        println!();

        // Phase 5: Verification
        println!("Phase 5: Verification");
        println!("────────────────────");
        self.verify_setup().await?;
        println!();

        println!("✅ Clambake initialization completed successfully!");
        println!();
        println!("🚀 Next steps:");
        println!("   • my-little-soda pop      # Claim your first task");
        println!("   • clambake status   # Check system status");
        println!("   • gh issue create --title 'Your task' --label 'route:ready'");
        
        Ok(())
    }

    async fn validate_environment(&self) -> Result<()> {
        // Check GitHub CLI authentication
        print!("✅ Verifying GitHub CLI authentication... ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        
        if !self.dry_run {
            let output = tokio::process::Command::new("gh")
                .args(&["auth", "status"])
                .output()
                .await
                .map_err(|e| anyhow!("Failed to run 'gh auth status': {}. Make sure GitHub CLI is installed.", e))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow!("GitHub CLI not authenticated: {}. Run 'gh auth login' first.", stderr));
            }
        }
        println!("✅");

        // Check repository write permissions
        print!("✅ Checking repository write permissions... ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        
        if !self.dry_run {
            let github_client = GitHubClient::new()
                .map_err(|e| anyhow!("Failed to create GitHub client: {}", e))?;
            
            // Test permissions by trying to fetch repository info
            let octocrab = Octocrab::builder()
                .personal_token(std::env::var("GITHUB_TOKEN").or_else(|_| std::env::var("MY_LITTLE_SODA_GITHUB_TOKEN"))?)
                .build()?;
            
            let repo = octocrab.repos(github_client.owner(), github_client.repo())
                .get()
                .await
                .map_err(|e| anyhow!("Failed to access repository: {}. Check your GitHub token permissions.", e))?;

            if !repo.permissions.as_ref().map(|p| p.admin || p.push).unwrap_or(false) {
                return Err(anyhow!("Insufficient repository permissions. Need 'push' access to manage labels and issues."));
            }
        }
        println!("✅");

        // Ensure git repository is clean
        print!("✅ Checking git repository status... ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        
        if !self.dry_run {
            let output = tokio::process::Command::new("git")
                .args(&["status", "--porcelain"])
                .output()
                .await
                .map_err(|e| anyhow!("Failed to check git status: {}", e))?;

            if !output.stdout.is_empty() {
                println!("⚠️");
                println!("   Warning: Repository has uncommitted changes.");
                if !self.force {
                    return Err(anyhow!("Repository has uncommitted changes. Use --force to proceed anyway."));
                }
                println!("   Proceeding due to --force flag.");
            } else {
                println!("✅");
            }
        } else {
            println!("✅ (dry run)");
        }

        Ok(())
    }

    async fn setup_labels(&self) -> Result<()> {
        let labels = self.get_required_labels();
        
        if self.dry_run {
            println!("Would create {} labels:", labels.len());
            for label in &labels {
                println!("  🏷️  {} (#{}) - {}", label.name, label.color, label.description);
            }
            return Ok(());
        }

        let github_client = GitHubClient::new()
            .map_err(|e| anyhow!("Failed to create GitHub client: {}", e))?;

        let octocrab = Octocrab::builder()
            .personal_token(std::env::var("GITHUB_TOKEN").or_else(|_| std::env::var("MY_LITTLE_SODA_GITHUB_TOKEN"))?)
            .build()?;

        for label in &labels {
            print!("🏷️  Creating label '{}' ", label.name);
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            
            match octocrab.issues(github_client.owner(), github_client.repo())
                .create_label(&label.name, &label.color, &label.description)
                .await 
            {
                Ok(_) => println!("✅"),
                Err(octocrab::Error::GitHub { source, .. }) if source.message.contains("already_exists") => {
                    println!("⚠️ (already exists)");
                }
                Err(e) => {
                    return Err(anyhow!("Failed to create label '{}': {}", label.name, e));
                }
            }
        }

        Ok(())
    }

    fn get_required_labels(&self) -> Vec<LabelSpec> {
        vec![
            // Core routing labels
            LabelSpec {
                name: "route:ready".to_string(),
                color: "0052cc".to_string(),
                description: "Available for agent assignment".to_string(),
            },
            LabelSpec {
                name: "route:ready_to_merge".to_string(),
                color: "5319e7".to_string(),
                description: "Completed work ready for merge".to_string(),
            },
            LabelSpec {
                name: "route:unblocker".to_string(),
                color: "d73a4a".to_string(),
                description: "Critical system issues blocking other work".to_string(),
            },
            LabelSpec {
                name: "route:review".to_string(),
                color: "fbca04".to_string(),
                description: "Under review".to_string(),
            },
            LabelSpec {
                name: "route:human-only".to_string(),
                color: "7057ff".to_string(),
                description: "Requires human attention".to_string(),
            },
            // Priority labels
            LabelSpec {
                name: "route:priority-low".to_string(),
                color: "c5def5".to_string(),
                description: "Low priority task (Priority: 1)".to_string(),
            },
            LabelSpec {
                name: "route:priority-medium".to_string(),
                color: "1d76db".to_string(),
                description: "Medium priority task (Priority: 2)".to_string(),
            },
            LabelSpec {
                name: "route:priority-high".to_string(),
                color: "b60205".to_string(),
                description: "High priority task (Priority: 3)".to_string(),
            },
            LabelSpec {
                name: "route:priority-very-high".to_string(),
                color: "ee0701".to_string(),
                description: "Very high priority task (Priority: 4)".to_string(),
            },
            // Additional operational labels
            LabelSpec {
                name: "code-review-feedback".to_string(),
                color: "e99695".to_string(),
                description: "Issues created from code review feedback".to_string(),
            },
            LabelSpec {
                name: "supertask-decomposition".to_string(),
                color: "bfdadc".to_string(),
                description: "Task broken down from larger work".to_string(),
            },
        ]
    }

    async fn generate_configuration(&self) -> Result<()> {
        let config_path = "clambake.toml";
        
        if Path::new(config_path).exists() && !self.force {
            return Err(anyhow!("Configuration file {} already exists. Use --force to overwrite.", config_path));
        }

        if self.dry_run {
            println!("Would create configuration file: {}", config_path);
            println!("Would create directory: .clambake/");
            return Ok(());
        }

        // Create .clambake directory
        print!("📁 Creating .clambake directory... ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        
        fs::create_dir_all(".clambake/credentials").map_err(|e| anyhow!("Failed to create .clambake directory: {}", e))?;
        println!("✅");

        // Detect repository information
        let (owner, repo) = self.detect_repository_info().await?;
        
        // Generate configuration
        print!("⚙️  Generating clambake.toml... ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        
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
                max_agents: self.agents,
                coordination_timeout_seconds: 300,
                bundle_processing: BundleConfig {
                    max_queue_size: 50,
                    processing_timeout_seconds: 1800,
                },
                process_management: AgentProcessConfig {
                    claude_code_path: "claude-code".to_string(),
                    timeout_minutes: 30,
                    cleanup_on_failure: true,
                    work_dir_prefix: ".clambake/agents".to_string(),
                    enable_real_agents: false,
                },
                ci_mode: CIModeConfig {
                    enabled: self.ci_mode,
                    artifact_handling: "standard".to_string(),
                    github_token_strategy: "standard".to_string(),
                    workflow_state_persistence: true,
                    ci_timeout_adjustment: 300,
                    enhanced_error_reporting: true,
                },
            },
            database: Some(DatabaseConfig {
                url: ".clambake/clambake.db".to_string(),
                max_connections: 10,
                auto_migrate: true,
            }),
        };

        config.save_to_file(config_path)
            .map_err(|e| anyhow!("Failed to save configuration: {}", e))?;
        println!("✅");

        Ok(())
    }

    async fn detect_repository_info(&self) -> Result<(String, String)> {
        let output = tokio::process::Command::new("git")
            .args(&["remote", "get-url", "origin"])
            .output()
            .await
            .map_err(|e| anyhow!("Failed to get git remote URL: {}", e))?;

        if !output.status.success() {
            return Err(anyhow!("No git remote 'origin' found"));
        }

        let remote_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        
        // Parse GitHub URL (supports both SSH and HTTPS)
        let (owner, repo) = if let Some(captures) = regex::Regex::new(r"github\.com[:/]([^/]+)/([^/]+?)(?:\.git)?$")
            .unwrap()
            .captures(&remote_url) 
        {
            (captures[1].to_string(), captures[2].to_string())
        } else {
            return Err(anyhow!("Could not parse GitHub repository from remote URL: {}", remote_url));
        };

        Ok((owner, repo))
    }

    async fn setup_agents(&self) -> Result<()> {
        if self.dry_run {
            println!("Would configure {} agents with capacity settings", self.agents);
            println!("Would create agent state tracking");
            return Ok(());
        }

        print!("🤖 Configuring agent capacity ({} agents)... ", self.agents);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        
        // Create agent working directories
        fs::create_dir_all(".clambake/agents")
            .map_err(|e| anyhow!("Failed to create agent directories: {}", e))?;
        
        println!("✅");

        Ok(())
    }

    async fn verify_setup(&self) -> Result<()> {
        if self.dry_run {
            println!("Would test GitHub API connectivity");
            println!("Would verify all labels were created");
            println!("Would confirm configuration is loadable");
            return Ok(());
        }

        // Test GitHub API connectivity
        print!("✅ Testing GitHub API connectivity... ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        
        let github_client = GitHubClient::new()
            .map_err(|e| anyhow!("Failed to create GitHub client: {}", e))?;
        
        // Try to fetch a few issues to test API access
        match github_client.fetch_issues().await {
            Ok(_) => println!("✅"),
            Err(e) => return Err(anyhow!("GitHub API test failed: {}", e)),
        }

        // Verify configuration is loadable
        print!("✅ Verifying configuration is loadable... ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        
        let _config = crate::config::MyLittleSodaConfig::load()
            .map_err(|e| anyhow!("Generated configuration is invalid: {}", e))?;
        println!("✅");

        // Basic routing test
        print!("✅ Running basic routing test... ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        
        // This is a simple test - just verify we can create an agent router
        match crate::agents::AgentRouter::new().await {
            Ok(_) => println!("✅"),
            Err(e) => return Err(anyhow!("Routing system test failed: {}", e)),
        }

        Ok(())
    }
}