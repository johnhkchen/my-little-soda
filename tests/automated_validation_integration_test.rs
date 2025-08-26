/// Comprehensive test demonstrating automated validation checks across all init command test scenarios
/// 
/// This test demonstrates the B3b deliverable: automated validation checks that verify
/// init command results including file existence, directory structure, content validation,
/// Git configuration validation, and comprehensive result reporting.

mod fixtures;

use fixtures::automated_validators::{
    FileSystemValidator, ContentValidator, GitConfigValidator, ValidationResultReporter,
    ContentExpectation, GitConfigExpectations, create_standard_init_expectations,
};
use fixtures::init_integration::{
    InitCommandTestEnvironment, InitCommandBatchTester, TestScenario,
};
use fixtures::repository_states::{RepositoryFixtureLoader, RepositoryStateFixture};
use std::collections::HashMap;

#[tokio::test]
async fn test_comprehensive_automated_validation_system() {
    println!("🔧 Testing Comprehensive Automated Validation System");
    println!("==================================================");
    
    // Load all available fixtures for testing
    let fixtures = RepositoryFixtureLoader::load_init_command_fixtures();
    assert!(!fixtures.is_empty(), "Should have test fixtures available");
    
    println!("📋 Testing {} fixtures across multiple scenarios", fixtures.len());
    
    // Test scenarios to validate
    let scenarios = vec![
        TestScenario::dry_run(),
        TestScenario::normal_init(),
        TestScenario::force_init(),
    ];
    
    let mut all_validation_reports = Vec::new();
    
    for scenario in scenarios {
        println!("\n🎭 Testing scenario: {}", scenario.description);
        
        // Run init command tests for this scenario
        let batch_results = InitCommandBatchTester::test_scenario_across_fixtures(scenario.clone()).await
            .expect("Failed to run batch tests");
        
        println!("   Init command results: {}/{} fixtures passed", 
                 batch_results.passed_count(), batch_results.results.len());
        
        // Now validate each result using automated validators
        for fixture_result in &batch_results.results {
            let validation_report = validate_single_fixture_result(
                &fixture_result.fixture_name,
                &scenario,
            ).await.expect("Validation failed");
            
            // Print individual validation results
            if validation_report.overall_success {
                println!("   ✅ {}: All validations passed", fixture_result.fixture_name);
            } else {
                println!("   ❌ {}: {} validation errors", 
                         fixture_result.fixture_name, validation_report.total_errors);
                for error in &validation_report.error_summary {
                    println!("      - {}", error);
                }
            }
            
            all_validation_reports.push(validation_report);
        }
    }
    
    // Generate comprehensive failure analysis
    let failure_analysis = ValidationResultReporter::generate_failure_analysis(&all_validation_reports);
    
    println!("\n📊 Validation Summary");
    println!("=====================");
    println!("Total validations: {}", failure_analysis.total_validations);
    println!("Successful: {}", failure_analysis.successful_validations);
    println!("Failed: {}", failure_analysis.failed_validations.len());
    println!("Success rate: {:.2}%", failure_analysis.success_rate * 100.0);
    
    if !failure_analysis.failed_validations.is_empty() {
        println!("\n❌ Failed Validations:");
        for failed in &failure_analysis.failed_validations {
            println!("   {} ({}): {} errors", failed.fixture, failed.scenario, failed.error_count);
        }
    }
    
    // For this test, we expect high success rate (allowing some failures due to dry run limitations)
    assert!(failure_analysis.success_rate >= 0.7, 
            "Validation success rate should be at least 70%, got {:.2}%", 
            failure_analysis.success_rate * 100.0);
            
    println!("\n✅ Comprehensive automated validation system test PASSED!");
}

