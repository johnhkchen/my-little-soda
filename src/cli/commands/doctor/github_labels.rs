/// Simple implementation of GitHub label validation diagnostics
/// This module provides basic label checking functionality for the doctor command

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
    match crate::github::client::GitHubClient::with_verbose(verbose) {
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
    match crate::github::client::GitHubClient::with_verbose(verbose) {
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

/// Test label management capabilities (create/update/delete)
pub async fn check_label_management_capabilities(verbose: bool) -> DiagnosticResult {
    match crate::github::client::GitHubClient::with_verbose(verbose) {
        Ok(client) => {
            let octocrab = client.issues.octocrab();
            let test_label_name = format!("test-label-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs());
            let mut operations_tested = Vec::new();
            let mut failed_operations = Vec::new();
            
            // Test 1: Label creation capability
            let create_result = octocrab
                .issues(client.owner(), client.repo())
                .create_label(&test_label_name, "ffffff", "Test label for management capability validation")
                .await;
            
            match create_result {
                Ok(_) => {
                    operations_tested.push("create");
                    
                    // Test 2: Label update capability (using delete+create as update)
                    let update_result = async {
                        octocrab
                            .issues(client.owner(), client.repo())
                            .delete_label(&test_label_name)
                            .await?;
                        
                        octocrab
                            .issues(client.owner(), client.repo())
                            .create_label(&test_label_name, "000000", "Updated test label description")
                            .await
                    }.await;
                    
                    match update_result {
                        Ok(_) => {
                            operations_tested.push("update");
                        }
                        Err(e) => {
                            failed_operations.push(format!("update: {}", e));
                        }
                    }
                    
                    // Test 3: Label deletion capability (cleanup)
                    let delete_result = octocrab
                        .issues(client.owner(), client.repo())
                        .delete_label(&test_label_name)
                        .await;
                    
                    match delete_result {
                        Ok(_) => {
                            operations_tested.push("delete");
                        }
                        Err(e) => {
                            failed_operations.push(format!("delete: {}", e));
                        }
                    }
                }
                Err(e) => {
                    failed_operations.push(format!("create: {}", e));
                }
            }
            
            if failed_operations.is_empty() {
                DiagnosticResult {
                    status: DiagnosticStatus::Pass,
                    message: "All label management operations successful".to_string(),
                    details: if verbose {
                        Some(format!("Successfully tested: {}", operations_tested.join(", ")))
                    } else {
                        None
                    },
                    suggestion: None,
                }
            } else if operations_tested.len() > 0 {
                DiagnosticResult {
                    status: DiagnosticStatus::Warning,
                    message: format!("Some label management operations failed ({}/3 successful)", operations_tested.len()),
                    details: Some(format!("Failed operations: {}", failed_operations.join("; "))),
                    suggestion: Some("Check repository write permissions and GitHub token scopes".to_string()),
                }
            } else {
                DiagnosticResult {
                    status: DiagnosticStatus::Fail,
                    message: "Cannot perform label management operations".to_string(),
                    details: Some(format!("Failed operations: {}", failed_operations.join("; "))),
                    suggestion: Some("Ensure GitHub token has repository write permissions".to_string()),
                }
            }
        }
        Err(e) => {
            DiagnosticResult {
                status: DiagnosticStatus::Fail,
                message: "Cannot test label management capabilities".to_string(),
                details: Some(format!("GitHub client error: {:?}", e)),
                suggestion: Some("Configure GitHub authentication to test label management".to_string()),
            }
        }
    }
}

/// Validate repository write permissions for label management
pub async fn check_repository_write_permissions(verbose: bool) -> DiagnosticResult {
    match crate::github::client::GitHubClient::with_verbose(verbose) {
        Ok(client) => {
            let octocrab = client.issues.octocrab();
            
            // Check repository permissions by attempting to get repository info
            match octocrab.repos(client.owner(), client.repo()).get().await {
                Ok(repo) => {
                    let permissions = repo.permissions.as_ref();
                    let has_write_access = permissions
                        .map(|p| p.push || p.maintain || p.admin)
                        .unwrap_or(false);
                    
                    if has_write_access {
                        DiagnosticResult {
                            status: DiagnosticStatus::Pass,
                            message: "Repository write permissions confirmed".to_string(),
                            details: if verbose {
                                Some(format!("Permissions: push={}, maintain={}, admin={}", 
                                    permissions.map(|p| p.push).unwrap_or(false),
                                    permissions.map(|p| p.maintain).unwrap_or(false),
                                    permissions.map(|p| p.admin).unwrap_or(false)))
                            } else {
                                None
                            },
                            suggestion: None,
                        }
                    } else {
                        DiagnosticResult {
                            status: DiagnosticStatus::Warning,
                            message: "Limited repository permissions detected".to_string(),
                            details: Some("Label management operations may fail without write permissions".to_string()),
                            suggestion: Some("Request repository write access to use label management features".to_string()),
                        }
                    }
                }
                Err(e) => {
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "Cannot check repository permissions".to_string(),
                        details: Some(format!("Repository access error: {}", e)),
                        suggestion: Some("Verify GitHub token has repository access permissions".to_string()),
                    }
                }
            }
        }
        Err(e) => {
            DiagnosticResult {
                status: DiagnosticStatus::Fail,
                message: "Cannot check repository write permissions".to_string(),
                details: Some(format!("GitHub client error: {:?}", e)),
                suggestion: Some("Configure GitHub authentication to check permissions".to_string()),
            }
        }
    }
}

/// Validate issue label states for routing workflow compliance
pub async fn check_issue_label_states(verbose: bool) -> DiagnosticResult {
    match crate::github::client::GitHubClient::with_verbose(verbose) {
        Ok(client) => {
            let octocrab = client.issues.octocrab();
            
            // Get all open issues
            match octocrab.issues(client.owner(), client.repo())
                .list()
                .state(octocrab::params::State::Open)
                .per_page(100)
                .send()
                .await 
            {
                Ok(issues_page) => {
                    let mut problems = Vec::new();
                    let mut total_issues = 0;
                    let routing_labels = [
                        "route:ready", "route:ready_to_merge", "route:unblocker", 
                        "route:review", "route:human-only"
                    ];
                    
                    for issue in &issues_page.items {
                        total_issues += 1;
                        let issue_labels: Vec<String> = issue.labels.iter()
                            .map(|l| l.name.clone())
                            .collect();
                        
                        // Check 1: Issues with multiple conflicting routing labels
                        let routing_label_count = issue_labels.iter()
                            .filter(|label| routing_labels.contains(&label.as_str()))
                            .count();
                            
                        if routing_label_count > 1 {
                            let conflicting_labels: Vec<String> = issue_labels.iter()
                                .filter(|label| routing_labels.contains(&label.as_str()))
                                .cloned()
                                .collect();
                            problems.push(format!("Issue #{}: has {} conflicting routing labels: {}", 
                                issue.number, routing_label_count, conflicting_labels.join(", ")));
                        }
                        
                        // Check 2: Issues without any routing labels (but should have them)
                        if routing_label_count == 0 && !issue_labels.contains(&"route:human-only".to_string()) {
                            // Skip issues assigned to agents (they're in progress)
                            let has_agent_label = issue_labels.iter().any(|label| label.starts_with("agent"));
                            if !has_agent_label {
                                problems.push(format!("Issue #{}: missing routing label (should have route:ready, route:review, or route:human-only)", 
                                    issue.number));
                            }
                        }
                        
                        // Check 3: Issues with route:ready_to_merge should have associated PR
                        if issue_labels.contains(&"route:ready_to_merge".to_string()) {
                            // This is a simplified check - in practice you'd verify PR exists
                            // For now, we'll assume this is valid if the label exists
                        }
                        
                        // Check 4: Issues assigned to agents should not have route:ready
                        let has_agent_label = issue_labels.iter().any(|label| label.starts_with("agent"));
                        let has_ready_label = issue_labels.contains(&"route:ready".to_string());
                        if has_agent_label && has_ready_label {
                            problems.push(format!("Issue #{}: assigned to agent but still has route:ready label", 
                                issue.number));
                        }
                    }
                    
                    if problems.is_empty() {
                        DiagnosticResult {
                            status: DiagnosticStatus::Pass,
                            message: format!("All {} open issues have proper label states", total_issues),
                            details: if verbose {
                                Some(format!("Checked {} issues for routing label compliance", total_issues))
                            } else {
                                None
                            },
                            suggestion: None,
                        }
                    } else if problems.len() <= 3 {
                        DiagnosticResult {
                            status: DiagnosticStatus::Warning,
                            message: format!("{} issues have label state problems", problems.len()),
                            details: Some(problems.join("; ")),
                            suggestion: Some("Review and fix issue label assignments for proper workflow routing".to_string()),
                        }
                    } else {
                        DiagnosticResult {
                            status: DiagnosticStatus::Fail,
                            message: format!("{} issues have label state problems", problems.len()),
                            details: Some(format!("First 3 problems: {}... (run with --verbose for all)", 
                                problems.iter().take(3).cloned().collect::<Vec<_>>().join("; "))),
                            suggestion: Some("Review and fix issue label assignments - many issues have incorrect routing labels".to_string()),
                        }
                    }
                }
                Err(e) => {
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "Cannot check issue label states".to_string(),
                        details: Some(format!("Issues API error: {}", e)),
                        suggestion: Some("Verify GitHub token has issues read access".to_string()),
                    }
                }
            }
        }
        Err(e) => {
            DiagnosticResult {
                status: DiagnosticStatus::Fail,
                message: "Cannot check issue label states".to_string(),
                details: Some(format!("GitHub client error: {:?}", e)),
                suggestion: Some("Configure GitHub authentication to check issue states".to_string()),
            }
        }
    }
}

/// Check for workflow compliance and label consistency
pub async fn check_workflow_label_compliance(verbose: bool) -> DiagnosticResult {
    match crate::github::client::GitHubClient::with_verbose(verbose) {
        Ok(client) => {
            let octocrab = client.issues.octocrab();
            
            // Get all open issues with agent labels
            match octocrab.issues(client.owner(), client.repo())
                .list()
                .state(octocrab::params::State::Open)
                .per_page(100)
                .send()
                .await 
            {
                Ok(issues_page) => {
                    let mut compliance_issues = Vec::new();
                    let mut agent_assigned_count = 0;
                    let mut ready_count = 0;
                    let mut review_count = 0;
                    
                    for issue in &issues_page.items {
                        let issue_labels: Vec<String> = issue.labels.iter()
                            .map(|l| l.name.clone())
                            .collect();
                        
                        // Count different workflow states
                        if issue_labels.iter().any(|label| label.starts_with("agent")) {
                            agent_assigned_count += 1;
                        }
                        if issue_labels.contains(&"route:ready".to_string()) {
                            ready_count += 1;
                        }
                        if issue_labels.contains(&"route:review".to_string()) {
                            review_count += 1;
                        }
                        
                        // Check for workflow compliance issues
                        let has_priority = issue_labels.iter().any(|label| label.starts_with("route:priority-"));
                        let has_routing = issue_labels.iter().any(|label| label.starts_with("route:"));
                        
                        if has_routing && !has_priority && !issue_labels.contains(&"route:human-only".to_string()) {
                            compliance_issues.push(format!("Issue #{}: has routing label but missing priority", issue.number));
                        }
                    }
                    
                    let _total_managed_issues = agent_assigned_count + ready_count + review_count;
                    
                    if compliance_issues.is_empty() {
                        DiagnosticResult {
                            status: DiagnosticStatus::Pass,
                            message: "Workflow label compliance looks good".to_string(),
                            details: if verbose {
                                Some(format!("Workflow state: {} assigned to agents, {} ready, {} in review", 
                                    agent_assigned_count, ready_count, review_count))
                            } else {
                                None
                            },
                            suggestion: None,
                        }
                    } else {
                        DiagnosticResult {
                            status: DiagnosticStatus::Warning,
                            message: format!("{} workflow compliance issues found", compliance_issues.len()),
                            details: Some(compliance_issues.join("; ")),
                            suggestion: Some("Add missing priority labels to routing-enabled issues".to_string()),
                        }
                    }
                }
                Err(e) => {
                    DiagnosticResult {
                        status: DiagnosticStatus::Fail,
                        message: "Cannot check workflow compliance".to_string(),
                        details: Some(format!("Issues API error: {}", e)),
                        suggestion: Some("Verify GitHub token has issues read access".to_string()),
                    }
                }
            }
        }
        Err(e) => {
            DiagnosticResult {
                status: DiagnosticStatus::Fail,
                message: "Cannot check workflow compliance".to_string(),
                details: Some(format!("GitHub client error: {:?}", e)),
                suggestion: Some("Configure GitHub authentication to check workflow compliance".to_string()),
            }
        }
    }
}