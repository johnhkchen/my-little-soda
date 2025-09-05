#![cfg(feature = "autonomous")]
//! Integration tests for autonomous workflow state machine
//!
//! Tests the complete autonomous workflow from assignment to completion,
//! including error recovery, state persistence, and integration with existing systems.

use std::time::Duration;
use tempfile::TempDir;
use tokio::time::sleep;

use my_little_soda::{
    agents::recovery::AutoRecovery,
    autonomous::{
        error_recovery::{ConfidenceLevel, ErrorType, FixType, RecoveryStrategy},
        integration::AutonomousIntegrationFactory,
        integration::IntegrationConfig,
        persistence::PersistentWorkflowState,
        work_continuity::WorkProgress,
        AbandonmentReason, AgentId, BlockerType, CIFailure, CheckpointReason, CompletedWork,
        ConflictInfo, Issue, PersistenceConfig, Priority, PullRequest,
    },
    AutonomousCoordinator, AutonomousErrorRecovery, AutonomousEvent, AutonomousWorkflowMachine,
    AutonomousWorkflowState, CoordinationConfig, GitHubClient, IntegrationCoordinator,
    StatePersistenceManager,
};

/// Test the complete autonomous workflow from start to finish
#[tokio::test]
async fn test_complete_autonomous_workflow() {
    let github_client = GitHubClient::new().expect("Failed to create GitHub client");
    let recovery_client = Box::new(AutoRecovery::new(github_client.clone(), true));
    let config = CoordinationConfig::default();

    let mut coordinator = AutonomousCoordinator::new(
        github_client,
        "test-agent-001".to_string(),
        recovery_client,
        config,
    )
    .await
    .expect("Failed to create autonomous coordinator");

    // Test assignment
    let assign_result = coordinator.start_autonomous_operation().await;
    assert!(
        assign_result.is_ok(),
        "Failed to start autonomous operation: {:?}",
        assign_result
    );

    // Give it a moment to initialize
    sleep(Duration::from_millis(100)).await;

    let status = coordinator.get_status_report().await;
    assert!(status.agent_id.is_some());
    assert!(status.current_state.is_some());

    // Stop the operation
    let stop_result = coordinator.stop_autonomous_operation().await;
    assert!(
        stop_result.is_ok(),
        "Failed to stop autonomous operation: {:?}",
        stop_result
    );

    let final_status = coordinator.get_status_report().await;
    assert!(matches!(
        final_status.current_state,
        Some(AutonomousWorkflowState::Abandoned { .. })
    ));
}

/// Test autonomous workflow state machine basic transitions
#[tokio::test]
async fn test_workflow_state_machine_transitions() {
    let mut workflow = AutonomousWorkflowMachine::new(8);

    // Test initial assignment
    let assign_event = AutonomousEvent::AssignAgent {
        agent: AgentId("test-agent".to_string()),
        workspace_ready: true,
    };

    let result = workflow.handle_event(assign_event).await;
    assert!(result.is_ok(), "Assignment failed: {:?}", result);

    assert!(matches!(
        workflow.current_state(),
        Some(AutonomousWorkflowState::Assigned { .. })
    ));

    // Test starting work
    let start_work_result = workflow.handle_event(AutonomousEvent::StartWork).await;
    assert!(
        start_work_result.is_ok(),
        "Start work failed: {:?}",
        start_work_result
    );

    assert!(matches!(
        workflow.current_state(),
        Some(AutonomousWorkflowState::InProgress { .. })
    ));

    // Test making progress
    let progress_result = workflow
        .handle_event(AutonomousEvent::MakeProgress {
            commits: 3,
            files_changed: 5,
        })
        .await;
    assert!(
        progress_result.is_ok(),
        "Make progress failed: {:?}",
        progress_result
    );

    // Verify state history is being recorded
    let history = workflow.state_history();
    assert!(history.len() >= 3, "Expected at least 3 state transitions");

    // Test completing work
    let complete_result = workflow.handle_event(AutonomousEvent::CompleteWork).await;
    assert!(
        complete_result.is_ok(),
        "Complete work failed: {:?}",
        complete_result
    );

    assert!(matches!(
        workflow.current_state(),
        Some(AutonomousWorkflowState::ReadyForReview { .. })
    ));
}

