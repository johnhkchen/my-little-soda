/// Updated init command tests using the new repository state fixtures
/// 
/// This module demonstrates how to migrate existing init command tests to use the comprehensive
/// fixture system, providing better coverage and more realistic test scenarios.

use crate::cli::commands::init::InitCommand;
use crate::fs::MockFileSystemOperations;
use crate::tests::fixtures::init_integration::{
    InitCommandTestEnvironment, InitCommandBatchTester, TestScenario, assertions
};
use crate::tests::fixtures::repository_states::{RepositoryStateFixture, RepositoryFixtureLoader};
use crate::tests::fixtures::validation_helpers::{RepositoryStateValidator, InitCommandValidator};
use mockall::predicate::*;
use std::process::{Output, ExitStatus};
use std::sync::Arc;

// Helper functions for mock setup
fn create_successful_exit_status() -> ExitStatus {
    std::process::Command::new("true").status().unwrap()
}

fn create_failed_exit_status() -> ExitStatus {
    std::process::Command::new("false").status().unwrap()
}

/// Test init command against empty repository using fixtures
/// This replaces the old manual test setup with fixture-based testing
#[tokio::test]
async fn test_init_on_empty_repository_with_fixture() {
    let env = InitCommandTestEnvironment::from_fixture_name("empty_repository")
        .expect("Failed to create test environment");
    
    // Verify fixture is correctly set up
    assert!(env.has_file("README.md"), "Empty repository should have README");
    assert!(!env.has_file("clambake.toml"), "Should not have config initially");
    
    // Run init command (dry run for test safety)
    let result = env.run_and_validate_init(1, false, true).await
        .expect("Failed to run init command");
    
    // Validate using fixture expectations
    assertions::assert_result_matches_expectation(&result, "empty_repository");
    assertions::assert_init_succeeds_for_clean_repo(&result, "empty_repository");
    
    // Validate post-init state
    let post_init = env.validate_post_init_state(true).await
        .expect("Failed to validate post-init state");
    assertions::assert_post_init_validation_passes(&post_init, "empty_repository");
}

/// Test init command behavior with existing configuration using fixtures
/// This demonstrates testing conflict scenarios that weren't easily testable before
#[tokio::test]
async fn test_init_with_existing_config_using_fixtures() {
    let env = InitCommandTestEnvironment::from_fixture_name("repository_with_partial_initialization")
        .expect("Failed to create test environment");
    
    // Verify existing configuration
    assert!(env.has_file("clambake.toml"), "Should have existing config");
    let existing_config = env.read_file("clambake.toml")
        .expect("Failed to read existing config");
    assert!(existing_config.contains("old-owner"), "Should contain old owner");
    
    // Test without force - should fail
    let result_no_force = env.run_and_validate_init(1, false, true).await
        .expect("Failed to run init without force");
    assertions::assert_init_fails_without_force(&result_no_force, "repository_with_partial_initialization");
    
    // Test with force - should succeed
    let result_with_force = env.run_and_validate_init(1, true, true).await
        .expect("Failed to run init with force");
    assertions::assert_init_succeeds_with_force(&result_with_force, "repository_with_partial_initialization");
}

/// Test init command against repository with conflicts
/// This tests error handling scenarios that are difficult to set up manually
#[tokio::test]
async fn test_init_with_repository_conflicts_using_fixtures() {
    let env = InitCommandTestEnvironment::from_fixture_name("repository_with_conflicts")
        .expect("Failed to create test environment");
    
    // Verify conflict markers are present
    let main_rs = env.read_file("src/main.rs")
        .expect("Failed to read main.rs");
    assert!(main_rs.contains("<<<<<<< HEAD"), "Should have conflict markers");
    assert!(main_rs.contains(">>>>>>> feature-branch"), "Should have conflict markers");
    
    // Init should fail due to uncommitted changes
    let result = env.run_and_validate_init(1, false, true).await
        .expect("Failed to run init command");
    
    assertions::assert_result_matches_expectation(&result, "repository_with_conflicts");
    assertions::assert_init_fails_without_force(&result, "repository_with_conflicts");
}