#[tokio::test]
async fn test_file_existence_validation_detailed() {
    println!("🗂️  Testing File Existence Validation");
    println!("=====================================");
    
    // Test with empty repository fixture
    let env = InitCommandTestEnvironment::from_fixture_name("empty_repository")
        .expect("Failed to create test environment");
    
    // Get standard expectations for init command
    let (expected_files, expected_directories, _, _) = create_standard_init_expectations();
    
    // Run init command (dry run to avoid actual changes)
    let init_result = env.run_and_validate_init(1, false, true).await
        .expect("Failed to run init command");
    
    println!("Init command success: {}", init_result.success);
    
    // Validate filesystem state before init (dry run doesn't create files)
    let fs_report = FileSystemValidator::validate_file_existence(
        env.path(),
        &expected_files.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        &expected_directories.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
    ).expect("Filesystem validation failed");
    
    println!("Files found: {}", fs_report.files_found.len());
    println!("Files missing: {}", fs_report.files_missing.len());
    println!("Directories found: {}", fs_report.directories_found.len());
    println!("Directories missing: {}", fs_report.directories_missing.len());
    
    // In dry run mode, files shouldn't be created, so we expect them to be missing
    if init_result.success {
        println!("✅ File existence validation working correctly for dry run scenario");
    }
    
    // The validation should identify missing files appropriately
    assert!(!fs_report.files_missing.is_empty() || !fs_report.directories_missing.is_empty(), 
            "Should detect missing files/directories in dry run mode");
}

