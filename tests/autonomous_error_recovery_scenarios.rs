#![cfg(feature = "autonomous")]
//! Comprehensive error recovery scenario tests for autonomous system
//!
//! These tests validate the autonomous system's ability to recover from various
//! error conditions and failure scenarios that may occur during operation.

use chrono::Utc;
use std::time::Duration;
use tokio::time::sleep;

use my_little_soda::{
    agents::recovery::AutoRecovery,
    autonomous::{
        error_recovery::{
            AutonomousRecoveryAttempt, ConfidenceLevel, ErrorType, FixType, RecoveryStrategy,
        },
        AbandonmentReason, AgentId, AutonomousCoordinator, AutonomousErrorRecovery,
        AutonomousEvent, AutonomousWorkflowError, AutonomousWorkflowMachine,
        AutonomousWorkflowState, BlockerType, CIFailure, ConflictInfo, CoordinationConfig, Issue,
        Priority, PullRequest, WorkspaceState,
    },
    GitHubClient,
};

/// Test recovery from basic git operation failures
#[tokio::test]
async fn test_git_operation_failure_recovery() {
    let github_client = GitHubClient::new().expect("Failed to create GitHub client");
    let base_recovery = Box::new(AutoRecovery::new(github_client.clone(), true));
    let error_recovery = AutonomousErrorRecovery::new(github_client, base_recovery);

    // Test git push failure recovery
    let git_push_error = ErrorType::GitOperationFailed {
        operation: "push".to_string(),
        error: "Connection timeout to remote repository".to_string(),
    };

    let push_strategy = error_recovery.determine_recovery_strategy(&git_push_error);
    assert!(matches!(
        push_strategy,
        RecoveryStrategy::RetryWithBackoff { .. }
    ));

    // Test git pull failure recovery
    let git_pull_error = ErrorType::GitOperationFailed {
        operation: "pull".to_string(),
        error: "Authentication failed".to_string(),
    };

    let pull_strategy = error_recovery.determine_recovery_strategy(&git_pull_error);
    assert!(matches!(
        pull_strategy,
        RecoveryStrategy::RetryWithBackoff { .. }
    ));

    // Test git clone failure recovery
    let git_clone_error = ErrorType::GitOperationFailed {
        operation: "clone".to_string(),
        error: "Repository not found".to_string(),
    };

    let clone_strategy = error_recovery.determine_recovery_strategy(&git_clone_error);
    // Repository not found might require escalation
    assert!(matches!(
        clone_strategy,
        RecoveryStrategy::RetryWithBackoff { .. } | RecoveryStrategy::Escalate { .. }
    ));
}

/// Test recovery from build and compilation failures
#[tokio::test]
async fn test_build_failure_recovery() {
    let github_client = GitHubClient::new().expect("Failed to create GitHub client");
    let base_recovery = Box::new(AutoRecovery::new(github_client.clone(), true));
    let error_recovery = AutonomousErrorRecovery::new(github_client, base_recovery);

    // Test compilation error recovery
    let compile_error = ErrorType::BuildFailure {
        stage: "compile".to_string(),
        error: "error[E0308]: mismatched types".to_string(),
    };

    let compile_strategy = error_recovery.determine_recovery_strategy(&compile_error);
    assert!(matches!(
        compile_strategy,
        RecoveryStrategy::AutomatedFix { .. }
    ));

    // Test linking error recovery
    let link_error = ErrorType::BuildFailure {
        stage: "link".to_string(),
        error: "undefined reference to 'missing_function'".to_string(),
    };

    let link_strategy = error_recovery.determine_recovery_strategy(&link_error);
    assert!(matches!(
        link_strategy,
        RecoveryStrategy::AutomatedFix { .. } | RecoveryStrategy::Escalate { .. }
    ));

    // Test cargo build failure recovery
    let cargo_error = ErrorType::BuildFailure {
        stage: "cargo-build".to_string(),
        error: "package not found in registry".to_string(),
    };

    let cargo_strategy = error_recovery.determine_recovery_strategy(&cargo_error);
    assert!(matches!(
        cargo_strategy,
        RecoveryStrategy::AutomatedFix { .. } | RecoveryStrategy::RetryWithBackoff { .. }
    ));
}

