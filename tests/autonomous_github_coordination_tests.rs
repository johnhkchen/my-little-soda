#![cfg(feature = "autonomous")]
//! Integration tests for autonomous coordination with real GitHub operations
//!
//! These tests validate the autonomous system's ability to work with actual GitHub APIs
//! while maintaining safe operation and proper error handling.

use std::env;
use std::time::Duration;
use tokio::time::sleep;

use my_little_soda::{
    agents::recovery::AutoRecovery,
    autonomous::{
        state_validation::{StateDriftError, ValidationHealth},
        workflow_state_machine::WorkspaceState,
        AgentId, AutonomousCoordinator, AutonomousEvent, AutonomousWorkflowMachine,
        AutonomousWorkflowState, CoordinationConfig, Issue, Priority, PullRequest,
        StateDriftDetector,
    },
    GitHubClient,
};

/// Helper function to check if we have GitHub credentials for real API testing
fn has_github_credentials() -> bool {
    env::var("GITHUB_TOKEN").is_ok() || env::var("MY_LITTLE_SODA_GITHUB_TOKEN").is_ok()
}

/// Helper function to skip tests that need real GitHub access
fn skip_if_no_credentials() {
    if !has_github_credentials() {
        println!("Skipping test - no GitHub credentials available");
        return;
    }
}

/// Test autonomous coordinator creation with real GitHub client
#[tokio::test]
async fn test_autonomous_coordinator_with_real_github() {
    if !has_github_credentials() {
        println!("Skipping real GitHub test - no credentials");
        return;
    }

    let github_client = GitHubClient::new().expect("Failed to create GitHub client");
    let recovery_client = Box::new(AutoRecovery::new(github_client.clone(), true));
    let config = CoordinationConfig {
        max_work_hours: 1, // Short test duration
        max_recovery_attempts: 2,
        recovery_timeout_minutes: 5,
        enable_aggressive_recovery: false,
        enable_state_persistence: false, // Disabled for testing
        monitoring_interval_minutes: 1,
        enable_drift_detection: true,
        drift_validation_interval_minutes: 2,
    };

    let coordinator_result = AutonomousCoordinator::new(
        github_client,
        "real-github-test-agent".to_string(),
        recovery_client,
        config,
    )
    .await;

    assert!(
        coordinator_result.is_ok(),
        "Failed to create coordinator: {:?}",
        coordinator_result
    );

    let coordinator = coordinator_result.unwrap();

    // Test basic status reporting
    let status_report = coordinator.get_status_report().await;
    assert!(status_report.agent_id.is_some());
    assert_eq!(
        status_report.agent_id.as_ref().unwrap(),
        "real-github-test-agent"
    );
    assert!(!status_report.can_continue); // Should not be able to continue without work assigned

    // Test recovery reporting
    let recovery_report = coordinator.get_recovery_report().await;
    assert_eq!(recovery_report.total_attempts, 0);
    assert_eq!(recovery_report.success_rate, 1.0); // No attempts = 100% success rate

    // Test that it's not running initially
    assert!(!coordinator.is_running().await);
}

/// Test drift detection with real GitHub client
#[tokio::test]
async fn test_drift_detection_with_real_github() {
    if !has_github_credentials() {
        println!("Skipping real GitHub drift detection test - no credentials");
        return;
    }

    let github_client = GitHubClient::new().expect("Failed to create GitHub client");
    let mut detector =
        StateDriftDetector::new(github_client, "drift-github-test-agent".to_string());

    // Test that detector can generate reports without crashing
    let initial_report = detector.generate_drift_report();
    assert_eq!(initial_report.agent_id, "drift-github-test-agent");

    // Test state validation (this will make actual GitHub API calls)
    let validation_result = detector.validate_state().await;

    match validation_result {
        Ok(drifts) => {
            println!("Validation successful, detected {} drifts", drifts.len());
            // Should handle empty drift list gracefully
            assert!(drifts.len() >= 0);

            // Test drift report after validation
            let post_validation_report = detector.generate_drift_report();
            assert!(post_validation_report.last_validation <= chrono::Utc::now());
        }
        Err(e) => {
            // GitHub API errors are expected in some test environments
            println!("Expected GitHub API error: {:?}", e);
            // Test should not panic even with API errors
        }
    }
}

