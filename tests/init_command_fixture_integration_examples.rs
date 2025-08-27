/// Example test cases demonstrating how to use repository state fixtures with init command testing
/// 
/// These examples showcase the integration utilities and provide templates for common testing scenarios.
/// Use these patterns as a starting point for comprehensive init command test coverage.

use crate::tests::fixtures::init_integration::{
    InitCommandTestEnvironment, InitCommandBatchTester, TestScenario, assertions
};
use crate::tests::fixtures::repository_states::{RepositoryStateFixture, RepositoryFixtureLoader};

/// Example 1: Basic fixture usage - test init on empty repository
#[tokio::test]
async fn example_init_on_empty_repository() {
    // Create test environment using fixture
    let env = InitCommandTestEnvironment::from_fixture_name("empty_repository")
        .expect("Failed to create test environment");
    
    // Verify initial state  
    assert!(env.has_file("README.md"), "Empty repository should have README.md");
    assert!(!env.has_file("my-little-soda.toml"), "Should not have clambake config initially");
    assert!(!env.has_file(".my-little-soda"), "Should not have .my-little-soda directory initially");
    
    // Run init command (dry run for safety)
    let result = env.run_and_validate_init(1, false, true).await
        .expect("Failed to run init command");
    
    // Validate result matches fixture expectations
    assertions::assert_result_matches_expectation(&result, "empty_repository");
    
    // For empty repository, init should succeed without force
    assertions::assert_init_succeeds_for_clean_repo(&result, "empty_repository");
    
    println!("✅ Example 1 passed: Init succeeds on empty repository");
}

/// Example 2: Testing force flag behavior across different repository states
#[tokio::test]
async fn example_force_flag_behavior() {
    let test_fixtures = [
        "empty_repository",
        "repository_with_existing_files", 
        "repository_with_partial_initialization",
        "repository_with_conflicts"
    ];
    
    for fixture_name in &test_fixtures {
        let env = InitCommandTestEnvironment::from_fixture_name(fixture_name)
            .expect("Failed to create test environment");
        
        // Test without force flag - behavior depends on repository state
        let result_no_force = env.run_and_validate_init(1, false, true).await
            .expect("Failed to run init without force");
        
        // Test with force flag - should always succeed
        let result_with_force = env.run_and_validate_init(1, true, true).await
            .expect("Failed to run init with force");
        
        // Validate behavior matches fixture expectations
        assertions::assert_result_matches_expectation(&result_no_force, fixture_name);
        assertions::assert_init_succeeds_with_force(&result_with_force, fixture_name);
        
        println!("✅ Force flag test passed for fixture: {}", fixture_name);
    }
    
    println!("✅ Example 2 passed: Force flag works correctly across all fixtures");
}

/// Example 3: Comprehensive batch testing across all fixtures
#[tokio::test]
async fn example_batch_testing_dry_run() {
    // Test dry run scenario across all available fixtures
    let scenario = TestScenario::dry_run();
    let results = InitCommandBatchTester::test_scenario_across_fixtures(scenario).await
        .expect("Batch testing failed");
    
    // All fixtures should handle dry run without issues
    assertions::assert_all_fixtures_pass(&results, "dry run mode");
    
    // Print detailed results
    println!("Dry run test results:");
    for result in &results.results {
        println!("  {} ({}): {}", 
                 result.fixture_name,
                 result.fixture_description,
                 if result.passed() { "PASSED" } else { "FAILED" });
        
        if let Some(failure) = result.failure_summary() {
            println!("    Failure: {}", failure);
        }
    }
    
    println!("✅ Example 3 passed: Batch testing with {} fixtures", results.results.len());
}

/// Example 4: Multi-agent configuration testing
#[tokio::test]
async fn example_multi_agent_configuration() {
    let env = InitCommandTestEnvironment::from_fixture_name("empty_repository")
        .expect("Failed to create test environment");
    
    // Test different agent counts
    let agent_counts = [1, 2, 4, 8, 12];
    
    for agent_count in &agent_counts {
        let result = env.run_and_validate_init(*agent_count, false, true).await
            .expect("Failed to run init with multiple agents");
        
        assert!(result.success, "Init should succeed with {} agents", agent_count);
        
        println!("✅ Multi-agent test passed for {} agents", agent_count);
    }
    
    println!("✅ Example 4 passed: Multi-agent configuration works correctly");
}

