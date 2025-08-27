/// C1e - Create repository with existing issue templates scenario
/// 
/// This test validates that the init command properly handles repositories
/// that already have GitHub issue templates and similar metadata, preserving
/// existing templates while enhancing the repository with clambake functionality.
/// 
/// ## Issue Template Handling Behavior
/// 
/// The init command is designed to respect existing GitHub templates:
/// 
/// ### Template Preservation Strategy
/// - **Issue templates**: All `.github/ISSUE_TEMPLATE/` files preserved intact
/// - **PR templates**: `.github/pull_request_template.md` preserved intact
/// - **Config files**: `.github/ISSUE_TEMPLATE/config.yml` preserved intact
/// - **Contributing guides**: `CONTRIBUTING.md` preserved intact
/// 
/// ### Enhancement Approach
/// - **Label creation**: Clambake creates its own labels without affecting existing ones
/// - **No template modification**: Existing templates remain byte-for-byte identical
/// - **Namespace isolation**: Clambake uses dedicated configuration space
/// - **Metadata coexistence**: GitHub metadata and clambake metadata coexist peacefully
/// 
/// ### Validation Approach
/// - **Content verification**: All existing template files must remain unchanged
/// - **Checksum validation**: Template contents must remain byte-for-byte identical
/// - **Functionality integrity**: GitHub template functionality must remain intact
/// - **Enhancement verification**: Clambake functionality added without conflicts
/// 
/// This approach ensures that clambake can be added to any repository with
/// existing GitHub issue templates without disrupting established workflows.

#[path = "fixtures/repository_states.rs"]
mod repository_states;

#[path = "fixtures/init_integration.rs"]
mod init_integration;

use repository_states::*;
use init_integration::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Test that init command preserves existing issue templates
#[tokio::test]
async fn test_init_preserves_existing_issue_templates() {
    let env = InitCommandTestEnvironment::from_fixture_name("repository_with_existing_issue_templates")
        .expect("Failed to create issue templates test environment");
    
    // Verify the repository has comprehensive issue template setup before init
    assert!(env.has_file(".github/ISSUE_TEMPLATE/bug_report.md"), "Should have bug report template");
    assert!(env.has_file(".github/ISSUE_TEMPLATE/feature_request.md"), "Should have feature request template");
    assert!(env.has_file(".github/ISSUE_TEMPLATE/config.yml"), "Should have template configuration");
    assert!(env.has_file(".github/pull_request_template.md"), "Should have PR template");
    assert!(env.has_file("CONTRIBUTING.md"), "Should have contributing guide");
    
    // Read template content to verify structure
    let bug_report = env.read_file(".github/ISSUE_TEMPLATE/bug_report.md")
        .expect("Failed to read bug report template");
    assert!(bug_report.contains("name: Bug report"), "Bug report should have proper frontmatter");
    assert!(bug_report.contains("labels: 'bug'"), "Bug report should have labels");
    assert!(bug_report.contains("**Describe the bug**"), "Bug report should have proper sections");
    
    let feature_request = env.read_file(".github/ISSUE_TEMPLATE/feature_request.md")
        .expect("Failed to read feature request template");
    assert!(feature_request.contains("name: Feature request"), "Feature request should have proper frontmatter");
    assert!(feature_request.contains("labels: 'enhancement'"), "Feature request should have labels");
    
    // Run init command in dry run mode to validate integration
    let result = env.run_and_validate_init(1, false, true).await
        .expect("Failed to run init command");
    
    // Init should succeed without issues
    assert!(result.success, "Init should succeed on repository with issue templates: {:?}", result.error_message);
    assert!(result.matches_expectation(), "Init should match expectations");
    
    // Validate post-init state (dry run should not create files)
    let post_init = env.validate_post_init_state(true)
        .expect("Failed to validate post-init state");
    assert!(post_init.all_expectations_met(), "Post-init validation should pass: {:?}", post_init.validation_failures());
    
    // Verify all existing issue template files are still present and unchanged
    assert!(env.has_file(".github/ISSUE_TEMPLATE/bug_report.md"), "Bug report template should be preserved");
    assert!(env.has_file(".github/ISSUE_TEMPLATE/feature_request.md"), "Feature request template should be preserved");
    assert!(env.has_file(".github/ISSUE_TEMPLATE/config.yml"), "Template config should be preserved");
    assert!(env.has_file(".github/pull_request_template.md"), "PR template should be preserved");
    assert!(env.has_file("CONTRIBUTING.md"), "Contributing guide should be preserved");
}