/// Comprehensive test across all fixture types using batch testing
/// This replaces multiple individual tests with comprehensive coverage
#[tokio::test]
async fn test_init_dry_run_comprehensive_fixture_coverage() {
    let results = InitCommandBatchTester::test_all_fixtures(1, false, true).await
        .expect("Batch testing failed");
    
    // All fixtures should handle dry run appropriately
    assertions::assert_all_fixtures_pass(&results, "dry run comprehensive test");
    
    // Validate each result individually
    for result in &results.results {
        assert!(result.passed(), "Fixture {} should pass dry run test: {}", 
                result.fixture_name, 
                result.failure_summary().unwrap_or_default());
    }
    
    // Should test all available fixtures
    let expected_fixture_count = RepositoryFixtureLoader::load_all_fixtures().len();
    assert_eq!(results.results.len(), expected_fixture_count,
               "Should test all available fixtures");
}

/// Test force flag behavior across all fixtures
/// Demonstrates comprehensive scenario testing
#[tokio::test]
async fn test_force_flag_across_all_fixtures() {
    let force_scenario = TestScenario::force_init();
    let results = InitCommandBatchTester::test_scenario_across_fixtures(force_scenario).await
        .expect("Force scenario testing failed");
    
    // Force flag should make all fixtures succeed
    assert!(results.all_passed(), "All fixtures should pass with force flag");
    assert_eq!(results.failed_count(), 0, "No fixtures should fail with force flag");
    
    // Validate cross-fixture consistency
    let cross_validation_results: Vec<_> = results.results.iter()
        .map(|r| (r.fixture_name.clone(), r.command_result.clone(), r.post_init_validation.clone()))
        .collect();
    
    let cross_report = InitCommandValidator::validate_cross_fixture_consistency(
        &cross_validation_results, "force"
    ).expect("Cross-fixture validation failed");
    
    assert!(cross_report.passed(), "Cross-fixture validation should pass for force scenario");
    assert_eq!(cross_report.success_rate(), 1.0, "All fixtures should succeed with force");
}

/// Test multi-agent configuration using fixtures
/// This replaces the basic agent count validation with realistic repository testing
#[tokio::test]
async fn test_multi_agent_setup_with_realistic_repositories() {
    // Test with a repository that has existing files (more realistic scenario)
    let env = InitCommandTestEnvironment::from_fixture_name("repository_with_existing_files")
        .expect("Failed to create test environment");
    
    let agent_counts = [1, 2, 4, 8, 12];
    
    for &agent_count in &agent_counts {
        let result = env.run_and_validate_init(agent_count, false, true).await
            .expect(&format!("Failed to run init with {} agents", agent_count));
        
        assertions::assert_result_matches_expectation(&result, "repository_with_existing_files");
        assert!(result.success, "Init should succeed with {} agents on existing project", agent_count);
    }
}

/// Test error boundary conditions using fixture validation
/// This demonstrates integration with the validation helpers
#[tokio::test]
async fn test_error_conditions_with_fixture_validation() {
    // First validate that our fixtures are consistent
    let fixtures = RepositoryFixtureLoader::load_all_fixtures();
    
    for fixture in &fixtures {
        let validation_report = RepositoryStateValidator::validate_fixture_consistency(fixture)
            .expect("Fixture validation failed");
        
        assert!(validation_report.passed, 
                "Fixture '{}' should pass validation: {}", 
                fixture.name, validation_report.summary());
    }
    
    // Test error conditions specifically
    let problematic_fixtures = ["repository_with_partial_initialization", "repository_with_conflicts"];
    
    for fixture_name in &problematic_fixtures {
        let env = InitCommandTestEnvironment::from_fixture_name(fixture_name)
            .expect("Failed to create test environment");
        
        let result = env.run_and_validate_init(1, false, true).await
            .expect("Failed to run init command");
        let post_init = env.validate_post_init_state(true).await
            .expect("Failed to validate post-init state");
        
        // Use comprehensive validation
        let comprehensive_report = InitCommandValidator::validate_comprehensive_result(
            &result, &post_init, &env.fixture, "error_condition_test"
        ).expect("Comprehensive validation failed");
        
        assert!(comprehensive_report.passed(), 
                "Comprehensive validation should pass for fixture '{}'", fixture_name);
    }
}