/// Example 5: Post-init state validation
#[tokio::test]
async fn example_post_init_state_validation() {
    let env = InitCommandTestEnvironment::from_fixture_name("repository_with_existing_files")
        .expect("Failed to create test environment");
    
    // Verify initial state
    assert!(env.has_file("Cargo.toml"), "Should have existing Cargo.toml");
    assert!(env.has_file("src/main.rs"), "Should have existing main.rs");
    assert!(!env.has_file("my-little-soda.toml"), "Should not have clambake config initially");
    
    // Run actual init command (not dry run)
    let command_result = env.run_and_validate_init(1, false, false).await
        .expect("Failed to run init command");
    
    // Validate post-init state
    let post_init_validation = env.validate_post_init_state(false)
        .expect("Failed to validate post-init state");
    
    assertions::assert_result_matches_expectation(&command_result, "repository_with_existing_files");
    assertions::assert_post_init_validation_passes(&post_init_validation, "repository_with_existing_files");
    
    // Verify existing files are preserved
    assert!(env.has_file("Cargo.toml"), "Existing Cargo.toml should be preserved");
    assert!(env.has_file("src/main.rs"), "Existing main.rs should be preserved");
    
    // Verify new files were created
    assert!(env.has_file("my-little-soda.toml"), "my-little-soda.toml should be created");
    assert!(env.has_file(".my-little-soda"), ".my-little-soda directory should be created");
    
    println!("✅ Example 5 passed: Post-init validation works correctly");
}

/// Example 6: Error handling and problematic repository states
#[tokio::test]
async fn example_error_handling() {
    // Test repositories that should fail without force flag
    let problematic_fixtures = [
        "repository_with_partial_initialization",
        "repository_with_conflicts"
    ];
    
    for fixture_name in &problematic_fixtures {
        let env = InitCommandTestEnvironment::from_fixture_name(fixture_name)
            .expect("Failed to create test environment");
        
        // Should fail without force
        let result_no_force = env.run_and_validate_init(1, false, true).await
            .expect("Failed to run init without force");
        
        assertions::assert_init_fails_without_force(&result_no_force, fixture_name);
        
        // Should succeed with force
        let result_with_force = env.run_and_validate_init(1, true, true).await
            .expect("Failed to run init with force");
        
        assertions::assert_init_succeeds_with_force(&result_with_force, fixture_name);
        
        println!("✅ Error handling test passed for problematic fixture: {}", fixture_name);
    }
    
    println!("✅ Example 6 passed: Error handling works correctly");
}

/// Example 7: Custom fixture usage and validation
#[tokio::test]
async fn example_custom_fixture_validation() {
    // Load fixture directly and inspect its properties
    let fixture = RepositoryStateFixture::repository_with_partial_initialization();
    
    // Validate fixture properties
    assert_eq!(fixture.name, "repository_with_partial_initialization");
    assert!(fixture.existing_clambake_config.is_some(), "Should have existing config");
    assert!(fixture.files.contains_key("my-little-soda.toml"), "Should contain my-little-soda.toml file");
    
    let expected_behavior = fixture.expected_init_behavior();
    assert!(!expected_behavior.should_succeed_without_force, "Should require force flag");
    assert!(!expected_behavior.validation_warnings.is_empty(), "Should have validation warnings");
    
    // Create environment from fixture instance
    let env = InitCommandTestEnvironment::from_fixture(fixture)
        .expect("Failed to create environment from fixture");
    
    // Verify the existing config content
    let config_content = env.read_file("my-little-soda.toml")
        .expect("Failed to read existing config");
    assert!(config_content.contains("old-owner"), "Should contain old owner in config");
    assert!(config_content.contains("tracing_enabled = false"), "Should have existing settings");
    
    println!("✅ Example 7 passed: Custom fixture validation works correctly");
}

/// Example 8: Comprehensive scenario testing
#[tokio::test]
async fn example_comprehensive_scenario_testing() {
    let scenarios = [
        TestScenario::normal_init(),
        TestScenario::force_init(), 
        TestScenario::dry_run(),
        TestScenario::multi_agent(),
    ];
    
    let mut all_results = Vec::new();
    
    for scenario in scenarios {
        println!("Testing scenario: {}", scenario.description);
        
        let results = InitCommandBatchTester::test_scenario_across_fixtures(scenario.clone()).await
            .expect("Scenario testing failed");
        
        println!("  Results: {}/{} fixtures passed", 
                 results.passed_count(), 
                 results.results.len());
        
        if !results.all_passed() {
            println!("  Failures:");
            for (fixture_name, failure_summary) in results.failure_summaries() {
                println!("    {}: {}", fixture_name, failure_summary);
            }
        }
        
        assertions::assert_all_fixtures_pass(&results, &scenario.description);
        all_results.push((scenario.description.clone(), results));
    }
    
    println!("✅ Example 8 passed: All {} scenarios completed successfully", all_results.len());
}

/// Example 9: Integration with existing test patterns
#[tokio::test]  
async fn example_migration_from_manual_setup() {
    // This demonstrates how to migrate from manual test setup to fixture-based testing
    
    // OLD PATTERN (manual setup):
    // let temp_dir = tempfile::tempdir().unwrap();
    // std::env::set_current_dir(temp_dir.path()).unwrap();
    // std::fs::write(temp_dir.path().join("README.md"), "# Test").unwrap();
    // let init_command = InitCommand::new(1, None, false, true, fs_ops);
    // let result = init_command.execute().await;
    // assert!(result.is_ok()); // Hardcoded expectation
    
    // NEW PATTERN (fixture-based):
    let env = InitCommandTestEnvironment::from_fixture_name("empty_repository")
        .expect("Failed to create test environment");
    
    let result = env.run_and_validate_init(1, false, true).await
        .expect("Failed to run init command");
    
    // Use expectation-based assertions instead of hardcoded assumptions
    assertions::assert_result_matches_expectation(&result, "empty_repository");
    
    println!("✅ Example 9 passed: Migration pattern demonstrated");
}