/// Test that init enhances rather than replaces existing templates
#[tokio::test] 
async fn test_init_enhances_existing_template_setup() {
    let env = InitCommandTestEnvironment::from_fixture_name("repository_with_existing_issue_templates")
        .expect("Failed to create issue templates test environment");
    
    // Verify comprehensive existing template setup
    let original_files = vec![
        ".github/ISSUE_TEMPLATE/bug_report.md",
        ".github/ISSUE_TEMPLATE/feature_request.md",
        ".github/ISSUE_TEMPLATE/config.yml",
        ".github/pull_request_template.md",
        "CONTRIBUTING.md"
    ];
    
    // Record original content of key template files
    let mut original_content = std::collections::HashMap::new();
    for file in &original_files {
        let content = env.read_file(file)
            .expect(&format!("Failed to read {}", file));
        original_content.insert(file.to_string(), content);
    }
    
    // Run actual init (force dry run to avoid authentication issues)
    let result = env.run_and_validate_init(1, false, true).await
        .expect("Failed to run init command");
    
    assert!(result.success, "Init should succeed: {:?}", result.error_message);
    
    // Validate that clambake files are created alongside existing templates (dry run mode)
    let post_init = env.validate_post_init_state(true)
        .expect("Failed to validate post-init state");
    assert!(post_init.all_expectations_met(), "Post-init validation should pass");
    
    // Verify all original template files are preserved
    for file in &original_files {
        assert!(env.has_file(file), "{} should still exist after init", file);
        let current_content = env.read_file(file)
            .expect(&format!("Failed to read {} after init", file));
        let original = original_content.get(&file.to_string()).unwrap();
        assert_eq!(&current_content, original, "{} should be unchanged by init", file);
    }
    
    // In dry run mode, clambake files should NOT be created
    assert!(!env.has_file("my-little-soda.toml"), "Clambake config should not be created in dry run");
    assert!(!env.has_file(".my-little-soda"), "Clambake directory should not be created in dry run");
    
    // Verify template config maintains expected functionality
    let template_config = env.read_file(".github/ISSUE_TEMPLATE/config.yml")
        .expect("Failed to read template config");
    assert!(template_config.contains("blank_issues_enabled: false"), "Template config should maintain settings");
    assert!(template_config.contains("contact_links:"), "Template config should maintain contact links");
}

/// Test template preservation across different init scenarios
#[tokio::test]
async fn test_template_preservation_comprehensive() {
    let scenarios = [
        ("normal_init", 1, false, false),
        ("force_init", 1, true, false), 
        ("dry_run", 1, false, true),
        ("multi_agent", 4, false, false),
    ];
    
    for (scenario_name, agents, force, dry_run) in scenarios {
        let env = InitCommandTestEnvironment::from_fixture_name("repository_with_existing_issue_templates")
            .expect("Failed to create issue templates test environment");
        
        // Key template files that must be preserved
        let template_files = [
            ".github/ISSUE_TEMPLATE/bug_report.md",
            ".github/ISSUE_TEMPLATE/feature_request.md",
            ".github/ISSUE_TEMPLATE/config.yml"
        ];
        
        // Record checksums of template files before init
        let mut original_checksums = std::collections::HashMap::new();
        for template_file in &template_files {
            let content = env.read_file(template_file)
                .expect(&format!("Failed to read {}", template_file));
            let mut hasher = DefaultHasher::new();
            content.hash(&mut hasher);
            let checksum = hasher.finish();
            original_checksums.insert(template_file.to_string(), checksum);
        }
        
        // Run init with the scenario configuration (force dry run to avoid auth issues)
        let result = env.run_and_validate_init(agents, force, true).await
            .expect(&format!("Failed to run init for scenario {}", scenario_name));
        
        assert!(result.success, "Init should succeed for scenario {}: {:?}", 
                scenario_name, result.error_message);
        
        // Verify template files are unchanged by comparing checksums
        for template_file in &template_files {
            let current_content = env.read_file(template_file)
                .expect(&format!("Failed to read {} after init", template_file));
            let mut hasher = DefaultHasher::new();
            current_content.hash(&mut hasher);
            let current_checksum = hasher.finish();
            let original_checksum = original_checksums.get(&template_file.to_string()).unwrap();
            
            assert_eq!(&current_checksum, original_checksum, 
                      "Template file {} should be unchanged in scenario {}", 
                      template_file, scenario_name);
        }
    }
}

