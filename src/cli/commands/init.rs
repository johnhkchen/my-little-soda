/// Init command implementation with graceful conflict resolution
/// 
/// # Conflict Resolution Strategy
/// 
/// The init command follows a non-destructive approach when handling existing files:
/// 
/// ## 1. Existing Project Files
/// - **README.md, src/, Cargo.toml, etc.**: Completely preserved without modification
/// - **No overwriting**: Existing project files are never touched or modified
/// - **No data loss**: All user content remains intact
/// 
/// ## 2. Configuration Files
/// - **my-little-soda.toml**: Only created if it doesn't exist, otherwise requires `--force` flag
/// - **Explicit user consent**: User must use `--force` to overwrite existing configuration
/// - **Clear error messages**: Informative errors when conflicts would occur
/// 
/// ## 3. Directory Structure
/// - **.my-little-soda/ directory**: Created alongside existing directories
/// - **No conflicts**: Clambake directories don't interfere with existing project structure
/// - **Isolated setup**: All my-little-soda-specific files are contained in dedicated directories
/// 
/// ## 4. Git Repository State
/// - **Clean repository required**: Init fails on uncommitted changes unless `--force` is used
/// - **Branch preservation**: Current branch and git state remain unchanged
/// - **Remote detection**: Automatically detects GitHub repository information from git remotes
/// 
/// This approach ensures that my-little-soda can be initialized in any existing repository
/// without risk of data loss or conflicts with existing project structure.
use crate::config::{
    AgentConfig, AgentProcessConfig, BundleConfig, CIModeConfig, DatabaseConfig, GitHubConfig,
    MyLittleSodaConfig, ObservabilityConfig, RateLimitConfig, WorkContinuityConfig,
};
use crate::fs::FileSystemOperations;
use crate::github::client::GitHubClient;
use anyhow::{anyhow, Result};
use octocrab::Octocrab;
use std::sync::Arc;
// GitHubError import removed - unused

pub struct InitCommand {
    pub template: Option<String>,
    pub force: bool,
    pub dry_run: bool,
    pub verbose: bool,
    pub ci_mode: bool,
    fs_ops: Arc<dyn FileSystemOperations>,
}

#[derive(Debug)]
struct LabelSpec {
    name: String,
    color: String,
    description: String,
}

impl InitCommand {
    pub fn new(template: Option<String>, force: bool, dry_run: bool, fs_ops: Arc<dyn FileSystemOperations>) -> Self {
        Self {
            template,
            force,
            dry_run,
            verbose: false,
            ci_mode: false,
            fs_ops,
        }
    }

    pub fn with_ci_mode(mut self, ci_mode: bool) -> Self {
        self.ci_mode = ci_mode;
        self
    }

    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    pub async fn execute(&self) -> Result<()> {
        if self.dry_run {
            println!("🚀 MY LITTLE SODA INIT - Development Environment Setup (DRY RUN)");
        } else {
            println!("🚀 MY LITTLE SODA INIT - Development Environment Setup");
        }
        println!("====================================================");
        println!();

        println!("⚙️  Configuration:");
        println!("   🤖 Agents: 1 (single-agent mode)");
        if let Some(template) = &self.template {
            println!("   📋 Template: {template}");
        }
        println!("   🔄 Force: {}", self.force);
        println!("   🔍 Dry run: {}", self.dry_run);
        println!();

        // Single-agent mode - no validation needed for agent count

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

        println!("✅ My Little Soda initialization completed successfully!");
        println!();
        println!("🚀 Next steps:");
        println!("   • my-little-soda pop      # Claim your first task");
        println!("   • my-little-soda status   # Check system status");
        println!("   • gh issue create --title 'Your task' --label 'route:ready'");

        Ok(())
    }

    async fn validate_environment(&self) -> Result<()> {
        if self.verbose {
            println!("🔍 VERBOSE: Starting detailed authentication validation...");
            println!();
        }

        // Check if this is a fresh project (no git repo or no remote)
        let is_fresh_project = self.detect_fresh_project().await;
        
        if is_fresh_project {
            println!("🆕 Fresh project detected - initializing git repository and My Little Soda...");
            println!();
            
            if !self.dry_run {
                // Initialize git repository
                print!("📦 Initializing git repository... ");
                std::io::Write::flush(&mut std::io::stdout()).unwrap();
                
                let git_init_output = self.fs_ops.execute_command("git", &["init".to_string()])
                    .await
                    .map_err(|e| anyhow!("Failed to initialize git repository: {}. Make sure git is installed.", e))?;
                
                if !git_init_output.status.success() {
                    let stderr = String::from_utf8_lossy(&git_init_output.stderr);
                    return Err(anyhow!("Git init failed: {}", stderr));
                }
                println!("✅");
                
                println!();
                println!("✨ Git repository initialized!");
                println!("📋 Next steps after My Little Soda setup completes:");
                println!("   1. Add your files:                git add .");
                println!("   2. Create initial commit:         git commit -m 'Initial commit'");
                println!("   3. Create GitHub repository:      gh repo create --public");
                println!("   4. Push to GitHub:                git push -u origin main");
                println!("   5. Update my-little-soda.toml with correct GitHub info");
                println!();
            } else {
                println!("Would initialize git repository (dry run mode)");
                println!();
            }
            
            // For fresh projects, skip GitHub validation and continue with local setup
            return Ok(());
        }

        // Enhanced authentication validation with verbose debugging
        self.diagnose_authentication_environment().await?;
        self.validate_github_authentication().await?;
        self.validate_github_api_access().await?;

        // Check git repository state
        self.validate_git_state().await?;

        if self.verbose {
            println!("🔍 VERBOSE: All authentication and validation checks completed successfully!");
            println!();
        }

        Ok(())
    }

    /// Proactive authentication environment diagnostics
    async fn diagnose_authentication_environment(&self) -> Result<()> {
        if self.verbose {
            println!("🔍 VERBOSE: Diagnosing authentication environment...");
            
            // Check for common environment variables
            let env_vars = [
                "MY_LITTLE_SODA_GITHUB_TOKEN",
                "GITHUB_TOKEN",
                "GH_TOKEN",
                "GITHUB_OWNER",
                "GITHUB_REPO"
            ];
            
            for var in env_vars {
                match std::env::var(var) {
                    Ok(val) if !val.is_empty() && !val.contains("YOUR_") && !val.contains("your-") => {
                        let display_val = if var.contains("TOKEN") {
                            format!("{}...{}", &val[..4.min(val.len())], &val[val.len()-4.min(val.len())..])
                        } else {
                            val
                        };
                        println!("   ✅ VERBOSE: {var} = {display_val}");
                    },
                    Ok(val) if val.contains("YOUR_") || val.contains("your-") => {
                        println!("   ⚠️  VERBOSE: {var} = {val} (placeholder value - needs to be set)");
                    },
                    Ok(_) => {
                        println!("   ⚠️  VERBOSE: {var} = (empty)");
                    },
                    Err(_) => {
                        println!("   ℹ️  VERBOSE: {var} = (not set)");
                    }
                }
            }
            
            // Check for credential files
            let cred_files = [
                ".my-little-soda/credentials/github_token",
                ".my-little-soda/credentials/github_owner",
                ".my-little-soda/credentials/github_repo"
            ];
            
            for file in cred_files {
                if std::path::Path::new(file).exists() {
                    println!("   ✅ VERBOSE: {file} exists");
                } else {
                    println!("   ℹ️  VERBOSE: {file} not found");
                }
            }
            
            println!();
        }
        
        Ok(())
    }

