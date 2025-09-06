use super::types::{DiagnosticResult, DiagnosticStatus};
use crate::config::config;
use anyhow::Result;
use std::collections::HashMap;
use std::env;
use std::path::Path;

/// Configuration diagnostics functionality
pub struct ConfigDiagnostics {
    verbose: bool,
}

impl ConfigDiagnostics {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }

    fn is_verbose(&self) -> bool {
        self.verbose
    }

    pub fn check_toml_configuration(
        &self,
        checks: &mut HashMap<String, DiagnosticResult>,
    ) -> Result<()> {
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
                    details: Some(
                        "Found my-little-soda.example.toml but no my-little-soda.toml".to_string(),
                    ),
                    suggestion: Some(
                        "Copy my-little-soda.example.toml to my-little-soda.toml and customize it"
                            .to_string(),
                    ),
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
                Ok(content) => match toml::from_str::<toml::Value>(&content) {
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
                                suggestion: Some(
                                    "Fix TOML syntax errors in my-little-soda.toml".to_string(),
                                ),
                            },
                        );
                    }
                },
                Err(e) => {
                    checks.insert(
                        "config_toml_syntax".to_string(),
                        DiagnosticResult {
                            status: DiagnosticStatus::Fail,
                            message: "Cannot read configuration file".to_string(),
                            details: Some(format!("File read error: {}", e)),
                            suggestion: Some(
                                "Check file permissions for my-little-soda.toml".to_string(),
                            ),
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
                        issues.push(
                            "GitHub owner has placeholder value 'your-github-username'".to_string(),
                        );
                    } else {
                        warnings.push("GitHub owner uses default value 'johnhkchen'".to_string());
                    }
                }

                if cfg.github.repo == "your-repo-name" || cfg.github.repo == "my-little-soda" {
                    if cfg.github.repo == "your-repo-name" {
                        issues
                            .push("GitHub repo has placeholder value 'your-repo-name'".to_string());
                    } else {
                        warnings
                            .push("GitHub repo uses default value 'my-little-soda'".to_string());
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
                    Some(
                        "Update my-little-soda.toml with your actual repository information"
                            .to_string(),
                    )
                } else if has_warnings {
                    Some("Consider updating configuration values if they don't match your repository".to_string())
                } else {
                    None
                };

                checks.insert(
                    "config_completeness".to_string(),
                    DiagnosticResult {
                        status,
                        message,
                        details,
                        suggestion,
                    },
                );
            }
            Err(e) => {
                checks.insert(
                    "config_completeness".to_string(),
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "Cannot load configuration for validation".to_string(),
                        details: Some(format!("Configuration load error: {}", e)),
                        suggestion: Some(
                            "Fix configuration file syntax or create my-little-soda.toml"
                                .to_string(),
                        ),
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
                    validation_issues
                        .push("Rate limit requests per hour must be positive".to_string());
                }
                if cfg.github.rate_limit.burst_capacity == 0 {
                    validation_issues
                        .push("Rate limit burst capacity must be positive".to_string());
                }
                if cfg.agents.coordination_timeout_seconds == 0 {
                    validation_issues
                        .push("Agent coordination timeout must be positive".to_string());
                }
                if cfg.agents.bundle_processing.max_queue_size == 0 {
                    validation_issues.push("Bundle queue size must be positive".to_string());
                }
                if cfg.agents.bundle_processing.processing_timeout_seconds == 0 {
                    validation_issues
                        .push("Bundle processing timeout must be positive".to_string());
                }
                if cfg.agents.process_management.timeout_minutes == 0 {
                    validation_issues.push("Process timeout must be positive".to_string());
                }

                // Validate log level
                let valid_log_levels = ["trace", "debug", "info", "warn", "error"];
                if !valid_log_levels.contains(&cfg.observability.log_level.as_str()) {
                    validation_issues.push(format!(
                        "Invalid log level '{}', must be one of: {}",
                        cfg.observability.log_level,
                        valid_log_levels.join(", ")
                    ));
                }

                // Validate CI mode artifact handling
                let valid_artifact_strategies = ["standard", "optimized", "enhanced"];
                if !valid_artifact_strategies
                    .contains(&cfg.agents.ci_mode.artifact_handling.as_str())
                {
                    validation_issues.push(format!(
                        "Invalid artifact handling strategy '{}', must be one of: {}",
                        cfg.agents.ci_mode.artifact_handling,
                        valid_artifact_strategies.join(", ")
                    ));
                }

                // Validate paths
                if cfg.agents.process_management.claude_code_path.is_empty() {
                    validation_issues.push("Claude code path cannot be empty".to_string());
                }
                if cfg.agents.work_continuity.state_file_path.is_empty() {
                    validation_issues
                        .push("Work continuity state file path cannot be empty".to_string());
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
                        validation_issues
                            .push("Database max connections must be positive".to_string());
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
                    format!(
                        "Configuration has {} validation error(s)",
                        validation_issues.len()
                    )
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
                    Some(
                        "Fix validation errors in my-little-soda.toml using valid values"
                            .to_string(),
                    )
                } else {
                    None
                };

                checks.insert(
                    "config_field_validation".to_string(),
                    DiagnosticResult {
                        status,
                        message,
                        details,
                        suggestion,
                    },
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
                        inconsistencies.push(format!(
                            "GITHUB_OWNER env var ('{}') differs from config ('{}')",
                            env_owner, cfg.github.owner
                        ));
                    }
                }

                if let Ok(env_repo) = env::var("GITHUB_REPO") {
                    if env_repo != cfg.github.repo {
                        inconsistencies.push(format!(
                            "GITHUB_REPO env var ('{}') differs from config ('{}')",
                            env_repo, cfg.github.repo
                        ));
                    }
                }

                if let Ok(env_log_level) = env::var("MY_LITTLE_SODA_OBSERVABILITY_LOG_LEVEL") {
                    if env_log_level != cfg.observability.log_level {
                        inconsistencies.push(format!("MY_LITTLE_SODA_OBSERVABILITY_LOG_LEVEL env var ('{}') differs from config ('{}')", 
                                                    env_log_level, cfg.observability.log_level));
                    }
                }

                // Check for token configuration
                let has_env_token = env::var("MY_LITTLE_SODA_GITHUB_TOKEN").is_ok()
                    || env::var("GITHUB_TOKEN").is_ok();
                let has_config_token = cfg.github.token.is_some();

                if !has_env_token && !has_config_token {
                    inconsistencies.push(
                        "No GitHub token found in environment variables or configuration"
                            .to_string(),
                    );
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
                    format!(
                        "Configuration has {} consistency issue(s)",
                        inconsistencies.len()
                    )
                };

                let details = if inconsistencies.is_empty() {
                    if self.is_verbose() {
                        Some(
                            "Configuration values match environment variable overrides".to_string(),
                        )
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
                    DiagnosticResult {
                        status,
                        message,
                        details,
                        suggestion,
                    },
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
}