/// Integration test showing migration from old test pattern to fixture-based pattern
#[tokio::test]
async fn test_migration_example_old_vs_new_pattern() {
    // OLD PATTERN - commented out but shown for comparison:
    /*
    let mut mock_fs = MockFileSystemOperations::new();
    mock_fs
        .expect_exists()
        .with(eq("clambake.toml"))
        .return_const(false);
    
    let fs_ops = Arc::new(mock_fs);
    let init_command = InitCommand::new(1, None, false, true, fs_ops);
    let result = init_command.execute().await;
    assert!(result.is_ok(), "Init command should succeed in clean repository");
    */
    
    // NEW PATTERN using fixtures:
    let env = InitCommandTestEnvironment::from_fixture_name("empty_repository")
        .expect("Failed to create test environment");
    
    let result = env.run_and_validate_init(1, false, true).await
        .expect("Failed to run init command");
    
    // More sophisticated validation than simple success/failure
    assertions::assert_result_matches_expectation(&result, "empty_repository");
    
    let post_init = env.validate_post_init_state(true).await
        .expect("Failed to validate post-init state");
    assertions::assert_post_init_validation_passes(&post_init, "empty_repository");
    
    // The fixture-based approach provides:
    // - Realistic repository states
    // - Expectation-based validation instead of hardcoded assumptions
    // - Comprehensive post-init state checking
    // - Better error messages and debugging information
}

/// Demonstrate testing edge cases that are now easily accessible with fixtures
#[tokio::test]
async fn test_edge_cases_now_accessible_with_fixtures() {
    // Test 1: Repository with substantial existing codebase
    let env_with_code = InitCommandTestEnvironment::from_fixture_name("repository_with_existing_files")
        .expect("Failed to create environment");
    
    // Verify it has realistic project structure
    assert!(env_with_code.has_file("Cargo.toml"), "Should have Cargo.toml");
    assert!(env_with_code.has_file("src/main.rs"), "Should have main.rs");
    assert!(env_with_code.has_file("src/lib.rs"), "Should have lib.rs with tests");
    
    let result = env_with_code.run_and_validate_init(1, false, true).await
        .expect("Failed to run init on existing codebase");
    assertions::assert_init_succeeds_for_clean_repo(&result, "repository_with_existing_files");
    
    // Test 2: Repository with merge conflicts (previously very difficult to test)
    let env_with_conflicts = InitCommandTestEnvironment::from_fixture_name("repository_with_conflicts")
        .expect("Failed to create conflict environment");
    
    let main_content = env_with_conflicts.read_file("src/main.rs")
        .expect("Failed to read conflicted file");
    assert!(main_content.contains("Version from main branch"), "Should have main branch content");
    assert!(main_content.contains("Version from feature branch"), "Should have feature branch content");
    
    let result = env_with_conflicts.run_and_validate_init(1, false, true).await
        .expect("Failed to run init on conflicted repository");
    
    // Should fail due to uncommitted changes
    assert!(!result.success, "Init should fail on repository with conflicts");
    assertions::assert_result_matches_expectation(&result, "repository_with_conflicts");
}

/// Stress test showing comprehensive scenarios
#[tokio::test]
async fn test_comprehensive_init_scenarios() {
    let scenarios = [
        TestScenario::normal_init(),
        TestScenario::force_init(),
        TestScenario::dry_run(),
        TestScenario::multi_agent(),
    ];
    
    let mut all_passed = true;
    let mut scenario_results = Vec::new();
    
    for scenario in scenarios {
        println!("Testing scenario: {}", scenario.description);
        
        let results = InitCommandBatchTester::test_scenario_across_fixtures(scenario.clone()).await
            .expect(&format!("Scenario '{}' failed", scenario.description));
        
        println!("  Results: {}/{} fixtures passed", results.passed_count(), results.results.len());
        
        if !results.all_passed() {
            all_passed = false;
            println!("  Failures:");
            for (fixture_name, failure_summary) in results.failure_summaries() {
                println!("    {}: {}", fixture_name, failure_summary);
            }
        }
        
        scenario_results.push((scenario, results));
    }
    
    assert!(all_passed, "All scenarios should pass across all fixtures");
    
    // Demonstrate summary reporting
    println!("\nComprehensive Test Summary:");
    for (scenario, results) in scenario_results {
        println!("  {}: {}/{} fixtures passed", 
                 scenario.description, 
                 results.passed_count(), 
                 results.results.len());
    }
}

