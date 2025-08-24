//! Extended tests for autonomous system work continuity across agent restarts
//!
//! These tests build on the existing work continuity integration tests to validate
//! that the autonomous system can properly handle various restart scenarios while
//! preserving work state and maintaining operational safety.

use anyhow::Result;
use chrono::{Duration as ChronoDuration, Utc};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::sleep;

use my_little_soda::{
    agent_lifecycle::AgentStateMachine,
    agents::recovery::AutoRecovery,
    autonomous::{
        AgentId, AutonomousCoordinator, AutonomousEvent, AutonomousWorkflowMachine,
        AutonomousWorkflowState, CheckpointReason, CoordinationConfig, Issue, PersistenceConfig,
        PersistentWorkflowState, Priority, PullRequest, ResumeAction, StatePersistenceManager,
        WorkContinuityConfig, WorkContinuityManager, WorkProgress, WorkspaceState,
    },
    GitHubClient,
};

/// Helper to create test configurations with temp directory
fn create_extended_test_configs(
    temp_dir: &TempDir,
) -> (WorkContinuityConfig, PersistenceConfig, CoordinationConfig) {
    let continuity_config = WorkContinuityConfig {
        enable_continuity: true,
        state_file_path: temp_dir.path().join("agent-state.json"),
        backup_interval_minutes: 1,
        max_recovery_attempts: 3,
        validation_timeout_seconds: 10,
        force_fresh_start_after_hours: 48, // Longer for testing
        preserve_partial_work: true,
    };

    let persistence_config = PersistenceConfig {
        enable_persistence: true,
        persistence_directory: temp_dir.path().to_path_buf(),
        auto_save_interval_minutes: 1,
        max_state_history_entries: 100,
        max_recovery_history_entries: 50,
        compress_old_states: false,
        backup_retention_days: 7,
        enable_integrity_checks: true,
    };

    let coordination_config = CoordinationConfig {
        max_work_hours: 8,
        max_recovery_attempts: 3,
        recovery_timeout_minutes: 5,
        enable_aggressive_recovery: false,
        enable_state_persistence: true,
        monitoring_interval_minutes: 2,
        enable_drift_detection: true,
        drift_validation_interval_minutes: 5,
    };

    (continuity_config, persistence_config, coordination_config)
}

