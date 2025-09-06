use super::types::{DiagnosticResult, DiagnosticStatus};
use anyhow::Result;
use std::collections::HashMap;

/// Git diagnostics functionality
pub struct GitDiagnostics {
    verbose: bool,
}

impl GitDiagnostics {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }

    fn is_verbose(&self) -> bool {
        self.verbose
    }

    pub fn check_git_repository(&self, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
        // Check if we're in a git repository using git2
        match git2::Repository::open(".") {
            Ok(repo) => {
                let git_dir = repo.path();
                let workdir = repo.workdir();

                checks.insert(
                    "git_repository".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Pass,
                        message: "Git repository detected".to_string(),
                        details: if self.is_verbose() {
                            Some(format!(
                                "Git directory: {}, Working directory: {}",
                                git_dir.display(),
                                workdir.map_or("bare repository".to_string(), |p| p
                                    .display()
                                    .to_string())
                            ))
                        } else {
                            None
                        },
                        suggestion: None,
                    },
                );

                // If we have a repository, run comprehensive Git validation
                if let Err(e) = self.check_git_comprehensive_validation(&repo, checks) {
                    checks.insert(
                        "git_validation_error".to_string(),
                        DiagnosticResult {
                            status: DiagnosticStatus::Warning,
                            message: "Git validation failed to complete".to_string(),
                            details: Some(format!("Error: {}", e)),
                            suggestion: Some(
                                "Check Git repository integrity and permissions".to_string(),
                            ),
                        },
                    );
                }
            }
            Err(e) => {
                let (message, suggestion) = match e.code() {
                    git2::ErrorCode::NotFound => (
                        "Not in a git repository".to_string(),
                        Some("Run 'git init' or navigate to a git repository".to_string()),
                    ),
                    _ => (
                        "Git repository access error".to_string(),
                        Some(format!(
                            "Git error: {}. Check repository permissions and integrity",
                            e.message()
                        )),
                    ),
                };

                checks.insert(
                    "git_repository".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message,
                        details: if self.is_verbose() {
                            Some(e.message().to_string())
                        } else {
                            None
                        },
                        suggestion,
                    },
                );
            }
        }
        Ok(())
    }

    fn check_git_comprehensive_validation(
        &self,
        repo: &git2::Repository,
        checks: &mut HashMap<String, DiagnosticResult>,
    ) -> Result<()> {
        // Run all Git validation checks
        self.check_git_remote_configuration(repo, checks)?;
        self.check_git_remote_github_match(repo, checks)?;
        self.check_git_branch_setup(repo, checks)?;
        self.check_git_working_directory_state(repo, checks)?;
        self.check_git_user_configuration(repo, checks)?;
        self.check_git_operations_capability(repo, checks)?;
        self.check_git_my_little_soda_requirements(repo, checks)?;
        Ok(())
    }

    fn check_git_remote_configuration(
        &self,
        repo: &git2::Repository,
        checks: &mut HashMap<String, DiagnosticResult>,
    ) -> Result<()> {
        match repo.find_remote("origin") {
            Ok(remote) => {
                let url = remote.url().unwrap_or("invalid URL");
                checks.insert(
                    "git_remote_origin".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Pass,
                        message: "Git origin remote found".to_string(),
                        details: if self.is_verbose() {
                            Some(format!("Origin URL: {}", url))
                        } else {
                            None
                        },
                        suggestion: None,
                    },
                );
            }
            Err(e) => {
                let suggestion = match e.code() {
                    git2::ErrorCode::NotFound => {
                        "Add origin remote with: git remote add origin <repository-url>"
                    }
                    _ => "Check git remote configuration and repository setup",
                };

                checks.insert(
                    "git_remote_origin".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "Git origin remote not found".to_string(),
                        details: if self.is_verbose() {
                            Some(e.message().to_string())
                        } else {
                            None
                        },
                        suggestion: Some(suggestion.to_string()),
                    },
                );
            }
        }
        Ok(())
    }

    fn check_git_remote_github_match(
        &self,
        repo: &git2::Repository,
        checks: &mut HashMap<String, DiagnosticResult>,
    ) -> Result<()> {
        if let Ok(remote) = repo.find_remote("origin") {
            if let Some(url) = remote.url() {
                // Check if URL is a GitHub URL and if it matches configuration
                if let Ok(cfg) = crate::config::config() {
                    let expected_repo_patterns = vec![
                        format!("github.com/{}/{}", cfg.github.owner, cfg.github.repo),
                        format!("github.com/{}/{}.git", cfg.github.owner, cfg.github.repo),
                    ];

                    let matches_config = expected_repo_patterns
                        .iter()
                        .any(|pattern| url.contains(pattern));

                    if url.contains("github.com") {
                        if matches_config {
                            checks.insert(
                                "git_remote_github_match".to_string(),
                                DiagnosticResult {
                                    status: DiagnosticStatus::Pass,
                                    message: "Git remote matches GitHub configuration".to_string(),
                                    details: if self.is_verbose() {
                                        Some(format!(
                                            "Remote URL '{}' matches configured repository {}/{}",
                                            url, cfg.github.owner, cfg.github.repo
                                        ))
                                    } else {
                                        None
                                    },
                                    suggestion: None,
                                },
                            );
                        } else {
                            checks.insert(
                                "git_remote_github_match".to_string(),
                                DiagnosticResult {
                                    status: DiagnosticStatus::Warning,
                                    message: "Git remote does not match GitHub configuration".to_string(),
                                    details: Some(format!(
                                        "Remote points to '{}' but configuration expects {}/{}", 
                                        url, cfg.github.owner, cfg.github.repo
                                    )),
                                    suggestion: Some("Update my-little-soda.toml to match your repository or change remote URL".to_string()),
                                },
                            );
                        }
                    } else {
                        checks.insert(
                            "git_remote_github_match".to_string(),
                            DiagnosticResult {
                                status: DiagnosticStatus::Warning,
                                message: "Git remote is not a GitHub repository".to_string(),
                                details: Some(format!("Remote URL: {}", url)),
                                suggestion: Some("My Little Soda is designed for GitHub repositories. Consider using a GitHub remote.".to_string()),
                            },
                        );
                    }
                } else {
                    checks.insert(
                        "git_remote_github_match".to_string(),
                        DiagnosticResult {
                            status: DiagnosticStatus::Warning,
                            message: "Cannot validate remote against configuration".to_string(),
                            details: Some(
                                "Unable to load My Little Soda configuration".to_string(),
                            ),
                            suggestion: Some(
                                "Create or fix my-little-soda.toml configuration".to_string(),
                            ),
                        },
                    );
                }
            }
        } else {
            checks.insert(
                "git_remote_github_match".to_string(),
                DiagnosticResult {
                    status: DiagnosticStatus::Fail,
                    message: "Cannot validate remote - no origin found".to_string(),
                    details: None,
                    suggestion: Some("Add origin remote first".to_string()),
                },
            );
        }
        Ok(())
    }

    fn check_git_branch_setup(
        &self,
        repo: &git2::Repository,
        checks: &mut HashMap<String, DiagnosticResult>,
    ) -> Result<()> {
        // Check current branch
        match repo.head() {
            Ok(head) => {
                let current_branch = if head.is_branch() {
                    head.shorthand().unwrap_or("unknown").to_string()
                } else {
                    "detached HEAD".to_string()
                };

                // Check if we have a default/main branch
                let has_main = repo.find_branch("main", git2::BranchType::Local).is_ok();
                let has_master = repo.find_branch("master", git2::BranchType::Local).is_ok();

                let status = if current_branch == "detached HEAD" {
                    DiagnosticStatus::Warning
                } else {
                    DiagnosticStatus::Pass
                };

                checks.insert(
                    "git_branch_setup".to_string(),
                    DiagnosticResult {
                        status,
                        message: format!("Current branch: {}", current_branch),
                        details: if self.is_verbose() {
                            Some(format!(
                                "Main branch available: {}, Master branch available: {}",
                                has_main, has_master
                            ))
                        } else {
                            None
                        },
                        suggestion: if current_branch == "detached HEAD" {
                            Some(
                                "Switch to a proper branch with 'git checkout <branch-name>'"
                                    .to_string(),
                            )
                        } else {
                            None
                        },
                    },
                );
            }
            Err(e) => {
                checks.insert(
                    "git_branch_setup".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "Cannot determine current branch".to_string(),
                        details: if self.is_verbose() {
                            Some(e.message().to_string())
                        } else {
                            None
                        },
                        suggestion: Some("Check repository integrity and branch setup".to_string()),
                    },
                );
            }
        }
        Ok(())
    }

    fn check_git_working_directory_state(
        &self,
        repo: &git2::Repository,
        checks: &mut HashMap<String, DiagnosticResult>,
    ) -> Result<()> {
        match repo.statuses(None) {
            Ok(statuses) => {
                let mut modified_files = Vec::new();
                let mut untracked_files = Vec::new();
                let mut staged_files = Vec::new();

                for entry in statuses.iter() {
                    let status = entry.status();
                    let path = entry.path().unwrap_or("unknown").to_string();

                    if status.contains(git2::Status::WT_MODIFIED)
                        || status.contains(git2::Status::WT_DELETED)
                        || status.contains(git2::Status::WT_TYPECHANGE)
                    {
                        modified_files.push(path.clone());
                    }

                    if status.contains(git2::Status::WT_NEW) {
                        untracked_files.push(path.clone());
                    }

                    if status.contains(git2::Status::INDEX_MODIFIED)
                        || status.contains(git2::Status::INDEX_NEW)
                        || status.contains(git2::Status::INDEX_DELETED)
                        || status.contains(git2::Status::INDEX_TYPECHANGE)
                    {
                        staged_files.push(path);
                    }
                }

                let is_clean = modified_files.is_empty() && staged_files.is_empty();
                let total_changes = modified_files.len() + staged_files.len();

                let (status, message, suggestion) = if is_clean {
                    if untracked_files.is_empty() {
                        (
                            DiagnosticStatus::Pass,
                            "Working directory is clean".to_string(),
                            None,
                        )
                    } else {
                        (
                            DiagnosticStatus::Info,
                            format!(
                                "Working directory clean but has {} untracked files",
                                untracked_files.len()
                            ),
                            None,
                        )
                    }
                } else {
                    (
                        DiagnosticStatus::Warning,
                        format!(
                            "Working directory has {} uncommitted changes",
                            total_changes
                        ),
                        Some(
                            "Commit or stash changes before running My Little Soda operations"
                                .to_string(),
                        ),
                    )
                };

                checks.insert(
                    "git_working_directory_state".to_string(),
                    DiagnosticResult {
                        status,
                        message,
                        details: if self.is_verbose() && total_changes > 0 {
                            let mut details = Vec::new();
                            if !modified_files.is_empty() {
                                details.push(format!("Modified: {}", modified_files.len()));
                            }
                            if !staged_files.is_empty() {
                                details.push(format!("Staged: {}", staged_files.len()));
                            }
                            if !untracked_files.is_empty() {
                                details.push(format!("Untracked: {}", untracked_files.len()));
                            }
                            Some(details.join(", "))
                        } else {
                            None
                        },
                        suggestion,
                    },
                );
            }
            Err(e) => {
                checks.insert(
                    "git_working_directory_state".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Warning,
                        message: "Cannot check working directory status".to_string(),
                        details: if self.is_verbose() {
                            Some(e.message().to_string())
                        } else {
                            None
                        },
                        suggestion: Some("Check repository integrity".to_string()),
                    },
                );
            }
        }
        Ok(())
    }

    fn check_git_user_configuration(
        &self,
        repo: &git2::Repository,
        checks: &mut HashMap<String, DiagnosticResult>,
    ) -> Result<()> {
        let config = repo.config()?;

        let user_name = config.get_string("user.name").ok();
        let user_email = config.get_string("user.email").ok();

        let mut issues = Vec::new();
        let mut warnings = Vec::new();

        match user_name.as_ref() {
            Some(name) if name.trim().is_empty() => {
                issues.push("user.name is empty".to_string());
            }
            Some(_) => {
                // Name is set and not empty
            }
            None => {
                issues.push("user.name not configured".to_string());
            }
        }

        match user_email.as_ref() {
            Some(email) if email.trim().is_empty() => {
                issues.push("user.email is empty".to_string());
            }
            Some(email) if !email.contains('@') => {
                warnings.push("user.email format may be invalid".to_string());
            }
            Some(_) => {
                // Email is set and looks reasonable
            }
            None => {
                issues.push("user.email not configured".to_string());
            }
        }

        let (status, message) = if !issues.is_empty() {
            (
                DiagnosticStatus::Fail,
                format!(
                    "Git user configuration incomplete ({} issues)",
                    issues.len()
                ),
            )
        } else if !warnings.is_empty() {
            (
                DiagnosticStatus::Warning,
                format!("Git user configuration has {} warnings", warnings.len()),
            )
        } else {
            (
                DiagnosticStatus::Pass,
                "Git user configuration is complete".to_string(),
            )
        };

        let details = if self.is_verbose() || !issues.is_empty() || !warnings.is_empty() {
            let mut all_details = Vec::new();
            if let Some(name) = user_name {
                all_details.push(format!("user.name: {}", name));
            }
            if let Some(email) = user_email {
                all_details.push(format!("user.email: {}", email));
            }
            all_details.extend(issues.iter().map(|i| format!("Issue: {}", i)));
            all_details.extend(warnings.iter().map(|w| format!("Warning: {}", w)));
            Some(all_details.join("; "))
        } else {
            None
        };

        let suggestion = if !issues.is_empty() {
            Some("Configure Git user with: git config user.name \"Your Name\" && git config user.email \"your@email.com\"".to_string())
        } else {
            None
        };

        checks.insert(
            "git_user_configuration".to_string(),
            DiagnosticResult {
                status,
                message,
                details,
                suggestion,
            },
        );

        Ok(())
    }

    fn check_git_operations_capability(
        &self,
        repo: &git2::Repository,
        checks: &mut HashMap<String, DiagnosticResult>,
    ) -> Result<()> {
        let mut operations_tested = Vec::new();
        let mut operations_failed = Vec::new();

        // Test 1: Can we create a reference (branch)
        let test_branch_name = "my-little-soda-doctor-test";
        match repo.head() {
            Ok(head) => {
                if let Ok(commit) = head.peel_to_commit() {
                    match repo.reference(
                        &format!("refs/heads/{}", test_branch_name),
                        commit.id(),
                        false,
                        "Doctor test branch",
                    ) {
                        Ok(mut test_ref) => {
                            operations_tested.push("Branch creation ✅".to_string());
                            // Clean up test branch immediately
                            let _ = test_ref.delete();
                        }
                        Err(e) => {
                            operations_failed
                                .push(format!("Branch creation failed: {}", e.message()));
                        }
                    }
                } else {
                    operations_failed
                        .push("Cannot access current commit for branch tests".to_string());
                }
            }
            Err(e) => {
                operations_failed.push(format!(
                    "Cannot access HEAD for branch tests: {}",
                    e.message()
                ));
            }
        }

        // Test 2: Check if we can access remotes for push capability assessment
        match repo.find_remote("origin") {
            Ok(remote) => {
                if let Some(url) = remote.url() {
                    if url.starts_with("git@") || url.contains("https://") {
                        operations_tested.push("Remote access configured ✅".to_string());
                    } else {
                        operations_failed.push("Remote URL format not recognized".to_string());
                    }
                } else {
                    operations_failed.push("Remote has invalid URL".to_string());
                }
            }
            Err(_) => {
                operations_failed.push("No origin remote for push operations".to_string());
            }
        }

        let status = if operations_failed.is_empty() {
            DiagnosticStatus::Pass
        } else if operations_tested.len() > operations_failed.len() {
            DiagnosticStatus::Warning
        } else {
            DiagnosticStatus::Fail
        };

        checks.insert(
            "git_operations_capability".to_string(),
            DiagnosticResult {
                status,
                message: if operations_failed.is_empty() {
                    "Git operations capability verified".to_string()
                } else {
                    format!("Git operations issues detected ({} failed)", operations_failed.len())
                },
                details: if self.is_verbose() {
                    let mut details = operations_tested;
                    details.extend(operations_failed.iter().map(|f| format!("❌ {}", f)));
                    Some(details.join("; "))
                } else {
                    Some(format!("Successful: {}, Failed: {}", operations_tested.len(), operations_failed.len()))
                },
                suggestion: if !operations_failed.is_empty() {
                    Some("Ensure Git user configuration is complete and remote access is properly set up".to_string())
                } else {
                    None
                },
            },
        );

        Ok(())
    }

    fn check_git_my_little_soda_requirements(
        &self,
        repo: &git2::Repository,
        checks: &mut HashMap<String, DiagnosticResult>,
    ) -> Result<()> {
        let mut requirements_met = Vec::new();
        let mut requirements_failed = Vec::new();

        // Requirement 1: Repository must have at least one commit
        match repo.head() {
            Ok(head) => {
                if head.peel_to_commit().is_ok() {
                    requirements_met.push("Has commit history ✅".to_string());
                } else {
                    requirements_failed.push("Repository has no commits".to_string());
                }
            }
            Err(_) => {
                requirements_failed.push("Cannot access repository history".to_string());
            }
        }

        // Requirement 2: Must have origin remote configured
        if repo.find_remote("origin").is_ok() {
            requirements_met.push("Origin remote configured ✅".to_string());
        } else {
            requirements_failed.push("No origin remote for agent operations".to_string());
        }

        // Requirement 3: Working directory should be reasonably clean for agent operations
        if let Ok(statuses) = repo.statuses(None) {
            let has_staged_changes = statuses.iter().any(|entry| {
                let status = entry.status();
                status.contains(git2::Status::INDEX_MODIFIED)
                    || status.contains(git2::Status::INDEX_NEW)
                    || status.contains(git2::Status::INDEX_DELETED)
            });

            if !has_staged_changes {
                requirements_met.push("No staged changes blocking operations ✅".to_string());
            } else {
                requirements_failed
                    .push("Staged changes may interfere with agent operations".to_string());
            }
        }

        // Requirement 4: Should be able to determine current branch for agent workflows
        match repo.head() {
            Ok(head) if head.is_branch() => {
                requirements_met.push("On proper branch for workflow ✅".to_string());
            }
            _ => {
                requirements_failed.push(
                    "Not on a proper branch - agent workflows require branch context".to_string(),
                );
            }
        }

        let status = if requirements_failed.is_empty() {
            DiagnosticStatus::Pass
        } else if requirements_met.len() > requirements_failed.len() {
            DiagnosticStatus::Warning
        } else {
            DiagnosticStatus::Fail
        };

        checks.insert(
            "git_my_little_soda_requirements".to_string(),
            DiagnosticResult {
                status,
                message: if requirements_failed.is_empty() {
                    "Repository meets My Little Soda requirements".to_string()
                } else {
                    format!(
                        "Repository has {} My Little Soda compatibility issues",
                        requirements_failed.len()
                    )
                },
                details: if self.is_verbose() {
                    let mut details = requirements_met;
                    details.extend(requirements_failed.iter().map(|f| format!("❌ {}", f)));
                    Some(details.join("; "))
                } else {
                    Some(format!(
                        "Requirements met: {}, Failed: {}",
                        requirements_met.len(),
                        requirements_failed.len()
                    ))
                },
                suggestion: if !requirements_failed.is_empty() {
                    Some(
                        "Address Git setup issues to ensure proper My Little Soda agent operations"
                            .to_string(),
                    )
                } else {
                    None
                },
            },
        );

        Ok(())
    }
}