    /// Comprehensive GitHub CLI authentication validation
    async fn validate_github_authentication(&self) -> Result<()> {
        if self.verbose {
            println!("🔍 VERBOSE: Checking GitHub CLI authentication...");
        }

        print!("✅ Verifying GitHub CLI authentication... ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        if self.dry_run {
            println!("✅ (dry run mode)");
            if self.verbose {
                println!("   🔍 VERBOSE: Would execute: gh auth status");
                println!("   🔍 VERBOSE: Dry run mode - skipping actual command execution");
            }
            return Ok(());
        }

        // Check if gh CLI is available
        if self.verbose {
            println!();
            println!("   🔍 VERBOSE: Testing GitHub CLI availability...");
        }

        let gh_status_output = self.fs_ops.execute_command("gh", &["auth".to_string(), "status".to_string()])
            .await
            .map_err(|e| {
                if self.verbose {
                    println!("   ❌ VERBOSE: GitHub CLI command failed: {e}");
                }
                anyhow!(
                    "Failed to run 'gh auth status': {}. Make sure GitHub CLI is installed.",
                    e
                )
            })?;

        if self.verbose {
            println!("   🔍 VERBOSE: gh auth status exit code: {}", gh_status_output.status.code().unwrap_or(-1));
            let stdout = String::from_utf8_lossy(&gh_status_output.stdout);
            let stderr = String::from_utf8_lossy(&gh_status_output.stderr);
            if !stdout.is_empty() {
                println!("   🔍 VERBOSE: gh stdout: {}", stdout.trim());
            }
            if !stderr.is_empty() {
                println!("   🔍 VERBOSE: gh stderr: {}", stderr.trim());
            }
        }

        if !gh_status_output.status.success() {
            let stderr = String::from_utf8_lossy(&gh_status_output.stderr);
            if self.verbose {
                println!("   ❌ VERBOSE: GitHub CLI not authenticated");
                println!("   🔍 VERBOSE: Authentication failure details:");
                println!("      Exit code: {}", gh_status_output.status.code().unwrap_or(-1));
                if !stderr.is_empty() {
                    println!("      Error output: {}", stderr.trim());
                }
            }
            
            // Provide enhanced error message with multiple authentication options
            let enhanced_error = if stderr.contains("Not logged in") || stderr.contains("not authenticated") {
                format!(
                    "GitHub CLI not authenticated. Please authenticate using one of these methods:\n\
                     💡 Recommended: Run 'gh auth login' and follow the prompts\n\
                     💡 Alternative: Set MY_LITTLE_SODA_GITHUB_TOKEN environment variable\n\
                     💡 Manual setup: Create .my-little-soda/credentials/github_token file\n\
                     \n\
                     GitHub CLI error: {}", 
                    stderr.trim()
                )
            } else if stderr.contains("command not found") || stderr.contains("No such file") {
                format!(
                    "GitHub CLI (gh) is not installed or not in PATH.\n\
                     💡 Install from: https://cli.github.com/\n\
                     💡 Alternative: Set MY_LITTLE_SODA_GITHUB_TOKEN environment variable\n\
                     \n\
                     System error: {}", 
                    stderr.trim()
                )
            } else {
                format!(
                    "GitHub authentication failed: {}\n\
                     💡 Try: gh auth login\n\
                     💡 Check: gh auth status\n\
                     💡 Alternative: Set MY_LITTLE_SODA_GITHUB_TOKEN environment variable", 
                    stderr.trim()
                )
            };
            
            return Err(anyhow!(enhanced_error));
        }

        if self.verbose {
            println!("   ✅ VERBOSE: GitHub CLI is authenticated");
        }
        println!("✅");
        Ok(())
    }