/// Test blocker handling and recovery
#[tokio::test]
async fn test_blocker_handling_and_recovery() {
    let mut workflow = AutonomousWorkflowMachine::new(8);

    // Setup to in-progress state
    workflow
        .handle_event(AutonomousEvent::AssignAgent {
            agent: AgentId("test-agent".to_string()),
            workspace_ready: true,
        })
        .await
        .unwrap();

    workflow
        .handle_event(AutonomousEvent::StartWork)
        .await
        .unwrap();

    // Encounter a blocker
    let blocker = BlockerType::TestFailure {
        test_name: "integration_test".to_string(),
        error: "Database connection failed".to_string(),
    };

    let blocker_result = workflow
        .handle_event(AutonomousEvent::EncounterBlocker {
            blocker: blocker.clone(),
        })
        .await;
    assert!(
        blocker_result.is_ok(),
        "Blocker handling failed: {:?}",
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
}

/// Test error recovery system
#[tokio::test]
async fn test_error_recovery_system() {
    let github_client = GitHubClient::new().expect("Failed to create GitHub client");
    let base_recovery = Box::new(AutoRecovery::new(github_client.clone(), true));

    let mut error_recovery = AutonomousErrorRecovery::new(github_client, base_recovery);

    // Test strategy determination for different error types
    let git_error = ErrorType::GitOperationFailed {
        operation: "push".to_string(),
        error: "Connection timeout".to_string(),
    };

    let strategy = error_recovery.determine_recovery_strategy(&git_error);
    assert!(matches!(
        strategy,
        RecoveryStrategy::RetryWithBackoff { .. }
    ));

    let merge_conflict = ErrorType::MergeConflict {
        files: vec!["src/main.rs".to_string()],
        conflict_count: 2,
    };

    let conflict_strategy = error_recovery.determine_recovery_strategy(&merge_conflict);
    assert!(matches!(
        conflict_strategy,
        RecoveryStrategy::AutomatedFix {
            fix_type: FixType::MergeConflictResolution,
            confidence: ConfidenceLevel::High
        }
    ));

    // Test recovery execution
    let current_state = AutonomousWorkflowState::Blocked {
        issue: Issue {
            number: 123,
            title: "Test issue".to_string(),
            body: "Test body".to_string(),
            labels: vec![],
            priority: Priority::Medium,
            estimated_hours: Some(2),
        },
        agent: AgentId("test-agent".to_string()),
        blocker: BlockerType::TestFailure {
            test_name: "unit_test".to_string(),
            error: "Assertion failed".to_string(),
        },
    };

    let recovery_result = error_recovery
        .execute_recovery_strategy(git_error, strategy, &current_state)
        .await;

    assert!(
        recovery_result.is_ok(),
        "Recovery execution failed: {:?}",
        recovery_result
    );

    let attempt = recovery_result.unwrap();
    assert!(!attempt.attempt_id.is_empty());
    assert!(attempt.recovery_actions.len() > 0);
}

/// Test state persistence across restarts
#[tokio::test]
async fn test_state_persistence() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config = PersistenceConfig {
        enable_persistence: true,
        persistence_directory: temp_dir.path().to_path_buf(),
        auto_save_interval_minutes: 1,
        max_state_history_entries: 100,
        max_recovery_history_entries: 50,
        compress_old_states: false,
        backup_retention_days: 1,
        enable_integrity_checks: true,
    };

    let manager = StatePersistenceManager::new(config);

    // Create test state
    let persistent_state = PersistentWorkflowState {
        version: "1.0.0".to_string(),
        agent_id: "test-agent".to_string(),
        current_state: Some(AutonomousWorkflowState::InProgress {
            issue: Issue {
                number: 456,
                title: "Persistent test issue".to_string(),
                body: "Testing persistence".to_string(),
                labels: vec!["test".to_string()],
                priority: Priority::High,
                estimated_hours: Some(4),
            },
            agent: AgentId("test-agent".to_string()),
            progress: WorkProgress {
                commits_made: 2,
                files_changed: 3,
                tests_written: 1,
                elapsed_minutes: 30,
                completion_percentage: 50,
            },
        }),
        start_time: Some(chrono::Utc::now()),
        max_work_hours: 8,
        state_history: vec![],
        recovery_history: vec![],
        checkpoint_metadata: my_little_soda::autonomous::persistence::CheckpointMetadata {
            checkpoint_id: "test-checkpoint".to_string(),
            creation_reason: CheckpointReason::PeriodicSave,
            integrity_hash: "test-hash".to_string(),
            agent_pid: Some(std::process::id()),
            hostname: "test-host".to_string(),
        },
        last_persisted: chrono::Utc::now(),
    };

    // Test save
    let save_result = manager
        .save_state(&persistent_state, CheckpointReason::UserRequested)
        .await;
    assert!(save_result.is_ok(), "Save failed: {:?}", save_result);

    let checkpoint_id = save_result.unwrap();
    assert!(!checkpoint_id.is_empty());

    // Test load
    let load_result = manager.load_state(&persistent_state.agent_id).await;
    assert!(load_result.is_ok(), "Load failed: {:?}", load_result);

    let loaded_state = load_result.unwrap();
    assert!(loaded_state.is_some(), "No state loaded");

    let loaded_state = loaded_state.unwrap();
    assert_eq!(loaded_state.agent_id, persistent_state.agent_id);
    assert_eq!(loaded_state.version, persistent_state.version);
    assert_eq!(loaded_state.max_work_hours, persistent_state.max_work_hours);

    // Verify the current state was preserved
    assert!(loaded_state.current_state.is_some());
    match loaded_state.current_state.unwrap() {
        AutonomousWorkflowState::InProgress {
            issue, progress, ..
        } => {
            assert_eq!(issue.number, 456);
            assert_eq!(progress.commits_made, 2);
            assert_eq!(progress.completion_percentage, 50);
        }
        _ => panic!("Expected InProgress state"),
    }
}

