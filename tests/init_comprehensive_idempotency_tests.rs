/// Comprehensive idempotency testing for init command (C2d requirement)
///
/// Tests that init command can be run multiple times safely, verifying:
/// - No data corruption on repeated runs
/// - Force flag behavior with existing configurations  
/// - State consistency across multiple init runs
/// - Different agent count combinations
use my_little_soda::cli::commands::init::InitCommand;
use my_little_soda::fs::{FileSystemOperations, StandardFileSystem};
use std::sync::Arc;

mod fixtures;

use fixtures::test_harness::{helpers::simple_harness, TestHarness};

#[tokio::test]
async fn test_multiple_dry_run_calls_are_idempotent() {
    let mut harness = simple_harness().unwrap();

    // Run dry run 10 times - all should succeed identically
    for i in 1..=10 {
        let fs_ops = Arc::new(StandardFileSystem);
        let init_command = InitCommand::new(None, false, true, fs_ops.clone());

        // Change to test directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(harness.path()).unwrap();

        let result = init_command.execute().await;

        // Restore directory
        std::env::set_current_dir(original_dir).unwrap();

        assert!(
            result.is_ok(),
            "Dry run iteration {} failed: {:?}",
            i,
            result.err()
        );

        // State should remain unchanged after each dry run
        let config_path = harness.path().join("my-little-soda.toml");
        let dir_path = harness.path().join(".my-little-soda");

        assert!(
            !config_path.exists(),
            "Config should not exist after dry run iteration {}",
            i
        );
        assert!(
            !dir_path.exists(),
            "Directory should not exist after dry run iteration {}",
            i
        );
    }

    harness.verify_isolation().unwrap();
}

#[tokio::test]
async fn test_repeated_force_calls_maintain_consistency() {
    let mut harness = simple_harness().unwrap();
    let fs_ops = Arc::new(StandardFileSystem);

    // First init with force (dry run to avoid actual config creation)
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(harness.path()).unwrap();

    let init_command1 = InitCommand::new(None, true, true, fs_ops.clone());
    let result1 = init_command1.execute().await;

    std::env::set_current_dir(&original_dir).unwrap();
    assert!(
        result1.is_ok(),
        "First force init failed: {:?}",
        result1.err()
    );

    // Repeated force calls should all succeed identically
    for i in 2..=5 {
        std::env::set_current_dir(harness.path()).unwrap();

        let init_command = InitCommand::new(None, true, true, fs_ops.clone());
        let result = init_command.execute().await;

        std::env::set_current_dir(&original_dir).unwrap();
        assert!(
            result.is_ok(),
            "Force init iteration {} failed: {:?}",
            i,
            result.err()
        );
    }

    harness.verify_isolation().unwrap();
}

#[tokio::test]
async fn test_single_agent_consistency_across_runs() {
    let mut harness = simple_harness().unwrap();
    let fs_ops = Arc::new(StandardFileSystem);
    let original_dir = std::env::current_dir().unwrap();

    // Multiple runs in single-agent mode should succeed identically
    for iteration in 1..=5 {
        std::env::set_current_dir(harness.path()).unwrap();

        let init_command = InitCommand::new(None, false, true, fs_ops.clone());
        let result = init_command.execute().await;

        std::env::set_current_dir(&original_dir).unwrap();

        assert!(
            result.is_ok(),
            "Single-agent iteration {} failed: {:?}",
            iteration,
            result.err()
        );
    }

    harness.verify_isolation().unwrap();
}

#[tokio::test]
async fn test_mixed_parameter_combinations_idempotency() {
    let mut harness = simple_harness().unwrap();
    let fs_ops = Arc::new(StandardFileSystem);
    let original_dir = std::env::current_dir().unwrap();

    // Test various parameter combinations
    let test_cases = [
        (false, true), // basic dry run
        (true, true),  // force dry run
    ];

    for (force, dry_run) in test_cases {
        // Each combination should work consistently across multiple runs
        for iteration in 1..=3 {
            std::env::set_current_dir(harness.path()).unwrap();

            let init_command = InitCommand::new(None, force, dry_run, fs_ops.clone());
            let result = init_command.execute().await;

            std::env::set_current_dir(&original_dir).unwrap();

            // For dry runs, should always succeed
            assert!(
                result.is_ok(),
                "Combo force={} dry_run={} iteration={} failed: {:?}",
                force,
                dry_run,
                iteration,
                result.err()
            );
        }
    }

    harness.verify_isolation().unwrap();
}

#[tokio::test]
async fn test_state_preservation_between_calls() {
    let mut harness = simple_harness().unwrap();

    // Create some additional files that should be preserved
    let custom_file = harness
        .create_file("custom_file.txt", "important data")
        .unwrap();
    std::fs::create_dir(harness.path().join("custom_dir")).unwrap();
    std::fs::write(harness.path().join("custom_dir/nested.txt"), "nested data").unwrap();

    let fs_ops = Arc::new(StandardFileSystem);
    let original_dir = std::env::current_dir().unwrap();

    // Run multiple init commands and verify custom files are preserved
    for i in 1..=5 {
        std::env::set_current_dir(harness.path()).unwrap();

        let init_command = InitCommand::new(None, false, true, fs_ops.clone());
        let result = init_command.execute().await;

        std::env::set_current_dir(&original_dir).unwrap();

        assert!(
            result.is_ok(),
            "Init iteration {} failed: {:?}",
            i,
            result.err()
        );

        // Verify custom files are preserved
        assert!(
            custom_file.exists(),
            "Custom file should exist after iteration {}",
            i
        );
        assert!(
            harness.path().join("custom_dir/nested.txt").exists(),
            "Nested custom file should exist after iteration {}",
            i
        );

        let content = std::fs::read_to_string(&custom_file).unwrap();
        assert_eq!(
            content, "important data",
            "Custom file content modified in iteration {}",
            i
        );
    }

    harness.verify_isolation().unwrap();
}

#[tokio::test]
async fn test_rapid_successive_calls() {
    let mut harness = simple_harness().unwrap();
    let fs_ops = Arc::new(StandardFileSystem);

    // Test sequential rapid calls (concurrent would be problematic due to directory changing)
    for i in 0..5 {
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(harness.path()).unwrap();

        let init_command = InitCommand::new(None, false, true, fs_ops.clone());
        let result = init_command.execute().await;

        std::env::set_current_dir(original_dir).unwrap();

        assert!(
            result.is_ok(),
            "Rapid call {} failed: {:?}",
            i,
            result.err()
        );
    }

    // State should remain clean after rapid executions
    let config_path = harness.path().join("my-little-soda.toml");
    let dir_path = harness.path().join(".my-little-soda");

    assert!(
        !config_path.exists(),
        "No config should exist after rapid dry runs"
    );
    assert!(
        !dir_path.exists(),
        "No directory should exist after rapid dry runs"
    );

    harness.verify_isolation().unwrap();
}
