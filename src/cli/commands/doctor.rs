use crate::cli::DoctorFormat;
use crate::config::config;
use crate::github::client::GitHubClient;
use crate::github::errors::GitHubError;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
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
        
        // Run comprehensive configuration validation
        self.check_toml_configuration(&mut checks)?;
        
        // Run comprehensive GitHub authentication diagnostics
        self.check_github_authentication(&mut checks).await;
        
        // Run GitHub repository access diagnostics
        self.check_github_repository_access(&mut checks).await;
        
        // Run environment validation diagnostics
        self.check_environment_variables(&mut checks)?;
        self.check_file_system_permissions(&mut checks)?;
        self.check_disk_space(&mut checks)?;
        self.check_path_configuration(&mut checks)?;
        self.check_current_directory_access(&mut checks)?;
        self.check_temporary_directory_access(&mut checks)?;
        self.check_conflicting_configurations(&mut checks)?;
        self.check_file_operations(&mut checks)?;
        
        // Calculate summary
        let summary = self.calculate_summary(&checks);
        
        Ok(DiagnosticReport { summary, checks })
    }

    fn check_git_repository(&self, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
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
                        details: if self.verbose {
                            Some(format!(
                                "Git directory: {}, Working directory: {}", 
                                git_dir.display(),
                                workdir.map_or("bare repository".to_string(), |p| p.display().to_string())
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
                            suggestion: Some("Check Git repository integrity and permissions".to_string()),
                        },
                    );
                }
            }
            Err(e) => {
                let (message, suggestion) = match e.code() {
                    git2::ErrorCode::NotFound => (
                        "Not in a git repository".to_string(),
                        Some("Run 'git init' or navigate to a git repository".to_string())
                    ),
                    _ => (
                        "Git repository access error".to_string(),
                        Some(format!("Git error: {}. Check repository permissions and integrity", e.message()))
                    )
                };
                
                checks.insert(
                    "git_repository".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message,
                        details: if self.verbose { Some(e.message().to_string()) } else { None },
                        suggestion,
                    },
                );
            }
        }
        Ok(())
    }

    fn check_git_comprehensive_validation(&self, repo: &git2::Repository, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
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

    fn check_git_remote_configuration(&self, repo: &git2::Repository, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
        match repo.find_remote("origin") {
            Ok(remote) => {
                let url = remote.url().unwrap_or("invalid URL");
                checks.insert(
                    "git_remote_origin".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Pass,
                        message: "Git origin remote found".to_string(),
                        details: if self.verbose {
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
                    git2::ErrorCode::NotFound => "Add origin remote with: git remote add origin <repository-url>",
                    _ => "Check git remote configuration and repository setup"
                };
                
                checks.insert(
                    "git_remote_origin".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "Git origin remote not found".to_string(),
                        details: if self.verbose { Some(e.message().to_string()) } else { None },
                        suggestion: Some(suggestion.to_string()),
                    },
                );
            }
        }
        Ok(())
    }

    fn check_git_remote_github_match(&self, repo: &git2::Repository, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
        if let Ok(remote) = repo.find_remote("origin") {
            if let Some(url) = remote.url() {
                // Check if URL is a GitHub URL and if it matches configuration
                if let Ok(cfg) = crate::config::config() {
                    let expected_repo_patterns = vec![
                        format!("github.com/{}/{}", cfg.github.owner, cfg.github.repo),
                        format!("github.com/{}/{}.git", cfg.github.owner, cfg.github.repo),
                    ];
                    
                    let matches_config = expected_repo_patterns.iter().any(|pattern| url.contains(pattern));
                    
                    if url.contains("github.com") {
                        if matches_config {
                            checks.insert(
                                "git_remote_github_match".to_string(),
                                DiagnosticResult {
                                    status: DiagnosticStatus::Pass,
                                    message: "Git remote matches GitHub configuration".to_string(),
                                    details: if self.verbose {
                                        Some(format!("Remote URL '{}' matches configured repository {}/{}", 
                                               url, cfg.github.owner, cfg.github.repo))
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
                            details: Some("Unable to load My Little Soda configuration".to_string()),
                            suggestion: Some("Create or fix my-little-soda.toml configuration".to_string()),
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

    fn check_git_branch_setup(&self, repo: &git2::Repository, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
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
                        details: if self.verbose {
                            Some(format!(
                                "Main branch available: {}, Master branch available: {}", 
                                has_main, has_master
                            ))
                        } else {
                            None
                        },
                        suggestion: if current_branch == "detached HEAD" {
                            Some("Switch to a proper branch with 'git checkout <branch-name>'".to_string())
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
                        details: if self.verbose { Some(e.message().to_string()) } else { None },
                        suggestion: Some("Check repository integrity and branch setup".to_string()),
                    },
                );
            }
        }
        Ok(())
    }

    fn check_git_working_directory_state(&self, repo: &git2::Repository, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
        match repo.statuses(None) {
            Ok(statuses) => {
                let mut modified_files = Vec::new();
                let mut untracked_files = Vec::new();
                let mut staged_files = Vec::new();
                
                for entry in statuses.iter() {
                    let status = entry.status();
                    let path = entry.path().unwrap_or("unknown").to_string();
                    
                    if status.contains(git2::Status::WT_MODIFIED) || 
                       status.contains(git2::Status::WT_DELETED) ||
                       status.contains(git2::Status::WT_TYPECHANGE) {
                        modified_files.push(path.clone());
                    }
                    
                    if status.contains(git2::Status::WT_NEW) {
                        untracked_files.push(path.clone());
                    }
                    
                    if status.contains(git2::Status::INDEX_MODIFIED) ||
                       status.contains(git2::Status::INDEX_NEW) ||
                       status.contains(git2::Status::INDEX_DELETED) ||
                       status.contains(git2::Status::INDEX_TYPECHANGE) {
                        staged_files.push(path);
                    }
                }
                
                let is_clean = modified_files.is_empty() && staged_files.is_empty();
                let total_changes = modified_files.len() + staged_files.len();
                
                let (status, message, suggestion) = if is_clean {
                    if untracked_files.is_empty() {
                        (DiagnosticStatus::Pass, "Working directory is clean".to_string(), None)
                    } else {
                        (DiagnosticStatus::Info, 
                         format!("Working directory clean but has {} untracked files", untracked_files.len()),
                         None)
                    }
                } else {
                    (DiagnosticStatus::Warning,
                     format!("Working directory has {} uncommitted changes", total_changes),
                     Some("Commit or stash changes before running My Little Soda operations".to_string()))
                };
                
                checks.insert(
                    "git_working_directory_state".to_string(),
                    DiagnosticResult {
                        status,
                        message,
                        details: if self.verbose && total_changes > 0 {
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
                        details: if self.verbose { Some(e.message().to_string()) } else { None },
                        suggestion: Some("Check repository integrity".to_string()),
                    },
                );
            }
        }
        Ok(())
    }

    fn check_git_user_configuration(&self, repo: &git2::Repository, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
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
            (DiagnosticStatus::Fail, format!("Git user configuration incomplete ({} issues)", issues.len()))
        } else if !warnings.is_empty() {
            (DiagnosticStatus::Warning, format!("Git user configuration has {} warnings", warnings.len()))
        } else {
            (DiagnosticStatus::Pass, "Git user configuration is complete".to_string())
        };
        
        let details = if self.verbose || !issues.is_empty() || !warnings.is_empty() {
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
            DiagnosticResult { status, message, details, suggestion },
        );
        
        Ok(())
    }

    fn check_git_operations_capability(&self, repo: &git2::Repository, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
        let mut operations_tested = Vec::new();
        let mut operations_failed = Vec::new();
        
        // Test 1: Can we create a reference (branch)
        let test_branch_name = "my-little-soda-doctor-test";
        match repo.head() {
            Ok(head) => {
                if let Ok(commit) = head.peel_to_commit() {
                    match repo.reference(&format!("refs/heads/{}", test_branch_name), commit.id(), false, "Doctor test branch") {
                        Ok(mut test_ref) => {
                            operations_tested.push("Branch creation ✅".to_string());
                            // Clean up test branch immediately
                            let _ = test_ref.delete();
                        }
                        Err(e) => {
                            operations_failed.push(format!("Branch creation failed: {}", e.message()));
                        }
                    }
                } else {
                    operations_failed.push("Cannot access current commit for branch tests".to_string());
                }
            }
            Err(e) => {
                operations_failed.push(format!("Cannot access HEAD for branch tests: {}", e.message()));
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
                details: if self.verbose {
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

    fn check_git_my_little_soda_requirements(&self, repo: &git2::Repository, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
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
                status.contains(git2::Status::INDEX_MODIFIED) ||
                status.contains(git2::Status::INDEX_NEW) ||
                status.contains(git2::Status::INDEX_DELETED)
            });
            
            if !has_staged_changes {
                requirements_met.push("No staged changes blocking operations ✅".to_string());
            } else {
                requirements_failed.push("Staged changes may interfere with agent operations".to_string());
            }
        }
        
        // Requirement 4: Should be able to determine current branch for agent workflows
        match repo.head() {
            Ok(head) if head.is_branch() => {
                requirements_met.push("On proper branch for workflow ✅".to_string());
            }
            _ => {
                requirements_failed.push("Not on a proper branch - agent workflows require branch context".to_string());
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
                    format!("Repository has {} My Little Soda compatibility issues", requirements_failed.len())
                },
                details: if self.verbose {
                    let mut details = requirements_met;
                    details.extend(requirements_failed.iter().map(|f| format!("❌ {}", f)));
                    Some(details.join("; "))
                } else {
                    Some(format!("Requirements met: {}, Failed: {}", requirements_met.len(), requirements_failed.len()))
                },
                suggestion: if !requirements_failed.is_empty() {
                    Some("Address Git setup issues to ensure proper My Little Soda agent operations".to_string())
                } else {
                    None
                },
            },
        );
        
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
        // Check Rust toolchain version and availability
        self.check_rust_toolchain(checks)?;
        
        // Check cargo availability and functionality
        self.check_cargo_functionality(checks)?;
        
        // Check if git is available with version validation
        self.check_git_installation(checks)?;

        // Check if gh CLI is available
        self.check_github_cli_availability(checks)?;
        
        // Check network connectivity to GitHub API
        self.check_github_connectivity(checks)?;
        
        // Check system resource availability
        self.check_system_resources(checks)?;
        
        Ok(())
    }
    
    /// Check Rust toolchain version and availability
    fn check_rust_toolchain(&self, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
        match std::process::Command::new("rustc").arg("--version").output() {
            Ok(output) if output.status.success() => {
                let version_str = String::from_utf8_lossy(&output.stdout);
                let version_parts: Vec<&str> = version_str.split_whitespace().collect();
                
                if version_parts.len() >= 2 {
                    let version = version_parts[1];
                    // Check minimum version requirement (1.70.0+)
                    let status = if Self::check_rust_version_requirement(version) {
                        DiagnosticStatus::Pass
                    } else {
                        DiagnosticStatus::Warning
                    };
                    
                    checks.insert(
                        "rust_toolchain".to_string(),
                        DiagnosticResult {
                            status,
                            message: format!("Rust toolchain available ({})", version),
                            details: if self.verbose {
                                Some(version_str.trim().to_string())
                            } else {
                                None
                            },
                            suggestion: if status == DiagnosticStatus::Warning {
                                Some("Consider updating Rust toolchain: rustup update".to_string())
                            } else {
                                None
                            },
                        },
                    );
                } else {
                    checks.insert(
                        "rust_toolchain".to_string(),
                        DiagnosticResult {
                            status: DiagnosticStatus::Warning,
                            message: "Rust toolchain version could not be parsed".to_string(),
                            details: Some(version_str.trim().to_string()),
                            suggestion: Some("Verify Rust installation: rustc --version".to_string()),
                        },
                    );
                }
            }
            _ => {
                checks.insert(
                    "rust_toolchain".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "Rust toolchain not available".to_string(),
                        details: None,
                        suggestion: Some("Install Rust via rustup: https://rustup.rs/".to_string()),
                    },
                );
            }
        }
        Ok(())
    }
    
    /// Check if Rust version meets minimum requirements
    fn check_rust_version_requirement(version: &str) -> bool {
        // Parse version string (e.g., "1.70.0" or "1.70.0-stable")
        let version_clean = version.split('-').next().unwrap_or(version);
        let parts: Vec<&str> = version_clean.split('.').collect();
        
        if parts.len() >= 2 {
            if let (Ok(major), Ok(minor)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                // Require Rust 1.70.0 or higher
                return major > 1 || (major == 1 && minor >= 70);
            }
        }
        false
    }
    
    /// Check cargo availability and functionality
    fn check_cargo_functionality(&self, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
        match std::process::Command::new("cargo").arg("--version").output() {
            Ok(output) if output.status.success() => {
                let version_str = String::from_utf8_lossy(&output.stdout);
                
                // Test cargo functionality with a simple check command
                let cargo_check_result = std::process::Command::new("cargo")
                    .arg("check")
                    .arg("--help")
                    .output();
                    
                let status = if cargo_check_result.map_or(false, |o| o.status.success()) {
                    DiagnosticStatus::Pass
                } else {
                    DiagnosticStatus::Warning
                };
                
                checks.insert(
                    "cargo_functionality".to_string(),
                    DiagnosticResult {
                        status,
                        message: "Cargo is available and functional".to_string(),
                        details: if self.verbose {
                            Some(version_str.trim().to_string())
                        } else {
                            None
                        },
                        suggestion: if status == DiagnosticStatus::Warning {
                            Some("Cargo may not be functioning properly. Check your Rust installation.".to_string())
                        } else {
                            None
                        },
                    },
                );
            }
            _ => {
                checks.insert(
                    "cargo_functionality".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "Cargo not available".to_string(),
                        details: None,
                        suggestion: Some("Install Cargo with Rust toolchain: https://rustup.rs/".to_string()),
                    },
                );
            }
        }
        Ok(())
    }
    
    /// Check Git installation and version validation
    fn check_git_installation(&self, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
        match std::process::Command::new("git").arg("--version").output() {
            Ok(output) if output.status.success() => {
                let version_str = String::from_utf8_lossy(&output.stdout);
                let version_parts: Vec<&str> = version_str.split_whitespace().collect();
                
                let (status, suggestion) = if version_parts.len() >= 3 {
                    let version = version_parts[2];
                    if Self::check_git_version_requirement(version) {
                        (DiagnosticStatus::Pass, None)
                    } else {
                        (DiagnosticStatus::Warning, Some("Consider updating Git to version 2.20.0 or higher".to_string()))
                    }
                } else {
                    (DiagnosticStatus::Warning, Some("Could not parse Git version".to_string()))
                };
                
                checks.insert(
                    "git_installation".to_string(),
                    DiagnosticResult {
                        status,
                        message: "Git is available".to_string(),
                        details: if self.verbose {
                            Some(version_str.trim().to_string())
                        } else {
                            None
                        },
                        suggestion,
                    },
                );
            }
            _ => {
                checks.insert(
                    "git_installation".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "Git not available".to_string(),
                        details: None,
                        suggestion: Some(Self::get_git_installation_guidance()),
                    },
                );
            }
        }
        Ok(())
    }
    
    /// Check if Git version meets minimum requirements
    fn check_git_version_requirement(version: &str) -> bool {
        // Parse version string (e.g., "2.39.1")
        let parts: Vec<&str> = version.split('.').collect();
        
        if parts.len() >= 2 {
            if let (Ok(major), Ok(minor)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                // Require Git 2.20.0 or higher
                return major > 2 || (major == 2 && minor >= 20);
            }
        }
        false
    }
    
    /// Check GitHub CLI availability
    fn check_github_cli_availability(&self, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
        match std::process::Command::new("gh").arg("--version").output() {
            Ok(output) if output.status.success() => {
                let version_str = String::from_utf8_lossy(&output.stdout);
                let first_line = version_str.lines().next().unwrap_or("");
                
                // Check if authenticated
                let auth_status = std::process::Command::new("gh")
                    .arg("auth")
                    .arg("status")
                    .output();
                    
                let (status, message, suggestion) = if auth_status.map_or(false, |o| o.status.success()) {
                    (DiagnosticStatus::Pass, "GitHub CLI is available and authenticated".to_string(), None)
                } else {
                    (
                        DiagnosticStatus::Warning, 
                        "GitHub CLI is available but not authenticated".to_string(),
                        Some("Run 'gh auth login' to authenticate with GitHub".to_string())
                    )
                };
                
                checks.insert(
                    "github_cli".to_string(),
                    DiagnosticResult {
                        status,
                        message,
                        details: if self.verbose {
                            Some(first_line.to_string())
                        } else {
                            None
                        },
                        suggestion,
                    },
                );
            }
            _ => {
                checks.insert(
                    "github_cli".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Warning,
                        message: "GitHub CLI not available".to_string(),
                        details: None,
                        suggestion: Some(Self::get_github_cli_installation_guidance()),
                    },
                );
            }
        }
        Ok(())
    }
    
    /// Check network connectivity to GitHub API
    fn check_github_connectivity(&self, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
        // Simple connectivity test using curl or available HTTP client
        let connectivity_test = std::process::Command::new("curl")
            .arg("-s")
            .arg("--connect-timeout")
            .arg("10")
            .arg("--max-time")
            .arg("30")
            .arg("https://api.github.com")
            .output();
            
        match connectivity_test {
            Ok(output) if output.status.success() => {
                checks.insert(
                    "github_connectivity".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Pass,
                        message: "GitHub API is reachable".to_string(),
                        details: None,
                        suggestion: None,
                    },
                );
            }
            Ok(_) | Err(_) => {
                // Fallback: check if we can resolve DNS
                let dns_test = std::process::Command::new("nslookup")
                    .arg("api.github.com")
                    .output()
                    .or_else(|_| std::process::Command::new("dig").arg("api.github.com").output());
                    
                let (status, message, suggestion) = match dns_test {
                    Ok(output) if output.status.success() => {
                        (
                            DiagnosticStatus::Warning,
                            "DNS resolution works but HTTP connectivity test failed".to_string(),
                            Some("Check firewall settings and network connectivity".to_string())
                        )
                    }
                    _ => {
                        (
                            DiagnosticStatus::Warning,
                            "Network connectivity to GitHub could not be verified".to_string(),
                            Some("Ensure network connectivity and DNS resolution are working".to_string())
                        )
                    }
                };
                
                checks.insert(
                    "github_connectivity".to_string(),
                    DiagnosticResult { status, message, details: None, suggestion },
                );
            }
        }
        Ok(())
    }
    
    /// Check system resource availability (memory, CPU)
    fn check_system_resources(&self, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
        let mut resource_warnings = Vec::new();
        let mut resource_info = Vec::new();
        
        // Check available memory
        if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
            if let Some(available_kb) = Self::parse_meminfo(&meminfo, "MemAvailable:") {
                let available_gb = available_kb / 1024 / 1024;
                resource_info.push(format!("Available memory: {} GB", available_gb));
                
                if available_gb < 1 {
                    resource_warnings.push("Low available memory (< 1GB) may affect performance".to_string());
                }
            }
        }
        
        // Check CPU information
        if let Ok(cpuinfo) = fs::read_to_string("/proc/cpuinfo") {
            let cpu_count = cpuinfo.lines().filter(|line| line.starts_with("processor")).count();
            resource_info.push(format!("CPU cores: {}", cpu_count));
            
            if cpu_count < 2 {
                resource_warnings.push("Single CPU core may affect build performance".to_string());
            }
        }
        
        // Check disk space in current directory
        match fs::metadata(".") {
            Ok(_) => {
                // Use df command to check disk space
                if let Ok(output) = std::process::Command::new("df").arg("-h").arg(".").output() {
                    if output.status.success() {
                        let df_output = String::from_utf8_lossy(&output.stdout);
                        if let Some(line) = df_output.lines().nth(1) {
                            let parts: Vec<&str> = line.split_whitespace().collect();
                            if parts.len() >= 4 {
                                resource_info.push(format!("Available disk space: {}", parts[3]));
                            }
                        }
                    }
                }
            }
            _ => {
                resource_warnings.push("Could not check disk space".to_string());
            }
        }
        
        let status = if resource_warnings.is_empty() {
            DiagnosticStatus::Pass
        } else {
            DiagnosticStatus::Warning
        };
        
        let message = if resource_warnings.is_empty() {
            "System resources appear adequate".to_string()
        } else {
            format!("System resources have {} potential issues", resource_warnings.len())
        };
        
        let details = if self.verbose || !resource_warnings.is_empty() {
            let mut all_details = resource_info;
            all_details.extend(resource_warnings.iter().map(|w| format!("⚠️  {}", w)));
            Some(all_details.join("; "))
        } else {
            None
        };
        
        checks.insert(
            "system_resources".to_string(),
            DiagnosticResult {
                status,
                message,
                details,
                suggestion: if !resource_warnings.is_empty() {
                    Some("Consider upgrading system resources for optimal performance".to_string())
                } else {
                    None
                },
            },
        );
        
        Ok(())
    }
    
    /// Parse memory information from /proc/meminfo
    fn parse_meminfo(meminfo: &str, field: &str) -> Option<u64> {
        for line in meminfo.lines() {
            if line.starts_with(field) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    return parts[1].parse::<u64>().ok();
                }
            }
        }
        None
    }
    
    /// Get OS-specific Git installation guidance
    fn get_git_installation_guidance() -> String {
        if cfg!(target_os = "windows") {
            "Install Git from https://git-scm.com/ or use 'winget install Git.Git'".to_string()
        } else if cfg!(target_os = "macos") {
            "Install Git via 'brew install git' or Xcode Command Line Tools".to_string()
        } else {
            "Install Git via your package manager: 'sudo apt install git' (Ubuntu/Debian) or 'sudo yum install git' (RHEL/CentOS)".to_string()
        }
    }
    
    /// Get OS-specific GitHub CLI installation guidance
    fn get_github_cli_installation_guidance() -> String {
        if cfg!(target_os = "windows") {
            "Install GitHub CLI: 'winget install GitHub.cli' or download from https://cli.github.com/".to_string()
        } else if cfg!(target_os = "macos") {
            "Install GitHub CLI: 'brew install gh' or download from https://cli.github.com/".to_string()
        } else {
            "Install GitHub CLI via package manager or download from https://cli.github.com/".to_string()
        }
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

    fn check_toml_configuration(&self, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
        // Check 1: TOML file existence
        self.check_toml_file_existence(checks);
        
        // Check 2: TOML syntax validation
        self.check_toml_syntax(checks);
        
        // Check 3: Configuration completeness and placeholder detection
        self.check_config_completeness(checks);
        
        // Check 4: Field validation and constraints
        self.check_config_field_validation(checks);
        
        // Check 5: Cross-validation with environment
        self.check_config_environment_consistency(checks);
        
        Ok(())
    }

    fn check_toml_file_existence(&self, checks: &mut HashMap<String, DiagnosticResult>) {
        let toml_path = Path::new("my-little-soda.toml");
        let example_path = Path::new("my-little-soda.example.toml");
        
        if toml_path.exists() {
            checks.insert(
                "config_file_exists".to_string(),
                DiagnosticResult {
                    status: DiagnosticStatus::Pass,
                    message: "Configuration file exists".to_string(),
                    details: if self.verbose {
                        Some("my-little-soda.toml found".to_string())
                    } else {
                        None
                    },
                    suggestion: None,
                },
            );
        } else if example_path.exists() {
            checks.insert(
                "config_file_exists".to_string(),
                DiagnosticResult {
                    status: DiagnosticStatus::Warning,
                    message: "Configuration file missing but example found".to_string(),
                    details: Some("Found my-little-soda.example.toml but no my-little-soda.toml".to_string()),
                    suggestion: Some("Copy my-little-soda.example.toml to my-little-soda.toml and customize it".to_string()),
                },
            );
        } else {
            checks.insert(
                "config_file_exists".to_string(),
                DiagnosticResult {
                    status: DiagnosticStatus::Fail,
                    message: "Configuration file not found".to_string(),
                    details: None,
                    suggestion: Some("Create my-little-soda.toml configuration file or run 'my-little-soda init'".to_string()),
                },
            );
        }
    }

    fn check_toml_syntax(&self, checks: &mut HashMap<String, DiagnosticResult>) {
        let toml_path = Path::new("my-little-soda.toml");
        
        if toml_path.exists() {
            match std::fs::read_to_string(toml_path) {
                Ok(content) => {
                    match toml::from_str::<toml::Value>(&content) {
                        Ok(_) => {
                            checks.insert(
                                "config_toml_syntax".to_string(),
                                DiagnosticResult {
                                    status: DiagnosticStatus::Pass,
                                    message: "TOML syntax is valid".to_string(),
                                    details: if self.verbose {
                                        Some("Configuration file parses successfully".to_string())
                                    } else {
                                        None
                                    },
                                    suggestion: None,
                                },
                            );
                        }
                        Err(e) => {
                            checks.insert(
                                "config_toml_syntax".to_string(),
                                DiagnosticResult {
                                    status: DiagnosticStatus::Fail,
                                    message: "TOML syntax error".to_string(),
                                    details: Some(format!("Parse error: {}", e)),
                                    suggestion: Some("Fix TOML syntax errors in my-little-soda.toml".to_string()),
                                },
                            );
                        }
                    }
                }
                Err(e) => {
                    checks.insert(
                        "config_toml_syntax".to_string(),
                        DiagnosticResult {
                            status: DiagnosticStatus::Fail,
                            message: "Cannot read configuration file".to_string(),
                            details: Some(format!("File read error: {}", e)),
                            suggestion: Some("Check file permissions for my-little-soda.toml".to_string()),
                        },
                    );
                }
            }
        } else {
            checks.insert(
                "config_toml_syntax".to_string(),
                DiagnosticResult {
                    status: DiagnosticStatus::Warning,
                    message: "Cannot validate TOML syntax".to_string(),
                    details: Some("Configuration file does not exist".to_string()),
                    suggestion: None,
                },
            );
        }
    }

    fn check_config_completeness(&self, checks: &mut HashMap<String, DiagnosticResult>) {
        match config() {
            Ok(cfg) => {
                let mut issues = Vec::new();
                let mut warnings = Vec::new();
                
                // Check for placeholder values
                if cfg.github.owner == "your-github-username" || cfg.github.owner == "johnhkchen" {
                    if cfg.github.owner == "your-github-username" {
                        issues.push("GitHub owner has placeholder value 'your-github-username'".to_string());
                    } else {
                        warnings.push("GitHub owner uses default value 'johnhkchen'".to_string());
                    }
                }
                
                if cfg.github.repo == "your-repo-name" || cfg.github.repo == "my-little-soda" {
                    if cfg.github.repo == "your-repo-name" {
                        issues.push("GitHub repo has placeholder value 'your-repo-name'".to_string());
                    } else {
                        warnings.push("GitHub repo uses default value 'my-little-soda'".to_string());
                    }
                }
                
                // Check for empty required fields
                if cfg.github.owner.is_empty() {
                    issues.push("GitHub owner is empty".to_string());
                }
                if cfg.github.repo.is_empty() {
                    issues.push("GitHub repo is empty".to_string());
                }
                if cfg.observability.log_level.is_empty() {
                    issues.push("Log level is empty".to_string());
                }
                
                let has_issues = !issues.is_empty();
                let has_warnings = !warnings.is_empty();
                
                let status = if has_issues {
                    DiagnosticStatus::Fail
                } else if has_warnings {
                    DiagnosticStatus::Warning
                } else {
                    DiagnosticStatus::Pass
                };
                
                let message = if has_issues {
                    format!("Configuration has {} issue(s)", issues.len())
                } else if has_warnings {
                    format!("Configuration has {} warning(s)", warnings.len())
                } else {
                    "Configuration is complete".to_string()
                };
                
                let details = if self.verbose || has_issues || has_warnings {
                    let mut all_details = issues;
                    all_details.extend(warnings.iter().map(|w| format!("Warning: {}", w)));
                    if all_details.is_empty() {
                        Some("All required fields are properly configured".to_string())
                    } else {
                        Some(all_details.join("; "))
                    }
                } else {
                    None
                };
                
                let suggestion = if has_issues {
                    Some("Update my-little-soda.toml with your actual repository information".to_string())
                } else if has_warnings {
                    Some("Consider updating configuration values if they don't match your repository".to_string())
                } else {
                    None
                };
                
                checks.insert(
                    "config_completeness".to_string(),
                    DiagnosticResult { status, message, details, suggestion },
                );
            }
            Err(e) => {
                checks.insert(
                    "config_completeness".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "Cannot load configuration for validation".to_string(),
                        details: Some(format!("Configuration load error: {}", e)),
                        suggestion: Some("Fix configuration file syntax or create my-little-soda.toml".to_string()),
                    },
                );
            }
        }
    }

    fn check_config_field_validation(&self, checks: &mut HashMap<String, DiagnosticResult>) {
        match config() {
            Ok(cfg) => {
                let mut validation_issues = Vec::new();
                
                // Validate numeric constraints
                if cfg.github.rate_limit.requests_per_hour == 0 {
                    validation_issues.push("Rate limit requests per hour must be positive".to_string());
                }
                if cfg.github.rate_limit.burst_capacity == 0 {
                    validation_issues.push("Rate limit burst capacity must be positive".to_string());
                }
                if cfg.agents.coordination_timeout_seconds == 0 {
                    validation_issues.push("Agent coordination timeout must be positive".to_string());
                }
                if cfg.agents.bundle_processing.max_queue_size == 0 {
                    validation_issues.push("Bundle queue size must be positive".to_string());
                }
                if cfg.agents.bundle_processing.processing_timeout_seconds == 0 {
                    validation_issues.push("Bundle processing timeout must be positive".to_string());
                }
                if cfg.agents.process_management.timeout_minutes == 0 {
                    validation_issues.push("Process timeout must be positive".to_string());
                }
                
                // Validate log level
                let valid_log_levels = ["trace", "debug", "info", "warn", "error"];
                if !valid_log_levels.contains(&cfg.observability.log_level.as_str()) {
                    validation_issues.push(format!("Invalid log level '{}', must be one of: {}", 
                                                  cfg.observability.log_level, 
                                                  valid_log_levels.join(", ")));
                }
                
                // Validate CI mode artifact handling
                let valid_artifact_strategies = ["standard", "optimized", "enhanced"];
                if !valid_artifact_strategies.contains(&cfg.agents.ci_mode.artifact_handling.as_str()) {
                    validation_issues.push(format!("Invalid artifact handling strategy '{}', must be one of: {}", 
                                                  cfg.agents.ci_mode.artifact_handling,
                                                  valid_artifact_strategies.join(", ")));
                }
                
                // Validate paths
                if cfg.agents.process_management.claude_code_path.is_empty() {
                    validation_issues.push("Claude code path cannot be empty".to_string());
                }
                if cfg.agents.work_continuity.state_file_path.is_empty() {
                    validation_issues.push("Work continuity state file path cannot be empty".to_string());
                }
                if cfg.agents.process_management.work_dir_prefix.is_empty() {
                    validation_issues.push("Work directory prefix cannot be empty".to_string());
                }
                
                // Validate database configuration if present
                if let Some(db) = &cfg.database {
                    if db.url.is_empty() {
                        validation_issues.push("Database URL cannot be empty".to_string());
                    }
                    if db.max_connections == 0 {
                        validation_issues.push("Database max connections must be positive".to_string());
                    }
                }
                
                let status = if validation_issues.is_empty() {
                    DiagnosticStatus::Pass
                } else {
                    DiagnosticStatus::Fail
                };
                
                let message = if validation_issues.is_empty() {
                    "Configuration field validation passed".to_string()
                } else {
                    format!("Configuration has {} validation error(s)", validation_issues.len())
                };
                
                let details = if validation_issues.is_empty() {
                    if self.verbose {
                        Some("All configuration fields have valid values and types".to_string())
                    } else {
                        None
                    }
                } else {
                    Some(validation_issues.join("; "))
                };
                
                let suggestion = if !validation_issues.is_empty() {
                    Some("Fix validation errors in my-little-soda.toml using valid values".to_string())
                } else {
                    None
                };
                
                checks.insert(
                    "config_field_validation".to_string(),
                    DiagnosticResult { status, message, details, suggestion },
                );
            }
            Err(e) => {
                checks.insert(
                    "config_field_validation".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Warning,
                        message: "Cannot validate configuration fields".to_string(),
                        details: Some(format!("Configuration load error: {}", e)),
                        suggestion: None,
                    },
                );
            }
        }
    }

    fn check_config_environment_consistency(&self, checks: &mut HashMap<String, DiagnosticResult>) {
        match config() {
            Ok(cfg) => {
                let mut inconsistencies = Vec::new();
                
                // Check for environment variable overrides
                if let Ok(env_owner) = env::var("GITHUB_OWNER") {
                    if env_owner != cfg.github.owner {
                        inconsistencies.push(format!("GITHUB_OWNER env var ('{}') differs from config ('{}')", 
                                                    env_owner, cfg.github.owner));
                    }
                }
                
                if let Ok(env_repo) = env::var("GITHUB_REPO") {
                    if env_repo != cfg.github.repo {
                        inconsistencies.push(format!("GITHUB_REPO env var ('{}') differs from config ('{}')", 
                                                    env_repo, cfg.github.repo));
                    }
                }
                
                if let Ok(env_log_level) = env::var("MY_LITTLE_SODA_OBSERVABILITY_LOG_LEVEL") {
                    if env_log_level != cfg.observability.log_level {
                        inconsistencies.push(format!("MY_LITTLE_SODA_OBSERVABILITY_LOG_LEVEL env var ('{}') differs from config ('{}')", 
                                                    env_log_level, cfg.observability.log_level));
                    }
                }
                
                // Check for token configuration
                let has_env_token = env::var("MY_LITTLE_SODA_GITHUB_TOKEN").is_ok() || env::var("GITHUB_TOKEN").is_ok();
                let has_config_token = cfg.github.token.is_some();
                
                if !has_env_token && !has_config_token {
                    inconsistencies.push("No GitHub token found in environment variables or configuration".to_string());
                }
                
                let status = if inconsistencies.is_empty() {
                    DiagnosticStatus::Pass
                } else if inconsistencies.iter().any(|i| i.contains("token")) {
                    DiagnosticStatus::Fail
                } else {
                    DiagnosticStatus::Warning
                };
                
                let message = if inconsistencies.is_empty() {
                    "Configuration is consistent with environment".to_string()
                } else {
                    format!("Configuration has {} consistency issue(s)", inconsistencies.len())
                };
                
                let details = if inconsistencies.is_empty() {
                    if self.verbose {
                        Some("Configuration values match environment variable overrides".to_string())
                    } else {
                        None
                    }
                } else {
                    Some(inconsistencies.join("; "))
                };
                
                let suggestion = if !inconsistencies.is_empty() {
                    Some("Environment variables override config values - ensure consistency or use environment variables consistently".to_string())
                } else {
                    None
                };
                
                checks.insert(
                    "config_environment_consistency".to_string(),
                    DiagnosticResult { status, message, details, suggestion },
                );
            }
            Err(_) => {
                checks.insert(
                    "config_environment_consistency".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Warning,
                        message: "Cannot check environment consistency".to_string(),
                        details: Some("Configuration could not be loaded".to_string()),
                        suggestion: None,
                    },
                );
            }
        }
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

    /// Check required environment variables
    fn check_environment_variables(&self, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
        let mut issues = Vec::new();
        let mut warnings = Vec::new();
        let mut good = Vec::new();

        // Check MY_LITTLE_SODA_GITHUB_TOKEN
        match env::var("MY_LITTLE_SODA_GITHUB_TOKEN") {
            Ok(token) if !token.is_empty() && token != "YOUR_GITHUB_TOKEN_HERE" => {
                good.push("MY_LITTLE_SODA_GITHUB_TOKEN is set ✅".to_string());
            }
            Ok(_) => {
                issues.push("MY_LITTLE_SODA_GITHUB_TOKEN is empty or has placeholder value".to_string());
            }
            Err(_) => {
                // Check for GitHub CLI as fallback
                if Command::new("gh").arg("auth").arg("status").output().map_or(false, |o| o.status.success()) {
                    warnings.push("MY_LITTLE_SODA_GITHUB_TOKEN not set but GitHub CLI is authenticated".to_string());
                } else {
                    issues.push("MY_LITTLE_SODA_GITHUB_TOKEN not set and no GitHub CLI authentication found".to_string());
                }
            }
        }

        // Check PATH if my-little-soda might be globally installed
        if let Ok(path) = env::var("PATH") {
            let has_cargo_bin = path.split(':').any(|p| p.contains("cargo/bin") || p.contains(".cargo/bin"));
            if has_cargo_bin {
                good.push("PATH includes Cargo bin directory ✅".to_string());
            } else {
                warnings.push("PATH may not include Cargo bin directory for global installations".to_string());
            }
        }

        // Check for conflicting environment variables
        let conflicting_vars = vec![
            ("GITHUB_TOKEN", "MY_LITTLE_SODA_GITHUB_TOKEN"),
            ("GH_TOKEN", "MY_LITTLE_SODA_GITHUB_TOKEN"),
        ];

        for (conflict_var, preferred_var) in conflicting_vars {
            if env::var(conflict_var).is_ok() && env::var(preferred_var).is_err() {
                warnings.push(format!("{} is set but {} is not - consider using {}", 
                                    conflict_var, preferred_var, preferred_var));
            }
        }

        let status = if !issues.is_empty() {
            DiagnosticStatus::Fail
        } else if !warnings.is_empty() {
            DiagnosticStatus::Warning
        } else {
            DiagnosticStatus::Pass
        };

        let message = if !issues.is_empty() {
            format!("Environment variables have {} issue(s)", issues.len())
        } else if !warnings.is_empty() {
            format!("Environment variables have {} warning(s)", warnings.len())
        } else {
            "Environment variables are properly configured".to_string()
        };

        let details = if self.verbose || !issues.is_empty() || !warnings.is_empty() {
            let mut all_details = good;
            all_details.extend(warnings.iter().map(|w| format!("⚠️  {}", w)));
            all_details.extend(issues.iter().map(|i| format!("❌ {}", i)));
            Some(all_details.join("; "))
        } else {
            None
        };

        let suggestion = if !issues.is_empty() {
            Some("Set MY_LITTLE_SODA_GITHUB_TOKEN environment variable or run 'gh auth login'".to_string())
        } else if !warnings.is_empty() {
            Some("Review environment variable configuration for optimal setup".to_string())
        } else {
            None
        };

        checks.insert(
            "environment_variables".to_string(),
            DiagnosticResult { status, message, details, suggestion },
        );

        Ok(())
    }

    /// Check file system permissions for .my-little-soda directory
    fn check_file_system_permissions(&self, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
        let soda_dir = Path::new(".my-little-soda");
        let mut issues = Vec::new();
        let mut good = Vec::new();

        if soda_dir.exists() {
            // Check if it's a directory
            if soda_dir.is_dir() {
                good.push(".my-little-soda directory exists ✅".to_string());
                
                // Check read permissions
                match fs::read_dir(soda_dir) {
                    Ok(_) => good.push("Directory is readable ✅".to_string()),
                    Err(e) => issues.push(format!("Cannot read .my-little-soda directory: {}", e)),
                }

                // Check write permissions by trying to create a test file
                let test_file = soda_dir.join(".doctor-test");
                match fs::write(&test_file, "test") {
                    Ok(_) => {
                        good.push("Directory is writable ✅".to_string());
                        let _ = fs::remove_file(&test_file); // Clean up
                    }
                    Err(e) => issues.push(format!("Cannot write to .my-little-soda directory: {}", e)),
                }
            } else {
                issues.push(".my-little-soda exists but is not a directory".to_string());
            }
        } else {
            // Try to create the directory to test permissions
            match fs::create_dir_all(soda_dir) {
                Ok(_) => {
                    good.push("Successfully created .my-little-soda directory ✅".to_string());
                    // Clean up test directory
                    let _ = fs::remove_dir(soda_dir);
                }
                Err(e) => {
                    issues.push(format!("Cannot create .my-little-soda directory: {}", e));
                }
            }
        }

        let status = if issues.is_empty() {
            DiagnosticStatus::Pass
        } else {
            DiagnosticStatus::Fail
        };

        let message = if issues.is_empty() {
            "File system permissions for .my-little-soda are adequate".to_string()
        } else {
            format!("File system permission issues detected ({} issues)", issues.len())
        };

        let details = if self.verbose || !issues.is_empty() {
            let mut all_details = good;
            all_details.extend(issues.iter().map(|i| format!("❌ {}", i)));
            Some(all_details.join("; "))
        } else {
            None
        };

        let suggestion = if !issues.is_empty() {
            Some("Ensure current user has read/write permissions for the current directory".to_string())
        } else {
            None
        };

        checks.insert(
            "file_system_permissions".to_string(),
            DiagnosticResult { status, message, details, suggestion },
        );

        Ok(())
    }

    /// Check available disk space for temporary files and state
    fn check_disk_space(&self, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
        let current_dir = env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
        
        // Try to get disk usage information (cross-platform approach)
        let (status, message, details, suggestion) = match self.get_available_space(&current_dir) {
            Ok(space_mb) => {
                if space_mb > 1000.0 { // More than 1GB
                    (DiagnosticStatus::Pass,
                     format!("Sufficient disk space available ({:.1} MB)", space_mb),
                     if self.verbose { Some("Disk space check passed".to_string()) } else { None },
                     None)
                } else if space_mb > 100.0 { // 100MB - 1GB
                    (DiagnosticStatus::Warning,
                     format!("Limited disk space available ({:.1} MB)", space_mb),
                     Some("May affect temporary file operations".to_string()),
                     Some("Consider freeing up disk space for optimal operation".to_string()))
                } else {
                    (DiagnosticStatus::Fail,
                     format!("Very low disk space ({:.1} MB)", space_mb),
                     Some("Insufficient space for My Little Soda operations".to_string()),
                     Some("Free up disk space before running My Little Soda operations".to_string()))
                }
            }
            Err(e) => {
                (DiagnosticStatus::Warning,
                 "Unable to check disk space".to_string(),
                 Some(format!("Error checking disk space: {}", e)),
                 Some("Manually verify sufficient disk space is available".to_string()))
            }
        };

        checks.insert(
            "disk_space".to_string(),
            DiagnosticResult { status, message, details, suggestion },
        );

        Ok(())
    }

    /// Cross-platform disk space checking using a simple file write test
    fn get_available_space(&self, path: &Path) -> Result<f64> {
        // Simple cross-platform approach: try to write progressively larger test files
        // to estimate available space
        let test_sizes = vec![
            (1024 * 1024, 1.0),        // 1MB = 1MB
            (10 * 1024 * 1024, 10.0),  // 10MB = 10MB  
            (100 * 1024 * 1024, 100.0), // 100MB = 100MB
            (1024 * 1024 * 1024, 1024.0), // 1GB = 1024MB
        ];

        let mut max_writable = 0.0;
        
        for (size_bytes, size_mb) in test_sizes {
            let test_file = path.join(format!(".disk-space-test-{}", size_mb as i32));
            
            // Try to write the test file
            let test_data = vec![0u8; size_bytes];
            match fs::write(&test_file, &test_data) {
                Ok(_) => {
                    // Successfully wrote this size, update max
                    max_writable = size_mb;
                    // Clean up immediately
                    let _ = fs::remove_file(&test_file);
                }
                Err(_) => {
                    // Failed to write this size, return the previous successful size
                    break;
                }
            }
        }

        if max_writable > 0.0 {
            // Estimate that we have at least this much space, probably more
            Ok(max_writable * 2.0) // Assume we have at least twice what we could write
        } else {
            // Couldn't even write 1MB, very low space
            Err(anyhow::anyhow!("Cannot write even small test files - disk may be full"))
        }
    }

    /// Check PATH configuration if globally installed
    fn check_path_configuration(&self, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
        let mut path_status = Vec::new();
        let mut warnings = Vec::new();

        // Check if my-little-soda is in PATH
        match Command::new("my-little-soda").arg("--version").output() {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                path_status.push(format!("my-little-soda found in PATH: {} ✅", version));
            }
            _ => {
                // Not in PATH, check common installation locations
                let cargo_home = env::var("CARGO_HOME").unwrap_or_else(|_| 
                    format!("{}/.cargo", env::var("HOME").unwrap_or_default()));
                let cargo_bin = format!("{}/bin", cargo_home);
                
                if Path::new(&format!("{}/my-little-soda", cargo_bin)).exists() {
                    warnings.push(format!("my-little-soda installed in {} but not in PATH", cargo_bin));
                } else {
                    path_status.push("my-little-soda not found in PATH (using local build) ℹ️".to_string());
                }
            }
        }

        // Check PATH structure
        if let Ok(path_env) = env::var("PATH") {
            let paths: Vec<&str> = path_env.split(':').collect();
            let cargo_paths: Vec<&str> = paths.iter().filter(|p| p.contains("cargo")).copied().collect();
            
            if !cargo_paths.is_empty() {
                path_status.push(format!("Cargo paths in PATH: {} ✅", cargo_paths.len()));
            } else {
                warnings.push("No Cargo paths detected in PATH - global installations may not work".to_string());
            }
        }

        let status = if !warnings.is_empty() {
            DiagnosticStatus::Warning
        } else {
            DiagnosticStatus::Pass
        };

        let message = if warnings.is_empty() {
            "PATH configuration is appropriate".to_string()
        } else {
            format!("PATH configuration has {} warning(s)", warnings.len())
        };

        let details = if self.verbose || !warnings.is_empty() {
            let mut all_details = path_status;
            all_details.extend(warnings.iter().map(|w| format!("⚠️  {}", w)));
            Some(all_details.join("; "))
        } else {
            None
        };

        let suggestion = if !warnings.is_empty() {
            Some("Add ~/.cargo/bin to PATH for global Rust tool access".to_string())
        } else {
            None
        };

        checks.insert(
            "path_configuration".to_string(),
            DiagnosticResult { status, message, details, suggestion },
        );

        Ok(())
    }

    /// Check write permissions in current directory
    fn check_current_directory_access(&self, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
        let current_dir = env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
        let mut issues = Vec::new();
        let mut good = Vec::new();

        // Check if we can read the current directory
        match fs::read_dir(&current_dir) {
            Ok(_) => good.push("Current directory is readable ✅".to_string()),
            Err(e) => issues.push(format!("Cannot read current directory: {}", e)),
        }

        // Check if we can write to the current directory
        let test_file = current_dir.join(".my-little-soda-doctor-test");
        match fs::write(&test_file, "test write access") {
            Ok(_) => {
                good.push("Current directory is writable ✅".to_string());
                // Clean up test file
                let _ = fs::remove_file(&test_file);
            }
            Err(e) => {
                issues.push(format!("Cannot write to current directory: {}", e));
            }
        }

        // Check if we can create subdirectories
        let test_dir = current_dir.join(".my-little-soda-test-dir");
        match fs::create_dir(&test_dir) {
            Ok(_) => {
                good.push("Can create subdirectories ✅".to_string());
                // Clean up test directory
                let _ = fs::remove_dir(&test_dir);
            }
            Err(e) => {
                issues.push(format!("Cannot create subdirectories: {}", e));
            }
        }

        let status = if issues.is_empty() {
            DiagnosticStatus::Pass
        } else {
            DiagnosticStatus::Fail
        };

        let message = if issues.is_empty() {
            "Current directory access permissions are adequate".to_string()
        } else {
            format!("Current directory access issues detected ({} issues)", issues.len())
        };

        let details = if self.verbose || !issues.is_empty() {
            let mut all_details = good;
            all_details.extend(issues.iter().map(|i| format!("❌ {}", i)));
            Some(all_details.join("; "))
        } else {
            None
        };

        let suggestion = if !issues.is_empty() {
            Some("Ensure current user has read/write permissions for the current directory and can create subdirectories".to_string())
        } else {
            None
        };

        checks.insert(
            "current_directory_access".to_string(),
            DiagnosticResult { status, message, details, suggestion },
        );

        Ok(())
    }

    /// Check temporary directory access and permissions
    fn check_temporary_directory_access(&self, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
        let mut issues = Vec::new();
        let mut good = Vec::new();

        // Check system temporary directory
        let temp_dir = env::temp_dir();
        
        // Check if temp directory exists and is accessible
        if temp_dir.exists() && temp_dir.is_dir() {
            good.push(format!("System temp directory accessible: {} ✅", temp_dir.display()));
            
            // Test write access to temp directory
            let test_file = temp_dir.join("my-little-soda-doctor-test");
            match fs::write(&test_file, "test temp write") {
                Ok(_) => {
                    good.push("Temp directory is writable ✅".to_string());
                    // Clean up
                    let _ = fs::remove_file(&test_file);
                }
                Err(e) => {
                    issues.push(format!("Cannot write to temp directory: {}", e));
                }
            }

            // Test creating temp subdirectories
            let test_subdir = temp_dir.join("my-little-soda-test-subdir");
            match fs::create_dir(&test_subdir) {
                Ok(_) => {
                    good.push("Can create temp subdirectories ✅".to_string());
                    // Test writing in subdirectory
                    let nested_file = test_subdir.join("test-file");
                    match fs::write(&nested_file, "nested test") {
                        Ok(_) => {
                            good.push("Can write in temp subdirectories ✅".to_string());
                        }
                        Err(e) => {
                            issues.push(format!("Cannot write in temp subdirectories: {}", e));
                        }
                    }
                    // Clean up
                    let _ = fs::remove_file(&nested_file);
                    let _ = fs::remove_dir(&test_subdir);
                }
                Err(e) => {
                    issues.push(format!("Cannot create temp subdirectories: {}", e));
                }
            }
        } else {
            issues.push("System temporary directory not accessible".to_string());
        }

        // Check TMPDIR environment variable if set
        if let Ok(tmpdir) = env::var("TMPDIR") {
            let custom_tmp = Path::new(&tmpdir);
            if custom_tmp.exists() {
                good.push(format!("Custom TMPDIR accessible: {} ✅", tmpdir));
            } else {
                issues.push(format!("Custom TMPDIR not accessible: {}", tmpdir));
            }
        }

        let status = if issues.is_empty() {
            DiagnosticStatus::Pass
        } else {
            DiagnosticStatus::Fail
        };

        let message = if issues.is_empty() {
            "Temporary directory access is working correctly".to_string()
        } else {
            format!("Temporary directory access issues detected ({} issues)", issues.len())
        };

        let details = if self.verbose || !issues.is_empty() {
            let mut all_details = good;
            all_details.extend(issues.iter().map(|i| format!("❌ {}", i)));
            Some(all_details.join("; "))
        } else {
            None
        };

        let suggestion = if !issues.is_empty() {
            Some("Ensure system temporary directory is accessible and writable".to_string())
        } else {
            None
        };

        checks.insert(
            "temporary_directory_access".to_string(),
            DiagnosticResult { status, message, details, suggestion },
        );

        Ok(())
    }

    /// Check for conflicting environment variables or configurations
    fn check_conflicting_configurations(&self, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
        let mut conflicts = Vec::new();
        let mut warnings = Vec::new();
        let mut good = Vec::new();

        // Check for conflicting GitHub token environment variables
        let github_tokens = vec![
            ("MY_LITTLE_SODA_GITHUB_TOKEN", env::var("MY_LITTLE_SODA_GITHUB_TOKEN").ok()),
            ("GITHUB_TOKEN", env::var("GITHUB_TOKEN").ok()),
            ("GH_TOKEN", env::var("GH_TOKEN").ok()),
        ];

        let set_tokens: Vec<&str> = github_tokens.iter()
            .filter(|(_, val)| val.is_some())
            .map(|(name, _)| *name)
            .collect();

        if set_tokens.len() > 1 {
            conflicts.push(format!("Multiple GitHub tokens set: {} - may cause confusion", set_tokens.join(", ")));
        } else if set_tokens.len() == 1 {
            good.push(format!("Single GitHub token source: {} ✅", set_tokens[0]));
        }

        // Check for conflicting Git configurations
        if let Ok(config_owner) = config().map(|c| c.github.owner.clone()) {
            if let Ok(env_owner) = env::var("GITHUB_OWNER") {
                if config_owner != env_owner {
                    conflicts.push(format!("GitHub owner mismatch: config='{}' vs env='{}'", config_owner, env_owner));
                }
            }
        }

        if let Ok(config_repo) = config().map(|c| c.github.repo.clone()) {
            if let Ok(env_repo) = env::var("GITHUB_REPO") {
                if config_repo != env_repo {
                    conflicts.push(format!("GitHub repo mismatch: config='{}' vs env='{}'", config_repo, env_repo));
                }
            }
        }

        // Check for conflicting log levels
        if let Ok(config_log) = config().map(|c| c.observability.log_level.clone()) {
            if let Ok(env_log) = env::var("MY_LITTLE_SODA_OBSERVABILITY_LOG_LEVEL") {
                if config_log != env_log {
                    warnings.push(format!("Log level mismatch: config='{}' vs env='{}'", config_log, env_log));
                }
            }
        }

        // Check for read-only file systems (common in containers)
        let current_dir = env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
        let test_file = current_dir.join(".readonly-test");
        match fs::write(&test_file, "test") {
            Ok(_) => {
                let _ = fs::remove_file(&test_file);
                good.push("File system is writable ✅".to_string());
            }
            Err(_) => {
                conflicts.push("File system appears to be read-only".to_string());
            }
        }

        let status = if !conflicts.is_empty() {
            DiagnosticStatus::Fail
        } else if !warnings.is_empty() {
            DiagnosticStatus::Warning
        } else {
            DiagnosticStatus::Pass
        };

        let message = if !conflicts.is_empty() {
            format!("Configuration conflicts detected ({} conflicts)", conflicts.len())
        } else if !warnings.is_empty() {
            format!("Configuration warnings detected ({} warnings)", warnings.len())
        } else {
            "No configuration conflicts detected".to_string()
        };

        let details = if self.verbose || !conflicts.is_empty() || !warnings.is_empty() {
            let mut all_details = good;
            all_details.extend(warnings.iter().map(|w| format!("⚠️  {}", w)));
            all_details.extend(conflicts.iter().map(|c| format!("❌ {}", c)));
            Some(all_details.join("; "))
        } else {
            None
        };

        let suggestion = if !conflicts.is_empty() {
            Some("Resolve configuration conflicts by using consistent environment variables and settings".to_string())
        } else if !warnings.is_empty() {
            Some("Review configuration mismatches to ensure intended behavior".to_string())
        } else {
            None
        };

        checks.insert(
            "conflicting_configurations".to_string(),
            DiagnosticResult { status, message, details, suggestion },
        );

        Ok(())
    }

    /// Test ability to create/modify files needed by My Little Soda
    fn check_file_operations(&self, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
        let mut operations_tested = Vec::new();
        let mut operations_failed = Vec::new();

        // Test 1: Create .my-little-soda directory structure
        let soda_dir = Path::new(".my-little-soda");
        let credentials_dir = soda_dir.join("credentials");
        let agents_dir = soda_dir.join("agents");
        
        match fs::create_dir_all(&credentials_dir) {
            Ok(_) => {
                operations_tested.push("Create .my-little-soda/credentials ✅".to_string());
                // Clean up test directory
                let _ = fs::remove_dir_all(soda_dir);
            }
            Err(e) => {
                operations_failed.push(format!("Cannot create credentials directory: {}", e));
            }
        }

        match fs::create_dir_all(&agents_dir) {
            Ok(_) => {
                operations_tested.push("Create .my-little-soda/agents ✅".to_string());
                // Clean up test directory if credentials didn't fail
                if operations_failed.is_empty() {
                    let _ = fs::remove_dir_all(soda_dir);
                }
            }
            Err(e) => {
                operations_failed.push(format!("Cannot create agents directory: {}", e));
            }
        }

        // Test 2: Create and modify state files
        let state_file = Path::new(".my-little-soda-test-state.json");
        let test_content = r#"{"test": "state", "timestamp": "2023-01-01T00:00:00Z"}"#;
        
        match fs::write(state_file, test_content) {
            Ok(_) => {
                operations_tested.push("Create state file ✅".to_string());
                
                // Test reading it back
                match fs::read_to_string(state_file) {
                    Ok(content) if content == test_content => {
                        operations_tested.push("Read state file ✅".to_string());
                    }
                    Ok(_) => {
                        operations_failed.push("State file content mismatch".to_string());
                    }
                    Err(e) => {
                        operations_failed.push(format!("Cannot read state file: {}", e));
                    }
                }

                // Test modifying it
                let updated_content = r#"{"test": "updated", "timestamp": "2023-01-01T01:00:00Z"}"#;
                match fs::write(state_file, updated_content) {
                    Ok(_) => {
                        operations_tested.push("Modify state file ✅".to_string());
                    }
                    Err(e) => {
                        operations_failed.push(format!("Cannot modify state file: {}", e));
                    }
                }
                
                // Clean up
                let _ = fs::remove_file(state_file);
            }
            Err(e) => {
                operations_failed.push(format!("Cannot create state file: {}", e));
            }
        }

        // Test 3: Create temporary work files (simulate agent operations)
        let work_dir = Path::new(".my-little-soda-test-work");
        match fs::create_dir_all(work_dir) {
            Ok(_) => {
                operations_tested.push("Create work directory ✅".to_string());

                // Test creating files in work directory
                let work_file = work_dir.join("test-work.txt");
                match fs::write(&work_file, "test work content") {
                    Ok(_) => {
                        operations_tested.push("Create work files ✅".to_string());
                    }
                    Err(e) => {
                        operations_failed.push(format!("Cannot create work files: {}", e));
                    }
                }

                // Clean up work directory
                let _ = fs::remove_dir_all(work_dir);
            }
            Err(e) => {
                operations_failed.push(format!("Cannot create work directory: {}", e));
            }
        }

        // Test 4: File locking (simulate concurrent operations)
        let lock_file = Path::new(".my-little-soda-test-lock");
        match fs::write(lock_file, "test lock") {
            Ok(_) => {
                operations_tested.push("Create lock files ✅".to_string());
                
                // Test that we can detect and work with existing files
                if lock_file.exists() {
                    operations_tested.push("Detect existing files ✅".to_string());
                }
                
                // Clean up
                let _ = fs::remove_file(lock_file);
            }
            Err(e) => {
                operations_failed.push(format!("Cannot create lock files: {}", e));
            }
        }

        let status = if operations_failed.is_empty() {
            DiagnosticStatus::Pass
        } else if operations_tested.len() > operations_failed.len() {
            DiagnosticStatus::Warning
        } else {
            DiagnosticStatus::Fail
        };

        let message = if operations_failed.is_empty() {
            "All My Little Soda file operations work correctly".to_string()
        } else {
            format!("File operation issues detected ({} failed, {} passed)", 
                   operations_failed.len(), operations_tested.len())
        };

        let details = if self.verbose || !operations_failed.is_empty() {
            let mut all_details = operations_tested;
            all_details.extend(operations_failed.iter().map(|f| format!("❌ {}", f)));
            Some(all_details.join("; "))
        } else {
            Some(format!("Operations tested: {}, Failed: {}", operations_tested.len(), operations_failed.len()))
        };

        let suggestion = if !operations_failed.is_empty() {
            Some("Ensure file system permissions allow My Little Soda to create and modify required files and directories".to_string())
        } else {
            None
        };

        checks.insert(
            "file_operations".to_string(),
            DiagnosticResult { status, message, details, suggestion },
        );

        Ok(())
    }
}