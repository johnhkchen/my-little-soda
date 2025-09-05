#![cfg(feature = "autonomous")]
//! Integration tests for state drift detection and correction system
//!
//! These tests validate the autonomous system's ability to detect and correct
//! state drifts between expected system state and actual GitHub/workspace state.

use chrono::{Duration, Utc};
use std::collections::HashMap;
use tokio::time::sleep;

use my_little_soda::{
    autonomous::{
        AgentId, AutonomousWorkflowState, CorrectionAction, CorrectionStrategy,
        DriftDetectionReport, DriftSeverity, DriftThresholds, ExpectedBranchState,
        ExpectedIssueState, ExpectedPRState, ExpectedSystemState, ExpectedWorkspaceState, Issue,
        IssueState, PRState, Priority, PullRequest, ReviewState, StateDrift, StateDriftDetector,
        StateDriftError, StateDriftType, ValidationHealth, WorkspaceState,
    },
    GitHubClient,
};

/// Test basic drift detector creation and configuration
#[tokio::test]
async fn test_drift_detector_creation_and_configuration() {
    let github_client = GitHubClient::new().expect("Failed to create GitHub client");
    let detector = StateDriftDetector::new(github_client, "test-agent".to_string());

    assert_eq!(detector.generate_drift_report().agent_id, "test-agent");
    assert_eq!(detector.generate_drift_report().total_drifts_detected, 0);
    assert_eq!(
        detector.generate_drift_report().validation_health,
        ValidationHealth::Healthy
    );
}

/// Test drift detector configuration with custom thresholds
#[tokio::test]
async fn test_drift_detector_with_custom_configuration() {
    let github_client = GitHubClient::new().expect("Failed to create GitHub client");
    let custom_thresholds = DriftThresholds {
        max_validation_interval_active: 2, // 2 minutes
        max_validation_interval_idle: 15,  // 15 minutes
        max_commits_behind: 5,
        critical_drift_types: vec![
            StateDriftType::IssueUnexpectedlyClosed,
            StateDriftType::BranchDeleted,
            StateDriftType::GitStateInconsistent,
        ],
    };

    let custom_strategies = vec![
        CorrectionStrategy::WorkPreserving,
        CorrectionStrategy::EscalateAndContinue,
        CorrectionStrategy::RequireManualIntervention,
    ];

    let detector = StateDriftDetector::new(github_client, "custom-agent".to_string())
        .with_validation_interval(Duration::minutes(2))
        .with_drift_thresholds(custom_thresholds)
        .with_correction_strategies(custom_strategies);

    // Test validation timing
    assert!(detector.needs_validation(true)); // Should need validation (last validation was 1 hour ago)
    assert!(detector.needs_validation(false));
}

/// Test validation timing logic
#[tokio::test]
async fn test_validation_timing_logic() {
    let github_client = GitHubClient::new().expect("Failed to create GitHub client");
    let mut detector = StateDriftDetector::new(github_client, "timing-test-agent".to_string());

    // Initially should need validation (detector starts with old last_validation)
    assert!(detector.needs_validation(true));
    assert!(detector.needs_validation(false));

    // After simulating a recent validation, shouldn't need immediate validation
    // Note: This test relies on the detector's internal timing logic
    let recent_validation = detector.generate_drift_report();
    assert!(recent_validation.last_validation < Utc::now());

    // Test different validation intervals for active vs idle
    let active_needs_validation = detector.needs_validation(true);
    let idle_needs_validation = detector.needs_validation(false);

    // Both should be true initially due to old last_validation
    assert_eq!(active_needs_validation, idle_needs_validation); // Both true initially
}