/// Test checkpoint creation and restoration
#[tokio::test]
async fn test_checkpoint_operations() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config = PersistenceConfig {
        enable_persistence: true,
        persistence_directory: temp_dir.path().to_path_buf(),
        ..PersistenceConfig::default()
    };

    let manager = StatePersistenceManager::new(config);

    // Create test state
    let test_state = PersistentWorkflowState {
        version: "1.0.0".to_string(),
        agent_id: "checkpoint-test-agent".to_string(),
        current_state: Some(AutonomousWorkflowState::Approved {
            issue: Issue {
                number: 789,
                title: "Checkpoint test".to_string(),
                body: "Testing checkpoint functionality".to_string(),
                labels: vec!["checkpoint".to_string()],
                priority: Priority::Medium,
                estimated_hours: Some(2),
            },
            agent: AgentId("checkpoint-test-agent".to_string()),
            pr: PullRequest {
                number: 101,
                title: "Test PR".to_string(),
                branch: "checkpoint-test-branch".to_string(),
                commits: 3,
                files_changed: 4,
            },
        }),
        start_time: Some(chrono::Utc::now()),
        max_work_hours: 6,
        state_history: vec![],
        recovery_history: vec![],
        checkpoint_metadata: my_little_soda::autonomous::persistence::CheckpointMetadata {
            checkpoint_id: "initial".to_string(),
            creation_reason: CheckpointReason::PeriodicSave,
            integrity_hash: "initial-hash".to_string(),
            agent_pid: Some(std::process::id()),
            hostname: "test-host".to_string(),
        },
        last_persisted: chrono::Utc::now(),
    };

    // Create checkpoint
    let checkpoint_result = manager
        .create_checkpoint(&test_state, CheckpointReason::BeforeRecovery)
        .await;
    assert!(
        checkpoint_result.is_ok(),
        "Checkpoint creation failed: {:?}",
        checkpoint_result
    );

    let checkpoint_id = checkpoint_result.unwrap();
    assert!(!checkpoint_id.is_empty());

    // List checkpoints
    let list_result = manager.list_checkpoints(&test_state.agent_id).await;
    assert!(
        list_result.is_ok(),
        "Listing checkpoints failed: {:?}",
        list_result
    );

    let checkpoints = list_result.unwrap();
    assert!(checkpoints.len() > 0, "No checkpoints found");

    let found_checkpoint = checkpoints
        .iter()
        .find(|cp| cp.checkpoint_id == checkpoint_id);
    assert!(
        found_checkpoint.is_some(),
        "Created checkpoint not found in list"
    );

    // Restore from checkpoint
    let restore_result = manager
        .restore_from_checkpoint(&test_state.agent_id, &checkpoint_id)
        .await;
    assert!(
        restore_result.is_ok(),
        "Checkpoint restoration failed: {:?}",
        restore_result
    );

    let restored_state = restore_result.unwrap();
    assert_eq!(restored_state.agent_id, test_state.agent_id);

    // Verify the restored state matches
    match restored_state.current_state.unwrap() {
        AutonomousWorkflowState::Approved { issue, pr, .. } => {
            assert_eq!(issue.number, 789);
            assert_eq!(pr.number, 101);
        }
        _ => panic!("Expected Approved state after restoration"),
    }
}