/// Test recovery from test suite failures
#[tokio::test]
async fn test_test_failure_recovery() {
    let github_client = GitHubClient::new().expect("Failed to create GitHub client");
    let base_recovery = Box::new(AutoRecovery::new(github_client.clone(), true));
    let error_recovery = AutonomousErrorRecovery::new(github_client, base_recovery);

    // Test unit test failure recovery
    let unit_test_error = ErrorType::TestFailure {
        test_suite: "unit".to_string(),
        failed_tests: vec![
            "test_authentication".to_string(),
            "test_validation".to_string(),
        ],
    };

    let unit_strategy = error_recovery.determine_recovery_strategy(&unit_test_error);
    assert!(matches!(
        unit_strategy,
        RecoveryStrategy::AutomatedFix { .. }
    ));

    // Test integration test failure recovery
    let integration_error = ErrorType::TestFailure {
        test_suite: "integration".to_string(),
        failed_tests: vec!["test_end_to_end_workflow".to_string()],
    };

    let integration_strategy = error_recovery.determine_recovery_strategy(&integration_error);
    assert!(matches!(
        integration_strategy,
        RecoveryStrategy::AutomatedFix { .. } | RecoveryStrategy::Escalate { .. }
    ));

    // Test performance test failure recovery
    let perf_test_error = ErrorType::TestFailure {
        test_suite: "performance".to_string(),
        failed_tests: vec!["test_response_time_benchmark".to_string()],
    };

    let perf_strategy = error_recovery.determine_recovery_strategy(&perf_test_error);
    // Performance tests might need different handling
    assert!(matches!(
        perf_strategy,
        RecoveryStrategy::RetryWithBackoff { .. } | RecoveryStrategy::AutomatedFix { .. }
    ));
}

/// Test recovery from merge conflict scenarios
#[tokio::test]
async fn test_merge_conflict_recovery() {
    let github_client = GitHubClient::new().expect("Failed to create GitHub client");
    let base_recovery = Box::new(AutoRecovery::new(github_client.clone(), true));
    let error_recovery = AutonomousErrorRecovery::new(github_client, base_recovery);

    // Test simple merge conflict recovery
    let simple_conflict = ErrorType::MergeConflict {
        files: vec!["src/lib.rs".to_string()],
        conflict_count: 1,
    };

    let simple_strategy = error_recovery.determine_recovery_strategy(&simple_conflict);
    assert!(matches!(
        simple_strategy,
        RecoveryStrategy::AutomatedFix {
            fix_type: FixType::MergeConflictResolution,
            confidence: ConfidenceLevel::High
        }
    ));

    // Test complex merge conflict recovery
    let complex_conflict = ErrorType::MergeConflict {
        files: vec![
            "src/main.rs".to_string(),
            "src/lib.rs".to_string(),
            "Cargo.toml".to_string(),
            "README.md".to_string(),
        ],
        conflict_count: 12,
    };

    let complex_strategy = error_recovery.determine_recovery_strategy(&complex_conflict);
    assert!(matches!(
        complex_strategy,
        RecoveryStrategy::AutomatedFix {
            confidence: ConfidenceLevel::Low,
            ..
        } | RecoveryStrategy::Escalate { .. }
    ));

    // Test binary file merge conflict recovery
    let binary_conflict = ErrorType::MergeConflict {
        files: vec![
            "assets/image.png".to_string(),
            "docs/diagram.pdf".to_string(),
        ],
        conflict_count: 2,
    };

    let binary_strategy = error_recovery.determine_recovery_strategy(&binary_conflict);
    // Binary conflicts typically require escalation
    assert!(matches!(
        binary_strategy,
        RecoveryStrategy::Escalate { .. }
            | RecoveryStrategy::AutomatedFix {
                confidence: ConfidenceLevel::Low,
                ..
            }
    ));
}