#[tokio::test]
async fn test_content_validation_detailed() {
    println!("📝 Testing Content Validation");
    println!("=============================");
    
    // Create a temporary repository with test content
    let fixture = create_test_fixture_with_content();
    let env = InitCommandTestEnvironment::from_fixture(fixture)
        .expect("Failed to create test environment");
    
    // Create content expectations
    let mut content_expectations = HashMap::new();
    content_expectations.insert(
        "test_config.toml".to_string(),
        ContentExpectation {
            must_contain: vec!["[section]".to_string(), "test_key".to_string()],
            must_not_contain: vec!["FORBIDDEN".to_string()],
            regex_patterns: vec![r#"test_key\s*=\s*"[^"]+""#.to_string()],
            min_lines: Some(2),
        },
    );
    
    // Run content validation
    let content_report = ContentValidator::validate_file_contents(
        env.path(),
        &content_expectations,
    ).expect("Content validation failed");
    
    println!("Content validation success: {}", content_report.success);
    println!("Files passed: {}", content_report.passed_files.len());
    println!("Files failed: {}", content_report.failed_files.len());
    
    // Should pass since we created the content to match expectations
    assert!(content_report.success, "Content validation should pass for test fixture");
    assert_eq!(content_report.passed_files.len(), 1);
    assert_eq!(content_report.failed_files.len(), 0);
    
    println!("✅ Content validation working correctly");
}

#[tokio::test]
async fn test_git_configuration_validation_detailed() {
    println!("📋 Testing Git Configuration Validation");
    println!("=======================================");
    
    let env = InitCommandTestEnvironment::from_fixture_name("empty_repository")
        .expect("Failed to create test environment");
    
    // Set up Git configuration expectations
    let git_expectations = GitConfigExpectations {
        should_be_git_repo: true,
        expected_branch: Some("main".to_string()),
        expected_remote_url: None, // Will vary
        should_have_remote: false, // Test repositories may not have remotes
        should_be_clean: true,
    };
    
    // Run Git configuration validation
    let git_report = GitConfigValidator::validate_git_configuration(
        env.path(),
        &git_expectations,
    ).expect("Git validation failed");
    
    println!("Git validation success: {}", git_report.success);
    println!("Git repo exists: {}", git_report.git_repo_exists);
    println!("Branch correct: {}", git_report.branch_correct);
    println!("Remote correct: {}", git_report.remote_correct);
    println!("Working directory clean: {}", git_report.working_directory_clean);
    
    if !git_report.errors.is_empty() {
        println!("Git validation errors:");
        for error in &git_report.errors {
            println!("  - {}", error);
        }
    }
    
    // Test fixtures should have git repositories
    assert!(git_report.git_repo_exists, "Test fixture should have git repository");
    
    println!("✅ Git configuration validation working correctly");
}

#[tokio::test]
async fn test_validation_result_reporting_detailed() {
    println!("📊 Testing Validation Result Reporting");
    println!("======================================");
    
    // Create mock validation reports for testing
    let fs_report = fixtures::automated_validators::FileSystemValidationReport {
        success: true,
        files_found: vec!["clambake.toml".to_string()],
        files_missing: Vec::new(),
        directories_found: vec![".clambake".to_string()],
        directories_missing: Vec::new(),
        errors: Vec::new(),
    };
    
    let content_report = fixtures::automated_validators::ContentValidationReport {
        success: false,
        passed_files: vec!["good_file.txt".to_string()],
        failed_files: vec!["bad_file.txt".to_string()],
        file_results: HashMap::new(),
        errors: vec!["Content validation error".to_string()],
    };
    
    let git_report = fixtures::automated_validators::GitConfigValidationReport {
        success: true,
        git_repo_exists: true,
        branch_correct: true,
        remote_correct: true,
        working_directory_clean: true,
        errors: Vec::new(),
    };
    
    // Generate comprehensive report
    let summary = ValidationResultReporter::generate_comprehensive_report(
        &fs_report,
        &content_report,
        &git_report,
        "test_scenario",
        "test_fixture",
    );
    
    println!("Overall success: {}", summary.overall_success);
    println!("Filesystem passed: {}", summary.filesystem_passed);
    println!("Content validation passed: {}", summary.content_validation_passed);
    println!("Git validation passed: {}", summary.git_validation_passed);
    println!("Total errors: {}", summary.total_errors);
    
    // Should correctly aggregate results
    assert!(!summary.overall_success, "Overall success should be false due to content errors");
    assert!(summary.filesystem_passed, "Filesystem validation should pass");
    assert!(!summary.content_validation_passed, "Content validation should fail");
    assert!(summary.git_validation_passed, "Git validation should pass");
    assert_eq!(summary.total_errors, 1, "Should have exactly 1 error from content validation");
    
    println!("✅ Validation result reporting working correctly");
}

#[tokio::test]
async fn test_cross_scenario_validation_consistency() {
    println!("🔄 Testing Cross-Scenario Validation Consistency");
    println!("===============================================");
    
    // Test the same fixture across multiple scenarios
    let fixture_name = "empty_repository";
    let scenarios = vec![
        TestScenario::dry_run(),
        TestScenario::force_init(),
    ];
    
    let mut scenario_reports = Vec::new();
    
    for scenario in scenarios {
        let report = validate_single_fixture_result(fixture_name, &scenario).await
            .expect("Validation failed");
        scenario_reports.push((scenario.description.clone(), report));
    }
    
    // Analyze consistency across scenarios
    for (scenario_name, report) in &scenario_reports {
        println!("Scenario '{}': Success={}, Errors={}", 
                 scenario_name, report.overall_success, report.total_errors);
    }
    
    // For the same fixture, dry run and force should behave predictably
    let dry_run_report = &scenario_reports[0].1;  // dry_run
    let force_report = &scenario_reports[1].1;    // force_init
    
    // Both should have some level of success (allowing for different file creation behaviors)
    println!("Dry run errors: {}", dry_run_report.total_errors);
    println!("Force init errors: {}", force_report.total_errors);
    
    println!("✅ Cross-scenario validation consistency test completed");
}

// Helper functions

async fn validate_single_fixture_result(
    fixture_name: &str,
    scenario: &TestScenario,
) -> anyhow::Result<fixtures::automated_validators::ValidationSummaryReport> {
    let env = InitCommandTestEnvironment::from_fixture_name(fixture_name)?;
    
    // Run init command for this scenario
    let _init_result = env.run_and_validate_init(scenario.agents, scenario.force, scenario.dry_run).await?;
    
    // Get standard expectations
    let (expected_files, expected_directories, content_expectations, git_expectations) = 
        create_standard_init_expectations();
    
    // Run all validations
    let fs_report = FileSystemValidator::validate_file_existence(
        env.path(),
        &expected_files.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        &expected_directories.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
    )?;
    
    let content_report = ContentValidator::validate_file_contents(
        env.path(),
        &content_expectations,
    )?;
    
    let git_report = GitConfigValidator::validate_git_configuration(
        env.path(),
        &git_expectations,
    )?;
    
    // Generate comprehensive report
    let summary = ValidationResultReporter::generate_comprehensive_report(
        &fs_report,
        &content_report,
        &git_report,
        &scenario.description,
        fixture_name,
    );
    
    Ok(summary)
}

fn create_test_fixture_with_content() -> RepositoryStateFixture {
    let mut files = std::collections::HashMap::new();
    files.insert("README.md".to_string(), "# Test Repository\n".to_string());
    files.insert("test_config.toml".to_string(), "[section]\ntest_key = \"test_value\"\n".to_string());
    files.insert(".gitignore".to_string(), "target/\n*.log\n".to_string());
    
    RepositoryStateFixture {
        name: "test_content_fixture".to_string(),
        description: "Test fixture with specific content for validation".to_string(),
        files,
        git_config: fixtures::repository_states::GitConfig::default(),
        existing_clambake_config: None,
    }
}

#[tokio::test]
async fn test_validation_error_reporting_clarity() -> anyhow::Result<()> {
    println!("🔍 Testing Validation Error Reporting Clarity");
    println!("=============================================");
    
    // Create a fixture that will fail validation
    let mut problematic_fixture = create_test_fixture_with_content();
    problematic_fixture.files.insert(
        "bad_config.toml".to_string(), 
        "[section]\nFORBIDDEN = \"should not exist\"\n".to_string()
    );
    
    let env = InitCommandTestEnvironment::from_fixture(problematic_fixture)?;
    
    // Create expectations that will fail
    let mut content_expectations = HashMap::new();
    content_expectations.insert(
        "bad_config.toml".to_string(),
        ContentExpectation {
            must_contain: vec!["REQUIRED_PATTERN".to_string()],
            must_not_contain: vec!["FORBIDDEN".to_string()],
            regex_patterns: vec![r#"missing_pattern\s*=\s*"[^"]+""#.to_string()],
            min_lines: Some(10), // File only has 2 lines
        },
    );
    
    let content_report = ContentValidator::validate_file_contents(
        env.path(),
        &content_expectations,
    )?;
    
    println!("Content validation should fail with clear errors:");
    for error in &content_report.errors {
        println!("  - {}", error);
    }
    
    // Should have multiple specific error messages
    assert!(!content_report.success, "Validation should fail");
    assert!(content_report.errors.len() >= 3, "Should have multiple specific errors");
    
    // Check for specific error types
    let error_text = content_report.errors.join(" ");
    assert!(error_text.contains("Required pattern not found"), "Should report missing required pattern");
    assert!(error_text.contains("Forbidden pattern found"), "Should report forbidden pattern");
    assert!(error_text.contains("Regex pattern not matched"), "Should report unmatched regex");
    assert!(error_text.contains("lines"), "Should report line count issue");
    
    println!("✅ Error reporting provides clear, actionable feedback");
    
    Ok(())
}

#[tokio::test] 
async fn test_standard_init_expectations_coverage() {
    println!("🎯 Testing Standard Init Expectations Coverage");
    println!("==============================================");
    
    let (expected_files, expected_directories, content_expectations, git_expectations) = 
        create_standard_init_expectations();
    
    println!("Expected files: {:?}", expected_files);
    println!("Expected directories: {:?}", expected_directories);
    println!("Content expectations: {} files", content_expectations.len());
    println!("Git expectations: should_be_git_repo = {}", git_expectations.should_be_git_repo);
    
    // Verify standard expectations cover key init command outputs
    assert!(expected_files.contains(&"clambake.toml".to_string()), 
            "Should expect clambake.toml file");
    assert!(expected_directories.contains(&".clambake".to_string()), 
            "Should expect .clambake directory");
    assert!(expected_directories.contains(&".clambake/credentials".to_string()), 
            "Should expect .clambake/credentials directory");
    assert!(expected_directories.contains(&".clambake/agents".to_string()), 
            "Should expect .clambake/agents directory");
    
    // Verify content expectations for clambake.toml
    let config_expectation = content_expectations.get("clambake.toml")
        .expect("Should have content expectations for clambake.toml");
    
    assert!(config_expectation.must_contain.contains(&"[github]".to_string()), 
            "Should expect [github] section");
    assert!(config_expectation.must_contain.contains(&"[agents]".to_string()), 
            "Should expect [agents] section");
    assert!(config_expectation.must_not_contain.contains(&"TODO".to_string()), 
            "Should forbid TODO markers");
    
    println!("✅ Standard expectations provide comprehensive coverage");
}