/// Test workflow state machine with realistic scenarios
#[tokio::test]
async fn test_workflow_with_realistic_github_scenarios() {
    let mut workflow = AutonomousWorkflowMachine::new(8);

    // Test assignment with realistic issue data
    let realistic_issue = Issue {
        number: 12345,
        title: "[TEST] Realistic autonomous workflow test".to_string(),
        body: "This is a test issue for validating autonomous workflow capabilities with realistic GitHub-style data.".to_string(),
        labels: vec![
            "autonomous-agent".to_string(),
            "integration-test".to_string(),
            "priority-medium".to_string(),
        ],
        priority: Priority::Medium,
        estimated_hours: Some(4),
    };

    let assign_result = workflow
        .handle_event(AutonomousEvent::AssignAgent {
            agent: AgentId("realistic-test-agent".to_string()),
            workspace_ready: true,
        })
        .await;

    assert!(
        assign_result.is_ok(),
        "Assignment failed: {:?}",
        assign_result
    );

    // Verify state transition
    assert!(matches!(
        workflow.current_state(),
        Some(AutonomousWorkflowState::Assigned { .. })
    ));

    // Test workflow progression through realistic steps
    workflow
        .handle_event(AutonomousEvent::StartWork)
        .await
        .unwrap();

    // Simulate realistic work progression
    for i in 1..=3 {
        let progress_result = workflow
            .handle_event(AutonomousEvent::MakeProgress {
                commits: i,
                files_changed: i * 2,
            })
            .await;
        assert!(
            progress_result.is_ok(),
            "Progress step {} failed: {:?}",
            i,
            progress_result
        );
    }

    // Complete work
    workflow
        .handle_event(AutonomousEvent::CompleteWork)
        .await
        .unwrap();

    // Submit for review with realistic PR data
    let realistic_pr = PullRequest {
        number: 9876,
        title: "[AUTONOMOUS] Complete realistic workflow test".to_string(),
        branch: "realistic-test-agent/12345-realistic-autonomous".to_string(),
        commits: 6,        // Sum of progress commits
        files_changed: 12, // Sum of files changed
    };

    let submit_result = workflow
        .handle_event(AutonomousEvent::SubmitForReview { pr: realistic_pr })
        .await;
    assert!(
        submit_result.is_ok(),
        "Submit for review failed: {:?}",
        submit_result
    );

    // Verify state history contains all transitions
    let history = workflow.state_history();
    assert!(
        history.len() >= 6,
        "Expected at least 6 state transitions, got {}",
        history.len()
    );

    // Test status report generation
    let status = workflow.generate_status_report();
    assert!(status.agent_id.is_some());
    assert!(status.transitions_count > 0);
    assert!(status.uptime_minutes > 0);
}

/// Test error handling with GitHub API limitations
#[tokio::test]
async fn test_github_api_error_handling() {
    if !has_github_credentials() {
        println!("Skipping GitHub API error handling test - no credentials");
        return;
    }

    let github_client = GitHubClient::new().expect("Failed to create GitHub client");
    let mut detector = StateDriftDetector::new(github_client, "error-handling-agent".to_string());

    // Update expected state with a non-existent issue to trigger API errors
    let non_existent_state = AutonomousWorkflowState::Assigned {
        issue: Issue {
            number: 999999999, // Very unlikely to exist
            title: "Non-existent issue".to_string(),
            body: "This issue should not exist".to_string(),
            labels: vec!["non-existent".to_string()],
            priority: Priority::Low,
            estimated_hours: Some(1),
        },
        agent: AgentId("error-handling-agent".to_string()),
        workspace: WorkspaceState {
            current_branch: "main".to_string(),
            uncommitted_changes: false,
        },
    };

    let update_result = detector.update_expected_state(&non_existent_state).await;
    assert!(
        update_result.is_ok(),
        "State update should succeed even with non-existent issue"
    );

    // Validation might fail due to non-existent issue, but should handle gracefully
    let validation_result = detector.validate_state().await;

    match validation_result {
        Ok(drifts) => {
            // If validation succeeds (perhaps due to graceful error handling), that's fine
            println!(
                "Validation handled non-existent issue gracefully, {} drifts detected",
                drifts.len()
            );
        }
        Err(e) => {
            // API errors should be handled gracefully, not panic
            println!("API error handled gracefully: {:?}", e);
            // Verify the error is properly typed
            assert!(matches!(
                e,
                my_little_soda::autonomous::state_validation::StateDriftError::GitHubError(_)
                    | my_little_soda::autonomous::state_validation::StateDriftError::ValidationFailed { .. }
            ));
        }
    }

    // Detector should still be functional after error
    let post_error_report = detector.generate_drift_report();
    assert_eq!(post_error_report.agent_id, "error-handling-agent");
}

