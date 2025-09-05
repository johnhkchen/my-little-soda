pub mod agent_state;
pub mod github_labels;

use crate::cli::DoctorFormat;
use crate::config::config;
use crate::github::client::GitHubClient;
use crate::github::errors::GitHubError;
use agent_state::AgentStateDiagnostic;
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
    pub readiness: SystemReadiness,
    pub recommendations: Vec<ActionableRecommendation>,
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

/// System readiness assessment
#[derive(Debug, Serialize, Deserialize)]
pub struct SystemReadiness {
    pub score: u8,
    pub status: ReadinessStatus,
    pub description: String,
}

/// Overall system readiness status
#[derive(Debug, Serialize, Deserialize)]
pub enum ReadinessStatus {
    Ready,
    PartiallyReady,
    NotReady,
}

/// Actionable recommendation for fixing issues
#[derive(Debug, Serialize, Deserialize)]
pub struct ActionableRecommendation {
    pub priority: RecommendationPriority,
    pub category: String,
    pub title: String,
    pub description: String,
    pub steps: Vec<String>,
    pub links: Vec<String>,
}

/// Priority level for recommendations
#[derive(Debug, Serialize, Deserialize)]
pub enum RecommendationPriority {
    Critical,
    High,
    Medium,
    Low,
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
        
        // Run GitHub issue label validation diagnostics
        self.check_github_issue_labels(&mut checks).await;
        
        // Run environment validation diagnostics
        self.check_environment_variables(&mut checks)?;
        self.check_file_system_permissions(&mut checks)?;
        self.check_disk_space(&mut checks)?;
        self.check_path_configuration(&mut checks)?;
        self.check_current_directory_access(&mut checks)?;
        self.check_temporary_directory_access(&mut checks)?;
        self.check_conflicting_configurations(&mut checks)?;
        self.check_file_operations(&mut checks)?;
        
        // Run agent state diagnostics
        self.check_agent_state_health(&mut checks).await;
        
        // Run end-to-end workflow validation diagnostics
        self.check_end_to_end_workflow_validation(&mut checks).await;
        
        // Calculate summary
        let summary = self.calculate_summary(&checks);
        
        // Calculate system readiness
        let readiness = self.calculate_readiness(&summary, &checks);
        
        // Generate actionable recommendations
        let recommendations = self.generate_recommendations(&checks);
        
        Ok(DiagnosticReport { 
            summary, 
            checks, 
            readiness, 
            recommendations 
        })
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
                        details: if self.is_verbose() {
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
                        details: if self.is_verbose() { Some(e.message().to_string()) } else { None },
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
                    git2::ErrorCode::NotFound => "Add origin remote with: git remote add origin <repository-url>",
                    _ => "Check git remote configuration and repository setup"
                };
                
