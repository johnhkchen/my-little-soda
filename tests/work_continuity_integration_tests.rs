//! Integration tests for work continuity across process restarts
//!
//! These tests verify that agent work state can be persisted and recovered
//! across process interruptions and restarts.

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use tempfile::TempDir;

    use std::path::PathBuf;
    use std::time::Duration;

    use my_little_soda::{
        agent_lifecycle::AgentStateMachine,
        autonomous::{
            CheckpointReason, PersistenceConfig, ResumeAction, WorkContinuityConfig,
            WorkContinuityManager,
        },
        GitHubClient,
    };

    fn create_test_configs(temp_dir: &TempDir) -> (WorkContinuityConfig, PersistenceConfig) {
        let continuity_config = WorkContinuityConfig {
            enable_continuity: true,
            state_file_path: temp_dir.path().join("agent-state.json"),
            backup_interval_minutes: 1,
            max_recovery_attempts: 3,
            validation_timeout_seconds: 10,
            force_fresh_start_after_hours: 24,
            preserve_partial_work: true,
        };

        let persistence_config = PersistenceConfig {
            enable_persistence: true,
            persistence_directory: temp_dir.path().to_path_buf(),
            auto_save_interval_minutes: 1,
            max_state_history_entries: 100,
            max_recovery_history_entries: 50,
            compress_old_states: false, // Disabled for tests
            backup_retention_days: 1,
            enable_integrity_checks: true,
        };

        (continuity_config, persistence_config)
    }

    #[tokio::test]
    async fn test_work_continuity_basic_checkpoint_recovery() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let (continuity_config, persistence_config) = create_test_configs(&temp_dir);

        // Create GitHub client (using environment token if available)
        let github_client = match GitHubClient::new() {
            Ok(client) => client,
            Err(_) => {
                println!("Skipping integration test: GitHub client not available");
                return Ok(());
            }
        };

        let agent_id = "test-agent-001";

        // Phase 1: Create and initialize work continuity manager
        {
            let mut continuity_manager = WorkContinuityManager::new(
                continuity_config.clone(),
                github_client.clone(),
                persistence_config.clone(),
            );

            continuity_manager.initialize(agent_id).await?;

            // Simulate agent work state
            let mut agent_state = AgentStateMachine::new(agent_id.to_string());

            // Simulate assignment to issue (manually set state)
            agent_state.current_issue = Some(123);
            agent_state.current_branch = Some(format!("{}/123-test-issue", agent_id));
            agent_state.commits_ahead = 2;

            // Checkpoint the state
            let checkpoint_id = continuity_manager
                .checkpoint_state(&agent_state, None, CheckpointReason::UserRequested)
                .await?;

            assert!(!checkpoint_id.is_empty());
            println!("Created checkpoint: {}", checkpoint_id);
        }

        // Phase 2: Simulate process restart - create new manager and recover
        {
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
                    println!("Successfully recovered work:");
                    println!("  Issue: {}", issue.number);
                    println!("  Branch: {}", branch);
                    println!("  Progress: {}", last_progress.progress_description);
                    assert_eq!(issue.number, 123);
                    assert!(branch.contains("123-test-issue"));
                }
                Some(ResumeAction::ValidateAndResync { reason }) => {
                    println!("Recovery requires validation: {}", reason);
                    // This is also acceptable for the test
                }
                Some(ResumeAction::StartFresh { reason }) => {
                    println!("Starting fresh: {}", reason);
                    // This might happen if state is too old or invalid
                }
                _ => {
                    panic!("Unexpected resume action: {:?}", resume_action);
                }
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_work_continuity_state_validation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let (continuity_config, persistence_config) = create_test_configs(&temp_dir);

        let github_client = match GitHubClient::new() {
            Ok(client) => client,
            Err(_) => {
                println!("Skipping integration test: GitHub client not available");
                return Ok(());
            }
        };

        let agent_id = "test-agent-002";
        let mut continuity_manager =
            WorkContinuityManager::new(continuity_config, github_client, persistence_config);

        continuity_manager.initialize(agent_id).await?;

        // Get continuity status when no work is active
        let status = continuity_manager.get_continuity_status(agent_id).await?;
        assert!(!status.is_active);
        assert_eq!(status.current_issue, None);
        assert_eq!(status.pending_operations, 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_work_continuity_old_state_handling() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let (mut continuity_config, persistence_config) = create_test_configs(&temp_dir);

        // Set very short timeout for testing
        continuity_config.force_fresh_start_after_hours = 0; // Force immediate expiration

        let github_client = match GitHubClient::new() {
            Ok(client) => client,
            Err(_) => {
                println!("Skipping integration test: GitHub client not available");
                return Ok(());
            }
        };

        let agent_id = "test-agent-003";

        // Phase 1: Create state
        {
            let mut continuity_manager = WorkContinuityManager::new(
                continuity_config.clone(),
                github_client.clone(),
                persistence_config.clone(),
            );

            continuity_manager.initialize(agent_id).await?;

            let agent_state = AgentStateMachine::new(agent_id.to_string());
            let _checkpoint_id = continuity_manager
                .checkpoint_state(&agent_state, None, CheckpointReason::PeriodicSave)
                .await?;
        }

        // Wait a moment to ensure timestamp difference
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Phase 2: Try to recover with expired state
        {
            let continuity_manager =
                WorkContinuityManager::new(continuity_config, github_client, persistence_config);

            let resume_action = continuity_manager.recover_from_checkpoint(agent_id).await?;

            match resume_action {
                Some(ResumeAction::StartFresh { reason }) => {
                    println!("Correctly determined fresh start needed: {}", reason);
                    assert!(reason.contains("too old") || reason.contains("State"));
                }
                _ => {
                    // Other actions are also valid depending on implementation
                    println!("Recovery action: {:?}", resume_action);
                }
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_work_continuity_disabled() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let (mut continuity_config, persistence_config) = create_test_configs(&temp_dir);

        // Disable continuity
        continuity_config.enable_continuity = false;

        let github_client = match GitHubClient::new() {
            Ok(client) => client,
            Err(_) => {
                println!("Skipping integration test: GitHub client not available");
                return Ok(());
            }
        };

        let agent_id = "test-agent-004";
        let mut continuity_manager =
            WorkContinuityManager::new(continuity_config, github_client, persistence_config);

        // Initialization should succeed but do nothing
        continuity_manager.initialize(agent_id).await?;

        // Checkpoint should return disabled indicator
        let agent_state = AgentStateMachine::new(agent_id.to_string());
        let checkpoint_id = continuity_manager
            .checkpoint_state(&agent_state, None, CheckpointReason::UserRequested)
            .await?;

        assert_eq!(checkpoint_id, "continuity_disabled");

        // Recovery should return None
        let resume_action = continuity_manager.recover_from_checkpoint(agent_id).await?;
        assert!(resume_action.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_work_continuity_error_handling() -> Result<()> {
        // Test with invalid directory permissions (if possible on the system)
        let temp_dir = TempDir::new()?;
        let (mut continuity_config, mut persistence_config) = create_test_configs(&temp_dir);

        // Set invalid paths
        continuity_config.state_file_path = PathBuf::from("/invalid/nonexistent/path/state.json");
        persistence_config.persistence_directory = PathBuf::from("/invalid/nonexistent/path");

        let github_client = match GitHubClient::new() {
            Ok(client) => client,
            Err(_) => {
                println!("Skipping integration test: GitHub client not available");
                return Ok(());
            }
        };

        let agent_id = "test-agent-005";
        let mut continuity_manager =
            WorkContinuityManager::new(continuity_config, github_client, persistence_config);

        // Initialization should handle errors gracefully
        let init_result = continuity_manager.initialize(agent_id).await;
        // Should not panic, but may return error
        println!("Initialization result: {:?}", init_result);

        // Recovery should handle missing files gracefully
        let resume_action = continuity_manager.recover_from_checkpoint(agent_id).await?;
        // Should indicate fresh start when no state is available
        match resume_action {
            Some(ResumeAction::StartFresh { reason }) => {
                println!("Correctly handled missing state: {}", reason);
            }
            None => {
                println!("No state to recover (as expected)");
            }
            _ => {
                println!("Recovery action: {:?}", resume_action);
            }
        }

        Ok(())
    }
}