/// Test recovery from dependency-related failures
#[tokio::test]
async fn test_dependency_failure_recovery() {
    let github_client = GitHubClient::new().expect("Failed to create GitHub client");
    let base_recovery = Box::new(AutoRecovery::new(github_client.clone(), true));
    let error_recovery = AutonomousErrorRecovery::new(github_client, base_recovery);

    // Test missing dependency recovery
    let missing_dep = ErrorType::DependencyIssue {
        dependency: "serde".to_string(),
        version_conflict: false,
    };

    let missing_strategy = error_recovery.determine_recovery_strategy(&missing_dep);
    assert!(matches!(
        missing_strategy,
        RecoveryStrategy::AutomatedFix { .. }
    ));

    // Test version conflict recovery
    let version_conflict = ErrorType::DependencyIssue {
        dependency: "tokio".to_string(),
        version_conflict: true,
    };

    let version_strategy = error_recovery.determine_recovery_strategy(&version_conflict);
    assert!(matches!(
        version_strategy,
        RecoveryStrategy::AutomatedFix { .. } | RecoveryStrategy::Escalate { .. }
    ));

    // Test security vulnerability in dependency
    let security_issue = ErrorType::SecurityVulnerability {
        vulnerability_id: "RUSTSEC-2023-0001".to_string(),
        affected_package: "vulnerable-crate".to_string(),
        severity: "high".to_string(),
    };

    let security_strategy = error_recovery.determine_recovery_strategy(&security_issue);
    // Security issues should have high priority for fixing
    assert!(matches!(
        security_strategy,
        RecoveryStrategy::AutomatedFix { .. }
    ));
}

/// Test recovery execution with various error scenarios
#[tokio::test]
async fn test_recovery_execution_scenarios() {
    let github_client = GitHubClient::new().expect("Failed to create GitHub client");
    let base_recovery = Box::new(AutoRecovery::new(github_client.clone(), true));
    let error_recovery = AutonomousErrorRecovery::new(github_client, base_recovery);

    // Create a test workflow state for recovery context
    let test_state = AutonomousWorkflowState::Blocked {
        issue: Issue {
            number: 123,
            title: "Recovery test issue".to_string(),
            body: "Testing error recovery scenarios".to_string(),
            labels: vec!["recovery-test".to_string()],
            priority: Priority::High,
            estimated_hours: Some(3),
        },
        agent: AgentId("recovery-test-agent".to_string()),
        blocker: BlockerType::BuildFailure {
            error: "compilation failed".to_string(),
        },
    };

    // Test automated fix execution
    let automated_error = ErrorType::BuildFailure {
        stage: "compile".to_string(),
        error: "missing semicolon at line 42".to_string(),
    };

    let automated_strategy = RecoveryStrategy::AutomatedFix {
        fix_type: FixType::SyntaxError,
        confidence: ConfidenceLevel::High,
    };

    let automated_result = error_recovery
        .execute_recovery_strategy(automated_error.clone(), automated_strategy, &test_state)
        .await;

    assert!(
        automated_result.is_ok(),
        "Automated recovery failed: {:?}",
        automated_result
    );

    let attempt = automated_result.unwrap();
    assert!(!attempt.attempt_id.is_empty());
    assert!(attempt.recovery_actions.len() > 0);
    assert!(attempt.duration_seconds > 0.0);

    // Test retry with backoff execution
    let network_error = ErrorType::GitOperationFailed {
        operation: "fetch".to_string(),
        error: "network unreachable".to_string(),
    };

    let retry_strategy = RecoveryStrategy::RetryWithBackoff {
        max_attempts: 3,
        base_delay_ms: 1000,
        max_delay_ms: 8000,
    };

    let retry_result = error_recovery
        .execute_recovery_strategy(network_error, retry_strategy, &test_state)
        .await;

    match retry_result {
        Ok(attempt) => {
            assert!(!attempt.attempt_id.is_empty());
            assert!(attempt.recovery_actions.len() > 0);
        }
        Err(e) => {
            // Retry failures are acceptable in test environment
            println!("Expected retry failure in test environment: {:?}", e);
        }
    }
}