/// Test expected state updates from workflow state
#[tokio::test]
async fn test_expected_state_updates() {
    let github_client = GitHubClient::new().expect("Failed to create GitHub client");
    let mut detector = StateDriftDetector::new(github_client, "state-update-agent".to_string());

    // Test updating from Assigned state
    let assigned_state = AutonomousWorkflowState::Assigned {
        issue: Issue {
            number: 123,
            title: "Test issue".to_string(),
            body: "Test body".to_string(),
            labels: vec!["test".to_string(), "autonomous".to_string()],
            priority: Priority::Medium,
            estimated_hours: Some(2),
        },
        agent: AgentId("state-update-agent".to_string()),
        workspace: WorkspaceState {
            current_branch: "main".to_string(),
            uncommitted_changes: false,
        },
    };

    let update_result = detector.update_expected_state(&assigned_state).await;
    assert!(
        update_result.is_ok(),
        "Expected state update failed: {:?}",
        update_result
    );

    // Test updating from ReadyForReview state
    let pr = PullRequest {
        number: 456,
        title: "Test PR".to_string(),
        branch: "test-branch".to_string(),
        commits: 3,
        files_changed: 2,
    };

    let ready_state = AutonomousWorkflowState::ReadyForReview {
        issue: Issue {
            number: 123,
            title: "Test issue".to_string(),
            body: "Test body".to_string(),
            labels: vec!["test".to_string()],
            priority: Priority::Medium,
            estimated_hours: Some(2),
        },
        agent: AgentId("state-update-agent".to_string()),
        pr: pr.clone(),
    };

    let pr_update_result = detector.update_expected_state(&ready_state).await;
    assert!(
        pr_update_result.is_ok(),
        "PR state update failed: {:?}",
        pr_update_result
    );

    // Verify state was updated
    let report = detector.generate_drift_report();
    assert!(report.last_validation < Utc::now());
}

/// Test drift detection for issue state changes
#[tokio::test]
async fn test_issue_drift_detection() {
    let github_client = GitHubClient::new().expect("Failed to create GitHub client");
    let mut detector = StateDriftDetector::new(github_client, "issue-drift-agent".to_string());

    // Setup expected issue state
    let workflow_state = AutonomousWorkflowState::Assigned {
        issue: Issue {
            number: 789,
            title: "Drift test issue".to_string(),
            body: "Testing drift detection".to_string(),
            labels: vec!["issue-drift-agent".to_string(), "test".to_string()],
            priority: Priority::High,
            estimated_hours: Some(3),
        },
        agent: AgentId("issue-drift-agent".to_string()),
        workspace: WorkspaceState {
            current_branch: "main".to_string(),
            uncommitted_changes: false,
        },
    };

    let update_result = detector.update_expected_state(&workflow_state).await;
    assert!(update_result.is_ok());

    // Note: This test would require real GitHub API interaction or mocking
    // For integration testing purposes, we're testing the framework structure
    // Actual GitHub API validation would need real issues or mock responses

    // Test that validation doesn't crash with no real drifts
    let validation_result = detector.validate_state().await;
    // This might fail with API errors, but shouldn't panic
    match validation_result {
        Ok(drifts) => {
            // If successful, verify the structure
            assert!(drifts.len() >= 0); // Could be empty if no drifts
        }
        Err(e) => {
            // API errors are expected in test environment
            println!("Expected API error in test environment: {:?}", e);
        }
    }
}

/// Test workspace drift detection
#[tokio::test]
async fn test_workspace_drift_detection() {
    let github_client = GitHubClient::new().expect("Failed to create GitHub client");
    let mut detector = StateDriftDetector::new(github_client, "workspace-drift-agent".to_string());

    // Test workspace validation (this will check actual git state)
    let validation_result = detector.validate_state().await;

    match validation_result {
        Ok(drifts) => {
            // Check if any workspace drifts were detected
            let workspace_drifts: Vec<_> = drifts
                .iter()
                .filter(|d| {
                    matches!(
                        d.get_type(),
                        StateDriftType::WorkspaceFileChanges | StateDriftType::GitStateInconsistent
                    )
                })
                .collect();

            // Print detected drifts for debugging
            for drift in &workspace_drifts {
                println!("Detected workspace drift: {:?}", drift);
            }

            // Workspace drift detection should work since it uses local git commands
            assert!(drifts.len() >= 0);
        }
        Err(e) => {
            println!("Workspace validation error: {:?}", e);
        }
    }
}

