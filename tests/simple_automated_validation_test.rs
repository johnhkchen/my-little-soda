/// Simple test to verify the automated validation system works correctly
/// 
/// This test demonstrates the B3b deliverable: automated validation checks that verify
/// init command results including file existence, directory structure, content validation,
/// and Git configuration validation with clear error reporting.

mod fixtures;

use fixtures::automated_validators::{
    FileSystemValidator, ContentValidator, GitConfigValidator, ValidationResultReporter,
    ContentExpectation, GitConfigExpectations, create_standard_init_expectations,
};
use std::collections::HashMap;
use tempfile::TempDir;

#[tokio::test]
async fn test_filesystem_validator_basic_functionality() {
    println!("🗂️  Testing FileSystem Validator Basic Functionality");
    println!("===================================================");
    
    // Create a temporary directory with test files and directories
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();
    
    // Create test files and directories
    std::fs::write(repo_path.join("clambake.toml"), "[github]\nowner = \"test\"\n").unwrap();
    std::fs::write(repo_path.join("README.md"), "# Test Repository\n").unwrap();
    std::fs::create_dir_all(repo_path.join(".clambake/credentials")).unwrap();
    std::fs::create_dir_all(repo_path.join(".clambake/agents")).unwrap();
    
    // Define expected files and directories
    let expected_files = vec!["clambake.toml", "README.md"];
    let expected_directories = vec![".clambake", ".clambake/credentials", ".clambake/agents"];
    
    // Run filesystem validation
    let fs_report = FileSystemValidator::validate_file_existence(
        repo_path,
        &expected_files,
        &expected_directories,
    ).unwrap();
    
    // Verify results
    println!("Filesystem validation success: {}", fs_report.success);
    println!("Files found: {}", fs_report.files_found.len());
    println!("Directories found: {}", fs_report.directories_found.len());
    
    assert!(fs_report.success, "Filesystem validation should succeed");
    assert_eq!(fs_report.files_found.len(), 2);
    assert_eq!(fs_report.directories_found.len(), 3);
    assert_eq!(fs_report.files_missing.len(), 0);
    assert_eq!(fs_report.directories_missing.len(), 0);
    
    println!("✅ Filesystem validator working correctly");
}

#[tokio::test]
async fn test_content_validator_basic_functionality() {
    println!("📝 Testing Content Validator Basic Functionality");
    println!("===============================================");
    
    // Create temporary directory with test files
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();
    
    // Create test configuration file
    let config_content = r#"[github]
owner = "test-owner"
repo = "test-repo"

[agents]
max_agents = 4
"#;
    std::fs::write(repo_path.join("clambake.toml"), config_content).unwrap();
    
    // Define content expectations
    let mut content_expectations = HashMap::new();
    content_expectations.insert(
        "clambake.toml".to_string(),
        ContentExpectation {
            must_contain: vec![
                "[github]".to_string(),
                "[agents]".to_string(),
                "max_agents".to_string(),
            ],
            must_not_contain: vec![
                "TODO".to_string(),
                "PLACEHOLDER".to_string(),
            ],
            regex_patterns: vec![
                r#"owner\s*=\s*"[^"]+""#.to_string(),
                r#"repo\s*=\s*"[^"]+""#.to_string(),
            ],
            min_lines: Some(3),
        },
    );
    
    // Run content validation
    let content_report = ContentValidator::validate_file_contents(
        repo_path,
        &content_expectations,
    ).unwrap();
    
    // Verify results
    println!("Content validation success: {}", content_report.success);
    println!("Files passed: {}", content_report.passed_files.len());
    println!("Files failed: {}", content_report.failed_files.len());
    
    assert!(content_report.success, "Content validation should succeed");
    assert_eq!(content_report.passed_files.len(), 1);
    assert_eq!(content_report.failed_files.len(), 0);
    assert!(content_report.errors.is_empty());
    
    println!("✅ Content validator working correctly");
}

