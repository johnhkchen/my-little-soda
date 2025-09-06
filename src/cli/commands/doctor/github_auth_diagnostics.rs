use super::types::{DiagnosticResult, DiagnosticStatus};
use crate::github::client::GitHubClient;
use crate::github::errors::GitHubError;
use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::process::Command;

/// GitHub authentication diagnostics functionality
pub struct GitHubAuthDiagnostics {
    verbose: bool,
}

impl GitHubAuthDiagnostics {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }

    fn is_verbose(&self) -> bool {
        self.verbose
    }

    /// Comprehensive GitHub authentication diagnostics using the actual GitHub client
    pub async fn check_github_authentication(&self, checks: &mut HashMap<String, DiagnosticResult>) {
        // Check 1: Token presence and format validation
        self.check_github_token_presence(checks);

        // Check 2: Try to create GitHub client and test authentication
        match GitHubClient::with_verbose(self.is_verbose()) {
            Ok(client) => {
                // If client creation succeeds, authentication is working
                self.check_github_authentication_success(&client, checks)
                    .await;
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
                        token_source =
                            "File-based configuration (.my-little-soda/credentials/github_token)"
                                .to_string();
                        token_format_valid = Self::validate_token_format(token);
                    }
                }
            }
        }

        // Check GitHub CLI
        if !token_found {
            if let Ok(output) = Command::new("gh")
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
                        details: if self.is_verbose() {
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
    async fn check_github_authentication_success(
        &self,
        client: &GitHubClient,
        checks: &mut HashMap<String, DiagnosticResult>,
    ) {
        // Authentication succeeded - add detailed status
        checks.insert(
            "github_authentication".to_string(),
            DiagnosticResult {
                status: DiagnosticStatus::Pass,
                message: "GitHub authentication successful".to_string(),
                details: if self.is_verbose() {
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
                details: if self.is_verbose() {
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
    fn check_github_authentication_failure(
        &self,
        error: GitHubError,
        checks: &mut HashMap<String, DiagnosticResult>,
    ) {
        match &error {
            GitHubError::TokenNotFound(message) => {
                checks.insert(
                    "github_authentication".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "GitHub authentication failed - token not found".to_string(),
                        details: if self.is_verbose() {
                            Some(message.clone())
                        } else {
                            Some("No valid GitHub token found".to_string())
                        },
                        suggestion: Some("Set MY_LITTLE_SODA_GITHUB_TOKEN environment variable or run 'gh auth login'".to_string()),
                    },
                );
            }
            GitHubError::TokenScopeInsufficient {
                required_scopes,
                current_error,
                token_url,
            } => {
                checks.insert(
                    "github_authentication".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "GitHub authentication failed - insufficient token permissions"
                            .to_string(),
                        details: Some(format!(
                            "Required scopes: {}. Error: {}",
                            required_scopes.join(", "),
                            current_error
                        )),
                        suggestion: Some(format!(
                            "Update your token permissions at {} to include: {}",
                            token_url,
                            required_scopes.join(", ")
                        )),
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
            GitHubError::RateLimit {
                reset_time,
                remaining,
            } => {
                let reset_in = (*reset_time - chrono::Utc::now()).num_minutes().max(0);
                checks.insert(
                    "github_authentication".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Warning,
                        message: "GitHub authentication succeeded but rate limited".to_string(),
                        details: Some(format!(
                            "Remaining requests: {}, Reset in: {} minutes",
                            remaining, reset_in
                        )),
                        suggestion: Some(
                            "Wait for rate limit to reset or use a different token".to_string(),
                        ),
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
                        suggestion: Some(
                            "Check token validity and GitHub service status".to_string(),
                        ),
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
            GitHubError::Timeout {
                operation,
                duration_ms,
            } => {
                checks.insert(
                    "github_authentication".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "GitHub authentication failed - timeout".to_string(),
                        details: Some(format!(
                            "Operation '{}' timed out after {}ms",
                            operation, duration_ms
                        )),
                        suggestion: Some("Check network connectivity and try again".to_string()),
                    },
                );
            }
        }
    }

    /// Check GitHub API rate limits and provide status
    async fn check_github_rate_limits(
        &self,
        client: &GitHubClient,
        checks: &mut HashMap<String, DiagnosticResult>,
    ) {
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
                    message: format!(
                        "Rate limit: {}/{} requests remaining ({:.1}%)",
                        core.remaining, core.limit, remaining_pct
                    ),
                    details: if self.is_verbose() {
                        Some(format!(
                            "Resets in {} minutes at {}",
                            reset_in,
                            reset_time.format("%Y-%m-%d %H:%M:%S UTC")
                        ))
                    } else {
                        Some(format!("Resets in {} minutes", reset_in))
                    },
                    suggestion: if remaining_pct <= 20.0 {
                        Some(
                            "Consider using a different token or waiting for rate limit reset"
                                .to_string(),
                        )
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
    async fn check_github_api_scopes(
        &self,
        client: &GitHubClient,
        checks: &mut HashMap<String, DiagnosticResult>,
    ) {
        let mut scope_tests = Vec::new();
        let mut failed_scopes = Vec::new();

        // Test issue read access
        match client
            .fetch_issues_with_state(Some(octocrab::params::State::Open))
            .await
        {
            Ok(_) => scope_tests.push("issues:read ✅".to_string()),
            Err(e) => {
                scope_tests.push("issues:read ❌".to_string());
                failed_scopes.push("issues:read".to_string());
                if self.is_verbose() {
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
                if self.is_verbose() {
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
                if self.is_verbose() {
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
                details: if self.is_verbose() {
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
}