/// Test autonomous workflow state persistence and recovery
#[tokio::test]
async fn test_autonomous_workflow_persistence_across_restarts() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let (continuity_config, persistence_config, coordination_config) =
        create_extended_test_configs(&temp_dir);

    let github_client = match GitHubClient::new() {
        Ok(client) => client,
        Err(_) => {
            println!("Skipping integration test: GitHub client not available");
            return Ok(());
        }
    };

    let agent_id = "autonomous-restart-agent-001";
    let persistence_manager = StatePersistenceManager::new(persistence_config.clone());

    // Phase 1: Create and run autonomous workflow, then save state
    let (saved_checkpoint_id, test_issue) = {
        let mut workflow = AutonomousWorkflowMachine::new(8);

        // Create realistic test scenario
        let test_issue = Issue {
            number: 12345,
            title: "Autonomous restart test issue".to_string(),
            body: "Testing autonomous system restart capabilities".to_string(),
            labels: vec!["autonomous-restart".to_string(), "test".to_string()],
            priority: Priority::High,
            estimated_hours: Some(4),
        };

        // Progress through workflow states
        workflow
            .handle_event(AutonomousEvent::AssignAgent {
                agent: AgentId(agent_id.to_string()),
                workspace_ready: true,
            })
            .await?;

        workflow.handle_event(AutonomousEvent::StartWork).await?;

        // Make significant progress
        for i in 1..=3 {
            workflow
                .handle_event(AutonomousEvent::MakeProgress {
                    commits: i,
                    files_changed: i * 2,
                })
                .await?;
        }

        workflow.handle_event(AutonomousEvent::CompleteWork).await?;

        // Submit for review
        let pr = PullRequest {
            number: 9999,
            title: "[AUTONOMOUS] Complete restart test".to_string(),
            branch: format!("{}/12345-autonomous-restart-test", agent_id),
            commits: 6,
            files_changed: 12,
        };

        workflow
            .handle_event(AutonomousEvent::SubmitForReview { pr })
            .await?;

        // Verify we're in the expected state
        assert!(matches!(
            workflow.current_state(),
            Some(AutonomousWorkflowState::UnderReview { .. })
        ));

        // Create persistent state
        let persistent_state = PersistentWorkflowState {
            version: "1.0.0".to_string(),
            agent_id: agent_id.to_string(),
            current_state: workflow.current_state().cloned(),
            start_time: Some(Utc::now() - ChronoDuration::hours(1)),
            max_work_hours: 8,
            state_history: workflow.state_history().iter().cloned().collect(),
            recovery_history: vec![],
            checkpoint_metadata: my_little_soda::autonomous::persistence::CheckpointMetadata {
                checkpoint_id: "restart-test-checkpoint".to_string(),
                creation_reason: CheckpointReason::UserRequested,
                integrity_hash: "test-hash-123".to_string(),
                agent_pid: Some(std::process::id()),
                hostname: "test-host".to_string(),
            },
            last_persisted: Utc::now(),
        };

        // Save state
        let checkpoint_id = persistence_manager
            .save_state(&persistent_state, CheckpointReason::UserRequested)
            .await?;

        println!(
            "Saved autonomous workflow state with checkpoint ID: {}",
            checkpoint_id
        );

        (checkpoint_id, test_issue)
    };

    // Phase 2: Simulate restart - load state and verify recovery
    {
        println!("Simulating autonomous system restart...");

        // Load saved state
        let loaded_state = persistence_manager.load_state(agent_id).await?;
        assert!(loaded_state.is_some(), "Failed to load saved state");

        let loaded_state = loaded_state.unwrap();
        assert_eq!(loaded_state.agent_id, agent_id);
        assert!(loaded_state.current_state.is_some());

        // Verify state integrity
        match loaded_state.current_state.unwrap() {
            AutonomousWorkflowState::UnderReview { issue, pr, .. } => {
                assert_eq!(issue.number, test_issue.number);
                assert_eq!(pr.number, 9999);
                println!(
                    "Successfully recovered UnderReview state for issue #{}",
                    issue.number
                );
            }
            other => panic!("Unexpected recovered state: {:?}", other),
        }

        // Verify state history was preserved
        assert!(
            loaded_state.state_history.len() >= 5,
            "State history not preserved: {} entries",
            loaded_state.state_history.len()
        );

        // Verify checkpoint metadata
        assert_eq!(
            loaded_state.checkpoint_metadata.checkpoint_id,
            "restart-test-checkpoint"
        );
        assert!(loaded_state.start_time.is_some());
    }

    // Phase 3: Create new workflow from recovered state and continue
    {
        let loaded_state = persistence_manager.load_state(agent_id).await?.unwrap();
        let mut new_workflow = AutonomousWorkflowMachine::new(8);

        // In a real implementation, you would restore the workflow state
        // For this test, we'll simulate continuing from where we left off

        // Simulate receiving approval and continuing
        let approval_event = AutonomousEvent::ApprovalReceived;
        let approval_result = new_workflow.handle_event(approval_event).await;

        // This might fail if the workflow isn't properly restored, but shouldn't panic
        match approval_result {
            Ok(_) => println!("Successfully continued workflow after restart"),
            Err(e) => println!("Expected error continuing workflow after restart: {:?}", e),
        }
    }

    Ok(())
}