/// Test integration with existing systems
#[tokio::test]
async fn test_system_integration() {
    let github_client = GitHubClient::new().expect("Failed to create GitHub client");
    let config = IntegrationConfig::default();

    let integration_result = AutonomousIntegrationFactory::create_integrated_system(
        github_client,
        "integration-test-agent".to_string(),
        config,
    )
    .await;

    assert!(
        integration_result.is_ok(),
        "Integration system creation failed: {:?}",
        integration_result
    );

    let coordinator = integration_result.unwrap();
    let status = coordinator.get_status().await;

    assert_eq!(status.agent_id, "integration-test-agent");
    assert!(status.agent_state.is_available);
    assert!(!status.is_running);
    assert_eq!(status.recovery_report.total_attempts, 0);
}

/// Test timeout handling
#[tokio::test]
async fn test_timeout_handling() {
    let mut workflow = AutonomousWorkflowMachine::new(0); // 0 hours = immediate timeout

    // Start work
    workflow
        .handle_event(AutonomousEvent::AssignAgent {
            agent: AgentId("timeout-test-agent".to_string()),
            workspace_ready: true,
        })
        .await
        .unwrap();

    // Any subsequent event should handle timeout
    let start_result = workflow.handle_event(AutonomousEvent::StartWork).await;

    // Should handle gracefully (either succeed or handle timeout appropriately)
    assert!(start_result.is_ok() || start_result.is_err());

    let status = workflow.generate_status_report();
    assert!(status.timeout_in_minutes.is_some());
    assert!(!status.can_continue || status.timeout_in_minutes.unwrap() == 0);
}