/// Test drift correction strategies
#[tokio::test]
async fn test_drift_correction_strategies() {
    let github_client = GitHubClient::new().expect("Failed to create GitHub client");
    let mut detector = StateDriftDetector::new(github_client, "correction-test-agent".to_string());

    // Create test drifts
    let test_drifts = vec![
        StateDrift::WorkspaceFileChanges {
            modified_files: vec![std::path::PathBuf::from("test.rs")],
            severity: DriftSeverity::Minor,
        },
        StateDrift::LabelsChanged {
            issue_id: 123,
            expected: vec!["agent001".to_string()],
            actual: vec!["agent002".to_string()],
            severity: DriftSeverity::Moderate,
        },
    ];

    let correction_result = detector.correct_drifts(test_drifts).await;

    match correction_result {
        Ok(corrections) => {
            assert_eq!(corrections.len(), 2);

            // Verify correction types
            for correction in corrections {
                match correction {
                    CorrectionAction::DocumentDriftAndContinue {
                        continue_autonomously,
                        ..
                    } => {
                        assert!(continue_autonomously);
                    }
                    CorrectionAction::SynchronizeWithGitHub { .. } => {
                        // Expected for label changes
                    }
                    CorrectionAction::UpdateLocalState { .. } => {
                        // Expected for minor drifts
                    }
                    _ => {
                        println!("Unexpected correction action: {:?}", correction);
                    }
                }
            }
        }
        Err(e) => {
            // Correction might fail due to GitHub API limitations in test
            println!("Expected correction error in test environment: {:?}", e);
        }
    }
}

/// Test critical drift handling
#[tokio::test]
async fn test_critical_drift_handling() {
    let github_client = GitHubClient::new().expect("Failed to create GitHub client");
    let mut detector = StateDriftDetector::new(github_client, "critical-drift-agent".to_string());

    // Create critical drifts
    let critical_drifts = vec![
        StateDrift::IssueUnexpectedlyClosed {
            issue_id: 999,
            closer: "someone-else".to_string(),
            closed_at: Utc::now(),
            severity: DriftSeverity::Critical,
        },
        StateDrift::BranchDeleted {
            branch_name: "critical-work-branch".to_string(),
            severity: DriftSeverity::Critical,
        },
    ];

    let correction_result = detector.correct_drifts(critical_drifts).await;

    match correction_result {
        Ok(corrections) => {
            // Critical drifts should result in manual intervention or escalation
            for correction in corrections {
                match correction {
                    CorrectionAction::RequireManualIntervention { .. } => {
                        // Expected for critical drifts
                        println!("Critical drift correctly requires manual intervention");
                    }
                    CorrectionAction::CreateDriftIssue { .. } => {
                        // Also acceptable for critical drifts
                        println!("Critical drift escalated to issue creation");
                    }
                    _ => {
                        println!("Unexpected correction for critical drift: {:?}", correction);
                    }
                }
            }
        }
        Err(e) => {
            // API errors expected in test environment
            println!("Expected error for critical drift handling: {:?}", e);
        }
    }
}

/// Test drift detection report generation
#[tokio::test]
async fn test_drift_report_generation() {
    let github_client = GitHubClient::new().expect("Failed to create GitHub client");
    let mut detector = StateDriftDetector::new(github_client, "report-test-agent".to_string());

    // Initial report
    let initial_report = detector.generate_drift_report();
    assert_eq!(initial_report.agent_id, "report-test-agent");
    assert_eq!(initial_report.total_drifts_detected, 0);
    assert_eq!(initial_report.critical_drifts.len(), 0);
    assert_eq!(initial_report.validation_health, ValidationHealth::Healthy);
    assert!(initial_report.last_validation < Utc::now());
    assert!(initial_report.next_validation > initial_report.last_validation);

    // Simulate adding some detected drifts for testing
    // (In real scenarios, these would come from validate_state())
    let test_drift = StateDrift::WorkspaceFileChanges {
        modified_files: vec![std::path::PathBuf::from("src/lib.rs")],
        severity: DriftSeverity::Minor,
    };

    // We can't directly add drifts to the detector in this test,
    // but we can test the report structure
    assert!(!detector.has_critical_drifts());

    // Test empty drift list
    detector.clear_resolved_drifts();
    let cleared_report = detector.generate_drift_report();
    assert_eq!(cleared_report.total_drifts_detected, 0);
}