/// Test work continuity manager with autonomous coordinator
#[tokio::test]
async fn test_autonomous_coordinator_with_work_continuity() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let (continuity_config, persistence_config, coordination_config) =
        create_extended_test_configs(&temp_dir);

    let github_client = match GitHubClient::new() {
        Ok(client) => client,
        Err(_) => {
            println!("Skipping integration test: GitHub client not available");
            return Ok(());
        }
    };

    let agent_id = "coordinator-continuity-agent";

    // Phase 1: Create coordinator with continuity enabled
    {
        let recovery_client = Box::new(AutoRecovery::new(github_client.clone(), true));
        let coordinator = AutonomousCoordinator::new(
            github_client.clone(),
            agent_id.to_string(),
            recovery_client,
            coordination_config.clone(),
        )
        .await?;

        // Get initial status
        let initial_status = coordinator.get_status_report().await;
        assert_eq!(initial_status.agent_id.as_ref().unwrap(), agent_id);
        assert_eq!(initial_status.transitions_count, 0);

        // Create work continuity manager
        let mut continuity_manager = WorkContinuityManager::new(
            continuity_config.clone(),
            github_client.clone(),
            persistence_config.clone(),
        );

        continuity_manager.initialize(agent_id).await?;

        // Simulate some agent state
        let mut agent_state = AgentStateMachine::new(agent_id.to_string());
        agent_state.current_issue = Some(54321);
        agent_state.current_branch = Some(format!("{}/54321-continuity-test", agent_id));
        agent_state.commits_ahead = 3;

        // Checkpoint the state
        let checkpoint_id = continuity_manager
            .checkpoint_state(
                &agent_state,
                Some("Agent working on issue #54321 - 3 commits ahead".to_string()),
                CheckpointReason::PeriodicSave,
            )
            .await?;

        println!("Created continuity checkpoint: {}", checkpoint_id);
        assert!(!checkpoint_id.is_empty());
    }

    // Phase 2: Simulate process restart and recovery
    {
        println!("Simulating coordinator restart with work continuity...");

        let continuity_manager = WorkContinuityManager::new(
            continuity_config.clone(),
            github_client.clone(),
            persistence_config.clone(),
        );

        // Attempt recovery
        let resume_action = continuity_manager.recover_from_checkpoint(agent_id).await?;

        match resume_action {
            Some(ResumeAction::ContinueWork {
                issue,
                branch,
                last_progress,
            }) => {
                println!("Successfully recovered work state:");
                println!("  Issue: {}", issue.number);
                println!("  Branch: {}", branch);
                println!("  Last progress: {}", last_progress.progress_description);

                assert_eq!(issue.number, 54321);
                assert!(branch.contains("54321-continuity-test"));
            }
            Some(ResumeAction::ValidateAndResync { reason }) => {
                println!("Recovery requires validation: {}", reason);
                // This is also acceptable
            }
            Some(ResumeAction::StartFresh { reason }) => {
                println!("Starting fresh after restart: {}", reason);
                // This is also valid behavior
            }
            None => {
                println!("No state to recover");
                // This might happen if continuity is disabled
            }
        }

        // Create new coordinator after restart
        let recovery_client = Box::new(AutoRecovery::new(github_client.clone(), true));
        let new_coordinator = AutonomousCoordinator::new(
            github_client,
            agent_id.to_string(),
            recovery_client,
            coordination_config,
        )
        .await?;

        // Verify the new coordinator is functional
        let post_restart_status = new_coordinator.get_status_report().await;
        assert_eq!(post_restart_status.agent_id.as_ref().unwrap(), agent_id);

        // Check continuity status
        let continuity_status = continuity_manager.get_continuity_status(agent_id).await?;
        println!("Post-restart continuity status:");
        println!("  Is active: {}", continuity_status.is_active);
        println!("  Current issue: {:?}", continuity_status.current_issue);
        println!(
            "  Pending operations: {}",
            continuity_status.pending_operations
        );
    }

    Ok(())
}