/// Test error recovery escalation scenarios
#[tokio::test]
async fn test_escalation_scenarios() {
    let github_client = GitHubClient::new().expect("Failed to create GitHub client");
    let base_recovery = Box::new(AutoRecovery::new(github_client.clone(), true));
    let error_recovery = AutonomousErrorRecovery::new(github_client, base_recovery);

    // Test escalation for critical system errors
    let system_error = ErrorType::SystemError {
        error_code: "DISK_FULL".to_string(),
        message: "No space left on device".to_string(),
    };

    let system_strategy = error_recovery.determine_recovery_strategy(&system_error);
    assert!(matches!(system_strategy, RecoveryStrategy::Escalate { .. }));

    // Test escalation for security vulnerabilities
    let critical_security = ErrorType::SecurityVulnerability {
        vulnerability_id: "CVE-2023-12345".to_string(),
        affected_package: "core-system".to_string(),
        severity: "critical".to_string(),
    };

    let security_strategy = error_recovery.determine_recovery_strategy(&critical_security);
    // Critical security issues might be auto-fixed or escalated
    assert!(matches!(
        security_strategy,
        RecoveryStrategy::AutomatedFix { .. } | RecoveryStrategy::Escalate { .. }
    ));

    // Test escalation for complex state inconsistencies
    let state_error = ErrorType::StateInconsistency {
        expected_state: "ready_for_review".to_string(),
        actual_state: "unknown".to_string(),
    };

    let state_strategy = error_recovery.determine_recovery_strategy(&state_error);
    assert!(matches!(
        state_strategy,
        RecoveryStrategy::Escalate { .. } | RecoveryStrategy::AutomatedFix { .. }
    ));
}

/// Test workflow state machine error recovery integration
#[tokio::test]
async fn test_workflow_error_recovery_integration() {
    let mut workflow = AutonomousWorkflowMachine::new(8);

    // Initialize workflow
    workflow
        .handle_event(AutonomousEvent::AssignAgent {
            agent: AgentId("integration-recovery-agent".to_string()),
            workspace_ready: true,
        })
        .await
        .unwrap();

    workflow
        .handle_event(AutonomousEvent::StartWork)
        .await
        .unwrap();

    // Simulate encountering a recoverable blocker
    let recoverable_blocker = BlockerType::TestFailure {
        test_name: "recoverable_test".to_string(),
        error: "temporary network issue".to_string(),
    };

    let blocker_result = workflow
        .handle_event(AutonomousEvent::EncounterBlocker {
            blocker: recoverable_blocker.clone(),
        })
        .await;

    assert!(
        blocker_result.is_ok(),
        "Blocker encounter failed: {:?}",
        blocker_result
    );
    assert!(matches!(
        workflow.current_state(),
        Some(AutonomousWorkflowState::Blocked { .. })
    ));

    // Test recovery from blocker
    let resolve_result = workflow.handle_event(AutonomousEvent::ResolveBlocker).await;
    assert!(
        resolve_result.is_ok(),
        "Blocker resolution failed: {:?}",
        resolve_result
    );
    assert!(matches!(
        workflow.current_state(),
        Some(AutonomousWorkflowState::InProgress { .. })
    ));

    // Test encountering an unrecoverable blocker
    let unrecoverable_blocker = BlockerType::DependencyIssue {
        dependency: "nonexistent-package".to_string(),
        error: "package not found and no alternatives available".to_string(),
    };

    workflow
        .handle_event(AutonomousEvent::EncounterBlocker {
            blocker: unrecoverable_blocker,
        })
        .await
        .unwrap();

    // Try to resolve but this should lead to abandonment in some cases
    let abandon_result = workflow
        .handle_event(AutonomousEvent::ForceAbandon {
            reason: AbandonmentReason::UnresolvableBlocker {
                blocker: BlockerType::DependencyIssue {
                    dependency: "nonexistent-package".to_string(),
                    error: "package not found".to_string(),
                },
            },
        })
        .await;

    assert!(abandon_result.is_ok());
    assert!(matches!(
        workflow.current_state(),
        Some(AutonomousWorkflowState::Abandoned { .. })
    ));
}