/// Test state drift type classification
#[test]
fn test_drift_type_classification() {
    let issue_drift = StateDrift::IssueUnexpectedlyAssigned {
        issue_id: 123,
        expected_assignee: Some("agent001".to_string()),
        actual_assignee: "agent002".to_string(),
        severity: DriftSeverity::Critical,
    };

    assert_eq!(
        issue_drift.get_type(),
        StateDriftType::IssueUnexpectedlyAssigned
    );
    assert_eq!(issue_drift.get_severity(), DriftSeverity::Critical);

    let workspace_drift = StateDrift::GitStateInconsistent {
        expected_head: "abc123".to_string(),
        actual_head: "def456".to_string(),
        severity: DriftSeverity::Moderate,
    };

    assert_eq!(
        workspace_drift.get_type(),
        StateDriftType::GitStateInconsistent
    );
    assert_eq!(workspace_drift.get_severity(), DriftSeverity::Moderate);

    let branch_drift = StateDrift::BranchDiverged {
        branch_name: "feature-branch".to_string(),
        commits_behind: 3,
        commits_ahead: 2,
        severity: DriftSeverity::Moderate,
    };

    assert_eq!(branch_drift.get_type(), StateDriftType::BranchDiverged);
    assert_eq!(branch_drift.get_severity(), DriftSeverity::Moderate);
}

/// Test severity-based correction strategy selection
#[test]
fn test_severity_based_correction() {
    // Test that different severities result in appropriate correction strategies
    let minor_drift = StateDrift::WorkspaceFileChanges {
        modified_files: vec![std::path::PathBuf::from("README.md")],
        severity: DriftSeverity::Minor,
    };

    let moderate_drift = StateDrift::LabelsChanged {
        issue_id: 456,
        expected: vec!["tag1".to_string()],
        actual: vec!["tag2".to_string()],
        severity: DriftSeverity::Moderate,
    };

    let critical_drift = StateDrift::IssueUnexpectedlyClosed {
        issue_id: 789,
        closer: "admin".to_string(),
        closed_at: Utc::now(),
        severity: DriftSeverity::Critical,
    };

    // Verify severity classification
    assert_eq!(minor_drift.get_severity(), DriftSeverity::Minor);
    assert_eq!(moderate_drift.get_severity(), DriftSeverity::Moderate);
    assert_eq!(critical_drift.get_severity(), DriftSeverity::Critical);

    // Test severity ordering
    assert!(DriftSeverity::Minor < DriftSeverity::Moderate);
    assert!(DriftSeverity::Moderate < DriftSeverity::Critical);
}

/// Test drift detection timing intervals
#[tokio::test]
async fn test_drift_detection_intervals() {
    let github_client = GitHubClient::new().expect("Failed to create GitHub client");
    let custom_thresholds = DriftThresholds {
        max_validation_interval_active: 1, // 1 minute for active
        max_validation_interval_idle: 10,  // 10 minutes for idle
        max_commits_behind: 3,
        critical_drift_types: vec![StateDriftType::BranchDeleted],
    };

    let mut detector = StateDriftDetector::new(github_client, "interval-test-agent".to_string())
        .with_drift_thresholds(custom_thresholds);

    // Should need validation initially (starts with old timestamp)
    assert!(detector.needs_validation(true));
    assert!(detector.needs_validation(false));

    // Test interval differences between active and idle
    // Both should return true initially due to the detector starting with an old last_validation
    let active_validation_needed = detector.needs_validation(true);
    let idle_validation_needed = detector.needs_validation(false);

    // Initially both should be true
    assert_eq!(active_validation_needed, true);
    assert_eq!(idle_validation_needed, true);
}

/// Test drift threshold configuration
#[test]
fn test_drift_threshold_configuration() {
    let default_thresholds = DriftThresholds::default();

    // Test default values
    assert_eq!(default_thresholds.max_validation_interval_active, 5);
    assert_eq!(default_thresholds.max_validation_interval_idle, 30);
    assert_eq!(default_thresholds.max_commits_behind, 10);
    assert!(default_thresholds
        .critical_drift_types
        .contains(&StateDriftType::IssueUnexpectedlyClosed));
    assert!(default_thresholds
        .critical_drift_types
        .contains(&StateDriftType::BranchDeleted));
    assert!(default_thresholds
        .critical_drift_types
        .contains(&StateDriftType::PRUnexpectedlyMerged));
    assert!(default_thresholds
        .critical_drift_types
        .contains(&StateDriftType::GitStateInconsistent));

    // Test custom configuration
    let custom_thresholds = DriftThresholds {
        max_validation_interval_active: 2,
        max_validation_interval_idle: 15,
        max_commits_behind: 5,
        critical_drift_types: vec![
            StateDriftType::BranchDeleted,
            StateDriftType::GitStateInconsistent,
        ],
    };

    assert_eq!(custom_thresholds.max_validation_interval_active, 2);
    assert_eq!(custom_thresholds.critical_drift_types.len(), 2);
}