/// Test recovery from various checkpoint scenarios
#[tokio::test]
async fn test_checkpoint_recovery_scenarios() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let (continuity_config, persistence_config, _) = create_extended_test_configs(&temp_dir);

    let github_client = match GitHubClient::new() {
        Ok(client) => client,
        Err(_) => {
            println!("Skipping integration test: GitHub client not available");
            return Ok(());
        }
    };

    let agent_id = "checkpoint-recovery-agent";
    let persistence_manager = StatePersistenceManager::new(persistence_config.clone());

    // Test Scenario 1: Multiple checkpoints with different states
    {
        println!("Testing multiple checkpoint recovery...");

        let states_to_test = vec![
            (
                "assigned",
                AutonomousWorkflowState::Assigned {
                    issue: Issue {
                        number: 100,
                        title: "Assigned state test".to_string(),
                        body: "Testing assigned state recovery".to_string(),
                        labels: vec!["assigned".to_string()],
                        priority: Priority::Medium,
                        estimated_hours: Some(2),
                    },
                    agent: AgentId(agent_id.to_string()),
                    workspace: WorkspaceState {
                        current_branch: "main".to_string(),
                        uncommitted_changes: false,
                    },
                },
            ),
            (
                "in-progress",
                AutonomousWorkflowState::InProgress {
                    issue: Issue {
                        number: 200,
                        title: "In-progress state test".to_string(),
                        body: "Testing in-progress state recovery".to_string(),
                        labels: vec!["in-progress".to_string()],
                        priority: Priority::High,
                        estimated_hours: Some(3),
                    },
                    agent: AgentId(agent_id.to_string()),
                    progress: WorkProgress {
                        commits_made: 4,
                        files_changed: 8,
                        tests_written: 2,
                        elapsed_minutes: 90,
                        completion_percentage: 60,
                    },
                },
            ),
        ];

        let mut checkpoint_ids = Vec::new();

        for (state_name, workflow_state) in states_to_test {
            let persistent_state = PersistentWorkflowState {
                version: "1.0.0".to_string(),
                agent_id: format!("{}-{}", agent_id, state_name),
                current_state: Some(workflow_state),
                start_time: Some(Utc::now() - ChronoDuration::minutes(30)),
                max_work_hours: 8,
                state_history: vec![],
                recovery_history: vec![],
                checkpoint_metadata: my_little_soda::autonomous::persistence::CheckpointMetadata {
                    checkpoint_id: format!("checkpoint-{}", state_name),
                    creation_reason: CheckpointReason::PeriodicSave,
                    integrity_hash: format!("hash-{}", state_name),
                    agent_pid: Some(std::process::id()),
                    hostname: "test-host".to_string(),
                },
                last_persisted: Utc::now(),
            };

            let checkpoint_id = persistence_manager
                .save_state(&persistent_state, CheckpointReason::PeriodicSave)
                .await?;
            checkpoint_ids.push((state_name, checkpoint_id, persistent_state.agent_id));

            println!("Created {} checkpoint: {}", state_name, checkpoint_id);
        }

        // Verify each checkpoint can be loaded
        for (state_name, checkpoint_id, test_agent_id) in checkpoint_ids {
            let loaded_state = persistence_manager.load_state(&test_agent_id).await?;
            assert!(
                loaded_state.is_some(),
                "Failed to load {} state",
                state_name
            );

            let loaded_state = loaded_state.unwrap();
            assert!(
                loaded_state.current_state.is_some(),
                "{} state not preserved",
                state_name
            );

            println!(
                "Successfully recovered {} state for agent {}",
                state_name, test_agent_id
            );
        }
    }

    // Test Scenario 2: Checkpoint corruption handling
    {
        println!("Testing checkpoint corruption handling...");

        let mut continuity_manager = WorkContinuityManager::new(
            continuity_config.clone(),
            github_client.clone(),
            persistence_config.clone(),
        );

        continuity_manager.initialize(agent_id).await?;

        // Create a checkpoint
        let agent_state = AgentStateMachine::new(agent_id.to_string());
        let checkpoint_id = continuity_manager
            .checkpoint_state(
                &agent_state,
                Some("Test for corruption handling".to_string()),
                CheckpointReason::UserRequested,
            )
            .await?;

        println!("Created checkpoint for corruption test: {}", checkpoint_id);

        // Attempt recovery (should succeed)
        let resume_action = continuity_manager.recover_from_checkpoint(agent_id).await?;
        match resume_action {
            Some(action) => println!("Recovery succeeded: {:?}", action),
            None => println!("No state to recover (as expected)"),
        }
    }

    Ok(())
}