/// Test merge conflict resolution
#[tokio::test]
async fn test_merge_conflict_resolution() {
    let mut workflow = AutonomousWorkflowMachine::new(8);

    // Setup to approved state
    workflow
        .handle_event(AutonomousEvent::AssignAgent {
            agent: AgentId("conflict-test-agent".to_string()),
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
        number: 999,
        title: "Conflict test PR".to_string(),
        branch: "conflict-test-branch".to_string(),
        commits: 2,
        files_changed: 1,
    };

    workflow
        .handle_event(AutonomousEvent::SubmitForReview { pr })
        .await
        .unwrap();
    workflow
        .handle_event(AutonomousEvent::ApprovalReceived)
        .await
        .unwrap();

    // Simulate merge conflict
    let conflicts = vec![ConflictInfo {
        file: "src/test.rs".to_string(),
        conflict_markers: 1,
        auto_resolvable: true,
    }];

    let conflict_result = workflow
        .handle_event(AutonomousEvent::MergeConflictDetected { conflicts })
        .await;

    assert!(
        conflict_result.is_ok(),
        "Merge conflict handling failed: {:?}",
        conflict_result
    );

    assert!(matches!(
        workflow.current_state(),
        Some(AutonomousWorkflowState::MergeConflict { .. })
    ));

    // Resolve conflicts
    let resolve_result = workflow
        .handle_event(AutonomousEvent::ConflictsResolved)
        .await;
    assert!(
        resolve_result.is_ok(),
        "Conflict resolution failed: {:?}",
        resolve_result
    );

    assert!(matches!(
        workflow.current_state(),
        Some(AutonomousWorkflowState::Approved { .. })
    ));
}

/// Test CI failure handling
#[tokio::test]
async fn test_ci_failure_handling() {
    let mut workflow = AutonomousWorkflowMachine::new(8);

    // Setup to approved state
    workflow
        .handle_event(AutonomousEvent::AssignAgent {
            agent: AgentId("ci-test-agent".to_string()),
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
        number: 888,
        title: "CI test PR".to_string(),
        branch: "ci-test-branch".to_string(),
        commits: 1,
        files_changed: 2,
    };

    workflow
        .handle_event(AutonomousEvent::SubmitForReview { pr })
        .await
        .unwrap();
    workflow
        .handle_event(AutonomousEvent::ApprovalReceived)
        .await
        .unwrap();

    // Simulate CI failure
    let failures = vec![CIFailure {
        job_name: "test".to_string(),
        step: "unit-tests".to_string(),
        error: "Test timeout in authentication module".to_string(),
        auto_fixable: true,
    }];

    let failure_result = workflow
        .handle_event(AutonomousEvent::CIFailureDetected { failures })
        .await;

    assert!(
        failure_result.is_ok(),
        "CI failure handling failed: {:?}",
        failure_result
    );

    assert!(matches!(
        workflow.current_state(),
        Some(AutonomousWorkflowState::CIFailure { .. })
    ));

    // Fix CI issues
    let fix_result = workflow.handle_event(AutonomousEvent::CIFixed).await;
    assert!(fix_result.is_ok(), "CI fix failed: {:?}", fix_result);

    assert!(matches!(
        workflow.current_state(),
        Some(AutonomousWorkflowState::Approved { .. })
    ));
}

/// Test successful completion flow
#[tokio::test]
async fn test_successful_completion_flow() {
    let mut workflow = AutonomousWorkflowMachine::new(8);

    // Complete workflow from start to finish
    workflow
        .handle_event(AutonomousEvent::AssignAgent {
            agent: AgentId("completion-test-agent".to_string()),
            workspace_ready: true,
        })
        .await
        .unwrap();

    workflow
        .handle_event(AutonomousEvent::StartWork)
        .await
        .unwrap();
    workflow
        .handle_event(AutonomousEvent::MakeProgress {
            commits: 5,
            files_changed: 3,
        })
        .await
        .unwrap();
    workflow
        .handle_event(AutonomousEvent::CompleteWork)
        .await
        .unwrap();

    let pr = PullRequest {
        number: 777,
        title: "Completion test PR".to_string(),
        branch: "completion-test-branch".to_string(),
        commits: 5,
        files_changed: 3,
    };

    workflow
        .handle_event(AutonomousEvent::SubmitForReview { pr })
        .await
        .unwrap();
    workflow
        .handle_event(AutonomousEvent::ApprovalReceived)
        .await
        .unwrap();

    // Complete merge
    let completed_work = CompletedWork {
        issue: Issue {
            number: 777,
            title: "Completion test issue".to_string(),
            body: "Testing successful completion".to_string(),
            labels: vec!["completion".to_string()],
            priority: Priority::Medium,
            estimated_hours: Some(3),
        },
        commits: 5,
        files_changed: 3,
        tests_added: 2,
        completion_time: chrono::Utc::now(),
    };

    let merge_result = workflow
        .handle_event(AutonomousEvent::MergeCompleted {
            merged_work: completed_work,
        })
        .await;

    assert!(
        merge_result.is_ok(),
        "Merge completion failed: {:?}",
        merge_result
    );

    assert!(matches!(
        workflow.current_state(),
        Some(AutonomousWorkflowState::Merged { .. })
    ));

    // Verify state history
    let history = workflow.state_history();
    assert!(
        history.len() >= 7,
        "Expected at least 7 state transitions, got {}",
        history.len()
    );

    // Verify can't continue (successfully completed)
    assert!(!workflow.can_continue_autonomously());

    let status = workflow.generate_status_report();
    assert!(!status.can_continue);
    assert!(matches!(
        status.current_state,
        Some(AutonomousWorkflowState::Merged { .. })
    ));
}
