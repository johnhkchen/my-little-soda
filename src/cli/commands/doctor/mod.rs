pub mod agent_state;
pub mod github_labels;
pub mod types;
pub mod git_diagnostics;
pub mod system_diagnostics;
pub mod config_diagnostics;
pub mod github_auth_diagnostics;
pub mod github_repo_diagnostics;
pub mod environment_diagnostics;
pub mod workflow_diagnostics;
pub mod output;

use crate::cli::DoctorFormat;
use anyhow::Result;
use std::collections::HashMap;

// Re-export types for public API
pub use types::{
    DiagnosticResult, DiagnosticStatus, DiagnosticReport, DiagnosticSummary,
    SystemReadiness, ReadinessStatus, ActionableRecommendation, RecommendationPriority,
};

use git_diagnostics::GitDiagnostics;
use system_diagnostics::SystemDiagnostics;
use config_diagnostics::ConfigDiagnostics;
use github_auth_diagnostics::GitHubAuthDiagnostics;
use github_repo_diagnostics::GitHubRepoDiagnostics;
use environment_diagnostics::EnvironmentDiagnostics;
use workflow_diagnostics::WorkflowDiagnostics;
use output::DiagnosticOutput;

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
        let output = DiagnosticOutput::new(self.format.clone(), self.verbose);
        output.output_report(&report)?;

        // Exit with error if any critical checks failed
        if report.summary.failed > 0 {
            std::process::exit(1);
        }

        Ok(())
    }

    async fn run_diagnostics(&self) -> Result<DiagnosticReport> {
        let mut checks = HashMap::new();

        // Initialize diagnostic modules
        let git_diagnostics = GitDiagnostics::new(self.verbose);
        let system_diagnostics = SystemDiagnostics::new(self.verbose);
        let config_diagnostics = ConfigDiagnostics::new(self.verbose);
        let github_auth_diagnostics = GitHubAuthDiagnostics::new(self.verbose);
        let github_repo_diagnostics = GitHubRepoDiagnostics::new(self.verbose);
        let environment_diagnostics = EnvironmentDiagnostics::new(self.verbose);
        let workflow_diagnostics = WorkflowDiagnostics::new(self.verbose);
        let output = DiagnosticOutput::new(self.format.clone(), self.verbose);

        // Run basic system checks
        git_diagnostics.check_git_repository(&mut checks)?;
        system_diagnostics.check_dependencies(&mut checks)?;

        // Run comprehensive configuration validation
        config_diagnostics.check_toml_configuration(&mut checks)?;

        // Run comprehensive GitHub authentication diagnostics
        github_auth_diagnostics.check_github_authentication(&mut checks).await;

        // Run GitHub repository access diagnostics
        github_repo_diagnostics.check_github_repository_access(&mut checks).await;

        // Run GitHub issue label validation diagnostics (skip in JSON mode to avoid hanging)
        if matches!(self.format, DoctorFormat::Json) {
            // Add placeholder results for JSON mode to avoid expensive API calls
            checks.insert(
                "required_labels_existence".to_string(),
                DiagnosticResult {
                    status: DiagnosticStatus::Info,
                    message: "Label check skipped in JSON mode".to_string(),
                    details: Some("Use text mode for full label validation".to_string()),
                    suggestion: Some(
                        "Run 'my-little-soda doctor --verbose' for complete label checks"
                            .to_string(),
                    ),
                },
            );
        } else {
            workflow_diagnostics.check_github_issue_labels(&mut checks).await;
        }

        // Run environment validation diagnostics
        environment_diagnostics.check_environment_variables(&mut checks)?;
        environment_diagnostics.check_file_system_permissions(&mut checks)?;
        environment_diagnostics.check_disk_space(&mut checks)?;
        environment_diagnostics.check_path_configuration(&mut checks)?;

        // Run agent state diagnostics (skip expensive operations in JSON mode)
        if matches!(self.format, DoctorFormat::Json) {
            // Add placeholder for JSON mode
            checks.insert(
                "agent_state_overall".to_string(),
                DiagnosticResult {
                    status: DiagnosticStatus::Info,
                    message: "Agent state check skipped in JSON mode".to_string(),
                    details: Some("Use text mode for full agent state validation".to_string()),
                    suggestion: Some(
                        "Run 'my-little-soda doctor --verbose' for complete agent diagnostics"
                            .to_string(),
                    ),
                },
            );
        } else {
            workflow_diagnostics.check_agent_state_health(&mut checks).await;
        }

        // Run end-to-end workflow validation diagnostics (skip in JSON mode)
        if matches!(self.format, DoctorFormat::Json) {
            // Add placeholder for JSON mode
            checks.insert(
                "workflow_validation".to_string(),
                DiagnosticResult {
                    status: DiagnosticStatus::Info,
                    message: "Workflow validation skipped in JSON mode".to_string(),
                    details: Some("Use text mode for full workflow validation".to_string()),
                    suggestion: Some(
                        "Run 'my-little-soda doctor --verbose' for complete workflow checks"
                            .to_string(),
                    ),
                },
            );
        } else {
            workflow_diagnostics.check_end_to_end_workflow_validation(&mut checks).await;
        }

        // Calculate summary and generate recommendations
        let summary = output.calculate_summary(&checks);
        let readiness = self.calculate_readiness(&summary);
        let recommendations = self.generate_recommendations(&checks);

        Ok(DiagnosticReport {
            summary,
            checks,
            readiness,
            recommendations,
        })
    }

    /// Calculate system readiness score based on diagnostic results
    fn calculate_readiness(&self, summary: &DiagnosticSummary) -> SystemReadiness {
        let total = summary.total_checks as f32;
        let passed = summary.passed as f32;
        let failed = summary.failed as f32;

        // Calculate score: passed checks contribute positively, failed checks negatively
        let score = if total > 0.0 {
            let base_score = (passed / total) * 100.0;
            let penalty = (failed / total) * 30.0; // Failed checks have higher penalty
            (base_score - penalty).max(0.0).min(100.0) as u8
        } else {
            0
        };

        let (status, description) = match (score, summary.failed) {
            (90..=100, 0) => (ReadinessStatus::Ready, "System is fully ready for My Little Soda operations"),
            (70..=89, 0) => (ReadinessStatus::PartiallyReady, "System is mostly ready with minor issues to address"),
            (_, n) if n > 0 => (ReadinessStatus::NotReady, "System has critical issues that must be resolved"),
            _ => (ReadinessStatus::PartiallyReady, "System needs significant improvements before use"),
        };

        SystemReadiness {
            score,
            status,
            description: description.to_string(),
        }
    }

    /// Generate actionable recommendations based on failed checks
    fn generate_recommendations(&self, checks: &HashMap<String, DiagnosticResult>) -> Vec<ActionableRecommendation> {
        let mut recommendations = Vec::new();

        // Group similar issues and create prioritized recommendations
        let mut auth_issues = false;
        let mut config_issues = false;
        let mut system_issues = false;

        for (key, result) in checks {
            if matches!(result.status, DiagnosticStatus::Fail) {
                if key.contains("github") && key.contains("auth") {
                    auth_issues = true;
                } else if key.contains("config") {
                    config_issues = true;
                } else if key.contains("git") || key.contains("rust") || key.contains("cargo") {
                    system_issues = true;
                }
            }
        }

        // Add recommendations based on issue types
        if auth_issues {
            recommendations.push(ActionableRecommendation {
                priority: RecommendationPriority::Critical,
                category: "Authentication".to_string(),
                title: "Configure GitHub Authentication".to_string(),
                description: "GitHub authentication is not properly configured".to_string(),
                steps: vec![
                    "Create a GitHub Personal Access Token at https://github.com/settings/tokens".to_string(),
                    "Set the MY_LITTLE_SODA_GITHUB_TOKEN environment variable".to_string(),
                    "Ensure token has appropriate permissions (repo, issues, pull requests)".to_string(),
                ],
                links: vec![
                    "https://github.com/settings/tokens".to_string(),
                    "https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/creating-a-personal-access-token".to_string(),
                ],
            });
        }

        if config_issues {
            recommendations.push(ActionableRecommendation {
                priority: RecommendationPriority::High,
                category: "Configuration".to_string(),
                title: "Fix Configuration Issues".to_string(),
                description: "My Little Soda configuration needs attention".to_string(),
                steps: vec![
                    "Review my-little-soda.toml configuration file".to_string(),
                    "Update placeholder values with actual repository information".to_string(),
                    "Validate configuration syntax and completeness".to_string(),
                ],
                links: vec![],
            });
        }

        if system_issues {
            recommendations.push(ActionableRecommendation {
                priority: RecommendationPriority::Medium,
                category: "System Dependencies".to_string(),
                title: "Install Missing Dependencies".to_string(),
                description: "Required system dependencies are not available".to_string(),
                steps: vec![
                    "Install Git (https://git-scm.com/)".to_string(),
                    "Install Rust toolchain (https://rustup.rs/)".to_string(),
                    "Install GitHub CLI (https://cli.github.com/)".to_string(),
                ],
                links: vec![
                    "https://git-scm.com/".to_string(),
                    "https://rustup.rs/".to_string(),
                    "https://cli.github.com/".to_string(),
                ],
            });
        }

        recommendations
    }
}