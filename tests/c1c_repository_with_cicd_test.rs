/// C1c - Create repository with existing CI/CD setup scenario
/// 
/// This test validates that the init command properly integrates with repositories
/// that already have CI/CD workflows, preserving existing automation while enhancing
/// the repository with clambake functionality.

#[path = "fixtures/repository_states.rs"]
mod repository_states;

#[path = "fixtures/init_integration.rs"] 
mod init_integration;

use repository_states::*;
use init_integration::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Test that init command preserves existing CI/CD workflows
#[tokio::test]
async fn test_init_preserves_existing_cicd_workflows() {
    let env = InitCommandTestEnvironment::from_fixture_name("repository_with_existing_cicd_files")
        .expect("Failed to create CI/CD test environment");
    
    // Verify the repository has comprehensive CI/CD setup before init
    assert!(env.has_file(".github/workflows/ci.yml"), "Should have CI workflow");
    assert!(env.has_file(".github/workflows/release.yml"), "Should have release workflow");
    assert!(env.has_file(".github/workflows/security.yml"), "Should have security workflow");
    assert!(env.has_file("Dockerfile"), "Should have Docker setup");
    assert!(env.has_file("docker-compose.yml"), "Should have Docker Compose setup");
    assert!(env.has_file("codecov.yml"), "Should have code coverage config");
    assert!(env.has_file(".pre-commit-config.yaml"), "Should have pre-commit hooks");
    
    // Read CI workflow content to verify structure
    let ci_workflow = env.read_file(".github/workflows/ci.yml")
        .expect("Failed to read CI workflow");
    assert!(ci_workflow.contains("cargo test --verbose"), "CI should run tests");
    assert!(ci_workflow.contains("cargo clippy -- -D warnings"), "CI should run clippy");
    assert!(ci_workflow.contains("cargo fmt -- --check"), "CI should check formatting");
    
    // Run init command in dry run mode to validate integration
    let result = env.run_and_validate_init(1, false, true).await
        .expect("Failed to run init command");
    
    // Init should succeed without issues
    assert!(result.success, "Init should succeed on repository with CI/CD: {:?}", result.error_message);
    assert!(result.matches_expectation(), "Init should match expectations");
    
    // Validate post-init state (dry run should not create files)
    let post_init = env.validate_post_init_state(true)
        .expect("Failed to validate post-init state");
    assert!(post_init.all_expectations_met(), "Post-init validation should pass: {:?}", post_init.validation_failures());
    
    // Verify all existing CI/CD files are still present and unchanged
    assert!(env.has_file(".github/workflows/ci.yml"), "CI workflow should be preserved");
    assert!(env.has_file(".github/workflows/release.yml"), "Release workflow should be preserved");
    assert!(env.has_file(".github/workflows/security.yml"), "Security workflow should be preserved");
    assert!(env.has_file("Dockerfile"), "Docker setup should be preserved");
    assert!(env.has_file("docker-compose.yml"), "Docker Compose should be preserved");
    
    // Verify CI workflow content is unchanged
    let post_init_ci_workflow = env.read_file(".github/workflows/ci.yml")
        .expect("Failed to read CI workflow after init");
    assert_eq!(ci_workflow, post_init_ci_workflow, "CI workflow should be unchanged");
}

/// Test that init enhances rather than replaces existing CI/CD setup
#[tokio::test] 
async fn test_init_enhances_existing_cicd_setup() {
    let env = InitCommandTestEnvironment::from_fixture_name("repository_with_existing_cicd_files")
        .expect("Failed to create CI/CD test environment");
    
    // Verify comprehensive existing setup
    let original_files = vec![
        ".github/workflows/ci.yml",
        ".github/workflows/release.yml", 
        ".github/workflows/security.yml",
        "Dockerfile",
        "docker-compose.yml",
        "codecov.yml",
        ".pre-commit-config.yaml"
    ];
    
    // Record original content of key files
    let mut original_content = std::collections::HashMap::new();
    for file in &original_files {
        let content = env.read_file(file)
            .expect(&format!("Failed to read {}", file));
        original_content.insert(file.to_string(), content);
    }
    
    // Run actual init (not dry run) to see enhancement
    let result = env.run_and_validate_init(1, false, false).await
        .expect("Failed to run init command");
    
    assert!(result.success, "Init should succeed: {:?}", result.error_message);
    
    // Validate that clambake files are created alongside existing CI/CD
    let post_init = env.validate_post_init_state(false)
        .expect("Failed to validate post-init state");
    assert!(post_init.all_expectations_met(), "Post-init validation should pass");
    
    // Verify all original CI/CD files are preserved
    for file in &original_files {
        assert!(env.has_file(file), "{} should still exist after init", file);
        let current_content = env.read_file(file)
            .expect(&format!("Failed to read {} after init", file));
        let original = original_content.get(&file.to_string()).unwrap();
        assert_eq!(&current_content, original, "{} should be unchanged by init", file);
    }
    
    // Verify clambake integration files are created
    assert!(env.has_file("my-little-soda.toml"), "Clambake config should be created");
    assert!(env.has_file(".my-little-soda"), "Clambake directory should be created");
    
    // Verify Docker Compose has expected services that won't conflict
    let docker_compose = env.read_file("docker-compose.yml")
        .expect("Failed to read docker-compose.yml");
    assert!(docker_compose.contains("postgres:"), "Should preserve postgres service");
    assert!(docker_compose.contains("redis:"), "Should preserve redis service");
}