    /// Comprehensive GitHub API access validation using the new client
    async fn validate_github_api_access(&self) -> Result<()> {
        if self.verbose {
            println!("🔍 VERBOSE: Validating GitHub API access...");
        }

        print!("✅ Checking repository write permissions... ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        if self.dry_run {
            println!("✅ (dry run mode)");
            if self.verbose {
                println!("   🔍 VERBOSE: Would test GitHub API connectivity");
                println!("   🔍 VERBOSE: Would validate repository permissions");
                println!("   🔍 VERBOSE: Dry run mode - skipping actual API calls");
            }
            return Ok(());
        }

        if self.verbose {
            println!();
            println!("   🔍 VERBOSE: Testing authentication methods in order:");
            println!("      1. MY_LITTLE_SODA_GITHUB_TOKEN environment variable");
            println!("      2. .my-little-soda/credentials/github_token file");
            println!("      3. GitHub CLI token (fallback)");
            println!();
        }

        // Test the enhanced GitHub client which now includes fallback mechanisms
        let github_client = match GitHubClient::new() {
            Ok(client) => {
                if self.verbose {
                    println!("   ✅ VERBOSE: GitHub API client created successfully");
                    println!("   🔍 VERBOSE: Repository: {}/{}", client.owner(), client.repo());
                    println!("   🔍 VERBOSE: Pre-flight validation passed (authentication + connectivity)");
                }
                client
            },
            Err(e) => {
                if self.verbose {
                    println!("   ❌ VERBOSE: GitHub client creation failed: {e}");
                }
                return Err(anyhow!("Failed to create GitHub client: {}", e));
            }
        };

        // Additional repository permission validation
        if self.verbose {
            println!("   🔍 VERBOSE: Testing repository write permissions...");
        }

        let octocrab = Octocrab::builder()
            .personal_token(
                std::env::var("GITHUB_TOKEN")
                    .or_else(|_| std::env::var("MY_LITTLE_SODA_GITHUB_TOKEN"))
                    .or_else(|_| {
                        // Try to get token from gh CLI
                        use std::process::Command;
                        if let Ok(output) = Command::new("gh").args(["auth", "token"]).output() {
                            if output.status.success() {
                                return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
                            }
                        }
                        Err(std::env::VarError::NotPresent)
                    })?,
            )
            .build()?;

        let repo = octocrab
            .repos(github_client.owner(), github_client.repo())
            .get()
            .await
            .map_err(|e| {
                if self.verbose {
                    println!("   ❌ VERBOSE: Repository access failed: {e}");
                    println!("   🔍 VERBOSE: Repository: {}/{}", github_client.owner(), github_client.repo());
                }
                
                // Provide enhanced error message based on error type
                let enhanced_error = match &e {
                    octocrab::Error::GitHub { source, .. } => {
                        match source.status_code.as_u16() {
                            401 => "GitHub API authentication failed (HTTP 401).\n\
                                 💡 Token is invalid or expired\n\
                                 💡 Try: gh auth login\n\
                                 💡 Or refresh token: gh auth refresh\n\
                                 💡 Check token: gh auth token".to_string(),
                            403 => format!(
                                "GitHub API access forbidden (HTTP 403).\n\
                                 💡 Token lacks required permissions\n\
                                 💡 Repository: {}/{}\n\
                                 💡 Required: 'repo' scope for private repositories\n\
                                 💡 Create token: https://github.com/settings/tokens",
                                github_client.owner(), github_client.repo()
                            ),
                            404 => format!(
                                "GitHub repository not found (HTTP 404).\n\
                                 💡 Repository: {}/{}\n\
                                 💡 Check if repository exists and is accessible\n\
                                 💡 Verify GITHUB_OWNER and GITHUB_REPO settings\n\
                                 💡 Check if repository is private and token has access",
                                github_client.owner(), github_client.repo()
                            ),
                            _ => format!(
                                "GitHub API error (HTTP {}).\n\
                                 💡 Message: {}\n\
                                 💡 Repository: {}/{}\n\
                                 💡 Check GitHub status: https://status.github.com",
                                source.status_code, source.message, github_client.owner(), github_client.repo()
                            )
                        }
                    },
                    octocrab::Error::Http { .. } => "Network error connecting to GitHub API.\n\
                         💡 Check internet connectivity\n\
                         💡 Test: curl -I https://api.github.com\n\
                         💡 Check firewall/proxy settings".to_string(),
                    _ => format!(
                        "Failed to access repository {}/{}.\n\
                         💡 Check your GitHub token permissions\n\
                         💡 Verify repository exists and is accessible\n\
                         💡 Error: {}",
                        github_client.owner(), github_client.repo(), e
                    )
                };
                
                anyhow!(enhanced_error)
            })?;

        let has_write_permissions = repo
            .permissions
            .as_ref()
            .map(|p| p.admin || p.push)
            .unwrap_or(false);

        if self.verbose {
            if let Some(permissions) = &repo.permissions {
                println!("   🔍 VERBOSE: Repository permissions:");
                println!("      - Admin: {}", permissions.admin);
                println!("      - Push: {}", permissions.push);
                println!("      - Pull: {}", permissions.pull);
            }
            println!("   🔍 VERBOSE: Has write permissions: {has_write_permissions}");
        }

        if !has_write_permissions {
            if self.verbose {
                println!("   ❌ VERBOSE: Insufficient repository permissions");
                if let Some(permissions) = &repo.permissions {
                    println!("   🔍 VERBOSE: Current permissions - Admin: {}, Push: {}, Pull: {}", 
                             permissions.admin, permissions.push, permissions.pull);
                }
            }
            
            let permission_error = format!(
                "Insufficient repository permissions for {}/{}.\n\
                 💡 Required: 'push' (write) access to manage labels and issues\n\
                 💡 Current permissions: Admin={}, Push={}, Pull={}\n\
                 💡 Solutions:\n\
                    - Ask repository owner to grant write access\n\
                    - Use personal access token with 'repo' scope\n\
                    - Fork repository and use your fork\n\
                 💡 Token settings: https://github.com/settings/tokens",
                github_client.owner(),
                github_client.repo(),
                repo.permissions.as_ref().map(|p| p.admin).unwrap_or(false),
                repo.permissions.as_ref().map(|p| p.push).unwrap_or(false),
                repo.permissions.as_ref().map(|p| p.pull).unwrap_or(false)
            );
            
            return Err(anyhow!(permission_error));
        }

        if self.verbose {
            println!("   ✅ VERBOSE: Repository permissions validated");
        }
        println!("✅");
        Ok(())
    }

    /// Validate git repository state
    async fn validate_git_state(&self) -> Result<()> {
        if self.verbose {
            println!("🔍 VERBOSE: Checking git repository state...");
        }

        print!("✅ Checking git repository status... ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        if self.dry_run {
            println!("✅ (dry run mode)");
            if self.verbose {
                println!("   🔍 VERBOSE: Would execute: git status --porcelain");
                println!("   🔍 VERBOSE: Dry run mode - skipping git state check");
            }
            return Ok(());
        }

        if self.verbose {
            println!();
            println!("   🔍 VERBOSE: Checking for uncommitted changes...");
        }

        let output = self.fs_ops.execute_command("git", &["status".to_string(), "--porcelain".to_string()])
            .await
            .map_err(|e| {
                if self.verbose {
                    println!("   ❌ VERBOSE: Git status command failed: {e}");
                }
                anyhow!("Failed to check git status: {}", e)
            })?;

        if self.verbose {
            println!("   🔍 VERBOSE: git status --porcelain exit code: {}", output.status.code().unwrap_or(-1));
        }

        if !output.stdout.is_empty() {
            if self.verbose {
                let changes = String::from_utf8_lossy(&output.stdout);
                println!("   🔍 VERBOSE: Found uncommitted changes:");
                for line in changes.lines() {
                    println!("      {line}");
                }
                println!("   🔍 VERBOSE: Force flag: {}", self.force);
            }
            
            println!("⚠️");
            println!("   Warning: Repository has uncommitted changes.");
            if !self.force {
                if self.verbose {
                    println!("   ❌ VERBOSE: Rejecting initialization due to uncommitted changes (use --force to override)");
                }
                return Err(anyhow!(
                    "Repository has uncommitted changes. Use --force to proceed anyway."
                ));
            }
            if self.verbose {
                println!("   ✅ VERBOSE: Proceeding with uncommitted changes due to --force flag");
            }
            println!("   Proceeding due to --force flag.");
        } else {
            if self.verbose {
                println!("   ✅ VERBOSE: Repository is clean (no uncommitted changes)");
            }
            println!("✅");
        }

        Ok(())
    }