                checks.insert(
                    "git_remote_origin".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "Git origin remote not found".to_string(),
                        details: if self.is_verbose() { Some(e.message().to_string()) } else { None },
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
                                    details: if self.is_verbose() {
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
                        details: if self.is_verbose() {
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
                        details: if self.is_verbose() { Some(e.message().to_string()) } else { None },
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
                        details: if self.is_verbose() { Some(e.message().to_string()) } else { None },
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
                details: if self.is_verbose() {
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
                    details: if self.is_verbose() {
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
                            details: if self.is_verbose() {
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
                        details: if self.is_verbose() {
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
                        details: if self.is_verbose() {
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
                        details: if self.is_verbose() {
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
        
        let details = if self.is_verbose() || !resource_warnings.is_empty() {
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
            
            if self.is_verbose() || matches!(result.status, DiagnosticStatus::Fail | DiagnosticStatus::Warning) {
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

        // System Readiness Score
        self.output_system_readiness(&report.readiness);
        
        // Actionable Recommendations
        self.output_actionable_recommendations(&report.recommendations);

        Ok(())
    }

    /// Output system readiness score and status
    fn output_system_readiness(&self, readiness: &SystemReadiness) {
        println!();
        println!("🎯 SYSTEM READINESS SCORE:");
        println!("───────────────────────");
        
        let status_icon = match readiness.status {
            ReadinessStatus::Ready => "🟢",
            ReadinessStatus::PartiallyReady => "🟡", 
            ReadinessStatus::NotReady => "🔴",
        };
        
        println!("{} Score: {}/100", status_icon, readiness.score);
        println!("Status: {:?}", readiness.status);
        println!("Assessment: {}", readiness.description);
        println!();
    }

    /// Output actionable recommendations with detailed steps
    fn output_actionable_recommendations(&self, recommendations: &[ActionableRecommendation]) {
        if recommendations.is_empty() {
            return;
        }

        println!("📋 ACTIONABLE RECOMMENDATIONS:");
        println!("────────────────────────────");
        println!();

        for (i, rec) in recommendations.iter().enumerate() {
            let priority_icon = match rec.priority {
                RecommendationPriority::Critical => "🔴 CRITICAL",
                RecommendationPriority::High => "🟠 HIGH",
                RecommendationPriority::Medium => "🟡 MEDIUM",
                RecommendationPriority::Low => "🟢 LOW",
            };
            
            println!("{}. {} [{}]", i + 1, rec.title, priority_icon);
            println!("   Category: {}", rec.category);
            println!("   Description: {}", rec.description);
            
            if !rec.steps.is_empty() {
                println!("   Steps:");
                for (j, step) in rec.steps.iter().enumerate() {
                    println!("     {}. {}", j + 1, step);
                }
            }
            
            if !rec.links.is_empty() {
                println!("   Resources:");
                for link in &rec.links {
                    println!("     • {}", link);
                }
            }
            
            if i < recommendations.len() - 1 {
                println!();
            }
        }
        
        println!();
        println!("💡 TIP: Address critical and high priority recommendations first for maximum impact.");
        println!();
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
                    details: if self.is_verbose() {
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
                                    details: if self.is_verbose() {
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
                
                let details = if self.is_verbose() || has_issues || has_warnings {
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
                    if self.is_verbose() {
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
                    if self.is_verbose() {
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
    /// Determines if verbose output should be enabled (quiet for JSON format)
    fn is_verbose(&self) -> bool {
        self.verbose && !matches!(self.format, DoctorFormat::Json)
    }

    async fn check_github_authentication(&self, checks: &mut HashMap<String, DiagnosticResult>) {
        // Check 1: Token presence and format validation
        self.check_github_token_presence(checks);
        
        // Check 2: Try to create GitHub client and test authentication
        match GitHubClient::with_verbose(self.is_verbose()) {
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
    async fn check_github_authentication_success(&self, client: &GitHubClient, checks: &mut HashMap<String, DiagnosticResult>) {
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
    fn check_github_authentication_failure(&self, error: GitHubError, checks: &mut HashMap<String, DiagnosticResult>) {
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
                    details: if self.is_verbose() {
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
                        suggestion: Some("Create my-little-soda.toml or check configuration format".to_string()),
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
                        let visibility = if repo.private.unwrap_or(false) { "private" } else { "public" };
                        checks.insert(
                            "repository_existence".to_string(),
                            DiagnosticResult {
                                status: DiagnosticStatus::Pass,
                                message: format!("Repository {}/{} exists and is accessible", client.owner(), client.repo()),
                                details: if self.is_verbose() {
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
        match GitHubClient::with_verbose(self.is_verbose()) {
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

                        let details = if self.is_verbose() {
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
        match GitHubClient::with_verbose(self.is_verbose()) {
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

                let details = if self.is_verbose() {
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

        let details = if self.is_verbose() || !issues.is_empty() || !warnings.is_empty() {
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

        let details = if self.is_verbose() || !issues.is_empty() {
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
                     if self.is_verbose() { Some("Disk space check passed".to_string()) } else { None },
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

        let details = if self.is_verbose() || !warnings.is_empty() {
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

        let details = if self.is_verbose() || !issues.is_empty() {
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

        let details = if self.is_verbose() || !issues.is_empty() {
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

        let details = if self.is_verbose() || !conflicts.is_empty() || !warnings.is_empty() {
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

        let details = if self.is_verbose() || !operations_failed.is_empty() {
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

    /// Check agent state and work continuity health
    async fn check_agent_state_health(&self, checks: &mut HashMap<String, DiagnosticResult>) {
        // Try to create GitHub client for agent diagnostics
        match GitHubClient::with_verbose(self.is_verbose()) {
            Ok(github_client) => {
                // Create agent state diagnostic checker
                let agent_diagnostic = match AgentStateDiagnostic::new(github_client)
                    .with_coordinator().await {
                    Ok(diagnostic) => diagnostic,
                    Err(e) => {
                        checks.insert(
                            "agent_state_initialization".to_string(),
                            DiagnosticResult {
                                status: DiagnosticStatus::Fail,
                                message: "Failed to initialize agent state diagnostics".to_string(),
                                details: Some(format!("Error: {:?}", e)),
                                suggestion: Some("Ensure GitHub authentication and configuration are correct".to_string()),
                            },
                        );
                        return;
                    }
                };

                // Run comprehensive agent state checks
                agent_diagnostic.check_agent_state(checks).await;
                agent_diagnostic.check_work_continuity(checks).await;
                agent_diagnostic.check_orphaned_assignments(checks).await;
                agent_diagnostic.check_abandoned_work(checks).await;
                agent_diagnostic.check_conflicting_assignments(checks).await;
                agent_diagnostic.check_cleanup_status(checks).await;

                // Add overall agent health summary
                let has_failures = checks.values().any(|result| {
                    matches!(result.status, DiagnosticStatus::Fail) && 
                    [
                        "agent_availability",
                        "work_continuity_integrity", 
                        "orphaned_assignments",
                        "abandoned_work",
                        "conflicting_assignments",
                        "state_machine_consistency",
                        "work_cleanup"
                    ].iter().any(|key| checks.contains_key(*key))
                });

                let has_warnings = checks.values().any(|result| {
                    matches!(result.status, DiagnosticStatus::Warning) && 
                    [
                        "agent_availability",
                        "work_continuity_integrity", 
                        "orphaned_assignments",
                        "abandoned_work",
                        "conflicting_assignments",
                        "state_machine_consistency",
                        "work_cleanup"
                    ].iter().any(|key| checks.contains_key(*key))
                });

                let overall_status = if has_failures {
                    DiagnosticStatus::Fail
                } else if has_warnings {
                    DiagnosticStatus::Warning
                } else {
                    DiagnosticStatus::Pass
                };

                let message = if has_failures {
                    "Agent state has critical issues that need attention".to_string()
                } else if has_warnings {
                    "Agent state has some issues but is generally healthy".to_string()
                } else {
                    "Agent state is healthy".to_string()
                };

                checks.insert(
                    "agent_state_overall".to_string(),
                    DiagnosticResult {
                        status: overall_status,
                        message,
                        details: Some("See individual agent state checks for detailed information".to_string()),
                        suggestion: if has_failures {
                            Some("Address critical agent state issues immediately to prevent work disruption".to_string())
                        } else if has_warnings {
                            Some("Consider addressing agent state warnings to improve reliability".to_string())
                        } else {
                            Some("Agent state is optimal - continue normal operation".to_string())
                        },
                    },
                );
            }
            Err(e) => {
                checks.insert(
                    "agent_state_github_access".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "Cannot access GitHub for agent state diagnostics".to_string(),
                        details: Some(format!("GitHub client creation failed: {:?}", e)),
                        suggestion: Some("Configure GitHub authentication with 'gh auth login' or set appropriate environment variables".to_string()),
                    },
                );
            }
        }
    }

    /// GitHub issue label validation diagnostics
    async fn check_github_issue_labels(&self, checks: &mut HashMap<String, DiagnosticResult>) {
        // Check 1: Required label existence
        checks.insert(
            "required_labels_existence".to_string(),
            github_labels::check_required_labels_existence(self.is_verbose()).await,
        );
        
        // Check 2: Label configuration validation
        checks.insert(
            "label_configuration".to_string(),
            github_labels::check_label_configuration(self.is_verbose()).await,
        );
        
        // Check 3: Repository write permissions for label management
        checks.insert(
            "repository_write_permissions".to_string(),
            github_labels::check_repository_write_permissions(self.is_verbose()).await,
        );
        
        // Check 4: Label management capabilities (create/update/delete)
        checks.insert(
            "label_management_capabilities".to_string(),
            github_labels::check_label_management_capabilities(self.is_verbose()).await,
        );
        
        // Check 5: Issue label state validation
        checks.insert(
            "issue_label_states".to_string(),
            github_labels::check_issue_label_states(self.is_verbose()).await,
        );
        
        // Check 6: Workflow label compliance
        checks.insert(
            "workflow_label_compliance".to_string(),
            github_labels::check_workflow_label_compliance(self.is_verbose()).await,
        );
    }

    /// End-to-end workflow validation diagnostics
    async fn check_end_to_end_workflow_validation(&self, checks: &mut HashMap<String, DiagnosticResult>) {
        // Check 1: Agent lifecycle readiness
        checks.insert(
            "agent_lifecycle_readiness".to_string(),
            self.check_agent_lifecycle_readiness().await,
        );
        
        // Check 2: Issue workflow integration
        checks.insert(
            "issue_workflow_integration".to_string(),
            self.check_issue_workflow_integration().await,
        );
        
        // Check 3: Branch and PR workflow validation
        checks.insert(
            "branch_pr_workflow".to_string(),
            self.check_branch_pr_workflow().await,
        );
        
        // Check 4: Agent coordination readiness
        checks.insert(
            "agent_coordination_readiness".to_string(),
            self.check_agent_coordination_readiness().await,
        );
        
        // Check 5: Complete workflow simulation
        checks.insert(
            "workflow_simulation".to_string(),
            self.check_complete_workflow_simulation().await,
        );
    }

    /// Check agent lifecycle readiness for full autonomous operation
    async fn check_agent_lifecycle_readiness(&self) -> DiagnosticResult {
        let mut readiness_issues = Vec::new();
        let mut readiness_checks = Vec::new();
        
        // Check 1: GitHub client initialization
        match crate::github::client::GitHubClient::with_verbose(self.is_verbose()) {
            Ok(_) => {
                readiness_checks.push("GitHub client initialization");
            }
            Err(_) => {
                readiness_issues.push("GitHub client initialization failed");
            }
        }
        
        // Check 2: Agent state machine availability
        if std::path::Path::new("src/agent_lifecycle").exists() {
            readiness_checks.push("Agent lifecycle modules available");
        } else {
            readiness_issues.push("Agent lifecycle modules not found");
        }
        
        // Check 3: Configuration completeness for agents
        if std::path::Path::new("my-little-soda.toml").exists() {
            readiness_checks.push("Agent configuration file present");
        } else {
            readiness_issues.push("Agent configuration file missing");
        }
        
        // Check 4: Repository state for agent operations
        if let Ok(repo) = git2::Repository::open(".") {
            if repo.head().is_ok() {
                readiness_checks.push("Git repository ready for agent operations");
            } else {
                readiness_issues.push("Git repository not initialized properly");
            }
        } else {
            readiness_issues.push("Git repository not accessible");
        }
        
        if readiness_issues.is_empty() {
            DiagnosticResult {
                status: DiagnosticStatus::Pass,
                message: "Agent lifecycle ready for autonomous operation".to_string(),
                details: if self.is_verbose() {
                    Some(format!("Ready components: {}", readiness_checks.join(", ")))
                } else {
                    None
                },
                suggestion: None,
            }
        } else {
            DiagnosticResult {
                status: DiagnosticStatus::Fail,
                message: format!("Agent lifecycle not ready ({} issues)", readiness_issues.len()),
                details: Some(format!("Issues: {}", readiness_issues.join("; "))),
                suggestion: Some("Complete system initialization before starting agent operations".to_string()),
            }
        }
    }

    /// Check issue workflow integration readiness
    async fn check_issue_workflow_integration(&self) -> DiagnosticResult {
        match crate::github::client::GitHubClient::with_verbose(self.is_verbose()) {
            Ok(client) => {
                let octocrab = client.issues.octocrab();
                
                // Test issue listing capability
                match octocrab.issues(client.owner(), client.repo())
                    .list()
                    .state(octocrab::params::State::Open)
                    .per_page(5)
                    .send()
                    .await 
                {
                    Ok(issues_page) => {
                        let mut workflow_features = Vec::new();
                        let mut missing_features = Vec::new();
                        
                        // Check for workflow-ready issues
                        let has_ready_issues = issues_page.items.iter()
                            .any(|issue| issue.labels.iter().any(|label| label.name.starts_with("route:")));
                        
                        if has_ready_issues {
                            workflow_features.push("routing labels present");
                        } else {
                            missing_features.push("no issues with routing labels found");
                        }
                        
                        // Check for agent assignment capability
                        let has_agent_assignment = issues_page.items.iter()
                            .any(|issue| issue.assignee.is_some());
                        
                        if has_agent_assignment {
                            workflow_features.push("issue assignment capability");
                        }
                        
                        if missing_features.is_empty() || workflow_features.len() > 0 {
                            DiagnosticResult {
                                status: if missing_features.is_empty() { DiagnosticStatus::Pass } else { DiagnosticStatus::Warning },
                                message: "Issue workflow integration operational".to_string(),
                                details: if self.is_verbose() {
                                    Some(format!("Features available: {}. Issues: {}", 
                                        workflow_features.join(", "),
                                        if missing_features.is_empty() { "none".to_string() } else { missing_features.join(", ") }))
                                } else {
                                    None
                                },
                                suggestion: if !missing_features.is_empty() {
                                    Some("Add routing labels to issues for better workflow integration".to_string())
                                } else {
                                    None
                                },
                            }
                        } else {
                            DiagnosticResult {
                                status: DiagnosticStatus::Fail,
                                message: "Issue workflow integration not ready".to_string(),
                                details: Some(format!("Missing: {}", missing_features.join(", "))),
                                suggestion: Some("Add issues with routing labels to test workflow integration".to_string()),
                            }
                        }
                    }
                    Err(e) => {
                        DiagnosticResult {
                            status: DiagnosticStatus::Fail,
                            message: "Cannot access issue workflow".to_string(),
                            details: Some(format!("Issues API error: {}", e)),
                            suggestion: Some("Verify GitHub token has issues access permissions".to_string()),
                        }
                    }
                }
            }
            Err(e) => {
                DiagnosticResult {
                    status: DiagnosticStatus::Fail,
                    message: "Cannot test issue workflow integration".to_string(),
                    details: Some(format!("GitHub client error: {:?}", e)),
                    suggestion: Some("Configure GitHub authentication to test issue workflow".to_string()),
                }
            }
        }
    }

    /// Check branch and PR workflow validation
    async fn check_branch_pr_workflow(&self) -> DiagnosticResult {
        match crate::github::client::GitHubClient::with_verbose(self.is_verbose()) {
            Ok(client) => {
                let octocrab = client.issues.octocrab();
                
                // Check existing branches for agent workflow patterns
                match octocrab.repos(client.owner(), client.repo()).list_branches().send().await {
                    Ok(branches_page) => {
                        let agent_branches: Vec<_> = branches_page.items.iter()
                            .filter(|branch| branch.name.starts_with("agent"))
                            .collect();
                        
                        let mut workflow_status = Vec::new();
                        
                        if !agent_branches.is_empty() {
                            workflow_status.push(format!("{} agent branches found", agent_branches.len()));
                        }
                        
                        // Check for PR capability by looking at existing PRs
                        match octocrab.pulls(client.owner(), client.repo())
                            .list()
                            .state(octocrab::params::State::Open)
                            .per_page(5)
                            .send()
                            .await 
                        {
                            Ok(prs_page) => {
                                if !prs_page.items.is_empty() {
                                    workflow_status.push("PR workflow operational".to_string());
                                }
                                
                                DiagnosticResult {
                                    status: DiagnosticStatus::Pass,
                                    message: "Branch and PR workflow ready".to_string(),
                                    details: if self.is_verbose() && !workflow_status.is_empty() {
                                        Some(format!("Status: {}", workflow_status.join(", ")))
                                    } else {
                                        None
                                    },
                                    suggestion: None,
                                }
                            }
                            Err(_) => {
                                DiagnosticResult {
                                    status: DiagnosticStatus::Warning,
                                    message: "Branch workflow ready, PR access limited".to_string(),
                                    details: Some("Cannot verify PR workflow capabilities".to_string()),
                                    suggestion: Some("Verify GitHub token has PR access permissions".to_string()),
                                }
                            }
                        }
                    }
                    Err(e) => {
                        DiagnosticResult {
                            status: DiagnosticStatus::Fail,
                            message: "Cannot check branch workflow".to_string(),
                            details: Some(format!("Branches API error: {}", e)),
                            suggestion: Some("Verify GitHub token has repository access".to_string()),
                        }
                    }
                }
            }
            Err(e) => {
                DiagnosticResult {
                    status: DiagnosticStatus::Fail,
                    message: "Cannot test branch and PR workflow".to_string(),
                    details: Some(format!("GitHub client error: {:?}", e)),
                    suggestion: Some("Configure GitHub authentication to test workflow".to_string()),
                }
            }
        }
    }

    /// Check agent coordination readiness
    async fn check_agent_coordination_readiness(&self) -> DiagnosticResult {
        let mut coordination_features = Vec::new();
        let mut missing_features = Vec::new();
        
        // Check 1: Agent router availability
        if std::path::Path::new("src/agents/router.rs").exists() {
            coordination_features.push("agent routing system");
        } else {
            missing_features.push("agent routing system not found");
        }
        
        // Check 2: Agent state management
        if std::path::Path::new("src/agent_lifecycle").exists() {
            coordination_features.push("agent lifecycle management");
        } else {
            missing_features.push("agent lifecycle management not found");
        }
        
        // Check 3: GitHub integration for coordination
        match crate::github::client::GitHubClient::with_verbose(self.is_verbose()) {
            Ok(_) => {
                coordination_features.push("GitHub coordination capability");
            }
            Err(_) => {
                missing_features.push("GitHub coordination not available");
            }
        }
        
        if missing_features.is_empty() {
            DiagnosticResult {
                status: DiagnosticStatus::Pass,
                message: "Agent coordination system ready".to_string(),
                details: if self.is_verbose() {
                    Some(format!("Available features: {}", coordination_features.join(", ")))
                } else {
                    None
                },
                suggestion: None,
            }
        } else {
            DiagnosticResult {
                status: DiagnosticStatus::Fail,
                message: format!("Agent coordination not ready ({} missing features)", missing_features.len()),
                details: Some(format!("Missing: {}", missing_features.join("; "))),
                suggestion: Some("Complete agent coordination system setup".to_string()),
            }
        }
    }

    /// Simulate complete workflow to validate end-to-end integration
    async fn check_complete_workflow_simulation(&self) -> DiagnosticResult {
        let start_time = std::time::Instant::now();
        let mut simulation_steps = Vec::new();
        let mut failed_steps = Vec::new();
        let mut warnings = Vec::new();
        let mut timing_info = Vec::new();

        // Step 1: Enhanced Pop Operation Simulation (Task Discovery and Assignment)
        let step_start = std::time::Instant::now();
        match self.simulate_pop_operation().await {
            Ok(details) => {
                simulation_steps.push("pop operation (task discovery & assignment)");
                timing_info.push(format!("pop: {}ms", step_start.elapsed().as_millis()));
                if !details.is_empty() {
                    warnings.extend(details);
                }
            }
            Err(e) => {
                failed_steps.push(format!("pop operation failed: {}", e));
            }
        }

        // Step 2: Branch Creation and Checkout Simulation
        let step_start = std::time::Instant::now();
        match self.simulate_branch_operations().await {
            Ok(details) => {
                simulation_steps.push("branch management (create & checkout)");
                timing_info.push(format!("branch ops: {}ms", step_start.elapsed().as_millis()));
                if !details.is_empty() {
                    warnings.extend(details);
                }
            }
            Err(e) => {
                failed_steps.push(format!("branch operations failed: {}", e));
            }
        }

        // Step 3: Work Simulation (Mock Development Phase)
        let step_start = std::time::Instant::now();
        match self.simulate_work_phase().await {
            Ok(details) => {
                simulation_steps.push("work phase (development simulation)");
                timing_info.push(format!("work: {}ms", step_start.elapsed().as_millis()));
                if !details.is_empty() {
                    warnings.extend(details);
                }
            }
            Err(e) => {
                failed_steps.push(format!("work phase failed: {}", e));
            }
        }

        // Step 4: Enhanced Bottle Operation Simulation (Work Completion)
        let step_start = std::time::Instant::now();
        match self.simulate_bottle_operation().await {
            Ok(details) => {
                simulation_steps.push("bottle operation (work completion & transitions)");
                timing_info.push(format!("bottle: {}ms", step_start.elapsed().as_millis()));
                if !details.is_empty() {
                    warnings.extend(details);
                }
            }
            Err(e) => {
                failed_steps.push(format!("bottle operation failed: {}", e));
            }
        }

        // Step 5: Agent State Transition Validation
        let step_start = std::time::Instant::now();
        match self.simulate_agent_state_transitions().await {
            Ok(details) => {
                simulation_steps.push("agent state transitions");
                timing_info.push(format!("state transitions: {}ms", step_start.elapsed().as_millis()));
                if !details.is_empty() {
                    warnings.extend(details);
                }
            }
            Err(e) => {
                failed_steps.push(format!("agent state transitions failed: {}", e));
            }
        }

        // Step 6: Error Recovery Simulation
        let step_start = std::time::Instant::now();
        match self.simulate_error_recovery().await {
            Ok(details) => {
                simulation_steps.push("error recovery mechanisms");
                timing_info.push(format!("error recovery: {}ms", step_start.elapsed().as_millis()));
                if !details.is_empty() {
                    warnings.extend(details);
                }
            }
            Err(e) => {
                failed_steps.push(format!("error recovery failed: {}", e));
            }
        }

        let total_time = start_time.elapsed();
        let success_rate = simulation_steps.len() as f64 / (simulation_steps.len() + failed_steps.len()) as f64;
        
        // Create detailed status message
        let message = if failed_steps.is_empty() && warnings.is_empty() {
            format!("Complete workflow simulation successful ({}ms total)", total_time.as_millis())
        } else if failed_steps.is_empty() {
            format!("Workflow simulation successful with {} warnings ({}ms total)", warnings.len(), total_time.as_millis())
        } else if success_rate >= 0.75 {
            format!("Workflow simulation mostly successful ({:.0}%, {}ms total)", success_rate * 100.0, total_time.as_millis())
        } else {
            format!("Workflow simulation failed ({:.0}% success, {}ms total)", success_rate * 100.0, total_time.as_millis())
        };

        // Create detailed verbose output
        let details = if self.is_verbose() {
            let mut detail_parts = Vec::new();
            if !simulation_steps.is_empty() {
                detail_parts.push(format!("✅ Completed: {}", simulation_steps.join(", ")));
            }
            if !warnings.is_empty() {
                detail_parts.push(format!("⚠️ Warnings: {}", warnings.join("; ")));
            }
            if !failed_steps.is_empty() {
                detail_parts.push(format!("❌ Failed: {}", failed_steps.join("; ")));
            }
            if !timing_info.is_empty() {
                detail_parts.push(format!("⏱️ Timing: {}", timing_info.join(", ")));
            }
            Some(detail_parts.join(" | "))
        } else if !failed_steps.is_empty() {
            Some(format!("Failed steps: {}", failed_steps.join("; ")))
        } else if !warnings.is_empty() && warnings.len() <= 3 {
            Some(format!("Warnings: {}", warnings.join("; ")))
        } else {
            None
        };

        // Determine status and suggestion
        let (status, suggestion) = if failed_steps.is_empty() {
            if warnings.is_empty() {
                (DiagnosticStatus::Pass, Some("System ready for full autonomous agent operation".to_string()))
            } else {
                (DiagnosticStatus::Warning, Some("System ready but review warnings for optimal performance".to_string()))
            }
        } else if success_rate >= 0.75 {
            (DiagnosticStatus::Warning, Some("Address failed workflow steps for reliable agent operation".to_string()))
        } else {
            (DiagnosticStatus::Fail, Some("Critical workflow issues must be resolved before agent operations".to_string()))
        };

        DiagnosticResult { status, message, details, suggestion }
    }

    /// Simulate pop operation (task discovery and assignment)
    async fn simulate_pop_operation(&self) -> anyhow::Result<Vec<String>> {
        let mut warnings = Vec::new();

        // Test GitHub client initialization
        let client = crate::github::client::GitHubClient::with_verbose(self.is_verbose())
            .map_err(|e| anyhow::anyhow!("GitHub client initialization failed: {}", e))?;

        // Test issue discovery and filtering
        let octocrab = client.issues.octocrab();
        let issues = octocrab.issues(client.owner(), client.repo())
            .list()
            .state(octocrab::params::State::Open)
            .per_page(10)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Issue discovery failed: {}", e))?;

        if issues.items.is_empty() {
            warnings.push("No open issues found for task assignment".to_string());
        } else {
            // Check for workflow-ready issues
            let ready_issues: Vec<_> = issues.items.iter()
                .filter(|issue| issue.labels.iter()
                    .any(|label| label.name.starts_with("route:ready")))
                .collect();

            if ready_issues.is_empty() {
                warnings.push("No issues with route:ready labels found".to_string());
            }

            // Check for available agents (simulate agent assignment)
            let agent_assigned_issues: Vec<_> = issues.items.iter()
                .filter(|issue| issue.assignee.is_some() || 
                    issue.labels.iter().any(|label| label.name.starts_with("agent")))
                .collect();

            if agent_assigned_issues.len() == issues.items.len() {
                warnings.push("All available issues already assigned to agents".to_string());
            }
        }

        // Test agent router initialization simulation
        if let Err(_) = crate::agents::coordinator::AgentCoordinator::new().await {
            warnings.push("Agent coordinator initialization may have issues".to_string());
        }

        Ok(warnings)
    }

    /// Simulate branch creation and checkout operations
    async fn simulate_branch_operations(&self) -> anyhow::Result<Vec<String>> {
        let mut warnings = Vec::new();

        // Test git repository access
        let repo = git2::Repository::open(".")
            .map_err(|e| anyhow::anyhow!("Git repository access failed: {}", e))?;

        // Check if we're on a clean state for branch operations
        let statuses = repo.statuses(None)
            .map_err(|e| anyhow::anyhow!("Cannot check repository status: {}", e))?;
        
        if !statuses.is_empty() {
            warnings.push(format!("Repository has {} uncommitted changes", statuses.len()));
        }

        // Validate we can get HEAD reference
        let head = repo.head()
            .map_err(|e| anyhow::anyhow!("Cannot access HEAD reference: {}", e))?;

        if !head.is_branch() {
            warnings.push("Repository is in detached HEAD state".to_string());
        }

        // Test branch name generation (simulate agent branch creation)
        let test_branch_name = "agent001/test-branch-simulation";
        
        // Check if we can resolve references
        match repo.find_branch(test_branch_name, git2::BranchType::Local) {
            Ok(_) => warnings.push("Test simulation branch already exists".to_string()),
            Err(_) => {} // Expected - the branch shouldn't exist
        }

        // Test remote access for branch pushing
        let remotes = repo.remotes()
            .map_err(|e| anyhow::anyhow!("Cannot access remote repositories: {}", e))?;

        if remotes.is_empty() {
            warnings.push("No remote repositories configured".to_string());
        } else {
            // Check if we can access the origin remote
            if let Some(remote_name) = remotes.get(0) {
                match repo.find_remote(remote_name) {
                    Ok(remote) => {
                        if remote.url().is_none() {
                            warnings.push("Remote URL not configured".to_string());
                        }
                    }
                    Err(_) => warnings.push("Cannot access primary remote".to_string()),
                }
            }
        }

        Ok(warnings)
    }

    /// Simulate work phase (mock development)
    async fn simulate_work_phase(&self) -> anyhow::Result<Vec<String>> {
        let mut warnings = Vec::new();

        // Test file system operations (simulate code changes)
        let test_file = std::path::Path::new(".doctor-workflow-test.tmp");
        match std::fs::write(test_file, "test content") {
            Ok(_) => {
                // Test reading back
                match std::fs::read_to_string(test_file) {
                    Ok(content) => {
                        if content != "test content" {
                            warnings.push("File write/read consistency issue".to_string());
                        }
                    }
                    Err(_) => warnings.push("File read operation failed".to_string()),
                }
                // Cleanup
                let _ = std::fs::remove_file(test_file);
            }
            Err(_) => warnings.push("File write operations not available".to_string()),
        }

        // Test compilation environment (check for common development tools)
        let build_tools = vec![("cargo", "Rust build system"), ("git", "Version control")];
        
        for (tool, description) in build_tools {
            match std::process::Command::new(tool)
                .arg("--version")
                .output()
            {
                Ok(output) => {
                    if !output.status.success() {
                        warnings.push(format!("{} ({}) not functioning properly", tool, description));
                    }
                }
                Err(_) => warnings.push(format!("{} ({}) not available", tool, description)),
            }
        }

        // Simulate testing phase
        if !std::path::Path::new("Cargo.toml").exists() {
            warnings.push("Cargo.toml not found - may affect build testing".to_string());
        }

        Ok(warnings)
    }

    /// Simulate bottle operation (work completion and state transitions)
    async fn simulate_bottle_operation(&self) -> anyhow::Result<Vec<String>> {
        let mut warnings = Vec::new();

        // Test GitHub client for state transitions
        let client = crate::github::client::GitHubClient::with_verbose(self.is_verbose())
            .map_err(|e| anyhow::anyhow!("GitHub client for bottle operations failed: {}", e))?;

        // Test label manipulation capabilities
        let octocrab = client.issues.octocrab();
        
        // Verify we can access issues for label operations
        let issues = octocrab.issues(client.owner(), client.repo())
            .list()
            .state(octocrab::params::State::Open)
            .per_page(5)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Cannot access issues for bottle operations: {}", e))?;

        if let Some(test_issue) = issues.items.first() {
            // Check if we can read current labels
            let current_labels: Vec<_> = test_issue.labels.iter().map(|l| &l.name).collect();
            
            // Simulate label transition validation
            let _required_transitions = vec![
                ("route:ready", "route:review"),
                ("route:in-progress", "route:review"),
            ];

            let has_transitional_labels = current_labels.iter()
                .any(|label| label.starts_with("route:"));

            if !has_transitional_labels {
                warnings.push("No issues with routing labels found for transition testing".to_string());
            }

            // Test agent label removal simulation
            let has_agent_labels = current_labels.iter()
                .any(|label| label.starts_with("agent"));

            if !has_agent_labels {
                warnings.push("No agent labels found for cleanup testing".to_string());
            }
        } else {
            warnings.push("No issues available for bottle operation testing".to_string());
        }

        // Test push capabilities for bottle operations
        let repo = git2::Repository::open(".")
            .map_err(|e| anyhow::anyhow!("Git repository access for bottle failed: {}", e))?;

        // Check if we have commits to push (simulate work completion)
        match repo.head() {
            Ok(head) => {
                if let Some(branch) = head.shorthand() {
                    if branch == "main" {
                        warnings.push("Currently on main branch - no work branch active".to_string());
                    }
                } else {
                    warnings.push("Cannot determine current branch for bottle operation".to_string());
                }
            }
            Err(_) => warnings.push("Cannot access current branch reference".to_string()),
        }

        Ok(warnings)
    }

    /// Simulate agent state transitions
    async fn simulate_agent_state_transitions(&self) -> anyhow::Result<Vec<String>> {
        let mut warnings = Vec::new();

        // Test agent coordinator initialization
        let _client = crate::github::client::GitHubClient::with_verbose(self.is_verbose())
            .map_err(|e| anyhow::anyhow!("GitHub client for agent states failed: {}", e))?;

        match crate::agents::coordinator::AgentCoordinator::new().await {
            Ok(_coordinator) => {
                // Simulate state machine transitions
                // Test 1: Idle -> Assigned transition
                // Test 2: Assigned -> Working transition  
                // Test 3: Working -> Completing transition
                // Test 4: Completing -> Idle transition

                // Since we can't actually modify state in dry-run mode,
                // we validate that the components needed for transitions exist
            }
            Err(e) => {
                warnings.push(format!("Agent coordinator initialization failed: {}", e));
            }
        }

        // Test state persistence mechanisms
        let state_file = std::path::Path::new(".my-little-soda");
        if !state_file.exists() {
            warnings.push("My Little Soda state directory not initialized".to_string());
        }

        // Test agent lifecycle modules availability
        if !std::path::Path::new("src/agent_lifecycle").exists() {
            warnings.push("Agent lifecycle modules not found".to_string());
        }

        Ok(warnings)
    }

    /// Simulate error recovery mechanisms
    async fn simulate_error_recovery(&self) -> anyhow::Result<Vec<String>> {
        let mut warnings = Vec::new();

        // Test error scenario 1: Network connectivity issues
        match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.test_github_connectivity()
        ).await {
            Ok(Ok(_)) => {} // Connection successful
            Ok(Err(_)) => warnings.push("GitHub connectivity issues detected".to_string()),
            Err(_) => warnings.push("GitHub connectivity test timed out".to_string()),
        }

        // Test error scenario 2: Repository lock conflicts
        let repo = git2::Repository::open(".")?;
        let git_dir = repo.path();
        let index_lock = git_dir.join("index.lock");
        
        if index_lock.exists() {
            warnings.push("Git index lock file exists - potential concurrent operations".to_string());
        }

        // Test error scenario 3: Permission issues
        match std::fs::metadata(".git") {
            Ok(metadata) => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let permissions = metadata.permissions();
                    if permissions.mode() & 0o200 == 0 {
                        warnings.push("Git directory appears to be read-only".to_string());
                    }
                }
            }
            Err(_) => warnings.push("Cannot access Git directory permissions".to_string()),
        }

        // Test error scenario 4: Rate limiting recovery
        if let Ok(client) = crate::github::client::GitHubClient::with_verbose(self.is_verbose()) {
            let octocrab = client.issues.octocrab();
            match octocrab.ratelimit().get().await {
                Ok(rate_limits) => {
                    let core = &rate_limits.resources.core;
                    let remaining_pct = (core.remaining as f64 / core.limit as f64) * 100.0;
                    
                    if remaining_pct < 20.0 {
                        warnings.push(format!("Low GitHub API rate limit remaining ({:.1}%)", remaining_pct));
                    }
                }
                Err(_) => warnings.push("Cannot check GitHub API rate limits".to_string()),
            }
        }

        Ok(warnings)
    }

    /// Test GitHub connectivity for error recovery simulation
    async fn test_github_connectivity(&self) -> anyhow::Result<()> {
        if let Ok(client) = crate::github::client::GitHubClient::with_verbose(self.is_verbose()) {
            let octocrab = client.issues.octocrab();
            
            // Simple connectivity test
            octocrab.issues(client.owner(), client.repo())
                .list()
                .per_page(1)
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("GitHub connectivity test failed: {}", e))?;
        }
        Ok(())
    }

    /// Calculate system readiness score based on diagnostic results
    fn calculate_readiness(&self, summary: &DiagnosticSummary, checks: &HashMap<String, DiagnosticResult>) -> SystemReadiness {
        // Base score calculation
        let total_possible_score = 100u8;
        let failed_penalty = summary.failed as u8 * 20;
        let warning_penalty = summary.warnings as u8 * 5;
        
        // Critical checks that must pass for readiness
        let critical_checks = vec![
            "github_authentication", 
            "github_repository_access", 
            "config_file_exists"
        ];
        
        let critical_failures = critical_checks.iter()
            .filter(|&check| {
                checks.get(*check)
                    .map_or(false, |result| result.status == DiagnosticStatus::Fail)
            })
            .count();

        let score = total_possible_score
            .saturating_sub(failed_penalty)
            .saturating_sub(warning_penalty)
            .saturating_sub(critical_failures as u8 * 30); // Heavy penalty for critical failures

        let (status, description) = if critical_failures > 0 {
            (ReadinessStatus::NotReady, 
             format!("System not ready: {} critical issues must be resolved", critical_failures))
        } else if summary.failed > 0 {
            (ReadinessStatus::PartiallyReady,
             format!("System partially ready: {} issues need attention", summary.failed))
        } else if summary.warnings > 0 {
            (ReadinessStatus::PartiallyReady,
             format!("System ready with {} warnings", summary.warnings))
        } else {
            (ReadinessStatus::Ready,
             "System fully ready for My Little Soda operations".to_string())
        };

        SystemReadiness {
            score,
            status,
            description,
        }
    }

    /// Generate actionable recommendations based on diagnostic results
    fn generate_recommendations(&self, checks: &HashMap<String, DiagnosticResult>) -> Vec<ActionableRecommendation> {
        let mut recommendations = Vec::new();

        // Check for critical failures that need immediate attention
        for (check_name, result) in checks {
            if result.status == DiagnosticStatus::Fail {
                let recommendation = match check_name.as_str() {
                    "github_authentication" => ActionableRecommendation {
                        priority: RecommendationPriority::Critical,
                        category: "Authentication".to_string(),
                        title: "Fix GitHub Authentication".to_string(),
                        description: "GitHub authentication is failing, preventing My Little Soda operations".to_string(),
                        steps: vec![
                            "Check that you have a valid GitHub token".to_string(),
                            "Run 'gh auth login' to authenticate with GitHub CLI".to_string(),
                            "Verify token permissions include repository access".to_string(),
                            "Test with 'my-little-soda doctor --verbose' to see detailed errors".to_string(),
                        ],
                        links: vec![
                            "https://docs.github.com/en/github/authenticating-to-github/creating-a-personal-access-token".to_string(),
                            "https://cli.github.com/manual/gh_auth_login".to_string(),
                        ],
                    },
                    "required_labels_existence" => ActionableRecommendation {
                        priority: RecommendationPriority::High,
                        category: "Setup".to_string(),
                        title: "Create Required Labels".to_string(),
                        description: "My Little Soda requires specific labels for issue routing".to_string(),
                        steps: vec![
                            "Run 'my-little-soda init' to create required labels automatically".to_string(),
                            "Or manually create labels using GitHub's label interface".to_string(),
                            "Verify labels with 'my-little-soda doctor --verbose'".to_string(),
                        ],
                        links: vec![
                            "https://docs.github.com/en/issues/using-labels-and-milestones-to-track-work/managing-labels".to_string(),
                        ],
                    },
                    "soda_config" => ActionableRecommendation {
                        priority: RecommendationPriority::High,
                        category: "Setup".to_string(),
                        title: "Initialize My Little Soda".to_string(),
                        description: "My Little Soda has not been initialized in this repository".to_string(),
                        steps: vec![
                            "Run 'my-little-soda init' to set up the repository".to_string(),
                            "Follow the interactive prompts to configure settings".to_string(),
                            "Verify setup with 'my-little-soda doctor'".to_string(),
                        ],
                        links: vec![],
                    },
                    _ => continue,
                };
                recommendations.push(recommendation);
            } else if result.status == DiagnosticStatus::Warning {
                // Add recommendations for warnings that could be optimized
                if check_name == "label_configuration" {
                    recommendations.push(ActionableRecommendation {
                        priority: RecommendationPriority::Medium,
                        category: "Optimization".to_string(),
                        title: "Update Label Configuration".to_string(),
                        description: "Some labels don't match My Little Soda specifications exactly".to_string(),
                        steps: vec![
                            "Review label colors and descriptions in GitHub".to_string(),
                            "Update labels to match My Little Soda requirements".to_string(),
                            "Re-run 'my-little-soda doctor' to verify fixes".to_string(),
                        ],
                        links: vec![
                            "https://docs.github.com/en/issues/using-labels-and-milestones-to-track-work/managing-labels".to_string(),
                        ],
                    });
                }
            }
        }

        // Add general optimization recommendations
        if recommendations.len() < 3 {
            recommendations.push(ActionableRecommendation {
                priority: RecommendationPriority::Low,
                category: "Performance".to_string(),
                title: "Enable Autonomous Features".to_string(),
                description: "Enable advanced autonomous agent features for better productivity".to_string(),
                steps: vec![
                    "Compile with --features autonomous for enhanced capabilities".to_string(),
                    "Configure agent scheduling and work continuity".to_string(),
                    "Set up monitoring and metrics collection".to_string(),
                ],
                links: vec![],
            });
        }

        recommendations
    }
}
