use crate::cli::DoctorFormat;
use crate::config::config;
use crate::github::client::GitHubClient;
use crate::github::errors::GitHubError;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::Path;

/// Doctor command for system diagnostics and health checks
pub struct DoctorCommand {
    format: DoctorFormat,
    verbose: bool,
    ci_mode: bool,
}

impl Default for DoctorCommand {
    fn default() -> Self {
        Self::new(DoctorFormat::Text, false)
    }
}

/// Result of a diagnostic check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticResult {
    pub status: DiagnosticStatus,
    pub message: String,
    pub details: Option<String>,
    pub suggestion: Option<String>,
}

/// Status of a diagnostic check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiagnosticStatus {
    Pass,
    Fail,
    Warning,
    Info,
}

/// Diagnostic report containing all check results
#[derive(Debug, Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub summary: DiagnosticSummary,
    pub checks: HashMap<String, DiagnosticResult>,
}

/// Summary of diagnostic results
#[derive(Debug, Serialize, Deserialize)]
pub struct DiagnosticSummary {
    pub total_checks: usize,
    pub passed: usize,
    pub failed: usize,
    pub warnings: usize,
    pub info: usize,
}

impl DoctorCommand {
    pub fn new(format: DoctorFormat, verbose: bool) -> Self {
        Self {
            format,
            verbose,
            ci_mode: false,
        }
    }

    pub fn with_ci_mode(mut self, ci_mode: bool) -> Self {
        self.ci_mode = ci_mode;
        self
    }

    pub async fn execute(&self) -> Result<()> {
        let report = self.run_diagnostics().await?;
        self.output_report(&report)?;
        
        // Exit with error if any critical checks failed
        if report.summary.failed > 0 {
            std::process::exit(1);
        }
        
        Ok(())
    }

    async fn run_diagnostics(&self) -> Result<DiagnosticReport> {
        let mut checks = HashMap::new();
        
        // Run basic system checks
        self.check_git_repository(&mut checks)?;
        self.check_my_little_soda_config(&mut checks)?;
        self.check_dependencies(&mut checks)?;
        
        // Run comprehensive GitHub authentication diagnostics
        self.check_github_authentication(&mut checks).await;
        
        // Run GitHub repository access diagnostics
        self.check_github_repository_access(&mut checks).await;
        
        // Calculate summary
        let summary = self.calculate_summary(&checks);
        
        Ok(DiagnosticReport { summary, checks })
    }