/// Test workflow preservation across different init scenarios
#[tokio::test]
async fn test_workflow_preservation_comprehensive() {
    let scenarios = [
        ("normal_init", 1, false, false),
        ("force_init", 1, true, false), 
        ("dry_run", 1, false, true),
        ("multi_agent", 4, false, false),
    ];
    
    for (scenario_name, agents, force, dry_run) in scenarios {
        let env = InitCommandTestEnvironment::from_fixture_name("repository_with_existing_cicd_files")
            .expect("Failed to create CI/CD test environment");
        
        // Key workflow files that must be preserved
        let workflow_files = [
            ".github/workflows/ci.yml",
            ".github/workflows/release.yml",
            ".github/workflows/security.yml"
        ];
        
        // Record checksums of workflow files before init
        let mut original_checksums = std::collections::HashMap::new();
        for workflow_file in &workflow_files {
            let content = env.read_file(workflow_file)
                .expect(&format!("Failed to read {}", workflow_file));
            let mut hasher = DefaultHasher::new();
            content.hash(&mut hasher);
            let checksum = hasher.finish();
            original_checksums.insert(workflow_file.to_string(), checksum);
        }
        
        // Run init with the scenario configuration
        let result = env.run_and_validate_init(agents, force, dry_run).await
            .expect(&format!("Failed to run init for scenario {}", scenario_name));
        
        assert!(result.success, "Init should succeed for scenario {}: {:?}", 
                scenario_name, result.error_message);
        
        // Verify workflow files are unchanged by comparing checksums
        for workflow_file in &workflow_files {
            let current_content = env.read_file(workflow_file)
                .expect(&format!("Failed to read {} after init", workflow_file));
            let mut hasher = DefaultHasher::new();
            current_content.hash(&mut hasher);
            let current_checksum = hasher.finish();
            let original_checksum = original_checksums.get(&workflow_file.to_string()).unwrap();
            
            assert_eq!(&current_checksum, original_checksum, 
                      "Workflow file {} should be unchanged in scenario {}", 
                      workflow_file, scenario_name);
        }
    }
}

/// Test CI/CD integration behavior documentation
#[test]
fn test_cicd_integration_behavior_specification() {
    let fixture = RepositoryStateFixture::repository_with_existing_cicd_files();
    
    // Verify fixture represents expected CI/CD repository state
    assert_eq!(fixture.name, "repository_with_existing_cicd_files");
    assert!(fixture.description.contains("comprehensive CI/CD setup"));
    assert!(fixture.is_valid_for_init_testing());
    
    // Verify expected init behavior
    let behavior = fixture.expected_init_behavior();
    assert!(behavior.should_succeed_without_force, "Clean CI/CD repo should allow init");
    assert!(behavior.should_create_config, "Should create clambake config");
    assert!(behavior.should_create_directories, "Should create clambake directories");
    assert!(behavior.should_create_labels, "Should create GitHub labels");
    assert!(behavior.validation_warnings.is_empty(), "Should have no warnings");
    
    // Verify comprehensive CI/CD file coverage
    let expected_cicd_files = [
        ".github/workflows/ci.yml",
        ".github/workflows/release.yml", 
        ".github/workflows/security.yml",
        "Dockerfile",
        "docker-compose.yml",
        "codecov.yml",
        ".pre-commit-config.yaml"
    ];
    
    for expected_file in &expected_cicd_files {
        assert!(fixture.files.contains_key(*expected_file), 
               "Fixture should contain CI/CD file: {}", expected_file);
    }
    
    // Verify CI workflow has comprehensive test matrix
    let ci_workflow = fixture.files.get(".github/workflows/ci.yml").unwrap();
    assert!(ci_workflow.contains("matrix:"), "CI should use build matrix");
    assert!(ci_workflow.contains("rust:"), "CI should test multiple Rust versions");
    assert!(ci_workflow.contains("stable"), "CI should test stable Rust");
    assert!(ci_workflow.contains("beta"), "CI should test beta Rust");
    assert!(ci_workflow.contains("nightly"), "CI should test nightly Rust");
    
    // Verify security workflow automation
    let security_workflow = fixture.files.get(".github/workflows/security.yml").unwrap();
    assert!(security_workflow.contains("security audit"), "Should have security auditing");
    assert!(security_workflow.contains("schedule:"), "Should run on schedule");
    assert!(security_workflow.contains("cron:"), "Should have cron trigger");
}