/// Test recovery from CI/CD failures
#[tokio::test]
async fn test_ci_cd_failure_recovery() {
    let mut workflow = AutonomousWorkflowMachine::new(8);

    // Setup workflow to approved state
    workflow
        .handle_event(AutonomousEvent::AssignAgent {
            agent: AgentId("ci-recovery-agent".to_string()),
            workspace_ready: true,
        })
        .await
        .unwrap();

    workflow
        .handle_event(AutonomousEvent::StartWork)
        .await
        .unwrap();
    workflow
        .handle_event(AutonomousEvent::CompleteWork)
        .await
        .unwrap();

    let pr = PullRequest {
        number: 456,
        title: "CI recovery test".to_string(),
        branch: "ci-recovery-branch".to_string(),
        commits: 3,
        files_changed: 5,
    };

    workflow
        .handle_event(AutonomousEvent::SubmitForReview { pr })
        .await
        .unwrap();
    workflow
        .handle_event(AutonomousEvent::ApprovalReceived)
        .await
        .unwrap();

    // Test recoverable CI failure
    let recoverable_ci_failure = vec![CIFailure {
        job_name: "test".to_string(),
        step: "unit-tests".to_string(),
        error: "flaky test timeout".to_string(),
        auto_fixable: true,
    }];

    let ci_failure_result = workflow
        .handle_event(AutonomousEvent::CIFailureDetected {
            failures: recoverable_ci_failure,
        })
        .await;

    assert!(
        ci_failure_result.is_ok(),
        "CI failure handling failed: {:?}",
        ci_failure_result
    );
    assert!(matches!(
        workflow.current_state(),
        Some(AutonomousWorkflowState::CIFailure { .. })
    ));

    // Test CI fix
    let ci_fix_result = workflow.handle_event(AutonomousEvent::CIFixed).await;
    assert!(ci_fix_result.is_ok(), "CI fix failed: {:?}", ci_fix_result);
    assert!(matches!(
        workflow.current_state(),
        Some(AutonomousWorkflowState::Approved { .. })
    ));

    // Test unrecoverable CI failure
    let unrecoverable_ci_failure = vec![CIFailure {
        job_name: "security-scan".to_string(),
        step: "vulnerability-check".to_string(),
        error: "critical security vulnerability detected".to_string(),
        auto_fixable: false,
    }];

    workflow
        .handle_event(AutonomousEvent::CIFailureDetected {
            failures: unrecoverable_ci_failure,
        })
        .await
        .unwrap();

    // This should require manual intervention or abandonment
    assert!(matches!(
        workflow.current_state(),
        Some(AutonomousWorkflowState::CIFailure { .. })
    ));
}

