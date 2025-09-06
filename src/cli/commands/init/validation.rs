/// Environment validation and authentication checks
///
/// Contains all validation logic for ensuring the system is ready for initialization.
/// This includes GitHub authentication, git repository state, and environment diagnostics.

use crate::github::client::GitHubClient;
use anyhow::{anyhow, Result};
use octocrab::Octocrab;
use std::io::Write;

use super::core::InitCommand;

/// Main environment validation entry point
pub async fn validate_environment(init_command: &InitCommand) -> Result<()> {
    if init_command.verbose {
        println!("🔍 VERBOSE: Starting detailed authentication validation...");
        println!();
    }

    // Check if this is a fresh project (no git repo or no remote)
    let is_fresh_project = detect_fresh_project(init_command).await;

    if is_fresh_project {
        println!(
            "🆕 Fresh project detected - initializing git repository and My Little Soda..."
        );
        println!();

        if !init_command.dry_run {
            // Initialize git repository
            print!("📦 Initializing git repository... ");
            std::io::stdout().flush().unwrap();

            let git_init_output = init_command
                .fs_ops()
                .execute_command("git", &["init".to_string()])
                .await
                .map_err(|e| {
                    anyhow!(
                        "Failed to initialize git repository: {}. Make sure git is installed.",
                        e
                    )
                })?;

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
    diagnose_authentication_environment(init_command).await?;
    validate_github_authentication(init_command).await?;
    validate_github_api_access(init_command).await?;

    // Check git repository state
    validate_git_state(init_command).await?;

    if init_command.verbose {
        println!(
            "🔍 VERBOSE: All authentication and validation checks completed successfully!"
        );
        println!();
    }

    Ok(())
}

/// Detect if this is a fresh project without git setup
pub async fn detect_fresh_project(init_command: &InitCommand) -> bool {
    // Check if git repository exists
    let git_status = init_command
        .fs_ops()
        .execute_command("git", &["status".to_string()])
        .await;
    if git_status.is_err() {
        return true; // No git repository
    }

    // Check if git remote origin exists
    let git_remote = init_command
        .fs_ops()
        .execute_command(
            "git",
            &[
                "remote".to_string(),
                "get-url".to_string(),
                "origin".to_string(),
            ],
        )
        .await;
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

/// Proactive authentication environment diagnostics
async fn diagnose_authentication_environment(init_command: &InitCommand) -> Result<()> {
    if init_command.verbose {
        println!("🔍 VERBOSE: Diagnosing authentication environment...");

        // Check for common environment variables
        let env_vars = [
            "MY_LITTLE_SODA_GITHUB_TOKEN",
            "GITHUB_TOKEN",
            "GH_TOKEN",
            "GITHUB_OWNER",
            "GITHUB_REPO",
        ];

        for var in env_vars {
            match std::env::var(var) {
                Ok(val)
                    if !val.is_empty() && !val.contains("YOUR_") && !val.contains("your-") =>
                {
                    let display_val = if var.contains("TOKEN") {
                        format!(
                            "{}...{}",
                            &val[..4.min(val.len())],
                            &val[val.len() - 4.min(val.len())..]
                        )
                    } else {
                        val
                    };
                    println!("   ✅ VERBOSE: {var} = {display_val}");
                }
                Ok(val) if val.contains("YOUR_") || val.contains("your-") => {
                    println!(
                        "   ⚠️  VERBOSE: {var} = {val} (placeholder value - needs to be set)"
                    );
                }
                Ok(_) => {
                    println!("   ⚠️  VERBOSE: {var} = (empty)");
                }
                Err(_) => {
                    println!("   ℹ️  VERBOSE: {var} = (not set)");
                }
            }
        }

        // Check for credential files
        let cred_files = [
            ".my-little-soda/credentials/github_token",
            ".my-little-soda/credentials/github_owner",
            ".my-little-soda/credentials/github_repo",
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
async fn validate_github_authentication(init_command: &InitCommand) -> Result<()> {
    if init_command.verbose {
        println!("🔍 VERBOSE: Checking GitHub CLI authentication...");
    }

    print!("✅ Verifying GitHub CLI authentication... ");
    std::io::stdout().flush().unwrap();

    if init_command.dry_run {
        println!("✅ (dry run mode)");
        if init_command.verbose {
            println!("   🔍 VERBOSE: Would execute: gh auth status");
            println!("   🔍 VERBOSE: Dry run mode - skipping actual command execution");
        }
        return Ok(());
    }

    // Check if gh CLI is available
    if init_command.verbose {
        println!();
        println!("   🔍 VERBOSE: Testing GitHub CLI availability...");
    }

    let gh_status_output = init_command
        .fs_ops()
        .execute_command("gh", &["auth".to_string(), "status".to_string()])
        .await
        .map_err(|e| {
            if init_command.verbose {
                println!("   ❌ VERBOSE: GitHub CLI command failed: {e}");
            }
            anyhow!(
                "Failed to run 'gh auth status': {}. Make sure GitHub CLI is installed.",
                e
            )
        })?;

    if init_command.verbose {
        println!(
            "   🔍 VERBOSE: gh auth status exit code: {}",
            gh_status_output.status.code().unwrap_or(-1)
        );
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
        if init_command.verbose {
            println!("   ❌ VERBOSE: GitHub CLI not authenticated");
            println!("   🔍 VERBOSE: Authentication failure details:");
            println!(
                "      Exit code: {}",
                gh_status_output.status.code().unwrap_or(-1)
            );
            if !stderr.is_empty() {
                println!("      Error output: {}", stderr.trim());
            }
        }

        // Provide enhanced error message with multiple authentication options
        let enhanced_error = if stderr.contains("Not logged in")
            || stderr.contains("not authenticated")
        {
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

    if init_command.verbose {
        println!("   ✅ VERBOSE: GitHub CLI is authenticated");
    }
    println!("✅");
    Ok(())
}

/// Comprehensive GitHub API access validation using the new client
async fn validate_github_api_access(init_command: &InitCommand) -> Result<()> {
    if init_command.verbose {
        println!("🔍 VERBOSE: Validating GitHub API access...");
    }

    print!("✅ Checking repository write permissions... ");
    std::io::stdout().flush().unwrap();

    if init_command.dry_run {
        println!("✅ (dry run mode)");
        if init_command.verbose {
            println!("   🔍 VERBOSE: Would test GitHub API connectivity");
            println!("   🔍 VERBOSE: Would validate repository permissions");
            println!("   🔍 VERBOSE: Dry run mode - skipping actual API calls");
        }
        return Ok(());
    }

    if init_command.verbose {
        println!();
        println!("   🔍 VERBOSE: Testing authentication methods in order:");
        println!("      1. MY_LITTLE_SODA_GITHUB_TOKEN environment variable");
        println!("      2. .my-little-soda/credentials/github_token file");
        println!("      3. GitHub CLI token (fallback)");
        println!();
    }

    // Test the enhanced GitHub client which now includes fallback mechanisms
    let github_client = match GitHubClient::with_verbose(init_command.verbose) {
        Ok(client) => {
            if init_command.verbose {
                println!("   ✅ VERBOSE: GitHub API client created successfully");
                println!(
                    "   🔍 VERBOSE: Repository: {}/{}",
                    client.owner(),
                    client.repo()
                );
                println!("   🔍 VERBOSE: Pre-flight validation passed (authentication + connectivity)");
            }
            client
        }
        Err(e) => {
            if init_command.verbose {
                println!("   ❌ VERBOSE: GitHub client creation failed: {e}");
            }
            return Err(anyhow!("Failed to create GitHub client: {}", e));
        }
    };

    // Additional repository permission validation
    if init_command.verbose {
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
                            return Ok(String::from_utf8_lossy(&output.stdout)
                                .trim()
                                .to_string());
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
            if init_command.verbose {
                println!("   ❌ VERBOSE: Repository access failed: {e}");
                println!(
                    "   🔍 VERBOSE: Repository: {}/{}",
                    github_client.owner(),
                    github_client.repo()
                );
            }

            // Provide enhanced error message based on error type
            let enhanced_error = match &e {
                octocrab::Error::GitHub { source, .. } => match source.status_code.as_u16() {
                    401 => "GitHub API authentication failed (HTTP 401).\n\
                             💡 Token is invalid or expired\n\
                             💡 Try: gh auth login\n\
                             💡 Or refresh token: gh auth refresh\n\
                             💡 Check token: gh auth token"
                        .to_string(),
                    403 => format!(
                        "GitHub API access forbidden (HTTP 403).\n\
                             💡 Token lacks required permissions\n\
                             💡 Repository: {}/{}\n\
                             💡 Required: 'repo' scope for private repositories\n\
                             💡 Create token: https://github.com/settings/tokens",
                        github_client.owner(),
                        github_client.repo()
                    ),
                    404 => format!(
                        "GitHub repository not found (HTTP 404).\n\
                             💡 Repository: {}/{}\n\
                             💡 Check if repository exists and is accessible\n\
                             💡 Verify GITHUB_OWNER and GITHUB_REPO settings\n\
                             💡 Check if repository is private and token has access",
                        github_client.owner(),
                        github_client.repo()
                    ),
                    _ => format!(
                        "GitHub API error (HTTP {}).\n\
                             💡 Message: {}\n\
                             💡 Repository: {}/{}\n\
                             💡 Check GitHub status: https://status.github.com",
                        source.status_code,
                        source.message,
                        github_client.owner(),
                        github_client.repo()
                    ),
                },
                octocrab::Error::Http { .. } => "Network error connecting to GitHub API.\n\
                     💡 Check internet connectivity\n\
                     💡 Test: curl -I https://api.github.com\n\
                     💡 Check firewall/proxy settings"
                    .to_string(),
                _ => format!(
                    "Failed to access repository {}/{}.\n\
                     💡 Check your GitHub token permissions\n\
                     💡 Verify repository exists and is accessible\n\
                     💡 Error: {}",
                    github_client.owner(),
                    github_client.repo(),
                    e
                ),
            };

            anyhow!(enhanced_error)
        })?;

    let has_write_permissions = repo
        .permissions
        .as_ref()
        .map(|p| p.admin || p.push)
        .unwrap_or(false);

    if init_command.verbose {
        if let Some(permissions) = &repo.permissions {
            println!("   🔍 VERBOSE: Repository permissions:");
            println!("      - Admin: {}", permissions.admin);
            println!("      - Push: {}", permissions.push);
            println!("      - Pull: {}", permissions.pull);
        }
        println!("   🔍 VERBOSE: Has write permissions: {has_write_permissions}");
    }

    if !has_write_permissions {
        if init_command.verbose {
            println!("   ❌ VERBOSE: Insufficient repository permissions");
            if let Some(permissions) = &repo.permissions {
                println!(
                    "   🔍 VERBOSE: Current permissions - Admin: {}, Push: {}, Pull: {}",
                    permissions.admin, permissions.push, permissions.pull
                );
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

    if init_command.verbose {
        println!("   ✅ VERBOSE: Repository permissions validated");
    }
    println!("✅");
    Ok(())
}

/// Validate git repository state
async fn validate_git_state(init_command: &InitCommand) -> Result<()> {
    if init_command.verbose {
        println!("🔍 VERBOSE: Checking git repository state...");
    }

    print!("✅ Checking git repository status... ");
    std::io::stdout().flush().unwrap();

    if init_command.dry_run {
        println!("✅ (dry run mode)");
        if init_command.verbose {
            println!("   🔍 VERBOSE: Would execute: git status --porcelain");
            println!("   🔍 VERBOSE: Dry run mode - skipping git state check");
        }
        return Ok(());
    }

    if init_command.verbose {
        println!();
        println!("   🔍 VERBOSE: Checking for uncommitted changes...");
    }

    let output = init_command
        .fs_ops()
        .execute_command("git", &["status".to_string(), "--porcelain".to_string()])
        .await
        .map_err(|e| {
            if init_command.verbose {
                println!("   ❌ VERBOSE: Git status command failed: {e}");
            }
            anyhow!("Failed to check git status: {}", e)
        })?;

    if init_command.verbose {
        println!(
            "   🔍 VERBOSE: git status --porcelain exit code: {}",
            output.status.code().unwrap_or(-1)
        );
    }

    if !output.stdout.is_empty() {
        if init_command.verbose {
            let changes = String::from_utf8_lossy(&output.stdout);
            println!("   🔍 VERBOSE: Found uncommitted changes:");
            for line in changes.lines() {
                println!("      {line}");
            }
            println!("   🔍 VERBOSE: Force flag: {}", init_command.force);
        }

        println!("⚠️");
        println!("   Warning: Repository has uncommitted changes.");
        if !init_command.force {
            if init_command.verbose {
                println!("   ❌ VERBOSE: Rejecting initialization due to uncommitted changes (use --force to override)");
            }
            return Err(anyhow!(
                "Repository has uncommitted changes. Use --force to proceed anyway."
            ));
        }
        if init_command.verbose {
            println!("   ✅ VERBOSE: Proceeding with uncommitted changes due to --force flag");
        }
        println!("   Proceeding due to --force flag.");
    } else {
        if init_command.verbose {
            println!("   ✅ VERBOSE: Repository is clean (no uncommitted changes)");
        }
        println!("✅");
    }

    Ok(())
}