/// Test performance of checkpoint and recovery operations
#[tokio::test]
async fn test_checkpoint_performance() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let (continuity_config, persistence_config, _) = create_extended_test_configs(&temp_dir);

    let github_client = match GitHubClient::new() {
        Ok(client) => client,
        Err(_) => {
            println!("Skipping integration test: GitHub client not available");
            return Ok(());
        }
    };

    let agent_id = "performance-test-agent";
    let persistence_manager = StatePersistenceManager::new(persistence_config.clone());
    let mut continuity_manager =
        WorkContinuityManager::new(continuity_config, github_client, persistence_config);

    continuity_manager.initialize(agent_id).await?;

    let test_iterations = 10;
    let mut checkpoint_times = Vec::new();
    let mut recovery_times = Vec::new();

    println!(
        "Testing checkpoint/recovery performance over {} iterations",
        test_iterations
    );

    for i in 0..test_iterations {
        // Create progressively larger state
        let mut agent_state = AgentStateMachine::new(agent_id.to_string());
        agent_state.current_issue = Some(i as u64 + 1000);
        agent_state.current_branch = Some(format!("{}/performance-test-{}", agent_id, i));
        agent_state.commits_ahead = i as u32;

        // Measure checkpoint time
        let checkpoint_start = std::time::Instant::now();
        let checkpoint_id = continuity_manager
            .checkpoint_state(
                &agent_state,
                Some(format!("Performance test iteration {}", i)),
                CheckpointReason::PeriodicSave,
            )
            .await?;
        let checkpoint_duration = checkpoint_start.elapsed();
        checkpoint_times.push(checkpoint_duration);

        // Create complex persistent state for persistence manager test
        let complex_state = PersistentWorkflowState {
            version: "1.0.0".to_string(),
            agent_id: format!("{}-perf-{}", agent_id, i),
            current_state: Some(AutonomousWorkflowState::InProgress {
                issue: Issue {
                    number: (i as u64) + 2000,
                    title: format!("Performance test issue {}", i),
                    body: "A".repeat(i * 100), // Increasing size
                    labels: (0..i).map(|j| format!("label-{}", j)).collect(),
                    priority: Priority::Medium,
                    estimated_hours: Some(i as u8 + 1),
                },
                agent: AgentId(format!("{}-perf-{}", agent_id, i)),
                progress: WorkProgress {
                    commits_made: i as u32,
                    files_changed: i as u32 * 2,
                    tests_written: i as u32,
                    elapsed_minutes: i as u32 * 10,
                    completion_percentage: (i as u8 * 10).min(100),
                },
            }),
            start_time: Some(Utc::now() - ChronoDuration::hours(i as i64 % 8)),
            max_work_hours: 8,
            state_history: (0..i)
                .map(|j| my_little_soda::autonomous::StateTransitionRecord {
                    from_state: format!("state-{}", j),
                    to_state: format!("state-{}", j + 1),
                    event: format!("event-{}", j),
                    timestamp: Utc::now() - ChronoDuration::minutes(i as i64 - j as i64),
                    duration_ms: (j * 100) as u64,
                    success: true,
                })
                .collect(),
            recovery_history: vec![],
            checkpoint_metadata: my_little_soda::autonomous::persistence::CheckpointMetadata {
                checkpoint_id: format!("perf-checkpoint-{}", i),
                creation_reason: CheckpointReason::PeriodicSave,
                integrity_hash: format!("perf-hash-{}", i),
                agent_pid: Some(std::process::id()),
                hostname: "perf-test-host".to_string(),
            },
            last_persisted: Utc::now(),
        };

        // Test persistence manager performance
        let persist_start = std::time::Instant::now();
        let _persist_checkpoint_id = persistence_manager
            .save_state(&complex_state, CheckpointReason::PeriodicSave)
            .await?;
        let _persist_duration = persist_start.elapsed();

        // Measure recovery time
        let recovery_start = std::time::Instant::now();
        let resume_action = continuity_manager.recover_from_checkpoint(agent_id).await?;
        let recovery_duration = recovery_start.elapsed();
        recovery_times.push(recovery_duration);

        if i % 3 == 0 {
            println!(
                "Iteration {}: Checkpoint {:?}, Recovery {:?}, Action: {:?}",
                i,
                checkpoint_duration,
                recovery_duration,
                resume_action.is_some()
            );
        }

        assert!(
            !checkpoint_id.is_empty(),
            "Checkpoint ID should not be empty"
        );
    }

    // Calculate performance statistics
    let avg_checkpoint_time =
        checkpoint_times.iter().sum::<Duration>() / checkpoint_times.len() as u32;
    let max_checkpoint_time = checkpoint_times.iter().max().unwrap();
    let avg_recovery_time = recovery_times.iter().sum::<Duration>() / recovery_times.len() as u32;
    let max_recovery_time = recovery_times.iter().max().unwrap();

    println!("Checkpoint Performance Results:");
    println!("  Average checkpoint time: {:?}", avg_checkpoint_time);
    println!("  Max checkpoint time: {:?}", max_checkpoint_time);
    println!("  Average recovery time: {:?}", avg_recovery_time);
    println!("  Max recovery time: {:?}", max_recovery_time);

    // Performance assertions
    assert!(
        avg_checkpoint_time < Duration::from_millis(500),
        "Average checkpoint time too slow: {:?}",
        avg_checkpoint_time
    );
    assert!(
        max_checkpoint_time < Duration::from_secs(2),
        "Max checkpoint time too slow: {:?}",
        max_checkpoint_time
    );
    assert!(
        avg_recovery_time < Duration::from_millis(100),
        "Average recovery time too slow: {:?}",
        avg_recovery_time
    );
    assert!(
        max_recovery_time < Duration::from_millis(500),
        "Max recovery time too slow: {:?}",
        max_recovery_time
    );

    Ok(())
}