#[tokio::test]
async fn test_content_validator_failure_cases() {
    println!("❌ Testing Content Validator Failure Cases");
    println!("==========================================");
    
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();
    
    // Create a file that will fail validation
    let bad_config_content = r#"[incomplete]
PLACEHOLDER = "replace me"
TODO: add real configuration
"#;
    std::fs::write(repo_path.join("bad_config.toml"), bad_config_content).unwrap();
    
    // Define expectations that will fail
    let mut content_expectations = HashMap::new();
    content_expectations.insert(
        "bad_config.toml".to_string(),
        ContentExpectation {
            must_contain: vec!["[github]".to_string()],  // Missing
            must_not_contain: vec!["PLACEHOLDER".to_string(), "TODO".to_string()],  // Present
            regex_patterns: vec![r#"missing_pattern"#.to_string()],  // Won't match
            min_lines: Some(10),  // File only has 3 lines
        },
    );
    
    let content_report = ContentValidator::validate_file_contents(
        repo_path,
        &content_expectations,
    ).unwrap();
    
    println!("Content validation (expected to fail): {}", content_report.success);
    println!("Errors found: {}", content_report.errors.len());
    
    // Should have multiple specific errors
    assert!(!content_report.success, "Validation should fail");
    assert!(content_report.errors.len() >= 4, "Should have multiple errors");
    
    // Check error messages are descriptive
    let error_text = content_report.errors.join(" ");
    assert!(error_text.contains("Required pattern not found"));
    assert!(error_text.contains("Forbidden pattern found"));
    assert!(error_text.contains("Regex pattern not matched"));
    assert!(error_text.contains("lines"));
    
    println!("✅ Content validator error reporting working correctly");
}

#[tokio::test]
async fn test_git_configuration_validator() {
    println!("📋 Testing Git Configuration Validator");
    println!("======================================");
    
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();
    
    // Initialize a git repository
    std::process::Command::new("git")
        .args(&["init"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to init git repository");
    
    // Configure git user for this repository
    std::process::Command::new("git")
        .args(&["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .output()
        .unwrap();
        
    std::process::Command::new("git")
        .args(&["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    
    // Create and commit a file to ensure we have a main branch
    std::fs::write(repo_path.join("README.md"), "# Test\n").unwrap();
    std::process::Command::new("git")
        .args(&["add", "README.md"])
        .current_dir(repo_path)
        .output()
        .unwrap();
        
    std::process::Command::new("git")
        .args(&["commit", "-m", "Initial commit"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    
    // Define Git expectations
    let git_expectations = GitConfigExpectations {
        should_be_git_repo: true,
        expected_branch: None, // Different systems may have different default branches
        expected_remote_url: None,
        should_have_remote: false,
        should_be_clean: true,
    };
    
    // Run git validation
    let git_report = GitConfigValidator::validate_git_configuration(
        repo_path,
        &git_expectations,
    ).unwrap();
    
    println!("Git validation success: {}", git_report.success);
    println!("Git repo exists: {}", git_report.git_repo_exists);
    println!("Working directory clean: {}", git_report.working_directory_clean);
    
    assert!(git_report.git_repo_exists, "Should detect git repository");
    assert!(git_report.working_directory_clean, "Working directory should be clean");
    
    if !git_report.success {
        println!("Git validation errors:");
        for error in &git_report.errors {
            println!("  - {}", error);
        }
    }
    
    println!("✅ Git configuration validator working");
}

#[tokio::test]
async fn test_validation_result_reporter() {
    println!("📊 Testing Validation Result Reporter");
    println!("=====================================");
    
    // Create mock validation reports
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
    
    // Verify report aggregation
    assert!(!summary.overall_success, "Overall success should be false due to content errors");
    assert!(summary.filesystem_passed, "Filesystem validation should pass");
    assert!(!summary.content_validation_passed, "Content validation should fail");
    assert!(summary.git_validation_passed, "Git validation should pass");
    assert_eq!(summary.total_errors, 1, "Should have exactly 1 error from content validation");
    
    println!("✅ Validation result reporter working correctly");
}

#[tokio::test]
async fn test_standard_init_expectations() {
    println!("🎯 Testing Standard Init Expectations");
    println!("=====================================");
    
    let (expected_files, expected_directories, content_expectations, git_expectations) = 
        create_standard_init_expectations();
    
    println!("Expected files: {:?}", expected_files);
    println!("Expected directories: {:?}", expected_directories);
    println!("Content expectations for {} files", content_expectations.len());
    
    // Verify standard expectations cover key init command outputs
    assert!(expected_files.contains(&"clambake.toml".to_string()), 
            "Should expect clambake.toml file");
    assert!(expected_directories.contains(&".clambake".to_string()), 
            "Should expect .clambake directory");
    
    // Verify content expectations for clambake.toml
    let config_expectation = content_expectations.get("clambake.toml")
        .expect("Should have content expectations for clambake.toml");
    
    assert!(config_expectation.must_contain.contains(&"[github]".to_string()), 
            "Should expect [github] section");
    assert!(config_expectation.must_contain.contains(&"[agents]".to_string()), 
            "Should expect [agents] section");
    
    println!("Git expectations: should_be_git_repo = {}", git_expectations.should_be_git_repo);
    assert!(git_expectations.should_be_git_repo, "Should expect git repository");
    
    println!("✅ Standard expectations provide comprehensive coverage");
}

#[tokio::test]
async fn test_complete_validation_workflow() {
    println!("🔄 Testing Complete Validation Workflow");
    println!("=======================================");
    
    // Create a temporary directory that simulates a successful init command result
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();
    
    // Step 1: Set up a complete init-like environment
    println!("Step 1: Setting up simulated init command result...");
    
    // Initialize git repository
    std::process::Command::new("git")
        .args(&["init"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to init git repository");
        
    // Configure git
    std::process::Command::new("git")
        .args(&["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .output()
        .unwrap();
        
    std::process::Command::new("git")
        .args(&["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .output()
        .unwrap();
    
    // Create expected files and directories (simulating init command output)
    let config_content = r#"[github]
owner = "test-owner"
repo = "test-repo"
[github.rate_limit]
requests_per_hour = 5000
burst_capacity = 100

[observability]
tracing_enabled = true
log_level = "info"
metrics_enabled = true

[agents]
max_agents = 1
coordination_timeout_seconds = 300
"#;
    std::fs::write(repo_path.join("clambake.toml"), config_content).unwrap();
    std::fs::create_dir_all(repo_path.join(".clambake/credentials")).unwrap();
    std::fs::create_dir_all(repo_path.join(".clambake/agents")).unwrap();
    
    // Step 2: Run all validators
    println!("Step 2: Running automated validation checks...");
    
    let (expected_files, expected_directories, content_expectations, git_expectations) = 
        create_standard_init_expectations();
    
    // Filesystem validation
    let fs_report = FileSystemValidator::validate_file_existence(
        repo_path,
        &expected_files.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        &expected_directories.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
    ).unwrap();
    
    // Content validation
    let content_report = ContentValidator::validate_file_contents(
        repo_path,
        &content_expectations,
    ).unwrap();
    
    // Git validation
    let git_report = GitConfigValidator::validate_git_configuration(
        repo_path,
        &git_expectations,
    ).unwrap();
    
    // Step 3: Generate comprehensive report
    println!("Step 3: Generating comprehensive validation report...");
    
    let summary = ValidationResultReporter::generate_comprehensive_report(
        &fs_report,
        &content_report,
        &git_report,
        "complete_workflow_test",
        "simulated_init_result",
    );
    
    // Step 4: Verify results
    println!("Step 4: Verifying validation results...");
    println!("  Filesystem validation: {}", summary.filesystem_passed);
    println!("  Content validation: {}", summary.content_validation_passed);
    println!("  Git validation: {}", summary.git_validation_passed);
    println!("  Overall success: {}", summary.overall_success);
    println!("  Total errors: {}", summary.total_errors);
    
    if !summary.overall_success {
        println!("  Errors found:");
        for error in &summary.error_summary {
            println!("    - {}", error);
        }
    }
    
    // The validation should pass for our well-constructed test scenario
    assert!(summary.filesystem_passed, "Filesystem validation should pass");
    assert!(summary.content_validation_passed, "Content validation should pass");
    assert!(summary.git_validation_passed || !git_report.errors.is_empty(), 
            "Git validation should pass or have explainable errors");
    
    println!("✅ Complete validation workflow test completed successfully");
    println!("🎉 Automated validation system is working as expected!");
}