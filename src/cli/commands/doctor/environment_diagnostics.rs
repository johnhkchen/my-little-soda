use super::types::{DiagnosticResult, DiagnosticStatus};
use anyhow::Result;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Environment and filesystem diagnostics functionality
pub struct EnvironmentDiagnostics {
    verbose: bool,
}

impl EnvironmentDiagnostics {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }

    fn is_verbose(&self) -> bool {
        self.verbose
    }

    /// Check required environment variables
    pub fn check_environment_variables(
        &self,
        checks: &mut HashMap<String, DiagnosticResult>,
    ) -> Result<()> {
        let mut issues = Vec::new();
        let mut warnings = Vec::new();
        let mut good = Vec::new();

        // Check MY_LITTLE_SODA_GITHUB_TOKEN
        match env::var("MY_LITTLE_SODA_GITHUB_TOKEN") {
            Ok(token) if !token.is_empty() && token != "YOUR_GITHUB_TOKEN_HERE" => {
                good.push("MY_LITTLE_SODA_GITHUB_TOKEN is set ✅".to_string());
            }
            _ => {
                // Check alternative token sources
                if env::var("GITHUB_TOKEN").is_ok() {
                    warnings.push(
                        "Using GITHUB_TOKEN instead of MY_LITTLE_SODA_GITHUB_TOKEN".to_string(),
                    );
                } else if Path::new(".my-little-soda/credentials/github_token").exists() {
                    good.push("GitHub token available via file-based config ✅".to_string());
                } else if Command::new("gh")
                    .args(["auth", "status"])
                    .output()
                    .map_or(false, |o| o.status.success())
                {
                    good.push("GitHub token available via gh CLI ✅".to_string());
                } else {
                    issues.push("No GitHub token found".to_string());
                }
            }
        }

        let status = if !issues.is_empty() {
            DiagnosticStatus::Fail
        } else if !warnings.is_empty() {
            DiagnosticStatus::Warning
        } else {
            DiagnosticStatus::Pass
        };

        checks.insert(
            "environment_variables".to_string(),
            DiagnosticResult {
                status,
                message: if issues.is_empty() && warnings.is_empty() {
                    "Required environment variables are configured".to_string()
                } else {
                    format!(
                        "Environment variable issues detected ({} problems)",
                        issues.len() + warnings.len()
                    )
                },
                details: if self.is_verbose() || !issues.is_empty() || !warnings.is_empty() {
                    let mut details = good;
                    details.extend(warnings.iter().map(|w| format!("⚠️  {}", w)));
                    details.extend(issues.iter().map(|i| format!("❌ {}", i)));
                    Some(details.join("; "))
                } else {
                    None
                },
                suggestion: if !issues.is_empty() {
                    Some("Set MY_LITTLE_SODA_GITHUB_TOKEN or configure alternative authentication".to_string())
                } else {
                    None
                },
            },
        );

        Ok(())
    }

    /// Check file system permissions for common operations
    pub fn check_file_system_permissions(
        &self,
        checks: &mut HashMap<String, DiagnosticResult>,
    ) -> Result<()> {
        let mut permission_issues = Vec::new();
        let mut permission_good = Vec::new();

        // Test current directory write permissions
        let test_file = ".my-little-soda-doctor-test";
        match fs::write(test_file, "test") {
            Ok(()) => {
                permission_good.push("Current directory writable ✅".to_string());
                let _ = fs::remove_file(test_file);
            }
            Err(_) => {
                permission_issues.push("Cannot write to current directory".to_string());
            }
        }

        let status = if permission_issues.is_empty() {
            DiagnosticStatus::Pass
        } else {
            DiagnosticStatus::Fail
        };

        checks.insert(
            "file_system_permissions".to_string(),
            DiagnosticResult {
                status,
                message: if permission_issues.is_empty() {
                    "File system permissions are adequate".to_string()
                } else {
                    format!("File system permission issues ({} problems)", permission_issues.len())
                },
                details: if self.is_verbose() || !permission_issues.is_empty() {
                    let mut details = permission_good;
                    details.extend(permission_issues.iter().map(|i| format!("❌ {}", i)));
                    Some(details.join("; "))
                } else {
                    None
                },
                suggestion: if !permission_issues.is_empty() {
                    Some("Check directory permissions and ensure write access".to_string())
                } else {
                    None
                },
            },
        );

        Ok(())
    }

    /// Check available disk space
    pub fn check_disk_space(&self, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
        match self.get_available_space(Path::new(".")) {
            Ok(available_gb) => {
                let status = if available_gb < 1.0 {
                    DiagnosticStatus::Fail
                } else if available_gb < 5.0 {
                    DiagnosticStatus::Warning
                } else {
                    DiagnosticStatus::Pass
                };

                checks.insert(
                    "disk_space".to_string(),
                    DiagnosticResult {
                        status,
                        message: format!("Available disk space: {:.1} GB", available_gb),
                        details: None,
                        suggestion: if available_gb < 1.0 {
                            Some("Free up disk space - at least 1GB recommended".to_string())
                        } else if available_gb < 5.0 {
                            Some("Consider freeing up more disk space for optimal performance".to_string())
                        } else {
                            None
                        },
                    },
                );
            }
            Err(_) => {
                checks.insert(
                    "disk_space".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Warning,
                        message: "Unable to check available disk space".to_string(),
                        details: None,
                        suggestion: Some("Manually verify sufficient disk space is available".to_string()),
                    },
                );
            }
        }

        Ok(())
    }

    fn get_available_space(&self, path: &Path) -> Result<f64> {
        if let Ok(output) = Command::new("df").arg("-BG").arg(path).output() {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                if let Some(line) = output_str.lines().nth(1) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 4 {
                        if let Ok(available) = parts[3].trim_end_matches('G').parse::<f64>() {
                            return Ok(available);
                        }
                    }
                }
            }
        }
        anyhow::bail!("Could not determine available disk space")
    }

    /// Additional environment checks can be added here
    pub fn check_path_configuration(
        &self,
        checks: &mut HashMap<String, DiagnosticResult>,
    ) -> Result<()> {
        // Simplified PATH check
        let path = env::var("PATH").unwrap_or_default();
        let status = if path.is_empty() {
            DiagnosticStatus::Fail
        } else {
            DiagnosticStatus::Pass
        };

        checks.insert(
            "path_configuration".to_string(),
            DiagnosticResult {
                status,
                message: if path.is_empty() {
                    "PATH environment variable is not set".to_string()
                } else {
                    "PATH environment variable is configured".to_string()
                },
                details: if self.is_verbose() && !path.is_empty() {
                    Some(format!("PATH entries: {}", path.split(':').count()))
                } else {
                    None
                },
                suggestion: if path.is_empty() {
                    Some("Configure PATH environment variable".to_string())
                } else {
                    None
                },
            },
        );

        Ok(())
    }
}