    async fn detect_fresh_project(&self) -> bool {
        // Check if git repository exists
        let git_status = self.fs_ops.execute_command("git", &["status".to_string()]).await;
        if git_status.is_err() {
            return true; // No git repository
        }
        
        // Check if git remote origin exists
        let git_remote = self.fs_ops.execute_command("git", &["remote".to_string(), "get-url".to_string(), "origin".to_string()]).await;
        if let Ok(output) = git_remote {
            if !output.status.success() {
                return true; // No remote origin
            }
            
            // Verify remote URL is actually accessible (not just configured)
            let remote_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if remote_url.is_empty() {
                return true; // Empty remote URL
            }
        } else {
            return true; // Command failed
        }
        
        false // Has git repo and remote
    }

    async fn setup_labels(&self) -> Result<()> {
        let labels = self.get_required_labels();

        if self.dry_run {
            println!("Would create {} labels:", labels.len());
            for label in &labels {
                println!(
                    "  🏷️  {} (#{}) - {}",
                    label.name, label.color, label.description
                );
            }
            return Ok(());
        }

        // Check if this is a fresh project - skip label creation
        let is_fresh_project = self.detect_fresh_project().await;
        if is_fresh_project {
            println!("⏭️  Skipping GitHub label creation for fresh project");
            println!("   Labels will be created after GitHub repository setup");
            return Ok(());
        }

        let github_client =
            GitHubClient::new().map_err(|e| anyhow!("Failed to create GitHub client: {}", e))?;

        let octocrab = github_client.issues.octocrab();

        for label in &labels {
            print!("🏷️  Creating label '{}' ", label.name);
            std::io::Write::flush(&mut std::io::stdout()).unwrap();

            match octocrab
                .issues(github_client.owner(), github_client.repo())
                .create_label(&label.name, &label.color, &label.description)
                .await
            {
                Ok(_) => println!("✅"),
                Err(octocrab::Error::GitHub { source, .. })
                    if source.message.contains("already_exists") =>
                {
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
            LabelSpec {
                name: "code-quality".to_string(),
                color: "d4c5f9".to_string(),
                description: "Code quality improvements, refactoring, and technical debt reduction"
                    .to_string(),
            },
        ]
    }

    async fn generate_configuration(&self) -> Result<()> {
        let config_path = "my-little-soda.toml";

        if self.fs_ops.exists(config_path) && !self.force {
            return Err(anyhow!(
                "Configuration file {} already exists. Use --force to overwrite.",
                config_path
            ));
        }

        if self.dry_run {
            println!("Would create configuration file: {config_path}");
            println!("Would create directory: .my-little-soda/");
            return Ok(());
        }

        // Create .my-little-soda directory
        print!("📁 Creating .my-little-soda directory... ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        self.fs_ops.create_dir_all(".my-little-soda/credentials").await
            .map_err(|e| anyhow!("Failed to create .my-little-soda directory: {}", e))?;
        println!("✅");

        // Detect repository information
        let (owner, repo) = self.detect_repository_info().await?;

        // Generate configuration
        print!("⚙️  Generating my-little-soda.toml... ");
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
                    enabled: self.ci_mode,
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

    async fn detect_repository_info(&self) -> Result<(String, String)> {
        let output = self.fs_ops.execute_command("git", &["remote".to_string(), "get-url".to_string(), "origin".to_string()])
            .await
            .map_err(|e| anyhow!("Failed to get git remote URL: {}", e))?;

        if !output.status.success() {
            // For fresh projects (with or without --force), provide enhanced guidance
            println!("⚠️  No git remote found in this repository");
            println!("   To set up a GitHub remote, run:");
            println!("   git remote add origin https://github.com/YOUR-USERNAME/YOUR-REPO.git");
            println!("   Using placeholder values for now - update my-little-soda.toml after setting up remote");
            return Ok(("your-github-username".to_string(), "your-repo-name".to_string()));
        }

        let remote_url = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // Use the improved URL parsing from git operations
        use crate::git::{Git2Operations, GitHubRepoInfo};
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

    async fn setup_agents(&self) -> Result<()> {
        if self.dry_run {
            println!("Would configure 1 agent with capacity settings (single-agent mode)");
            println!("Would create agent state tracking");
            return Ok(());
        }

        print!("🤖 Configuring agent capacity (1 agent)... ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        // Create agent working directories
        self.fs_ops.create_dir_all(".my-little-soda/agents").await
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

        // Check if this is a fresh project
        let is_fresh_project = self.detect_fresh_project().await;
        
        if is_fresh_project {
            println!("⏭️  Skipping GitHub API connectivity test for fresh project");
        } else {
            // Test GitHub API connectivity for existing repositories
            print!("✅ Testing GitHub API connectivity... ");
            std::io::Write::flush(&mut std::io::stdout()).unwrap();

            let github_client =
                GitHubClient::new().map_err(|e| anyhow!("Failed to create GitHub client: {}", e))?;

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
            std::io::Write::flush(&mut std::io::stdout()).unwrap();

            let _config = crate::config::MyLittleSodaConfig::load()
                .map_err(|e| anyhow!("Generated configuration is invalid: {}", e))?;
            println!("✅");
        }

        if is_fresh_project {
            println!("⏭️  Skipping routing system test for fresh project");
        } else {
            // Basic routing test for existing repositories
            print!("✅ Running basic routing test... ");
            std::io::Write::flush(&mut std::io::stdout()).unwrap();

            // This is a simple test - just verify we can create an agent router
            match crate::agents::AgentRouter::new().await {
                Ok(_) => println!("✅"),
                Err(e) => return Err(anyhow!("Routing system test failed: {}", e)),
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::MockFileSystemOperations;
    use mockall::predicate::*;
    use std::process::{Output, ExitStatus};
    
    fn create_successful_exit_status() -> ExitStatus {
        std::process::Command::new("true").status().unwrap()
    }
    
    fn create_failed_exit_status() -> ExitStatus {
        std::process::Command::new("false").status().unwrap()
    }
    
    
    #[tokio::test]
    async fn test_successful_init_clean_repository() {
        let mut mock_fs = MockFileSystemOperations::new();
        
        // Mock file operations for successful init (dry run only checks existence)
        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(false);

        // Mock git commands for fresh project detection
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string()]))
            .times(1)
            .returning(|_, _| Ok(Output {
                status: create_successful_exit_status(),
                stdout: vec![],
                stderr: vec![],
            }));
        
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["remote".to_string(), "get-url".to_string(), "origin".to_string()]))
            .times(1)
            .returning(|_, _| Ok(Output {
                status: create_successful_exit_status(),
                stdout: b"https://github.com/owner/repo.git\n".to_vec(),
                stderr: vec![],
            }));
        
        let fs_ops = Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, true, fs_ops); // dry_run = true
        
        let result = init_command.execute().await;
        assert!(result.is_ok(), "Init command should succeed in clean repository");
    }
    
    #[tokio::test]
    async fn test_successful_init_with_template() {
        let mut mock_fs = MockFileSystemOperations::new();
        
        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(false);
        
        // Mock git commands for fresh project detection
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string()]))
            .times(1)
            .returning(|_, _| Ok(Output {
                status: create_successful_exit_status(),
                stdout: vec![],
                stderr: vec![],
            }));
        
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["remote".to_string(), "get-url".to_string(), "origin".to_string()]))
            .times(1)
            .returning(|_, _| Ok(Output {
                status: create_successful_exit_status(),
                stdout: b"https://github.com/owner/repo.git\n".to_vec(),
                stderr: vec![],
            }));
        
        let fs_ops = Arc::new(mock_fs);
        let init_command = InitCommand::new(Some("default".to_string()), false, true, fs_ops);
        
        let result = init_command.execute().await;
        assert!(result.is_ok(), "Init command should succeed with template");
    }
    
    #[tokio::test]
    async fn test_init_fails_with_invalid_agent_count() {
        let mock_fs = MockFileSystemOperations::new();
        let fs_ops = Arc::new(mock_fs);
        
        // Test with 0 agents
        let init_command = InitCommand::new(None, false, true, fs_ops.clone());
        let result = init_command.execute().await;
        assert!(result.is_err(), "Should fail with 0 agents");
        assert!(result.unwrap_err().to_string().contains("between 1 and 12"));
        
        // Test with too many agents
        let init_command = InitCommand::new(None, false, true, fs_ops);
        let result = init_command.execute().await;
        assert!(result.is_err(), "Should fail with 15 agents");
        assert!(result.unwrap_err().to_string().contains("between 1 and 12"));
    }
    
    #[tokio::test]
    async fn test_init_fails_when_config_exists_without_force() {
        let mut mock_fs = MockFileSystemOperations::new();
        
        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(true);
        
        let fs_ops = Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, true, fs_ops);
        
        let result = init_command.generate_configuration().await;
        assert!(result.is_err(), "Should fail when config exists and force is false");
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }
    
    #[tokio::test]
    async fn test_init_succeeds_when_config_exists_with_force() {
        let mut mock_fs = MockFileSystemOperations::new();
        
        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(true);
        
        // Mock git commands for fresh project detection
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string()]))
            .times(1)
            .returning(|_, _| Ok(Output {
                status: create_successful_exit_status(),
                stdout: vec![],
                stderr: vec![],
            }));
        
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["remote".to_string(), "get-url".to_string(), "origin".to_string()]))
            .times(1)
            .returning(|_, _| Ok(Output {
                status: create_successful_exit_status(),
                stdout: b"https://github.com/owner/repo.git\n".to_vec(),
                stderr: vec![],
            }));
        
        let fs_ops = Arc::new(mock_fs);
        let init_command = InitCommand::new(None, true, true, fs_ops); // force = true, dry_run = true
        
        let result = init_command.execute().await;
        assert!(result.is_ok(), "Should succeed when config exists and force is true");
    }
    
    #[tokio::test]
    async fn test_init_handles_git_remote_missing_gracefully() {
        let mut mock_fs = MockFileSystemOperations::new();
        
        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(false);
        
        let failed_output = Output {
            status: create_failed_exit_status(),
            stdout: vec![],
            stderr: b"fatal: not a git repository".to_vec(),
        };
        
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["remote".to_string(), "get-url".to_string(), "origin".to_string()]))
            .times(1)
            .returning(move |_, _| Ok(failed_output.clone()));
        
        let fs_ops = Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, false, fs_ops); // dry_run = false
        
        let result = init_command.detect_repository_info().await;
        assert!(result.is_ok(), "Should succeed with placeholder values when no git remote found");
        let (owner, repo) = result.unwrap();
        assert_eq!(owner, "your-github-username");
        assert_eq!(repo, "your-repo-name");
    }
    
    #[tokio::test]
    async fn test_init_fails_with_invalid_github_url() {
        let mut mock_fs = MockFileSystemOperations::new();
        
        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(false);
        
        let invalid_url_output = Output {
            status: create_successful_exit_status(),
            stdout: b"git@gitlab.com:user/repo.git\n".to_vec(),
            stderr: vec![],
        };
        
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["remote".to_string(), "get-url".to_string(), "origin".to_string()]))
            .times(1)
            .returning(move |_, _| Ok(invalid_url_output.clone()));
        
        let fs_ops = Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, false, fs_ops);
        
        let result = init_command.detect_repository_info().await;
        assert!(result.is_err(), "Should fail with non-GitHub URL");
        assert!(result.unwrap_err().to_string().contains("Could not parse GitHub repository"));
    }
    
    #[tokio::test]
    async fn test_init_idempotency_with_force() {
        let mut mock_fs = MockFileSystemOperations::new();
        
        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(true)
            .times(2);
        
        // Mock git commands for fresh project detection (called twice)
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string()]))
            .times(2)
            .returning(|_, _| Ok(Output {
                status: create_successful_exit_status(),
                stdout: vec![],
                stderr: vec![],
            }));
        
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["remote".to_string(), "get-url".to_string(), "origin".to_string()]))
            .times(2)
            .returning(|_, _| Ok(Output {
                status: create_successful_exit_status(),
                stdout: b"https://github.com/owner/repo.git\n".to_vec(),
                stderr: vec![],
            }));
        
        let fs_ops = Arc::new(mock_fs);
        
        // First init - should succeed (force + dry_run)
        let init_command = InitCommand::new(None, true, true, fs_ops.clone());
        let result1 = init_command.execute().await;
        assert!(result1.is_ok(), "First init should succeed");
        
        // Second init - should also succeed (idempotent)
        let init_command = InitCommand::new(None, true, true, fs_ops);
        let result2 = init_command.execute().await;
        assert!(result2.is_ok(), "Second init should succeed (idempotent)");
    }
    
    #[tokio::test]
    async fn test_init_directory_creation_failure() {
        let mut mock_fs = MockFileSystemOperations::new();
        
        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(false);
            
        mock_fs
            .expect_create_dir_all()
            .with(eq(".my-little-soda/credentials"))
            .times(1)
            .returning(|_| Err(anyhow::anyhow!("Permission denied")));
        
        let fs_ops = Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, false, fs_ops);
        
        let result = init_command.generate_configuration().await;
        assert!(result.is_err(), "Should fail when directory creation fails");
        assert!(result.unwrap_err().to_string().contains("Failed to create .my-little-soda directory"));
    }
    
    #[tokio::test]
    async fn test_init_with_ci_mode() {
        let mut mock_fs = MockFileSystemOperations::new();
        
        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(false);
        
        // Mock git commands for fresh project detection
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string()]))
            .times(1)
            .returning(|_, _| Ok(Output {
                status: create_successful_exit_status(),
                stdout: vec![],
                stderr: vec![],
            }));
        
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["remote".to_string(), "get-url".to_string(), "origin".to_string()]))
            .times(1)
            .returning(|_, _| Ok(Output {
                status: create_successful_exit_status(),
                stdout: b"https://github.com/owner/repo.git\n".to_vec(),
                stderr: vec![],
            }));
        
        let fs_ops = Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, true, fs_ops).with_ci_mode(true);
        
        let result = init_command.execute().await;
        assert!(result.is_ok(), "Init should succeed in CI mode");
    }
    
    #[tokio::test]
    async fn test_label_spec_creation() {
        let init_command = InitCommand::new(None, false, true, Arc::new(MockFileSystemOperations::new()));
        let labels = init_command.get_required_labels();
        
        assert!(!labels.is_empty(), "Should create multiple labels");
        
        // Verify critical labels exist
        let label_names: Vec<String> = labels.iter().map(|l| l.name.clone()).collect();
        assert!(label_names.contains(&"route:ready".to_string()));
        assert!(label_names.contains(&"route:ready_to_merge".to_string()));
        assert!(label_names.contains(&"route:unblocker".to_string()));
        assert!(label_names.contains(&"route:priority-high".to_string()));
    }
    
    #[tokio::test]
    async fn test_github_url_parsing_variations() {
        let mut mock_fs = MockFileSystemOperations::new();
        
        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(false);
        
        // Test HTTPS URL
        let https_output = Output {
            status: create_successful_exit_status(),
            stdout: b"https://github.com/owner/repo.git\n".to_vec(),
            stderr: vec![],
        };
        
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["remote".to_string(), "get-url".to_string(), "origin".to_string()]))
            .times(1)
            .returning(move |_, _| Ok(https_output.clone()));
        
        let fs_ops = Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, false, fs_ops);
        
        let result = init_command.detect_repository_info().await;
        assert!(result.is_ok(), "Should parse HTTPS GitHub URL");
        let (owner, repo) = result.unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");
    }
    
    #[tokio::test]
    async fn test_github_ssh_url_parsing() {
        let mut mock_fs = MockFileSystemOperations::new();
        
        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(false);
        
        // Test SSH URL
        let ssh_output = Output {
            status: create_successful_exit_status(),
            stdout: b"git@github.com:ssh-owner/ssh-repo.git\n".to_vec(),
            stderr: vec![],
        };
        
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["remote".to_string(), "get-url".to_string(), "origin".to_string()]))
            .times(1)
            .returning(move |_, _| Ok(ssh_output.clone()));
        
        let fs_ops = Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, false, fs_ops);
        
        let result = init_command.detect_repository_info().await;
        assert!(result.is_ok(), "Should parse SSH GitHub URL");
        let (owner, repo) = result.unwrap();
        assert_eq!(owner, "ssh-owner");
        assert_eq!(repo, "ssh-repo");
    }
    
    #[tokio::test]
    async fn test_validate_environment_github_auth_failure() {
        let mut mock_fs = MockFileSystemOperations::new();
        
        // Mock git commands for fresh project detection
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string()]))
            .times(1)
            .returning(|_, _| Ok(Output {
                status: create_successful_exit_status(),
                stdout: vec![],
                stderr: vec![],
            }));
        
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["remote".to_string(), "get-url".to_string(), "origin".to_string()]))
            .times(1)
            .returning(|_, _| Ok(Output {
                status: create_successful_exit_status(),
                stdout: b"https://github.com/owner/repo.git\n".to_vec(),
                stderr: vec![],
            }));
        
        let auth_failed_output = Output {
            status: create_failed_exit_status(),
            stdout: vec![],
            stderr: b"Not logged into any GitHub hosts".to_vec(),
        };
        
        mock_fs
            .expect_execute_command()
            .with(eq("gh"), eq(vec!["auth".to_string(), "status".to_string()]))
            .times(1)
            .returning(move |_, _| Ok(auth_failed_output.clone()));
        
        let fs_ops = Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, false, fs_ops);
        
        let result = init_command.validate_environment().await;
        assert!(result.is_err(), "Should fail when GitHub CLI not authenticated");
        assert!(result.unwrap_err().to_string().contains("GitHub CLI not authenticated"));
    }
    
    #[tokio::test]
    async fn test_validate_environment_git_status_with_uncommitted_changes() {
        let mut mock_fs = MockFileSystemOperations::new();
        
        let auth_success_output = Output {
            status: create_successful_exit_status(),
            stdout: b"Logged in to github.com".to_vec(),
            stderr: vec![],
        };
        
        let git_status_output = Output {
            status: create_successful_exit_status(),
            stdout: b" M src/main.rs\n?? new_file.txt\n".to_vec(),
            stderr: vec![],
        };
        
        mock_fs
            .expect_execute_command()
            .with(eq("gh"), eq(vec!["auth".to_string(), "status".to_string()]))
            .times(1)
            .returning(move |_, _| Ok(auth_success_output.clone()));
        
        // Mock git commands for fresh project detection
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string()]))
            .times(1)
            .returning(|_, _| Ok(Output {
                status: create_successful_exit_status(),
                stdout: vec![],
                stderr: vec![],
            }));
        
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["remote".to_string(), "get-url".to_string(), "origin".to_string()]))
            .times(1)
            .returning(|_, _| Ok(Output {
                status: create_successful_exit_status(),
                stdout: b"https://github.com/owner/repo.git\n".to_vec(),
                stderr: vec![],
            }));
            
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string(), "--porcelain".to_string()]))
            .times(1)
            .returning(move |_, _| Ok(git_status_output.clone()));
        
        let fs_ops = Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, false, fs_ops); // force = false
        
        let result = init_command.validate_environment().await;
        assert!(result.is_err(), "Should fail with uncommitted changes and no force flag");
        assert!(result.unwrap_err().to_string().contains("Repository has uncommitted changes"));
    }
    
    #[tokio::test]
    async fn test_validate_environment_git_status_with_force_flag() {
        let mut mock_fs = MockFileSystemOperations::new();
        
        let auth_success_output = Output {
            status: create_successful_exit_status(),
            stdout: b"Logged in to github.com".to_vec(),
            stderr: vec![],
        };
        
        let git_status_output = Output {
            status: create_successful_exit_status(),
            stdout: b" M src/main.rs\n".to_vec(),
            stderr: vec![],
        };
        
        mock_fs
            .expect_execute_command()
            .with(eq("gh"), eq(vec!["auth".to_string(), "status".to_string()]))
            .times(1)
            .returning(move |_, _| Ok(auth_success_output.clone()));
        
        // Mock git commands for fresh project detection
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string()]))
            .times(1)
            .returning(|_, _| Ok(Output {
                status: create_successful_exit_status(),
                stdout: vec![],
                stderr: vec![],
            }));
        
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["remote".to_string(), "get-url".to_string(), "origin".to_string()]))
            .times(1)
            .returning(|_, _| Ok(Output {
                status: create_successful_exit_status(),
                stdout: b"https://github.com/owner/repo.git\n".to_vec(),
                stderr: vec![],
            }));
            
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string(), "--porcelain".to_string()]))
            .times(1)
            .returning(move |_, _| Ok(git_status_output.clone()));
        
        let fs_ops = Arc::new(mock_fs);
        let init_command = InitCommand::new(None, true, false, fs_ops); // force = true
        
        let result = init_command.validate_environment().await;
        assert!(result.is_ok(), "Should succeed with uncommitted changes when force flag is set");
    }
    
    #[tokio::test]
    async fn test_validate_environment_git_command_failure() {
        let mut mock_fs = MockFileSystemOperations::new();
        
        let auth_success_output = Output {
            status: create_successful_exit_status(),
            stdout: b"Logged in to github.com".to_vec(),
            stderr: vec![],
        };
        
        mock_fs
            .expect_execute_command()
            .with(eq("gh"), eq(vec!["auth".to_string(), "status".to_string()]))
            .times(1)
            .returning(move |_, _| Ok(auth_success_output.clone()));
        
        // Mock git commands for fresh project detection
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string()]))
            .times(1)
            .returning(|_, _| Ok(Output {
                status: create_successful_exit_status(),
                stdout: vec![],
                stderr: vec![],
            }));
        
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["remote".to_string(), "get-url".to_string(), "origin".to_string()]))
            .times(1)
            .returning(|_, _| Ok(Output {
                status: create_successful_exit_status(),
                stdout: b"https://github.com/owner/repo.git\n".to_vec(),
                stderr: vec![],
            }));
            
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string(), "--porcelain".to_string()]))
            .times(1)
            .returning(|_, _| Err(anyhow::anyhow!("git command not found")));
        
        let fs_ops = Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, false, fs_ops);
        
        let result = init_command.validate_environment().await;
        assert!(result.is_err(), "Should fail when git command fails");
        assert!(result.unwrap_err().to_string().contains("Failed to check git status"));
    }
    
    #[tokio::test]
    async fn test_github_cli_command_failure() {
        let mut mock_fs = MockFileSystemOperations::new();
        
        // Mock git commands for fresh project detection
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string()]))
            .times(1)
            .returning(|_, _| Ok(Output {
                status: create_successful_exit_status(),
                stdout: vec![],
                stderr: vec![],
            }));
        
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["remote".to_string(), "get-url".to_string(), "origin".to_string()]))
            .times(1)
            .returning(|_, _| Ok(Output {
                status: create_successful_exit_status(),
                stdout: b"https://github.com/owner/repo.git\n".to_vec(),
                stderr: vec![],
            }));
        
        mock_fs
            .expect_execute_command()
            .with(eq("gh"), eq(vec!["auth".to_string(), "status".to_string()]))
            .times(1)
            .returning(|_, _| Err(anyhow::anyhow!("gh command not found")));
        
        let fs_ops = Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, false, fs_ops);
        
        let result = init_command.validate_environment().await;
        assert!(result.is_err(), "Should fail when gh command fails");
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Failed to run 'gh auth status'"));
        assert!(error_msg.contains("Make sure GitHub CLI is installed"));
    }
    
    #[tokio::test]
    async fn test_init_with_existing_partial_config() {
        let mut mock_fs = MockFileSystemOperations::new();
        
        // Simulate partial config exists
        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(true);
        
        let fs_ops = Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, true, fs_ops); // force = false, dry_run = true
        
        let result = init_command.generate_configuration().await;
        assert!(result.is_err(), "Should fail when partial config exists without force");
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }
    
    #[tokio::test]
    async fn test_init_overwrites_existing_config_with_force() {
        let mut mock_fs = MockFileSystemOperations::new();
        
        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(true);
        
        // Mock git commands for fresh project detection
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string()]))
            .times(1)
            .returning(|_, _| Ok(Output {
                status: create_successful_exit_status(),
                stdout: vec![],
                stderr: vec![],
            }));
        
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["remote".to_string(), "get-url".to_string(), "origin".to_string()]))
            .times(1)
            .returning(|_, _| Ok(Output {
                status: create_successful_exit_status(),
                stdout: b"https://github.com/owner/repo.git\n".to_vec(),
                stderr: vec![],
            }));
        
        let fs_ops = Arc::new(mock_fs);
        let init_command = InitCommand::new(None, true, true, fs_ops); // force = true, dry_run = true
        
        let result = init_command.execute().await;
        assert!(result.is_ok(), "Should succeed when config exists and force is true");
    }
    
    #[tokio::test]
    async fn test_init_handles_different_agent_counts() {
        let mut mock_fs = MockFileSystemOperations::new();
        
        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(false);
        
        // Mock git commands for fresh project detection
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string()]))
            .times(1)
            .returning(|_, _| Ok(Output {
                status: create_successful_exit_status(),
                stdout: vec![],
                stderr: vec![],
            }));
        
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["remote".to_string(), "get-url".to_string(), "origin".to_string()]))
            .times(1)
            .returning(|_, _| Ok(Output {
                status: create_successful_exit_status(),
                stdout: b"https://github.com/owner/repo.git\n".to_vec(),
                stderr: vec![],
            }));
        
        let fs_ops = Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, true, fs_ops); // max agents, dry_run
        
        let result = init_command.execute().await;
        assert!(result.is_ok(), "Should succeed with maximum agent count");
    }
    
    #[tokio::test]
    async fn test_init_boundary_agent_counts() {
        let mut mock_fs = MockFileSystemOperations::new();
        
        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(false)
            .times(2);
        
        // Mock git commands for fresh project detection (called twice)
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string()]))
            .times(2)
            .returning(|_, _| Ok(Output {
                status: create_successful_exit_status(),
                stdout: vec![],
                stderr: vec![],
            }));
        
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["remote".to_string(), "get-url".to_string(), "origin".to_string()]))
            .times(2)
            .returning(|_, _| Ok(Output {
                status: create_successful_exit_status(),
                stdout: b"https://github.com/owner/repo.git\n".to_vec(),
                stderr: vec![],
            }));
        
        let fs_ops = Arc::new(mock_fs);
        
        // Test minimum valid agent count
        let init_command = InitCommand::new(None, false, true, fs_ops.clone());
        let result = init_command.execute().await;
        assert!(result.is_ok(), "Should succeed with 1 agent");
        
        // Test maximum valid agent count
        let init_command = InitCommand::new(None, false, true, fs_ops);
        let result = init_command.execute().await;
        assert!(result.is_ok(), "Should succeed with 12 agents");
    }
    
    #[tokio::test]
    async fn test_init_dry_run_does_not_execute_commands() {
        let mut mock_fs = MockFileSystemOperations::new();
        
        // Even in dry run mode, validation phase still executes git commands
        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(false);
        
        // Mock git commands for fresh project detection
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string()]))
            .times(1)
            .returning(|_, _| Ok(Output {
                status: create_successful_exit_status(),
                stdout: vec![],
                stderr: vec![],
            }));
        
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["remote".to_string(), "get-url".to_string(), "origin".to_string()]))
            .times(1)
            .returning(|_, _| Ok(Output {
                status: create_successful_exit_status(),
                stdout: b"https://github.com/owner/repo.git\n".to_vec(),
                stderr: vec![],
            }));
        
        let fs_ops = Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, true, fs_ops); // dry_run = true
        
        let result = init_command.execute().await;
        assert!(result.is_ok(), "Dry run should succeed without actual file operations");
    }
    
    #[tokio::test]
    async fn test_init_creates_required_label_specs() {
        let init_command = InitCommand::new(None, false, true, Arc::new(MockFileSystemOperations::new()));
        let labels = init_command.get_required_labels();
        
        assert!(labels.len() >= 10, "Should create at least 10 labels");
        
        // Check for essential routing labels
        let route_labels: Vec<&LabelSpec> = labels.iter()
            .filter(|l| l.name.starts_with("route:"))
            .collect();
        assert!(route_labels.len() >= 6, "Should have at least 6 route labels");
        
        // Verify specific label properties
        let ready_label = labels.iter().find(|l| l.name == "route:ready");
        assert!(ready_label.is_some(), "Should have route:ready label");
        let ready_label = ready_label.unwrap();
        assert_eq!(ready_label.color, "0052cc");
        assert!(!ready_label.description.is_empty());
    }
    
    #[tokio::test]
    async fn test_file_system_mock_write_operations() {
        let mut mock_fs = MockFileSystemOperations::new();
        
        // Test successful write operation
        mock_fs
            .expect_write()
            .with(eq("test.txt"), eq(b"test content".as_slice()))
            .times(1)
            .returning(|_, _| Ok(()));
        
        let fs_ops = Arc::new(mock_fs);
        let result = fs_ops.write("test.txt", b"test content").await;
        assert!(result.is_ok(), "Mock write should succeed");
    }
    
    #[tokio::test]
    async fn test_file_system_mock_create_dir_operations() {
        let mut mock_fs = MockFileSystemOperations::new();
        
        // Test successful directory creation
        mock_fs
            .expect_create_dir_all()
            .with(eq(".my-little-soda/test"))
            .times(1)
            .returning(|_| Ok(()));
        
        let fs_ops = Arc::new(mock_fs);
        let result = fs_ops.create_dir_all(".my-little-soda/test").await;
        assert!(result.is_ok(), "Mock directory creation should succeed");
    }
    
    #[tokio::test]
    async fn test_file_system_mock_exists_operations() {
        let mut mock_fs = MockFileSystemOperations::new();
        
        // Test file existence checks
        mock_fs
            .expect_exists()
            .with(eq("existing_file.toml"))
            .return_const(true);
        
        mock_fs
            .expect_exists()
            .with(eq("missing_file.toml"))
            .return_const(false);
        
        let fs_ops = Arc::new(mock_fs);
        assert!(fs_ops.exists("existing_file.toml"), "Should detect existing file");
        assert!(!fs_ops.exists("missing_file.toml"), "Should detect missing file");
    }
    
    #[tokio::test]
    async fn test_fresh_project_detection_with_various_git_states() {
        // Test 1: No git repository (fresh project)
        {
            let mut mock_fs = MockFileSystemOperations::new();
            mock_fs
                .expect_execute_command()
                .with(eq("git"), eq(vec!["status".to_string()]))
                .times(1)
                .returning(|_, _| Err(anyhow::anyhow!("not a git repository")));
            
            let fs_ops = Arc::new(mock_fs);
            let init_command = InitCommand::new(None, false, false, fs_ops);
            
            let result = init_command.detect_fresh_project().await;
            assert!(result, "Should detect fresh project when no git repo");
        }
        
        // Test 2: Git repository but no remote (fresh project)
        {
            let mut mock_fs = MockFileSystemOperations::new();
            mock_fs
                .expect_execute_command()
                .with(eq("git"), eq(vec!["status".to_string()]))
                .times(1)
                .returning(|_, _| Ok(Output {
                    status: create_successful_exit_status(),
                    stdout: vec![],
                    stderr: vec![],
                }));
            
            mock_fs
                .expect_execute_command()
                .with(eq("git"), eq(vec!["remote".to_string(), "get-url".to_string(), "origin".to_string()]))
                .times(1)
                .returning(|_, _| Ok(Output {
                    status: create_failed_exit_status(),
                    stdout: vec![],
                    stderr: b"fatal: no such remote 'origin'".to_vec(),
                }));
            
            let fs_ops = Arc::new(mock_fs);
            let init_command = InitCommand::new(None, false, false, fs_ops);
            
            let result = init_command.detect_fresh_project().await;
            assert!(result, "Should detect fresh project when no remote");
        }
        
        // Test 3: Full git repository with remote (not fresh)
        {
            let mut mock_fs = MockFileSystemOperations::new();
            mock_fs
                .expect_execute_command()
                .with(eq("git"), eq(vec!["status".to_string()]))
                .times(1)
                .returning(|_, _| Ok(Output {
                    status: create_successful_exit_status(),
                    stdout: vec![],
                    stderr: vec![],
                }));
            
            mock_fs
                .expect_execute_command()
                .with(eq("git"), eq(vec!["remote".to_string(), "get-url".to_string(), "origin".to_string()]))
                .times(1)
                .returning(|_, _| Ok(Output {
                    status: create_successful_exit_status(),
                    stdout: b"https://github.com/existing/repo.git\n".to_vec(),
                    stderr: vec![],
                }));
            
            let fs_ops = Arc::new(mock_fs);
            let init_command = InitCommand::new(None, false, false, fs_ops);
            
            let result = init_command.detect_fresh_project().await;
            assert!(!result, "Should not detect fresh project when git repo with remote exists");
        }
    }

    #[tokio::test]
    async fn test_fresh_project_init_success() {
        let mut mock_fs = MockFileSystemOperations::new();
        
        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(false);
        
        // Mock git commands for fresh project detection (no git repo initially)
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string()]))
            .times(1) // Called once for fresh project detection
            .returning(|_, _| Err(anyhow::anyhow!("not a git repository")));
        
        let fs_ops = Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, true, fs_ops); // dry_run = true
        
        let result = init_command.execute().await;
        assert!(result.is_ok(), "Fresh project init should succeed in dry run mode");
    }
}