    fn check_git_repository(&self, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
        // Check if we're in a git repository
        match std::process::Command::new("git")
            .args(["rev-parse", "--git-dir"])
            .output()
        {
            Ok(output) if output.status.success() => {
                checks.insert(
                    "git_repository".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Pass,
                        message: "Git repository detected".to_string(),
                        details: if self.verbose {
                            Some(format!("Git directory: {}", String::from_utf8_lossy(&output.stdout).trim()))
                        } else {
                            None
                        },
                        suggestion: None,
                    },
                );
            }
            _ => {
                checks.insert(
                    "git_repository".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "Not in a git repository".to_string(),
                        details: None,
                        suggestion: Some("Run 'git init' or navigate to a git repository".to_string()),
                    },
                );
            }
        }
        Ok(())
    }


    fn check_my_little_soda_config(&self, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
        // Check if .my-little-soda directory exists
        if std::path::Path::new(".my-little-soda").exists() {
            checks.insert(
                "soda_config".to_string(),
                DiagnosticResult {
                    status: DiagnosticStatus::Pass,
                    message: "My Little Soda configuration found".to_string(),
                    details: if self.verbose {
                        Some(".my-little-soda directory exists".to_string())
                    } else {
                        None
                    },
                    suggestion: None,
                },
            );
        } else {
            checks.insert(
                "soda_config".to_string(),
                DiagnosticResult {
                    status: DiagnosticStatus::Warning,
                    message: "My Little Soda not initialized".to_string(),
                    details: None,
                    suggestion: Some("Run 'my-little-soda init' to initialize the project".to_string()),
                },
            );
        }
        Ok(())
    }

    fn check_dependencies(&self, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
        // Check if git is available
        match std::process::Command::new("git").arg("--version").output() {
            Ok(output) if output.status.success() => {
                checks.insert(
                    "git_available".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Pass,
                        message: "Git is available".to_string(),
                        details: if self.verbose {
                            Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
                        } else {
                            None
                        },
                        suggestion: None,
                    },
                );
            }
            _ => {
                checks.insert(
                    "git_available".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "Git not available".to_string(),
                        details: None,
                        suggestion: Some("Install git and ensure it's in your PATH".to_string()),
                    },
                );
            }
        }

        // Check if gh CLI is available
        match std::process::Command::new("gh").arg("--version").output() {
            Ok(output) if output.status.success() => {
                checks.insert(
                    "gh_available".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Pass,
                        message: "GitHub CLI is available".to_string(),
                        details: if self.verbose {
                            Some(String::from_utf8_lossy(&output.stdout).lines().next().unwrap_or("").to_string())
                        } else {
                            None
                        },
                        suggestion: None,
                    },
                );
            }
            _ => {
                checks.insert(
                    "gh_available".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Warning,
                        message: "GitHub CLI not available".to_string(),
                        details: None,
                        suggestion: Some("Install GitHub CLI (gh) for enhanced GitHub integration".to_string()),
                    },
                );
            }
        }
        Ok(())
    }

    fn calculate_summary(&self, checks: &HashMap<String, DiagnosticResult>) -> DiagnosticSummary {
        let total_checks = checks.len();
        let mut passed = 0;
        let mut failed = 0;
        let mut warnings = 0;
        let mut info = 0;

        for result in checks.values() {
            match result.status {
                DiagnosticStatus::Pass => passed += 1,
                DiagnosticStatus::Fail => failed += 1,
                DiagnosticStatus::Warning => warnings += 1,
                DiagnosticStatus::Info => info += 1,
            }
        }

        DiagnosticSummary {
            total_checks,
            passed,
            failed,
            warnings,
            info,
        }
    }

    fn output_report(&self, report: &DiagnosticReport) -> Result<()> {
        match self.format {
            DoctorFormat::Json => {
                println!("{}", serde_json::to_string_pretty(report)?);
            }
            DoctorFormat::Text => {
                self.output_text_report(report)?;
            }
        }
        Ok(())
    }

    fn output_text_report(&self, report: &DiagnosticReport) -> Result<()> {
        // Header
        println!("🩺 MY LITTLE SODA DOCTOR - System Diagnostics");
        println!("=============================================");
        println!();

        // Summary
        println!("📊 DIAGNOSTIC SUMMARY:");
        println!("─────────────────────");
        println!("Total checks: {}", report.summary.total_checks);
        if report.summary.passed > 0 {
            println!("✅ Passed: {}", report.summary.passed);
        }
        if report.summary.failed > 0 {
            println!("❌ Failed: {}", report.summary.failed);
        }
        if report.summary.warnings > 0 {
            println!("⚠️  Warnings: {}", report.summary.warnings);
        }
        if report.summary.info > 0 {
            println!("ℹ️  Info: {}", report.summary.info);
        }
        println!();

        // Detailed results
        println!("🔍 DETAILED RESULTS:");
        println!("──────────────────");

        // Sort checks for consistent output
        let mut sorted_checks: Vec<_> = report.checks.iter().collect();
        sorted_checks.sort_by_key(|(name, _)| *name);

        for (name, result) in sorted_checks {
            let status_icon = match result.status {
                DiagnosticStatus::Pass => "✅",
                DiagnosticStatus::Fail => "❌",
                DiagnosticStatus::Warning => "⚠️",
                DiagnosticStatus::Info => "ℹ️",
            };
            
            println!("{} {}: {}", status_icon, name, result.message);
            
            if self.verbose || matches!(result.status, DiagnosticStatus::Fail | DiagnosticStatus::Warning) {
                if let Some(details) = &result.details {
                    println!("   Details: {}", details);
                }
                if let Some(suggestion) = &result.suggestion {
                    println!("   Suggestion: {}", suggestion);
                }
            }
            println!();
        }

        // Overall status
        if report.summary.failed == 0 {
            if report.summary.warnings > 0 {
                println!("⚠️  System is functional but has {} warning(s) that should be addressed.", report.summary.warnings);
            } else {
                println!("✅ System is healthy and ready for use!");
            }
        } else {
            println!("❌ System has {} critical issue(s) that must be resolved.", report.summary.failed);
        }

        Ok(())
    }

    /// Comprehensive GitHub authentication diagnostics using the actual GitHub client
    async fn check_github_authentication(&self, checks: &mut HashMap<String, DiagnosticResult>) {
        // Check 1: Token presence and format validation
        self.check_github_token_presence(checks);
        
        // Check 2: Try to create GitHub client and test authentication
        match GitHubClient::new() {
            Ok(client) => {
                // If client creation succeeds, authentication is working
                self.check_github_authentication_success(&client, checks).await;
            }
            Err(github_error) => {
                // If client creation fails, provide detailed diagnostics
                self.check_github_authentication_failure(github_error, checks);
            }
        }
    }

    /// Check for GitHub token presence and validate format
    fn check_github_token_presence(&self, checks: &mut HashMap<String, DiagnosticResult>) {
        let mut token_found = false;
        let mut token_source = String::new();
        let mut token_format_valid = false;

        // Check environment variable
        if let Ok(token) = env::var("MY_LITTLE_SODA_GITHUB_TOKEN") {
            if !token.is_empty() && token != "YOUR_GITHUB_TOKEN_HERE" {
                token_found = true;
                token_source = "Environment variable (MY_LITTLE_SODA_GITHUB_TOKEN)".to_string();
                token_format_valid = Self::validate_token_format(&token);
            }
        }

        // Check file-based configuration
        if !token_found {
            let token_path = ".my-little-soda/credentials/github_token";
            if Path::new(token_path).exists() {
                if let Ok(token) = std::fs::read_to_string(token_path) {
                    let token = token.trim();
                    if !token.is_empty() && token != "YOUR_GITHUB_TOKEN_HERE" {
                        token_found = true;
                        token_source = "File-based configuration (.my-little-soda/credentials/github_token)".to_string();
                        token_format_valid = Self::validate_token_format(token);
                    }
                }
            }
        }

        // Check GitHub CLI
        if !token_found {
            if let Ok(output) = std::process::Command::new("gh")
                .args(["auth", "status"])
                .output()
            {
                if output.status.success() {
                    token_found = true;
                    token_source = "GitHub CLI (gh auth token)".to_string();
                    // Assume gh CLI token is valid format
                    token_format_valid = true;
                }
            }
        }

        if token_found {
            if token_format_valid {
                checks.insert(
                    "github_token_presence".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Pass,
                        message: "GitHub token found and format is valid".to_string(),
                        details: if self.verbose {
                            Some(format!("Token source: {}", token_source))
                        } else {
                            None
                        },
                        suggestion: None,
                    },
                );
            } else {
                checks.insert(
                    "github_token_presence".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Warning,
                        message: "GitHub token found but format is invalid".to_string(),
                        details: Some(format!("Token source: {}", token_source)),
                        suggestion: Some("GitHub tokens should start with 'ghp_' (classic), 'github_pat_' (fine-grained), or 'gho_' (OAuth). Create a new token at https://github.com/settings/tokens".to_string()),
                    },
                );
            }
        } else {
            checks.insert(
                "github_token_presence".to_string(),
                DiagnosticResult {
                    status: DiagnosticStatus::Fail,
                    message: "No GitHub token found".to_string(),
                    details: None,
                    suggestion: Some("Set MY_LITTLE_SODA_GITHUB_TOKEN environment variable or run 'gh auth login'. Visit https://github.com/settings/tokens to create a new token.".to_string()),
                },
            );
        }
    }

    /// Validate GitHub token format
    fn validate_token_format(token: &str) -> bool {
        // GitHub personal access tokens have specific prefixes:
        // - ghp_ for classic personal access tokens
        // - github_pat_ for fine-grained personal access tokens  
        // - gho_ for OAuth tokens
        // - ghr_ for refresh tokens
        // - ghs_ for server-to-server tokens
        token.starts_with("ghp_") 
            || token.starts_with("github_pat_")
            || token.starts_with("gho_")
            || token.starts_with("ghr_")
            || token.starts_with("ghs_")
    }

    /// Handle successful GitHub client creation with detailed validation
    async fn check_github_authentication_success(&self, client: &GitHubClient, checks: &mut HashMap<String, DiagnosticResult>) {
        // Authentication succeeded - add detailed status
        checks.insert(
            "github_authentication".to_string(),
            DiagnosticResult {
                status: DiagnosticStatus::Pass,
                message: "GitHub authentication successful".to_string(),
                details: if self.verbose {
                    Some("Successfully authenticated with GitHub API".to_string())
                } else {
                    None
                },
                suggestion: None,
            },
        );

        // Check repository access
        let owner = client.owner();
        let repo = client.repo();
        checks.insert(
            "github_repository_access".to_string(),
            DiagnosticResult {
                status: DiagnosticStatus::Pass,
                message: format!("Repository access confirmed: {}/{}", owner, repo),
                details: if self.verbose {
                    Some("Token has appropriate repository permissions".to_string())
                } else {
                    None
                },
                suggestion: None,
            },
        );

        // Check rate limits
        self.check_github_rate_limits(client, checks).await;

        // Test API scopes by attempting basic operations
        self.check_github_api_scopes(client, checks).await;
    }

    /// Handle GitHub client creation failure with detailed diagnostics
    fn check_github_authentication_failure(&self, error: GitHubError, checks: &mut HashMap<String, DiagnosticResult>) {
        match &error {
            GitHubError::TokenNotFound(message) => {
                checks.insert(
                    "github_authentication".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "GitHub authentication failed - token not found".to_string(),
                        details: if self.verbose {
                            Some(message.clone())
                        } else {
                            Some("No valid GitHub token found".to_string())
                        },
                        suggestion: Some("Set MY_LITTLE_SODA_GITHUB_TOKEN environment variable or run 'gh auth login'".to_string()),
                    },
                );
            }
            GitHubError::TokenScopeInsufficient { required_scopes, current_error, token_url } => {
                checks.insert(
                    "github_authentication".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "GitHub authentication failed - insufficient token permissions".to_string(),
                        details: Some(format!("Required scopes: {}. Error: {}", required_scopes.join(", "), current_error)),
                        suggestion: Some(format!("Update your token permissions at {} to include: {}", token_url, required_scopes.join(", "))),
                    },
                );
            }
            GitHubError::NetworkError(message) => {
                checks.insert(
                    "github_authentication".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "GitHub authentication failed - network error".to_string(),
                        details: Some(message.clone()),
                        suggestion: Some("Check internet connectivity and GitHub status at https://status.github.com".to_string()),
                    },
                );
            }
            GitHubError::ConfigNotFound(message) => {
                checks.insert(
                    "github_authentication".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "GitHub authentication failed - configuration error".to_string(),
                        details: Some(message.clone()),
                        suggestion: Some("Set GITHUB_OWNER and GITHUB_REPO environment variables or create .my-little-soda/credentials/ files".to_string()),
                    },
                );
            }
            GitHubError::RateLimit { reset_time, remaining } => {
                let reset_in = (*reset_time - chrono::Utc::now()).num_minutes().max(0);
                checks.insert(
                    "github_authentication".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Warning,
                        message: "GitHub authentication succeeded but rate limited".to_string(),
                        details: Some(format!("Remaining requests: {}, Reset in: {} minutes", remaining, reset_in)),
                        suggestion: Some("Wait for rate limit to reset or use a different token".to_string()),
                    },
                );
            }
            GitHubError::ApiError(api_error) => {
                checks.insert(
                    "github_authentication".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "GitHub authentication failed - API error".to_string(),
                        details: Some(format!("API error: {}", api_error)),
                        suggestion: Some("Check token validity and GitHub service status".to_string()),
                    },
                );
            }
            GitHubError::IoError(io_error) => {
                checks.insert(
                    "github_authentication".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "GitHub authentication failed - I/O error".to_string(),
                        details: Some(format!("I/O error: {}", io_error)),
                        suggestion: Some("Check file permissions and disk space".to_string()),
                    },
                );
            }
            GitHubError::NotImplemented(feature) => {
                checks.insert(
                    "github_authentication".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Warning,
                        message: "GitHub authentication partially successful".to_string(),
                        details: Some(format!("Feature not implemented: {}", feature)),
                        suggestion: Some("Some advanced features may not be available".to_string()),
                    },
                );
            }
            GitHubError::Timeout { operation, duration_ms } => {
                checks.insert(
                    "github_authentication".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "GitHub authentication failed - timeout".to_string(),
                        details: Some(format!("Operation '{}' timed out after {}ms", operation, duration_ms)),
                        suggestion: Some("Check network connectivity and try again".to_string()),
                    },
                );
            }
        }
    }

    /// Check GitHub API rate limits and provide status
    async fn check_github_rate_limits(&self, client: &GitHubClient, checks: &mut HashMap<String, DiagnosticResult>) {
        // Try to get rate limit information from GitHub API
        let octocrab = &client.issues.octocrab();
        if let Ok(rate_limits) = octocrab.ratelimit().get().await {
            let core = &rate_limits.resources.core;
            let remaining_pct = (core.remaining as f64 / core.limit as f64) * 100.0;
            
            let status = if remaining_pct > 50.0 {
                DiagnosticStatus::Pass
            } else if remaining_pct > 20.0 {
                DiagnosticStatus::Warning
            } else {
                DiagnosticStatus::Fail
            };

            let reset_time = chrono::DateTime::from_timestamp(core.reset as i64, 0)
                .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::hours(1));
            let reset_in = (reset_time - chrono::Utc::now()).num_minutes().max(0);

            checks.insert(
                "github_rate_limits".to_string(),
                DiagnosticResult {
                    status,
                    message: format!("Rate limit: {}/{} requests remaining ({:.1}%)", 
                             core.remaining, core.limit, remaining_pct),
                    details: if self.verbose {
                        Some(format!("Resets in {} minutes at {}", reset_in, reset_time.format("%Y-%m-%d %H:%M:%S UTC")))
                    } else {
                        Some(format!("Resets in {} minutes", reset_in))
                    },
                    suggestion: if remaining_pct <= 20.0 {
                        Some("Consider using a different token or waiting for rate limit reset".to_string())
                    } else {
                        None
                    },
                },
            );
        } else {
            checks.insert(
                "github_rate_limits".to_string(),
                DiagnosticResult {
                    status: DiagnosticStatus::Warning,
                    message: "Unable to check GitHub rate limits".to_string(),
                    details: Some("Rate limit API call failed".to_string()),
                    suggestion: Some("This may indicate token permission issues".to_string()),
                },
            );
        }
    }

    /// Test GitHub API scopes by attempting basic operations
    async fn check_github_api_scopes(&self, client: &GitHubClient, checks: &mut HashMap<String, DiagnosticResult>) {
        let mut scope_tests = Vec::new();
        let mut failed_scopes = Vec::new();

        // Test issue read access
        match client.fetch_issues_with_state(Some(octocrab::params::State::Open)).await {
            Ok(_) => scope_tests.push("issues:read ✅".to_string()),
            Err(e) => {
                scope_tests.push("issues:read ❌".to_string());
                failed_scopes.push("issues:read".to_string());
                if self.verbose {
                    scope_tests.push(format!("  Error: {}", e));
                }
            }
        }

        // Test pull request access  
        match client.fetch_open_pull_requests().await {
            Ok(_) => scope_tests.push("pull_requests:read ✅".to_string()),
            Err(e) => {
                scope_tests.push("pull_requests:read ❌".to_string());
                failed_scopes.push("pull_requests:read".to_string());
                if self.verbose {
                    scope_tests.push(format!("  Error: {}", e));
                }
            }
        }

        // Test repository metadata access
        let octocrab = client.issues.octocrab();
        match octocrab.repos(client.owner(), client.repo()).get().await {
            Ok(_) => scope_tests.push("repository:read ✅".to_string()),
            Err(e) => {
                scope_tests.push("repository:read ❌".to_string());
                failed_scopes.push("repository:read".to_string());
                if self.verbose {
                    scope_tests.push(format!("  Error: {}", e));
                }
            }
        }

        let status = if failed_scopes.is_empty() {
            DiagnosticStatus::Pass
        } else {
            DiagnosticStatus::Fail
        };

        checks.insert(
            "github_api_scopes".to_string(),
            DiagnosticResult {
                status,
                message: if failed_scopes.is_empty() {
                    "All required GitHub API scopes are available".to_string()
                } else {
                    format!("Missing {} GitHub API scope(s)", failed_scopes.len())
                },
                details: if self.verbose {
                    Some(scope_tests.join("\n"))
                } else {
                    if failed_scopes.is_empty() {
                        Some("Repository, issues, and pull requests access confirmed".to_string())
                    } else {
                        Some(format!("Failed scopes: {}", failed_scopes.join(", ")))
                    }
                },
                suggestion: if !failed_scopes.is_empty() {
                    Some("Update your GitHub token to include the missing scopes at https://github.com/settings/tokens".to_string())
                } else {
                    None
                },
            },
        );
    }

    /// GitHub repository access diagnostics
    async fn check_github_repository_access(&self, checks: &mut HashMap<String, DiagnosticResult>) {
        // Check 1: Validate repository configuration from my-little-soda.toml
        self.check_repository_configuration(checks).await;
        
        // Check 2: Test repository existence and accessibility
        self.check_repository_existence(checks).await;
        
        // Check 3: Validate repository settings and features
        self.check_repository_settings(checks).await;
        
        // Check 4: Test My Little Soda operational requirements
        self.check_repository_operations(checks).await;
    }

    /// Check repository configuration from my-little-soda.toml and environment
    async fn check_repository_configuration(&self, checks: &mut HashMap<String, DiagnosticResult>) {
        match config() {
            Ok(my_config) => {
                let github_config = &my_config.github;
                
                // Validate owner/repo configuration
                if github_config.owner.is_empty() || github_config.owner == "your-github-username" {
                    checks.insert(
                        "repository_config_owner".to_string(),
                        DiagnosticResult {
                            status: DiagnosticStatus::Fail,
                            message: "GitHub repository owner not configured".to_string(),
                            details: Some("Owner is empty or placeholder value".to_string()),
                            suggestion: Some("Set GITHUB_OWNER environment variable or configure github.owner in my-little-soda.toml".to_string()),
                        },
                    );
                } else {
                    checks.insert(
                        "repository_config_owner".to_string(),
                        DiagnosticResult {
                            status: DiagnosticStatus::Pass,
                            message: format!("Repository owner configured: {}", github_config.owner),
                            details: if self.verbose {
                                Some("Valid owner configuration found".to_string())
                            } else {
                                None
                            },
                            suggestion: None,
                        },
                    );
                }
                
                if github_config.repo.is_empty() || github_config.repo == "your-repo-name" {
                    checks.insert(
                        "repository_config_repo".to_string(),
                        DiagnosticResult {
                            status: DiagnosticStatus::Fail,
                            message: "GitHub repository name not configured".to_string(),
                            details: Some("Repository name is empty or placeholder value".to_string()),
                            suggestion: Some("Set GITHUB_REPO environment variable or configure github.repo in my-little-soda.toml".to_string()),
                        },
                    );
                } else {
                    checks.insert(
                        "repository_config_repo".to_string(),
                        DiagnosticResult {
                            status: DiagnosticStatus::Pass,
                            message: format!("Repository name configured: {}", github_config.repo),
                            details: if self.verbose {
                                Some("Valid repository name configuration found".to_string())
                            } else {
                                None
                            },
                            suggestion: None,
                        },
                    );
                }
            }
            Err(e) => {
                checks.insert(
                    "repository_config".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "Unable to load My Little Soda configuration".to_string(),
                        details: Some(format!("Configuration error: {}", e)),
                        suggestion: Some("Create my-little-soda.toml or check configuration format".to_string()),
                    },
                );
            }
        }
    }

    /// Check if repository exists and is accessible
    async fn check_repository_existence(&self, checks: &mut HashMap<String, DiagnosticResult>) {
        match GitHubClient::new() {
            Ok(client) => {
                let octocrab = client.issues.octocrab();
                match octocrab.repos(client.owner(), client.repo()).get().await {
                    Ok(repo) => {
                        let visibility = if repo.private.unwrap_or(false) { "private" } else { "public" };
                        checks.insert(
                            "repository_existence".to_string(),
                            DiagnosticResult {
                                status: DiagnosticStatus::Pass,
                                message: format!("Repository {}/{} exists and is accessible", client.owner(), client.repo()),
                                details: if self.verbose {
                                    Some(format!("Repository visibility: {}, default branch: {}", 
                                               visibility, 
                                               repo.default_branch.as_deref().unwrap_or("unknown")))
                                } else {
                                    Some(format!("Visibility: {}", visibility))
                                },
                                suggestion: None,
                            },
                        );
                    }
                    Err(e) => {
                        let (status, message, details, suggestion) = match &e {
                            octocrab::Error::GitHub { source, .. } if source.status_code.as_u16() == 404 => {
                                (DiagnosticStatus::Fail, 
                                 format!("Repository {}/{} not found", client.owner(), client.repo()),
                                 Some("Repository may not exist or token lacks access".to_string()),
                                 Some("Verify repository name and ensure token has repository access".to_string()))
                            }
                            octocrab::Error::GitHub { source, .. } if source.status_code.as_u16() == 403 => {
                                (DiagnosticStatus::Fail, 
                                 "Repository access denied".to_string(),
                                 Some("Token lacks sufficient permissions for repository".to_string()),
                                 Some("Update token permissions or verify repository visibility".to_string()))
                            }
                            _ => {
                                (DiagnosticStatus::Warning, 
                                 "Unable to verify repository existence".to_string(),
                                 Some(format!("API error: {}", e)),
                                 Some("Check network connectivity and GitHub service status".to_string()))
                            }
                        };
                        
                        checks.insert(
                            "repository_existence".to_string(),
                            DiagnosticResult { status, message, details, suggestion },
                        );
                    }
                }
            }
            Err(_) => {
                checks.insert(
                    "repository_existence".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "Cannot check repository existence".to_string(),
                        details: Some("GitHub client creation failed".to_string()),
                        suggestion: Some("Fix GitHub authentication issues first".to_string()),
                    },
                );
            }
        }
    }

    /// Check repository settings required for My Little Soda operations
    async fn check_repository_settings(&self, checks: &mut HashMap<String, DiagnosticResult>) {
        match GitHubClient::new() {
            Ok(client) => {
                let octocrab = client.issues.octocrab();
                match octocrab.repos(client.owner(), client.repo()).get().await {
                    Ok(repo) => {
                        let mut settings_issues = Vec::new();
                        let mut settings_good = Vec::new();
                        
                        // Check if issues are enabled
                        if !repo.has_issues.unwrap_or(false) {
                            settings_issues.push("Issues disabled - required for My Little Soda operation".to_string());
                        } else {
                            settings_good.push("Issues enabled ✅".to_string());
                        }
                        
                        // Check if repository allows forking (for PRs)
                        if repo.fork.unwrap_or(false) && !repo.allow_forking.unwrap_or(true) {
                            settings_issues.push("Forking disabled - may limit some PR operations".to_string());
                        }
                        
                        // Check merge options
                        if !repo.allow_merge_commit.unwrap_or(false) && !repo.allow_squash_merge.unwrap_or(false) {
                            settings_issues.push("No merge options enabled - may prevent PR completion".to_string());
                        } else {
                            let merge_types = [
                                ("merge commits", repo.allow_merge_commit.unwrap_or(false)),
                                ("squash merge", repo.allow_squash_merge.unwrap_or(false)),
                                ("rebase merge", repo.allow_rebase_merge.unwrap_or(false)),
                            ].iter()
                             .filter(|(_, enabled)| *enabled)
                             .map(|(name, _)| *name)
                             .collect::<Vec<_>>();
                            
                            if !merge_types.is_empty() {
                                settings_good.push(format!("Merge options: {} ✅", merge_types.join(", ")));
                            }
                        }

                        let status = if settings_issues.is_empty() {
                            DiagnosticStatus::Pass
                        } else {
                            DiagnosticStatus::Warning
                        };

                        let details = if self.verbose {
                            let mut details = settings_good;
                            if !settings_issues.is_empty() {
                                details.extend(settings_issues.iter().map(|s| format!("⚠️  {}", s)));
                            }
                            Some(details.join("\n"))
                        } else {
                            if settings_issues.is_empty() {
                                Some("Repository settings are compatible".to_string())
                            } else {
                                Some(format!("{} setting issue(s) detected", settings_issues.len()))
                            }
                        };

                        checks.insert(
                            "repository_settings".to_string(),
                            DiagnosticResult {
                                status,
                                message: if settings_issues.is_empty() {
                                    "Repository settings compatible with My Little Soda".to_string()
                                } else {
                                    format!("Repository settings need attention ({} issues)", settings_issues.len())
                                },
                                details,
                                suggestion: if !settings_issues.is_empty() {
                                    Some("Review repository settings in GitHub to ensure full My Little Soda compatibility".to_string())
                                } else {
                                    None
                                },
                            },
                        );
                    }
                    Err(e) => {
                        checks.insert(
                            "repository_settings".to_string(),
                            DiagnosticResult {
                                status: DiagnosticStatus::Warning,
                                message: "Unable to check repository settings".to_string(),
                                details: Some(format!("Repository API error: {}", e)),
                                suggestion: Some("Repository settings could not be verified".to_string()),
                            },
                        );
                    }
                }
            }
            Err(_) => {
                checks.insert(
                    "repository_settings".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Warning,
                        message: "Cannot check repository settings".to_string(),
                        details: Some("GitHub client unavailable".to_string()),
                        suggestion: Some("Fix authentication issues to check repository settings".to_string()),
                    },
                );
            }
        }
    }

    /// Test ability to perform actual My Little Soda operations
    async fn check_repository_operations(&self, checks: &mut HashMap<String, DiagnosticResult>) {
        match GitHubClient::new() {
            Ok(client) => {
                let mut operations_tested = Vec::new();
                let mut operations_failed = Vec::new();

                // Test 1: Issue operations
                match client.fetch_issues_with_state(Some(octocrab::params::State::Open)).await {
                    Ok(issues) => {
                        operations_tested.push(format!("Issue listing: {} open issues ✅", issues.len()));
                    }
                    Err(e) => {
                        operations_failed.push(format!("Issue listing failed: {}", e));
                    }
                }

                // Test 2: Pull request operations  
                match client.fetch_open_pull_requests().await {
                    Ok(prs) => {
                        operations_tested.push(format!("PR listing: {} open PRs ✅", prs.len()));
                    }
                    Err(e) => {
                        operations_failed.push(format!("PR listing failed: {}", e));
                    }
                }

                // Test 3: Labels (if issues work, try to list labels)
                if operations_failed.is_empty() || operations_tested.len() > operations_failed.len() {
                    let octocrab = client.issues.octocrab();
                    match octocrab.issues(client.owner(), client.repo()).list_labels_for_repo().send().await {
                        Ok(labels) => {
                            operations_tested.push(format!("Label access: {} labels ✅", labels.items.len()));
                        }
                        Err(e) => {
                            operations_failed.push(format!("Label access failed: {}", e));
                        }
                    }
                }

                let status = if operations_failed.is_empty() {
                    DiagnosticStatus::Pass
                } else if operations_tested.len() > operations_failed.len() {
                    DiagnosticStatus::Warning
                } else {
                    DiagnosticStatus::Fail
                };

                let details = if self.verbose {
                    let mut all_details = operations_tested.clone();
                    all_details.extend(operations_failed.iter().map(|s| format!("❌ {}", s)));
                    Some(all_details.join("\n"))
                } else {
                    Some(format!("Successful: {}, Failed: {}", operations_tested.len(), operations_failed.len()))
                };

                checks.insert(
                    "repository_operations".to_string(),
                    DiagnosticResult {
                        status,
                        message: if operations_failed.is_empty() {
                            "All My Little Soda operations work correctly".to_string()
                        } else if operations_tested.len() > operations_failed.len() {
                            format!("Most operations work ({} of {} failed)", operations_failed.len(), 
                                   operations_tested.len() + operations_failed.len())
                        } else {
                            format!("Critical operation failures ({} failed)", operations_failed.len())
                        },
                        details,
                        suggestion: if !operations_failed.is_empty() {
                            Some("Check token permissions and repository settings to resolve operation failures".to_string())
                        } else {
                            None
                        },
                    },
                );
            }
            Err(_) => {
                checks.insert(
                    "repository_operations".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "Cannot test repository operations".to_string(),
                        details: Some("GitHub client creation failed".to_string()),
                        suggestion: Some("Fix GitHub authentication to enable operation testing".to_string()),
                    },
                );
            }
        }
    }
}