/// Example 10: Advanced validation and custom checks  
#[tokio::test]
async fn example_advanced_validation() {
    let env = InitCommandTestEnvironment::from_fixture_name("repository_with_existing_files")
        .expect("Failed to create test environment");
    
    // Pre-init custom validation
    let cargo_content = env.read_file("Cargo.toml")
        .expect("Failed to read Cargo.toml");
    assert!(cargo_content.contains("existing-project"), "Should have existing project name");
    
    // Run init
    let result = env.run_and_validate_init(2, false, false).await
        .expect("Failed to run init command");
    
    // Standard validation
    assertions::assert_result_matches_expectation(&result, "repository_with_existing_files");
    
    // Custom post-init validation
    if env.has_file("my-little-soda.toml") {
        let clambake_content = env.read_file("my-little-soda.toml")
            .expect("Failed to read clambake config");
        
        // Verify agent count was set correctly
        assert!(clambake_content.contains("max_agents = 2"), 
                "Config should reflect requested agent count");
        
        // Verify GitHub repo detection worked
        assert!(clambake_content.contains("[github]"),
                "Config should have GitHub section");
    }
    
    // Verify Rust project structure is preserved
    assert!(env.has_file("src/main.rs"), "Original main.rs should be preserved");
    assert!(env.has_file("src/lib.rs"), "Original lib.rs should be preserved");
    
    // Custom directory structure check
    let clambake_agents_dir = env.path().join(".my-little-soda/agents");
    assert!(clambake_agents_dir.exists(), "Agent working directory should be created");
    
    println!("✅ Example 10 passed: Advanced validation completed");
}

// Helper function to demonstrate programmatic fixture inspection
fn demonstrate_fixture_inspection() {
    println!("\n🔍 Fixture Inspection Demo:");
    
    let all_fixtures = RepositoryFixtureLoader::load_all_fixtures();
    println!("Available fixtures: {}", all_fixtures.len());
    
    for fixture in all_fixtures {
        let behavior = fixture.expected_init_behavior();
        println!("  📋 {} ({})", fixture.name, fixture.description);
        println!("    - Files: {}", fixture.files.len());  
        println!("    - Succeeds without force: {}", behavior.should_succeed_without_force);
        println!("    - Should create config: {}", behavior.should_create_config);
        println!("    - Warnings: {}", behavior.validation_warnings.len());
        
        if let Some(config) = fixture.existing_clambake_config {
            println!("    - Has existing config ({} chars)", config.len());
        }
        println!();
    }
}

// Run fixture inspection demo if this file is executed directly
#[tokio::test]
async fn run_fixture_inspection_demo() {
    demonstrate_fixture_inspection();
    println!("✅ Fixture inspection demo completed");
}

/// Integration test that exercises the complete fixture system
#[tokio::test]
async fn comprehensive_fixture_system_integration_test() {
    println!("🧪 Running comprehensive fixture system integration test...");
    
    // 1. Verify all fixtures are loadable
    let all_fixtures = RepositoryFixtureLoader::load_all_fixtures();
    assert!(!all_fixtures.is_empty(), "Should have fixtures available");
    println!("✅ Loaded {} fixtures", all_fixtures.len());
    
    // 2. Verify each fixture can create temporary repositories
    for fixture in &all_fixtures {
        let temp_repo = fixture.create_temp_repository()
            .expect(&format!("Failed to create temp repo for {}", fixture.name));
        assert!(temp_repo.path().exists(), "Temp repository should exist");
        println!("✅ Created temp repo for: {}", fixture.name);
    }
    
    // 3. Test fixture loading by name
    for fixture in &all_fixtures {
        let loaded = RepositoryFixtureLoader::load_fixture(&fixture.name);
        assert!(loaded.is_some(), "Should be able to load fixture by name: {}", fixture.name);
    }
    println!("✅ All fixtures loadable by name");
    
    // 4. Test init command integration with each fixture (dry run for safety)
    let mut successful_integrations = 0;
    for fixture in &all_fixtures {
        if let Ok(env) = InitCommandTestEnvironment::from_fixture(fixture.clone()) {
            if let Ok(result) = env.run_and_validate_init(1, false, true).await {
                if result.matches_expectation() {
                    successful_integrations += 1;
                }
            }
        }
    }
    
    assert_eq!(successful_integrations, all_fixtures.len(), 
               "All fixtures should integrate successfully with init command");
    println!("✅ All {} fixtures integrated successfully with init command", successful_integrations);
    
    // 5. Test batch processing
    let batch_results = InitCommandBatchTester::test_all_fixtures(1, false, true).await
        .expect("Batch testing should work");
    
    assert_eq!(batch_results.results.len(), all_fixtures.len(),
               "Batch processing should handle all fixtures");
    println!("✅ Batch processing handled {} fixtures", batch_results.results.len());
    
    println!("🎉 Comprehensive fixture system integration test PASSED!");
}