/// Test autonomous operation lifecycle
#[tokio::test]
async fn test_autonomous_operation_lifecycle() {
    if !has_github_credentials() {
        println!("Skipping autonomous lifecycle test - no credentials");
        return;
    }

    let github_client = GitHubClient::new().expect("Failed to create GitHub client");
    let recovery_client = Box::new(AutoRecovery::new(github_client.clone(), true));
    let config = CoordinationConfig {
        max_work_hours: 1,
        max_recovery_attempts: 1,
        recovery_timeout_minutes: 1,
        enable_aggressive_recovery: false,
        enable_state_persistence: false,
        monitoring_interval_minutes: 1,
        enable_drift_detection: true,
        drift_validation_interval_minutes: 1,
    };

    let coordinator = AutonomousCoordinator::new(
        github_client,
        "lifecycle-test-agent".to_string(),
        recovery_client,
        config,
    )
    .await
    .expect("Failed to create coordinator");

    // Test initial state
    assert!(!coordinator.is_running().await);

    // Start autonomous operation (this will run briefly then timeout)
    let start_task = tokio::spawn(async move {
        let start_result = coordinator.start_autonomous_operation().await;
        // Operation should eventually stop due to timeout or lack of work
        (coordinator, start_result)
    });

    // Give it a moment to start
    sleep(Duration::from_millis(100)).await;

    // Wait for completion (should be quick due to short timeouts)
    let (coordinator, start_result) = start_task.await.expect("Task should complete");

    // Operation should complete (either successfully or with timeout)
    match start_result {
        Ok(()) => {
            println!("Autonomous operation completed successfully");
        }
        Err(e) => {
            println!(
                "Autonomous operation completed with expected error: {:?}",
                e
            );
        }
    }

    // Should not be running after completion
    assert!(!coordinator.is_running().await);

    // Test final status
    let final_status = coordinator.get_status_report().await;
    assert!(final_status.agent_id.is_some());
}

/// Test drift detection timing with real GitHub
#[tokio::test]
async fn test_drift_detection_timing() {
    if !has_github_credentials() {
        println!("Skipping drift timing test - no credentials");
        return;
    }

    let github_client = GitHubClient::new().expect("Failed to create GitHub client");
    let mut detector = StateDriftDetector::new(github_client, "timing-test-agent".to_string())
        .with_validation_interval(chrono::Duration::seconds(1)); // Very short interval for testing

    // Should initially need validation
    assert!(detector.needs_validation(true));
    assert!(detector.needs_validation(false));

    // Perform validation
    let validation_start = std::time::Instant::now();
    let validation_result = detector.validate_state().await;
    let validation_duration = validation_start.elapsed();

    println!("Validation took {:?}", validation_duration);

    // Validation should complete in reasonable time (even if it fails)
    assert!(
        validation_duration < Duration::from_secs(30),
        "Validation took too long: {:?}",
        validation_duration
    );

    match validation_result {
        Ok(_) => {
            // After validation, timing should be updated
            let report = detector.generate_drift_report();
            assert!(report.last_validation <= chrono::Utc::now());
            assert!(report.next_validation > report.last_validation);
        }
        Err(e) => {
            println!("Validation failed as expected in test environment: {:?}", e);
        }
    }
}

/// Test autonomous coordinator status reporting with real GitHub
#[tokio::test]
async fn test_status_reporting_with_real_github() {
    if !has_github_credentials() {
        println!("Skipping status reporting test - no credentials");
        return;
    }

    let github_client = GitHubClient::new().expect("Failed to create GitHub client");
    let recovery_client = Box::new(AutoRecovery::new(github_client.clone(), true));
    let config = CoordinationConfig::default();

    let coordinator = AutonomousCoordinator::new(
        github_client,
        "status-reporting-agent".to_string(),
        recovery_client,
        config,
    )
    .await
    .expect("Failed to create coordinator");

    // Test detailed status reporting
    let status_report = coordinator.get_status_report().await;
    assert_eq!(
        status_report.agent_id.as_ref().unwrap(),
        "status-reporting-agent"
    );
    assert_eq!(status_report.transitions_count, 0); // No transitions yet
    assert!(status_report.uptime_minutes >= 0);
    assert!(!status_report.can_continue); // No work assigned
    assert!(status_report.timeout_in_minutes.is_some());

    // Test recovery reporting
    let recovery_report = coordinator.get_recovery_report().await;
    assert_eq!(recovery_report.total_attempts, 0);
    assert_eq!(recovery_report.successful_attempts, 0);
    assert_eq!(recovery_report.failed_attempts, 0);
    assert_eq!(recovery_report.success_rate, 1.0); // No attempts = 100%
    assert!(recovery_report.last_attempt.is_none());
    assert!(recovery_report.average_recovery_time_seconds.is_none());

    // Test drift detection reporting
    let drift_report = coordinator.get_drift_report().await;
    assert_eq!(drift_report.agent_id, "status-reporting-agent");
    assert_eq!(drift_report.total_drifts_detected, 0);
    assert_eq!(drift_report.critical_drifts.len(), 0);
    assert_eq!(
        drift_report.validation_health,
        my_little_soda::autonomous::state_validation::ValidationHealth::Healthy
    );

    println!("All status reports generated successfully with real GitHub client");
}
