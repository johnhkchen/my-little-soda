use super::types::{DiagnosticResult, DiagnosticStatus};
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::process::Command;

/// System diagnostics functionality for checking dependencies and system resources
pub struct SystemDiagnostics {
    verbose: bool,
}

impl SystemDiagnostics {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }

    fn is_verbose(&self) -> bool {
        self.verbose
    }

    pub fn check_dependencies(&self, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
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
        match Command::new("rustc")
            .arg("--version")
            .output()
        {
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
                            suggestion: Some(
                                "Verify Rust installation: rustc --version".to_string(),
                            ),
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
    fn check_cargo_functionality(
        &self,
        checks: &mut HashMap<String, DiagnosticResult>,
    ) -> Result<()> {
        match Command::new("cargo")
            .arg("--version")
            .output()
        {
            Ok(output) if output.status.success() => {
                let version_str = String::from_utf8_lossy(&output.stdout);

                // Test cargo functionality with a simple check command
                let cargo_check_result = Command::new("cargo")
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
                        suggestion: Some(
                            "Install Cargo with Rust toolchain: https://rustup.rs/".to_string(),
                        ),
                    },
                );
            }
        }
        Ok(())
    }

    /// Check Git installation and version validation
    fn check_git_installation(&self, checks: &mut HashMap<String, DiagnosticResult>) -> Result<()> {
        match Command::new("git").arg("--version").output() {
            Ok(output) if output.status.success() => {
                let version_str = String::from_utf8_lossy(&output.stdout);
                let version_parts: Vec<&str> = version_str.split_whitespace().collect();

                let (status, suggestion) = if version_parts.len() >= 3 {
                    let version = version_parts[2];
                    if Self::check_git_version_requirement(version) {
                        (DiagnosticStatus::Pass, None)
                    } else {
                        (
                            DiagnosticStatus::Warning,
                            Some("Consider updating Git to version 2.20.0 or higher".to_string()),
                        )
                    }
                } else {
                    (
                        DiagnosticStatus::Warning,
                        Some("Could not parse Git version".to_string()),
                    )
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
    fn check_github_cli_availability(
        &self,
        checks: &mut HashMap<String, DiagnosticResult>,
    ) -> Result<()> {
        match Command::new("gh").arg("--version").output() {
            Ok(output) if output.status.success() => {
                let version_str = String::from_utf8_lossy(&output.stdout);
                let first_line = version_str.lines().next().unwrap_or("");

                // Check if authenticated
                let auth_status = Command::new("gh")
                    .arg("auth")
                    .arg("status")
                    .output();

                let (status, message, suggestion) =
                    if auth_status.map_or(false, |o| o.status.success()) {
                        (
                            DiagnosticStatus::Pass,
                            "GitHub CLI is available and authenticated".to_string(),
                            None,
                        )
                    } else {
                        (
                            DiagnosticStatus::Warning,
                            "GitHub CLI is available but not authenticated".to_string(),
                            Some("Run 'gh auth login' to authenticate with GitHub".to_string()),
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
    fn check_github_connectivity(
        &self,
        checks: &mut HashMap<String, DiagnosticResult>,
    ) -> Result<()> {
        // Simple connectivity test using curl or available HTTP client
        let connectivity_test = Command::new("curl")
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
                let dns_test = Command::new("nslookup")
                    .arg("api.github.com")
                    .output()
                    .or_else(|_| {
                        Command::new("dig")
                            .arg("api.github.com")
                            .output()
                    });

                let (status, message, suggestion) = match dns_test {
                    Ok(output) if output.status.success() => (
                        DiagnosticStatus::Warning,
                        "DNS resolution works but HTTP connectivity test failed".to_string(),
                        Some("Check firewall settings and network connectivity".to_string()),
                    ),
                    _ => (
                        DiagnosticStatus::Warning,
                        "Network connectivity to GitHub could not be verified".to_string(),
                        Some(
                            "Ensure network connectivity and DNS resolution are working"
                                .to_string(),
                        ),
                    ),
                };

                checks.insert(
                    "github_connectivity".to_string(),
                    DiagnosticResult {
                        status,
                        message,
                        details: None,
                        suggestion,
                    },
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
                    resource_warnings
                        .push("Low available memory (< 1GB) may affect performance".to_string());
                }
            }
        }

        // Check CPU information
        if let Ok(cpuinfo) = fs::read_to_string("/proc/cpuinfo") {
            let cpu_count = cpuinfo
                .lines()
                .filter(|line| line.starts_with("processor"))
                .count();
            resource_info.push(format!("CPU cores: {}", cpu_count));

            if cpu_count < 2 {
                resource_warnings.push("Single CPU core may affect build performance".to_string());
            }
        }

        // Check disk space in current directory
        match fs::metadata(".") {
            Ok(_) => {
                // Use df command to check disk space
                if let Ok(output) = Command::new("df").arg("-h").arg(".").output() {
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
            format!(
                "System resources have {} potential issues",
                resource_warnings.len()
            )
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
            "Install GitHub CLI: 'brew install gh' or download from https://cli.github.com/"
                .to_string()
        } else {
            "Install GitHub CLI via package manager or download from https://cli.github.com/"
                .to_string()
        }
    }
}