/// Test no automation conflicts between clambake and existing CI/CD
#[tokio::test]
async fn test_no_automation_conflicts() {
    let env = InitCommandTestEnvironment::from_fixture_name("repository_with_existing_cicd_files")
        .expect("Failed to create CI/CD test environment");
    
    // Run init to set up clambake alongside existing CI/CD
    let result = env.run_and_validate_init(1, false, false).await
        .expect("Failed to run init command");
    
    assert!(result.success, "Init should succeed without conflicts");
    
    // Verify no file conflicts - clambake should use its own namespace
    assert!(env.has_file("my-little-soda.toml"), "Clambake config created");
    assert!(env.has_file(".my-little-soda"), "Clambake directory created");
    
    // Verify existing CI/CD automation is preserved
    assert!(env.has_file(".github/workflows/ci.yml"), "CI workflow preserved");
    assert!(env.has_file(".pre-commit-config.yaml"), "Pre-commit hooks preserved");
    
    // Verify no file naming conflicts
    assert!(!env.has_file(".my-little-soda/workflows"), "No workflow directory conflict");
    assert!(!env.has_file("clambake.yml"), "No YAML config naming conflict");
    
    // Read and verify CI workflow doesn't have clambake interference
    let ci_content = env.read_file(".github/workflows/ci.yml")
        .expect("Failed to read CI workflow");
    
    // CI workflow should maintain its original structure
    assert!(ci_content.contains("name: CI"), "CI workflow name preserved");
    assert!(ci_content.contains("cargo test --verbose"), "CI test command preserved");
    assert!(ci_content.contains("cargo clippy"), "CI linting preserved");
    
    // Verify pre-commit config doesn't conflict with clambake
    let precommit_content = env.read_file(".pre-commit-config.yaml")
        .expect("Failed to read pre-commit config");
    assert!(precommit_content.contains("cargo fmt"), "Pre-commit formatting preserved");
    assert!(precommit_content.contains("cargo clippy"), "Pre-commit linting preserved");
}

/// Integration test demonstrating complete C1c scenario
#[tokio::test]
async fn test_complete_c1c_scenario() {
    println!("🧪 C1c: Testing repository with existing CI/CD setup scenario");
    
    // Create test environment with comprehensive CI/CD
    let env = InitCommandTestEnvironment::from_fixture_name("repository_with_existing_cicd_files")
        .expect("Failed to create CI/CD test environment");
    
    println!("📋 Verifying existing CI/CD infrastructure...");
    
    // Validate comprehensive CI/CD setup exists
    let cicd_files = [
        ".github/workflows/ci.yml",
        ".github/workflows/release.yml",
        ".github/workflows/security.yml", 
        "Dockerfile",
        "docker-compose.yml",
        "codecov.yml",
        ".pre-commit-config.yaml"
    ];
    
    for file in &cicd_files {
        assert!(env.has_file(file), "CI/CD file {} should exist", file);
    }
    
    println!("✅ Existing CI/CD infrastructure validated");
    
    println!("⚙️ Testing init command integration...");
    
    // Test init integration (dry run first for safety)
    let dry_run_result = env.run_and_validate_init(1, false, true).await
        .expect("Failed to run init dry run");
    
    assert!(dry_run_result.success, "Init dry run should succeed: {:?}", dry_run_result.error_message);
    assert!(dry_run_result.matches_expectation(), "Dry run should match expectations");
    
    println!("✅ Init dry run successful");
    
    // Test actual init
    let init_result = env.run_and_validate_init(1, false, false).await
        .expect("Failed to run actual init");
    
    assert!(init_result.success, "Init should succeed: {:?}", init_result.error_message);
    
    println!("✅ Init command successful");
    
    println!("🔍 Validating workflow preservation...");
    
    // Verify all CI/CD workflows are preserved
    for file in &cicd_files {
        assert!(env.has_file(file), "CI/CD file {} should be preserved", file);
    }
    
    // Verify clambake integration
    assert!(env.has_file("my-little-soda.toml"), "Clambake config should be created");
    assert!(env.has_file(".my-little-soda"), "Clambake directory should be created");
    
    println!("✅ Workflow preservation validated");
    
    println!("📝 Validating integration behavior...");
    
    // Verify post-init state
    let post_init = env.validate_post_init_state(false)
        .expect("Failed to validate post-init state");
    
    assert!(post_init.all_expectations_met(), 
           "Post-init validation should pass: {:?}", post_init.validation_failures());
    
    // Verify no automation conflicts
    let ci_workflow = env.read_file(".github/workflows/ci.yml")
        .expect("Failed to read CI workflow");
    assert!(ci_workflow.contains("name: CI"), "CI workflow should maintain structure");
    
    println!("✅ Integration behavior validated");
    
    println!("🎉 C1c scenario test PASSED!");
    println!("📄 Summary: Init enhances repository with existing CI/CD without conflicts");
}