use crate::cli::DoctorFormat;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
        self.check_github_config(&mut checks)?;
        self.check_my_little_soda_config(&mut checks)?;
        self.check_dependencies(&mut checks)?;
        
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

    fn check_github_config(&self, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
        // Check for GitHub token environment variable
        if std::env::var("MY_LITTLE_SODA_GITHUB_TOKEN").is_ok() {
            checks.insert(
                "github_token".to_string(),
                DiagnosticResult {
                    status: DiagnosticStatus::Pass,
                    message: "GitHub token configured".to_string(),
                    details: if self.verbose {
                        Some("MY_LITTLE_SODA_GITHUB_TOKEN environment variable found".to_string())
                    } else {
                        None
                    },
                    suggestion: None,
                },
            );
        } else {
            checks.insert(
                "github_token".to_string(),
                DiagnosticResult {
                    status: DiagnosticStatus::Fail,
                    message: "GitHub token not configured".to_string(),
                    details: None,
                    suggestion: Some("Set MY_LITTLE_SODA_GITHUB_TOKEN environment variable with your GitHub token".to_string()),
                },
            );
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
}