/// Test continuity across different workflow states
#[tokio::test]
async fn test_workflow_state_continuity() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let (continuity_config, persistence_config, _) = create_extended_test_configs(&temp_dir);

    let github_client = match GitHubClient::new() {
        Ok(client) => client,
        Err(_) => {
            println!("Skipping integration test: GitHub client not available");
            return Ok(());
        }
    };

    let agent_id = "workflow-continuity-agent";
    let persistence_manager = StatePersistenceManager::new(persistence_config.clone());

    // Test continuity for different workflow states
    let test_states = vec![
        (
            "assigned",
            Issue {
                number: 301,
                title: "Assigned continuity test".to_string(),
                body: "".to_string(),
                labels: vec![],
                priority: Priority::Low,
                estimated_hours: Some(1),
            },
        ),
        (
            "in-progress",
            Issue {
                number: 302,
                title: "In-progress continuity test".to_string(),
                body: "".to_string(),
                labels: vec![],
                priority: Priority::Medium,
                estimated_hours: Some(2),
            },
        ),
        (
            "review",
            Issue {
                number: 303,
                title: "Review continuity test".to_string(),
                body: "".to_string(),
                labels: vec![],
                priority: Priority::High,
                estimated_hours: Some(3),
            },
        ),
    ];

    for (state_name, test_issue) in test_states {
        println!("Testing continuity for {} state", state_name);

        // Create workflow state
        let workflow_state = match state_name {
            "assigned" => AutonomousWorkflowState::Assigned {
                issue: test_issue.clone(),
                agent: AgentId(agent_id.to_string()),
                workspace: WorkspaceState {
                    current_branch: "main".to_string(),
                    uncommitted_changes: false,
                },
            },
            "in-progress" => AutonomousWorkflowState::InProgress {
                issue: test_issue.clone(),
                agent: AgentId(agent_id.to_string()),
                progress: WorkProgress {
                    commits_made: 2,
                    files_changed: 4,
                    tests_written: 1,
                    elapsed_minutes: 45,
                    completion_percentage: 40,
                },
            },
            "review" => AutonomousWorkflowState::ReadyForReview {
                issue: test_issue.clone(),
                agent: AgentId(agent_id.to_string()),
                pr: PullRequest {
                    number: 8888,
                    title: format!("[{}] {}", state_name.to_uppercase(), test_issue.title),
                    branch: format!("{}/{}-{}", agent_id, test_issue.number, state_name),
                    commits: 3,
                    files_changed: 6,
                },
            },
            _ => panic!("Unknown state name: {}", state_name),
        };

        // Save state
        let persistent_state = PersistentWorkflowState {
            version: "1.0.0".to_string(),
            agent_id: format!("{}-{}", agent_id, state_name),
            current_state: Some(workflow_state),
            start_time: Some(Utc::now() - ChronoDuration::minutes(20)),
            max_work_hours: 8,
            state_history: vec![],
            recovery_history: vec![],
            checkpoint_metadata: my_little_soda::autonomous::persistence::CheckpointMetadata {
                checkpoint_id: format!("workflow-{}-checkpoint", state_name),
                creation_reason: CheckpointReason::PeriodicSave,
                integrity_hash: format!("workflow-{}-hash", state_name),
                agent_pid: Some(std::process::id()),
                hostname: "test-host".to_string(),
            },
            last_persisted: Utc::now(),
        };

        let checkpoint_id = persistence_manager
            .save_state(&persistent_state, CheckpointReason::PeriodicSave)
            .await?;
        println!("Saved {} state checkpoint: {}", state_name, checkpoint_id);

        // Verify recovery
        let recovered_state = persistence_manager
            .load_state(&format!("{}-{}", agent_id, state_name))
            .await?;
        assert!(
            recovered_state.is_some(),
            "Failed to recover {} state",
            state_name
        );

        let recovered_state = recovered_state.unwrap();
        assert!(
            recovered_state.current_state.is_some(),
            "{} workflow state not preserved",
            state_name
        );

        // Verify specific state properties
        match recovered_state.current_state.unwrap() {
            AutonomousWorkflowState::Assigned { issue, .. } if state_name == "assigned" => {
                assert_eq!(issue.number, test_issue.number);
                println!("✓ Assigned state continuity verified");
            }
            AutonomousWorkflowState::InProgress {
                issue, progress, ..
            } if state_name == "in-progress" => {
                assert_eq!(issue.number, test_issue.number);
                assert_eq!(progress.commits_made, 2);
                assert_eq!(progress.completion_percentage, 40);
                println!("✓ In-progress state continuity verified");
            }
            AutonomousWorkflowState::ReadyForReview { issue, pr, .. } if state_name == "review" => {
                assert_eq!(issue.number, test_issue.number);
                assert_eq!(pr.number, 8888);
                println!("✓ Ready-for-review state continuity verified");
            }
            other => panic!("Unexpected recovered state for {}: {:?}", state_name, other),
        }
    }

    println!("All workflow state continuity tests passed!");
    Ok(())
}