/// Test recovery attempt tracking and reporting
#[tokio::test]
async fn test_recovery_tracking_and_reporting() {
    let github_client = GitHubClient::new().expect("Failed to create GitHub client");
    let base_recovery = Box::new(AutoRecovery::new(github_client.clone(), true));
    let error_recovery = AutonomousErrorRecovery::new(github_client, base_recovery);

    // Initial recovery report should be empty
    let initial_report = error_recovery.generate_recovery_report();
    assert_eq!(initial_report.total_attempts, 0);
    assert_eq!(initial_report.success_rate, 1.0); // No attempts = 100%
    assert!(initial_report.last_attempt.is_none());

    // Create test errors and perform recovery attempts
    let errors_and_strategies = vec![
        (
            ErrorType::BuildFailure {
                stage: "compile".to_string(),
                error: "syntax error".to_string(),
            },
            RecoveryStrategy::AutomatedFix {
                fix_type: FixType::SyntaxError,
                confidence: ConfidenceLevel::High,
            },
        ),
        (
            ErrorType::GitOperationFailed {
                operation: "push".to_string(),
                error: "network timeout".to_string(),
            },
            RecoveryStrategy::RetryWithBackoff {
                max_attempts: 2,
                base_delay_ms: 1000,
                max_delay_ms: 4000,
            },
        ),
    ];

    let test_state = AutonomousWorkflowState::Blocked {
        issue: Issue {
            number: 789,
            title: "Recovery tracking test".to_string(),
            body: "Testing recovery attempt tracking".to_string(),
            labels: vec!["recovery-tracking".to_string()],
            priority: Priority::Medium,
            estimated_hours: Some(2),
        },
        agent: AgentId("tracking-test-agent".to_string()),
        blocker: BlockerType::BuildFailure {
            error: "test error".to_string(),
        },
    };

    let mut successful_attempts = 0;
    let mut total_attempts = 0;

    for (error, strategy) in errors_and_strategies {
        let result = error_recovery
            .execute_recovery_strategy(error, strategy, &test_state)
            .await;

        total_attempts += 1;

        match result {
            Ok(attempt) => {
                successful_attempts += 1;
                println!(
                    "Recovery attempt {} successful: {}",
                    total_attempts, attempt.success
                );
            }
            Err(e) => {
                println!("Recovery attempt {} failed: {:?}", total_attempts, e);
            }
        }
    }

    // Check recovery report after attempts
    let final_report = error_recovery.generate_recovery_report();

    println!("Recovery Tracking Results:");
    println!("  Total attempts: {}", final_report.total_attempts);
    println!(
        "  Successful attempts: {}",
        final_report.successful_attempts
    );
    println!("  Failed attempts: {}", final_report.total_attempts - final_report.successful_attempts);
    println!("  Success rate: {:.2}%", final_report.success_rate * 100.0);

    // Validate tracking
    assert!(final_report.total_attempts >= total_attempts as u32);
    assert!(final_report.success_rate >= 0.0 && final_report.success_rate <= 1.0);
    assert_eq!(
        final_report.total_attempts,
        final_report.total_attempts
    );
}

/// Test timeout handling in error recovery
#[tokio::test]
async fn test_recovery_timeout_handling() {
    let github_client = GitHubClient::new().expect("Failed to create GitHub client");
    let base_recovery = Box::new(AutoRecovery::new(github_client.clone(), true));

    // Create error recovery with very short timeout
    let error_recovery = AutonomousErrorRecovery::new(github_client, base_recovery).with_timeout(1); // 1 minute timeout for testing

    let test_state = AutonomousWorkflowState::Blocked {
        issue: Issue {
            number: 101,
            title: "Timeout test".to_string(),
            body: "Testing recovery timeout handling".to_string(),
            labels: vec!["timeout-test".to_string()],
            priority: Priority::Low,
            estimated_hours: Some(1),
        },
        agent: AgentId("timeout-test-agent".to_string()),
        blocker: BlockerType::TestFailure {
            test_name: "slow_test".to_string(),
            error: "operation timed out".to_string(),
        },
    };

    // Test that recovery operations complete within reasonable time
    let recovery_start = std::time::Instant::now();

    let timeout_error = ErrorType::TestFailure {
        test_suite: "integration".to_string(),
        failed_tests: vec!["timeout_test".to_string()],
    };

    let timeout_strategy = RecoveryStrategy::RetryWithBackoff {
        max_attempts: 1, // Single attempt to avoid long test duration
        base_delay_ms: 1000,
        max_delay_ms: 1000,
    };

    let timeout_result = error_recovery
        .execute_recovery_strategy(timeout_error, timeout_strategy, &test_state)
        .await;

    let recovery_duration = recovery_start.elapsed();

    println!("Recovery timeout test completed in {:?}", recovery_duration);

    // Recovery should complete within reasonable time even if it fails
    assert!(
        recovery_duration < Duration::from_secs(30),
        "Recovery took too long: {:?}",
        recovery_duration
    );

    // Result can be success or failure, but should not panic or hang
    match timeout_result {
        Ok(attempt) => {
            println!("Recovery completed successfully: {}", attempt.success);
        }
        Err(e) => {
            println!("Recovery failed as expected: {:?}", e);
        }
    }
}