/// Performance comparison test (optional, for demonstration)
#[tokio::test]
async fn test_fixture_system_performance_characteristics() {
    use std::time::Instant;
    
    // Measure fixture loading time
    let start = Instant::now();
    let fixtures = RepositoryFixtureLoader::load_all_fixtures();
    let loading_time = start.elapsed();
    println!("Fixture loading time: {:?} for {} fixtures", loading_time, fixtures.len());
    
    // Measure temporary repository creation time
    let start = Instant::now();
    for fixture in &fixtures {
        let _temp_repo = fixture.create_temp_repository()
            .expect(&format!("Failed to create temp repo for {}", fixture.name));
    }
    let creation_time = start.elapsed();
    println!("Temp repository creation time: {:?} for {} repositories", creation_time, fixtures.len());
    
    // Measure batch testing time
    let start = Instant::now();
    let _results = InitCommandBatchTester::test_all_fixtures(1, false, true).await
        .expect("Batch testing failed");
    let batch_testing_time = start.elapsed();
    println!("Batch testing time: {:?} for all fixtures", batch_testing_time);
    
    // Performance should be reasonable (these are loose bounds for demonstration)
    assert!(loading_time.as_millis() < 1000, "Fixture loading should be fast");
    assert!(creation_time.as_millis() < 5000, "Temp repo creation should be reasonable");
    assert!(batch_testing_time.as_secs() < 30, "Batch testing should complete in reasonable time");
}

/// Test demonstrating fixture extensibility
#[tokio::test]
async fn test_fixture_extensibility() {
    // Create a custom fixture for a specific test case
    let custom_fixture = RepositoryStateFixture {
        name: "custom_test_scenario".to_string(),
        description: "Custom fixture for specific test case".to_string(),
        files: std::collections::HashMap::from([
            ("README.md".to_string(), "# Custom Test Project\n".to_string()),
            ("custom_config.toml".to_string(), "custom = true\n".to_string()),
            (".gitignore".to_string(), "target/\n*.log\ncustom/\n".to_string()),
        ]),
        git_config: crate::tests::fixtures::repository_states::GitConfig::default(),
        existing_clambake_config: None,
    };
    
    // Use the custom fixture
    let env = InitCommandTestEnvironment::from_fixture(custom_fixture)
        .expect("Failed to create environment from custom fixture");
    
    assert!(env.has_file("custom_config.toml"), "Should have custom config file");
    
    let result = env.run_and_validate_init(1, false, true).await
        .expect("Failed to run init on custom fixture");
    
    // Custom fixture should behave like a clean repository
    assert!(result.success, "Custom fixture should allow successful init");
}

/// Integration test demonstrating complete workflow
#[tokio::test]
async fn test_complete_fixture_integration_workflow() {
    println!("🧪 Running complete fixture integration workflow test...");
    
    // Step 1: Validate all fixtures are consistent
    println!("📋 Step 1: Validating fixture consistency...");
    let fixtures = RepositoryFixtureLoader::load_all_fixtures();
    for fixture in &fixtures {
        let validation = RepositoryStateValidator::validate_fixture_consistency(fixture)
            .expect("Fixture validation failed");
        assert!(validation.passed, "Fixture '{}' validation: {}", fixture.name, validation.summary());
    }
    println!("✅ All {} fixtures are consistent", fixtures.len());
    
    // Step 2: Test fixture instantiation
    println!("🏗️  Step 2: Testing fixture instantiation...");
    for fixture in &fixtures {
        let instantiation = RepositoryStateValidator::validate_fixture_instantiation(fixture).await
            .expect("Instantiation validation failed");
        assert!(instantiation.successful(), "Fixture '{}' should instantiate successfully", fixture.name);
    }
    println!("✅ All fixtures can be instantiated");
    
    // Step 3: Test init command integration
    println!("⚙️  Step 3: Testing init command integration...");
    let batch_results = InitCommandBatchTester::test_all_fixtures(1, false, true).await
        .expect("Batch init testing failed");
    assertions::assert_all_fixtures_pass(&batch_results, "integration workflow test");
    println!("✅ Init command integrates with all fixtures");
    
    // Step 4: Test scenario variations
    println!("🎭 Step 4: Testing scenario variations...");
    let scenarios = [
        TestScenario::normal_init(),
        TestScenario::force_init(),
        TestScenario::dry_run(),
    ];
    
    for scenario in scenarios {
        let results = InitCommandBatchTester::test_scenario_across_fixtures(scenario.clone()).await
            .expect(&format!("Scenario {} failed", scenario.description));
        assertions::assert_all_fixtures_pass(&results, &scenario.description);
        println!("✅ Scenario '{}' passed for all fixtures", scenario.description);
    }
    
    println!("🎉 Complete fixture integration workflow test PASSED!");
}