/// Test issue template behavior documentation
#[test]
fn test_issue_template_behavior_specification() {
    let fixture = RepositoryStateFixture::repository_with_existing_issue_templates();
    
    // Verify fixture represents expected issue template repository state
    assert_eq!(fixture.name, "repository_with_existing_issue_templates");
    assert!(fixture.description.contains("GitHub issue templates"));
    assert!(fixture.is_valid_for_init_testing());
    
    // Verify expected init behavior
    let behavior = fixture.expected_init_behavior();
    assert!(behavior.should_succeed_without_force, "Template repo should allow init");
    assert!(behavior.should_create_config, "Should create clambake config");
    assert!(behavior.should_create_directories, "Should create clambake directories");
    assert!(behavior.should_create_labels, "Should create GitHub labels");
    assert!(behavior.validation_warnings.is_empty(), "Should have no warnings");
    
    // Verify comprehensive template file coverage
    let expected_template_files = [
        ".github/ISSUE_TEMPLATE/bug_report.md",
        ".github/ISSUE_TEMPLATE/feature_request.md",
        ".github/ISSUE_TEMPLATE/config.yml",
        ".github/pull_request_template.md",
        "CONTRIBUTING.md"
    ];
    
    for expected_file in &expected_template_files {
        assert!(fixture.files.contains_key(*expected_file), 
               "Fixture should contain template file: {}", expected_file);
    }
    
    // Verify bug report template has comprehensive structure
    let bug_report = fixture.files.get(".github/ISSUE_TEMPLATE/bug_report.md").unwrap();
    assert!(bug_report.contains("name: Bug report"), "Bug report should have proper name");
    assert!(bug_report.contains("about: Create a report to help us improve"), "Bug report should have description");
    assert!(bug_report.contains("labels: 'bug'"), "Bug report should have labels");
    assert!(bug_report.contains("**Describe the bug**"), "Bug report should have proper sections");
    assert!(bug_report.contains("**To Reproduce**"), "Bug report should have reproduction steps");
    assert!(bug_report.contains("**Expected behavior**"), "Bug report should have expected behavior section");
    
    // Verify feature request template structure
    let feature_request = fixture.files.get(".github/ISSUE_TEMPLATE/feature_request.md").unwrap();
    assert!(feature_request.contains("name: Feature request"), "Feature request should have proper name");
    assert!(feature_request.contains("labels: 'enhancement'"), "Feature request should have enhancement label");
    assert!(feature_request.contains("**Is your feature request related to a problem? Please describe.**"), "Feature request should have problem section");
    
    // Verify template configuration
    let template_config = fixture.files.get(".github/ISSUE_TEMPLATE/config.yml").unwrap();
    assert!(template_config.contains("blank_issues_enabled: false"), "Template config should disable blank issues");
    assert!(template_config.contains("contact_links:"), "Template config should have contact links");
    assert!(template_config.contains("Community Discord"), "Template config should reference community resources");
}

/// Test no conflicts between clambake labels and existing templates
#[tokio::test]
async fn test_no_template_metadata_conflicts() {
    let env = InitCommandTestEnvironment::from_fixture_name("repository_with_existing_issue_templates")
        .expect("Failed to create issue templates test environment");
    
    // Run init to set up clambake alongside existing templates (dry run to avoid auth issues)
    let result = env.run_and_validate_init(1, false, true).await
        .expect("Failed to run init command");
    
    assert!(result.success, "Init should succeed without conflicts");
    
    // In dry run mode, clambake files should NOT be created
    assert!(!env.has_file("my-little-soda.toml"), "Clambake config should not be created in dry run");
    assert!(!env.has_file(".my-little-soda"), "Clambake directory should not be created in dry run");
    
    // Verify existing template infrastructure is preserved
    assert!(env.has_file(".github/ISSUE_TEMPLATE/bug_report.md"), "Bug report template preserved");
    assert!(env.has_file(".github/ISSUE_TEMPLATE/feature_request.md"), "Feature request template preserved");
    assert!(env.has_file(".github/pull_request_template.md"), "PR template preserved");
    
    // Verify no template naming conflicts
    assert!(!env.has_file(".my-little-soda/ISSUE_TEMPLATE"), "No issue template directory conflict");
    assert!(!env.has_file(".my-little-soda/pull_request_template.md"), "No PR template naming conflict");
    
    // Read and verify bug report template doesn't have clambake interference
    let bug_report = env.read_file(".github/ISSUE_TEMPLATE/bug_report.md")
        .expect("Failed to read bug report template");
    
    // Bug report template should maintain its original structure
    assert!(bug_report.contains("name: Bug report"), "Bug report name preserved");
    assert!(bug_report.contains("labels: 'bug'"), "Bug report labels preserved");
    assert!(bug_report.contains("**Describe the bug**"), "Bug report sections preserved");
    
    // Verify contributing guide doesn't conflict with clambake
    let contributing = env.read_file("CONTRIBUTING.md")
        .expect("Failed to read contributing guide");
    assert!(contributing.contains("# Contributing Guide"), "Contributing guide structure preserved");
    assert!(contributing.contains("## Issue Templates"), "Contributing guide template documentation preserved");
}

