/// Simple implementation of GitHub label validation diagnostics
/// This module provides basic label checking functionality for the doctor command

use std::collections::HashMap;
use crate::cli::commands::doctor::{DiagnosticResult, DiagnosticStatus};

/// Label specification structure 
#[derive(Debug)]
pub struct LabelSpec {
    pub name: String,
    pub color: String,
    pub description: String,
}

/// Get required labels specification
pub fn get_required_labels() -> Vec<LabelSpec> {
    vec![
        // Core routing labels
        LabelSpec {
            name: "route:ready".to_string(),
            color: "0052cc".to_string(),
            description: "Available for agent assignment".to_string(),
        },
        LabelSpec {
            name: "route:ready_to_merge".to_string(),
            color: "5319e7".to_string(),
            description: "Completed work ready for merge".to_string(),
        },
        LabelSpec {
            name: "route:unblocker".to_string(),
            color: "d73a4a".to_string(),
            description: "Critical system issues blocking other work".to_string(),
        },
        LabelSpec {
            name: "route:review".to_string(),
            color: "fbca04".to_string(),
            description: "Under review".to_string(),
        },
        LabelSpec {
            name: "route:human-only".to_string(),
            color: "7057ff".to_string(),
            description: "Requires human attention".to_string(),
        },
        // Priority labels
        LabelSpec {
            name: "route:priority-low".to_string(),
            color: "c5def5".to_string(),
            description: "Low priority task (Priority: 1)".to_string(),
        },
        LabelSpec {
            name: "route:priority-medium".to_string(),
            color: "1d76db".to_string(),
            description: "Medium priority task (Priority: 2)".to_string(),
        },
        LabelSpec {
            name: "route:priority-high".to_string(),
            color: "b60205".to_string(),
            description: "High priority task (Priority: 3)".to_string(),
        },
        LabelSpec {
            name: "route:priority-very-high".to_string(),
            color: "ee0701".to_string(),
            description: "Very high priority task (Priority: 4)".to_string(),
        },
        // Additional operational labels
        LabelSpec {
            name: "code-review-feedback".to_string(),
            color: "e99695".to_string(),
            description: "Issues created from code review feedback".to_string(),
        },
        LabelSpec {
            name: "supertask-decomposition".to_string(),
            color: "bfdadc".to_string(),
            description: "Task broken down from larger work".to_string(),
        },
        LabelSpec {
            name: "code-quality".to_string(),
            color: "d4c5f9".to_string(),
            description: "Code quality improvements, refactoring, and technical debt reduction".to_string(),
        },
        // Agent assignment labels
        LabelSpec {
            name: "agent001".to_string(),
            color: "0e8a16".to_string(),
            description: "Assigned to agent001".to_string(),
        },
    ]
}

/// Check for existence of required routing labels (basic implementation)
pub async fn check_required_labels_existence(verbose: bool) -> DiagnosticResult {
    match crate::github::client::GitHubClient::new() {
        Ok(client) => {
            let required_labels = get_required_labels();
            let octocrab = client.issues.octocrab();
            
            match octocrab.issues(client.owner(), client.repo()).list_labels_for_repo().send().await {
                Ok(labels_page) => {
                    let mut missing_labels = Vec::new();
                    let mut existing_labels = Vec::new();
                    
                    for required_label in &required_labels {
                        if labels_page.items.iter().any(|label| label.name == required_label.name) {
                            existing_labels.push(&required_label.name);
                        } else {
                            missing_labels.push(&required_label.name);
                        }
                    }
                    
                    if missing_labels.is_empty() {
                        DiagnosticResult {
                            status: DiagnosticStatus::Pass,
                            message: format!("All {} required labels exist", required_labels.len()),
                            details: if verbose {
                                Some(format!("Existing labels: {}", existing_labels.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")))
                            } else {
                                None
                            },
                            suggestion: None,
                        }
                    } else {
                        DiagnosticResult {
                            status: DiagnosticStatus::Fail,
                            message: format!("{} required labels are missing", missing_labels.len()),
                            details: Some(format!("Missing labels: {}", missing_labels.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "))),
                            suggestion: Some("Run 'my-little-soda init' to create missing labels automatically".to_string()),
                        }
                    }
                }
                Err(_) => {
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "Cannot access repository labels".to_string(),
                        details: Some("Failed to list repository labels".to_string()),
                        suggestion: Some("Configure GitHub authentication to check labels".to_string()),
                    }
                }
            }
        }
        Err(e) => {
            DiagnosticResult {
                status: DiagnosticStatus::Fail,
                message: "Cannot check required labels".to_string(),
                details: Some(format!("GitHub client error: {:?}", e)),
                suggestion: Some("Configure GitHub authentication to check labels".to_string()),
            }
        }
    }
}

/// Validate existing label configuration matches requirements (basic implementation)
pub async fn check_label_configuration(verbose: bool) -> DiagnosticResult {
    match crate::github::client::GitHubClient::new() {
        Ok(client) => {
            let required_labels = get_required_labels();
            let octocrab = client.issues.octocrab();
            
            match octocrab.issues(client.owner(), client.repo()).list_labels_for_repo().send().await {
                Ok(labels_page) => {
                    let mut configuration_issues = Vec::new();
                    let mut valid_labels = Vec::new();
                    
                    for required_label in &required_labels {
                        if let Some(existing_label) = labels_page.items.iter().find(|label| label.name == required_label.name) {
                            let mut issues = Vec::new();
                            
                            if existing_label.color != required_label.color {
                                issues.push(format!("color mismatch (expected: #{}, actual: #{})", required_label.color, existing_label.color));
                            }
                            
                            if existing_label.description.as_deref() != Some(&required_label.description) {
                                issues.push(format!("description mismatch"));
                            }
                            
                            if issues.is_empty() {
                                valid_labels.push(&required_label.name);
                            } else {
                                configuration_issues.push(format!("{}: {}", required_label.name, issues.join(", ")));
                            }
                        }
                    }
                    
                    if configuration_issues.is_empty() && !valid_labels.is_empty() {
                        DiagnosticResult {
                            status: DiagnosticStatus::Pass,
                            message: "All existing labels have correct configuration".to_string(),
                            details: if verbose {
                                Some(format!("Valid labels: {}", valid_labels.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")))
                            } else {
                                None
                            },
                            suggestion: None,
                        }
                    } else if !configuration_issues.is_empty() {
                        DiagnosticResult {
                            status: DiagnosticStatus::Warning,
                            message: format!("{} labels have configuration issues", configuration_issues.len()),
                            details: Some(format!("Configuration issues: {}", configuration_issues.join("; "))),
                            suggestion: Some("Update label colors and descriptions to match My Little Soda requirements".to_string()),
                        }
                    } else {
                        DiagnosticResult {
                            status: DiagnosticStatus::Pass,
                            message: "No labels found to validate configuration".to_string(),
                            details: None,
                            suggestion: None,
                        }
                    }
                }
                Err(_) => {
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "Cannot validate label configuration".to_string(),
                        details: Some("Failed to access repository labels".to_string()),
                        suggestion: Some("Configure GitHub authentication to validate labels".to_string()),
                    }
                }
            }
        }
        Err(e) => {
            DiagnosticResult {
                status: DiagnosticStatus::Fail,
                message: "Cannot validate label configuration".to_string(),
                details: Some(format!("GitHub client error: {:?}", e)),
                suggestion: Some("Configure GitHub authentication to validate labels".to_string()),
            }
        }
    }
}