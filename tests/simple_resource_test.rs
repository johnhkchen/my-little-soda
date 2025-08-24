//! Simple Resource Management Test
//!
//! Basic smoke test to verify the resource management system compiles and runs

use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;

use my_little_soda::agents::{
    AlertThresholds, LoggingEventHandler, ProcessLifecycleConfig, ProcessLifecycleManager,
    ProcessManagerConfig, ResourceLimits,
};

fn create_test_config() -> ProcessLifecycleConfig {
    ProcessLifecycleConfig {
        process_config: ProcessManagerConfig {
            claude_code_path: "echo".to_string(),
            max_concurrent_agents: 2,
            monitoring_interval_secs: 1,
            cleanup_interval_secs: 2,
            default_limits: ResourceLimits {
                max_memory_mb: 100,
                max_cpu_percent: 25.0,
                timeout_minutes: 1,
                max_file_descriptors: 100,
            },
            enable_resource_monitoring: true,
            enable_automatic_cleanup: true,
        },
        alert_thresholds: AlertThresholds::default(),
        enable_auto_cleanup: true,
        enable_resource_alerts: true,
        cleanup_failed_after_minutes: 1,
        cleanup_completed_after_minutes: 1,
        alert_check_interval_secs: 1,
    }
}

#[tokio::test]
async fn test_basic_lifecycle_manager() -> Result<()> {
    let config = create_test_config();
    let mut lifecycle_manager = ProcessLifecycleManager::new(config).await?;
    let event_handler = Arc::new(LoggingEventHandler);

    // Start the lifecycle manager
    lifecycle_manager.start(event_handler.clone()).await?;

    // Get system status (should be empty initially)
    let status = lifecycle_manager.get_system_status();
    assert_eq!(
        status.active_process_count, 0,
        "Should have no active processes initially"
    );

    Ok(())
}

#[tokio::test]
async fn test_resource_limits() -> Result<()> {
    let config = create_test_config();
    let lifecycle_manager = ProcessLifecycleManager::new(config).await?;
    let event_handler = Arc::new(LoggingEventHandler);

    // Test that we can spawn with custom resource limits
    let custom_limits = ResourceLimits {
        max_memory_mb: 256,
        max_cpu_percent: 75.0,
        timeout_minutes: 5,
        max_file_descriptors: 512,
    };

    // This should succeed
    let process_id = lifecycle_manager
        .spawn_agent(
            "test_agent",
            123,
            "test_branch".to_string(),
            Some(custom_limits),
            event_handler.clone(),
        )
        .await?;

    // Verify process ID format
    assert!(process_id.contains("test_agent"));
    assert!(process_id.contains("123"));

    Ok(())
}

#[tokio::test]
async fn test_system_status() -> Result<()> {
    let config = create_test_config();
    let mut lifecycle_manager = ProcessLifecycleManager::new(config).await?;
    let event_handler = Arc::new(LoggingEventHandler);

    lifecycle_manager.start(event_handler.clone()).await?;

    // Check initial status
    let initial_status = lifecycle_manager.get_system_status();
    assert_eq!(initial_status.active_process_count, 0);

    // Spawn an agent
    let _process_id = lifecycle_manager
        .spawn_agent(
            "status_test",
            456,
            "status_branch".to_string(),
            None,
            event_handler.clone(),
        )
        .await?;

    // Check status again
    let status = lifecycle_manager.get_system_status();
    assert_eq!(status.active_process_count, 1);
    assert_eq!(status.resource_summary.active_process_count, 1);

    Ok(())
}

#[tokio::test]
async fn test_concurrent_agent_limit() -> Result<()> {
    let config = create_test_config(); // max_concurrent_agents = 2
    let mut lifecycle_manager = ProcessLifecycleManager::new(config).await?;
    let event_handler = Arc::new(LoggingEventHandler);

    lifecycle_manager.start(event_handler.clone()).await?;

    // Should be able to spawn up to the limit
    for i in 0..2 {
        let _process_id = lifecycle_manager
            .spawn_agent(
                &format!("agent{}", i),
                100 + i as u64,
                format!("branch{}", i),
                None,
                event_handler.clone(),
            )
            .await?;
    }

    // Third spawn should fail
    let result = lifecycle_manager
        .spawn_agent(
            "agent_overflow",
            999,
            "overflow_branch".to_string(),
            None,
            event_handler.clone(),
        )
        .await;

    assert!(result.is_err());

    Ok(())
}
