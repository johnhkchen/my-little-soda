use super::types::{DiagnosticResult, DiagnosticStatus};
use crate::config::config;
use crate::github::client::GitHubClient;
use std::collections::HashMap;

/// GitHub repository diagnostics functionality
pub struct GitHubRepoDiagnostics {
    verbose: bool,
}

impl GitHubRepoDiagnostics {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }

    fn is_verbose(&self) -> bool {
        self.verbose
    }

    /// GitHub repository access diagnostics
    pub async fn check_github_repository_access(&self, checks: &mut HashMap<String, DiagnosticResult>) {
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
                            message: format!(
                                "Repository owner configured: {}",
                                github_config.owner
                            ),
                            details: if self.is_verbose() {
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
                            details: if self.is_verbose() {
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
                        suggestion: Some(
                            "Create my-little-soda.toml or check configuration format".to_string(),
                        ),
                    },
                );
            }
        }
    }

    /// Check if repository exists and is accessible
    async fn check_repository_existence(&self, checks: &mut HashMap<String, DiagnosticResult>) {
        match GitHubClient::with_verbose(self.is_verbose()) {
            Ok(client) => {
                let octocrab = client.issues.octocrab();
                match octocrab.repos(client.owner(), client.repo()).get().await {
                    Ok(repo) => {
                        let visibility = if repo.private.unwrap_or(false) {
                            "private"
                        } else {
                            "public"
                        };
                        checks.insert(
                            "repository_existence".to_string(),
                            DiagnosticResult {
                                status: DiagnosticStatus::Pass,
                                message: format!(
                                    "Repository {}/{} exists and is accessible",
                                    client.owner(),
                                    client.repo()
                                ),
                                details: if self.is_verbose() {
                                    Some(format!(
                                        "Repository visibility: {}, default branch: {}",
                                        visibility,
                                        repo.default_branch.as_deref().unwrap_or("unknown")
                                    ))
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
                            DiagnosticResult {
                                status,
                                message,
                                details,
                                suggestion,
                            },
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
        match GitHubClient::with_verbose(self.is_verbose()) {
            Ok(client) => {
                let octocrab = client.issues.octocrab();
                match octocrab.repos(client.owner(), client.repo()).get().await {
                    Ok(repo) => {
                        let mut settings_issues = Vec::new();
                        let mut settings_good = Vec::new();

                        // Check if issues are enabled
                        if !repo.has_issues.unwrap_or(false) {
                            settings_issues.push(
                                "Issues disabled - required for My Little Soda operation"
                                    .to_string(),
                            );
                        } else {
                            settings_good.push("Issues enabled ✅".to_string());
                        }

                        // Check if repository allows forking (for PRs)
                        if repo.fork.unwrap_or(false) && !repo.allow_forking.unwrap_or(true) {
                            settings_issues.push(
                                "Forking disabled - may limit some PR operations".to_string(),
                            );
                        }

                        // Check merge options
                        if !repo.allow_merge_commit.unwrap_or(false)
                            && !repo.allow_squash_merge.unwrap_or(false)
                        {
                            settings_issues.push(
                                "No merge options enabled - may prevent PR completion".to_string(),
                            );
                        } else {
                            let merge_types = [
                                ("merge commits", repo.allow_merge_commit.unwrap_or(false)),
                                ("squash merge", repo.allow_squash_merge.unwrap_or(false)),
                                ("rebase merge", repo.allow_rebase_merge.unwrap_or(false)),
                            ]
                            .iter()
                            .filter(|(_, enabled)| *enabled)
                            .map(|(name, _)| *name)
                            .collect::<Vec<_>>();

                            if !merge_types.is_empty() {
                                settings_good
                                    .push(format!("Merge options: {} ✅", merge_types.join(", ")));
                            }
                        }

                        let status = if settings_issues.is_empty() {
                            DiagnosticStatus::Pass
                        } else {
                            DiagnosticStatus::Warning
                        };

                        let details = if self.is_verbose() {
                            let mut details = settings_good;
                            if !settings_issues.is_empty() {
                                details
                                    .extend(settings_issues.iter().map(|s| format!("⚠️  {}", s)));
                            }
                            Some(details.join("\n"))
                        } else {
                            if settings_issues.is_empty() {
                                Some("Repository settings are compatible".to_string())
                            } else {
                                Some(format!(
                                    "{} setting issue(s) detected",
                                    settings_issues.len()
                                ))
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
                                suggestion: Some(
                                    "Repository settings could not be verified".to_string(),
                                ),
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
                        suggestion: Some(
                            "Fix authentication issues to check repository settings".to_string(),
                        ),
                    },
                );
            }
        }
    }

    /// Test ability to perform actual My Little Soda operations
    async fn check_repository_operations(&self, checks: &mut HashMap<String, DiagnosticResult>) {
        match GitHubClient::with_verbose(self.is_verbose()) {
            Ok(client) => {
                let mut operations_tested = Vec::new();
                let mut operations_failed = Vec::new();

                // Test 1: Issue operations
                match client
                    .fetch_issues_with_state(Some(octocrab::params::State::Open))
                    .await
                {
                    Ok(issues) => {
                        operations_tested
                            .push(format!("Issue listing: {} open issues ✅", issues.len()));
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
                if operations_failed.is_empty() || operations_tested.len() > operations_failed.len()
                {
                    let octocrab = client.issues.octocrab();
                    match octocrab
                        .issues(client.owner(), client.repo())
                        .list_labels_for_repo()
                        .send()
                        .await
                    {
                        Ok(labels) => {
                            operations_tested
                                .push(format!("Label access: {} labels ✅", labels.items.len()));
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

                let details = if self.is_verbose() {
                    let mut all_details = operations_tested.clone();
                    all_details.extend(operations_failed.iter().map(|s| format!("❌ {}", s)));
                    Some(all_details.join("\n"))
                } else {
                    Some(format!(
                        "Successful: {}, Failed: {}",
                        operations_tested.len(),
                        operations_failed.len()
                    ))
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
                        suggestion: Some(
                            "Fix GitHub authentication to enable operation testing".to_string(),
                        ),
                    },
                );
            }
        }
    }
}