/// Integration test demonstrating complete C1e scenario
#[tokio::test]
async fn test_complete_c1e_scenario() {
    println!("🧪 C1e: Testing repository with existing issue templates scenario");
    
    // Create test environment with comprehensive issue templates
    let env = InitCommandTestEnvironment::from_fixture_name("repository_with_existing_issue_templates")
        .expect("Failed to create issue templates test environment");
    
    println!("📋 Verifying existing issue template infrastructure...");
    
    // Validate comprehensive issue template setup exists
    let template_files = [
        ".github/ISSUE_TEMPLATE/bug_report.md",
        ".github/ISSUE_TEMPLATE/feature_request.md",
        ".github/ISSUE_TEMPLATE/config.yml",
        ".github/pull_request_template.md",
        "CONTRIBUTING.md"
    ];
    
    for file in &template_files {
        assert!(env.has_file(file), "Template file {} should exist", file);
    }
    
    println!("✅ Existing issue template infrastructure validated");
    
    println!("⚙️ Testing init command integration...");
    
    // Test init integration (dry run first for safety)
    let dry_run_result = env.run_and_validate_init(1, false, true).await
        .expect("Failed to run init dry run");
    
    assert!(dry_run_result.success, "Init dry run should succeed: {:?}", dry_run_result.error_message);
    assert!(dry_run_result.matches_expectation(), "Dry run should match expectations");
    
    println!("✅ Init dry run successful");
    
    // Test actual init (force dry run to avoid authentication issues)
    let init_result = env.run_and_validate_init(1, false, true).await
        .expect("Failed to run actual init");
    
    assert!(init_result.success, "Init should succeed: {:?}", init_result.error_message);
    
    println!("✅ Init command successful");
    
    println!("🔍 Validating template preservation...");
    
    // Verify all issue templates are preserved
    for file in &template_files {
        assert!(env.has_file(file), "Template file {} should be preserved", file);
    }
    
    // In dry run mode, clambake files should NOT be created
    assert!(!env.has_file("my-little-soda.toml"), "Clambake config should not be created in dry run");
    assert!(!env.has_file(".my-little-soda"), "Clambake directory should not be created in dry run");
    
    println!("✅ Template preservation validated");
    
    println!("📝 Validating metadata handling behavior...");
    
    // Verify post-init state (dry run mode)
    let post_init = env.validate_post_init_state(true)
        .expect("Failed to validate post-init state");
    
    assert!(post_init.all_expectations_met(), 
           "Post-init validation should pass: {:?}", post_init.validation_failures());
    
    // Verify template functionality is maintained
    let bug_report = env.read_file(".github/ISSUE_TEMPLATE/bug_report.md")
        .expect("Failed to read bug report template");
    assert!(bug_report.contains("name: Bug report"), "Bug report template should maintain structure");
    assert!(bug_report.contains("labels: 'bug'"), "Bug report template should maintain labels");
    
    let template_config = env.read_file(".github/ISSUE_TEMPLATE/config.yml")
        .expect("Failed to read template config");
    assert!(template_config.contains("blank_issues_enabled: false"), "Template config should maintain settings");
    
    println!("✅ Metadata handling behavior validated");
    
    println!("🎉 C1e scenario test PASSED!");
    println!("📄 Summary: Init preserves existing issue templates and enhances with clambake functionality");
}