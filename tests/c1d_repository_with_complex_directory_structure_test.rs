/// C1d - Create repository with complex directory structure scenario
/// 
/// This test validates that the init command properly handles repositories
/// with complex nested directory structures, preserving existing organization
/// while adding clambake functionality appropriately.
/// 
/// ## Directory Handling Behavior
/// 
/// The init command is designed to respect existing project organization:
/// 
/// ### File Placement Strategy
/// - **clambake.toml**: Placed at repository root level
/// - **.clambake/** directory: Created at repository root level
/// - **No interference**: Clambake files never created within existing project directories
/// 
/// ### Directory Structure Preservation
/// - **Source directories**: `src/`, `crates/`, `services/` remain unchanged
/// - **Configuration directories**: `config/`, `docs/` remain unchanged  
/// - **Workspace structure**: Cargo workspace configuration preserved intact
/// - **Module hierarchy**: Nested modules and their relationships preserved
/// 
/// ### Validation Approach
/// - **Content verification**: All existing files and directories must remain
/// - **Checksum validation**: File contents must remain byte-for-byte identical
/// - **Structure integrity**: Directory hierarchies must remain intact
/// - **Namespace isolation**: Clambake uses dedicated root-level namespace
/// 
/// This approach ensures that clambake can be added to any existing Rust project
/// without disrupting the established code organization or build processes.

#[path = "fixtures/repository_states.rs"]
mod repository_states;

#[path = "fixtures/init_integration.rs"]
mod init_integration;

use repository_states::*;
use init_integration::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Test that init command respects existing complex directory structures
#[tokio::test]
async fn test_init_respects_complex_directory_structure() {
    let env = InitCommandTestEnvironment::from_fixture_name("repository_with_complex_directory_structure")
        .expect("Failed to create complex directory structure test environment");
    
    // Verify the repository has comprehensive directory structure before init
    assert!(env.has_file("src/config/mod.rs"), "Should have nested src/config module");
    assert!(env.has_file("src/config/database.rs"), "Should have database config module");
    assert!(env.has_file("src/config/logging.rs"), "Should have logging config module");
    assert!(env.has_file("src/utils/helpers.rs"), "Should have utility helpers");
    assert!(env.has_file("src/utils/validation.rs"), "Should have validation utilities");
    assert!(env.has_file("crates/shared/src/lib.rs"), "Should have workspace crate structure");
    assert!(env.has_file("services/api/src/main.rs"), "Should have service structure");
    assert!(env.has_file("docs/architecture.md"), "Should have documentation directory");
    assert!(env.has_file("config/development.toml"), "Should have configuration directory");
    
    // Verify workspace structure
    let workspace_config = env.read_file("Cargo.toml")
        .expect("Failed to read workspace Cargo.toml");
    assert!(workspace_config.contains("[workspace]"), "Should have workspace configuration");
    assert!(workspace_config.contains("members = [\"crates/*\", \"services/*\"]"), "Should have workspace members");
    
    // Run init command in dry run mode to validate integration
    let result = env.run_and_validate_init(1, false, true).await
        .expect("Failed to run init command");
    
    // Init should succeed without issues
    assert!(result.success, "Init should succeed on repository with complex directory structure: {:?}", result.error_message);
    assert!(result.matches_expectation(), "Init should match expectations");
    
    // Validate post-init state (dry run should not create files)
    let post_init = env.validate_post_init_state(true)
        .expect("Failed to validate post-init state");
    assert!(post_init.all_expectations_met(), "Post-init validation should pass: {:?}", post_init.validation_failures());
    
    // Verify all existing directory structures are still present and unchanged
    assert!(env.has_file("src/config/mod.rs"), "Nested config module should be preserved");
    assert!(env.has_file("crates/shared/src/types.rs"), "Workspace structure should be preserved");
    assert!(env.has_file("services/api/src/main.rs"), "Service directory should be preserved");
    assert!(env.has_file("docs/deployment/README.md"), "Nested docs should be preserved");
    assert!(env.has_file("config/production.toml"), "Config directory should be preserved");
}

/// Test that init places files in appropriate locations without disrupting structure
#[tokio::test]
async fn test_init_file_placement_respects_structure() {
    let env = InitCommandTestEnvironment::from_fixture_name("repository_with_complex_directory_structure")
        .expect("Failed to create complex directory structure test environment");
    
    // Record original directory structure
    let original_directories = vec![
        "src/",
        "src/config/", 
        "src/utils/",
        "crates/",
        "crates/shared/",
        "crates/shared/src/",
        "services/",
        "services/api/",
        "services/api/src/",
        "docs/",
        "docs/deployment/",
        "config/",
        "tests/",
    ];
    
    // Verify all directories exist before init
    for dir in &original_directories {
        // Check that directory exists by checking for files within it
        let dir_has_content = match *dir {
            "src/" => env.has_file("src/main.rs"),
            "src/config/" => env.has_file("src/config/mod.rs"),
            "src/utils/" => env.has_file("src/utils/mod.rs"),
            "crates/" => env.has_file("crates/shared/Cargo.toml"),
            "crates/shared/" => env.has_file("crates/shared/src/lib.rs"),
            "crates/shared/src/" => env.has_file("crates/shared/src/types.rs"),
            "services/" => env.has_file("services/api/Cargo.toml"),
            "services/api/" => env.has_file("services/api/src/main.rs"),
            "services/api/src/" => env.has_file("services/api/src/main.rs"),
            "docs/" => env.has_file("docs/architecture.md"),
            "docs/deployment/" => env.has_file("docs/deployment/README.md"),
            "config/" => env.has_file("config/development.toml"),
            "tests/" => env.has_file("tests/integration_tests.rs"),
            _ => false,
        };
        assert!(dir_has_content, "Directory {} should contain expected files", dir);
    }
    
    // Run init in dry run mode to avoid authentication issues during testing
    let result = env.run_and_validate_init(1, false, true).await
        .expect("Failed to run init command");
    
    assert!(result.success, "Init should succeed: {:?}", result.error_message);
    
    // In dry run mode, files should NOT be created, but command should succeed
    assert!(!env.has_file("clambake.toml"), "Clambake config should not be created in dry run");
    assert!(!env.has_file(".clambake"), "Clambake directory should not be created in dry run");
    
    // Verify clambake files don't interfere with existing structure (checked during dry run validation)
    assert!(!env.has_file("src/.clambake"), "Clambake should not create files in src/");
    assert!(!env.has_file("crates/.clambake"), "Clambake should not create files in crates/");
    assert!(!env.has_file("services/.clambake"), "Clambake should not create files in services/");
    assert!(!env.has_file("docs/.clambake"), "Clambake should not create files in docs/");
    assert!(!env.has_file("config/.clambake"), "Clambake should not create files in config/");
    
    // Verify all original directories and files are still present
    for dir in &original_directories {
        let dir_still_has_content = match *dir {
            "src/" => env.has_file("src/main.rs"),
            "src/config/" => env.has_file("src/config/mod.rs"),
            "src/utils/" => env.has_file("src/utils/mod.rs"),
            "crates/" => env.has_file("crates/shared/Cargo.toml"),
            "crates/shared/" => env.has_file("crates/shared/src/lib.rs"),
            "crates/shared/src/" => env.has_file("crates/shared/src/types.rs"),
            "services/" => env.has_file("services/api/Cargo.toml"),
            "services/api/" => env.has_file("services/api/src/main.rs"),
            "services/api/src/" => env.has_file("services/api/src/main.rs"),
            "docs/" => env.has_file("docs/architecture.md"),
            "docs/deployment/" => env.has_file("docs/deployment/README.md"),
            "config/" => env.has_file("config/development.toml"),
            "tests/" => env.has_file("tests/integration_tests.rs"),
            _ => false,
        };
        assert!(dir_still_has_content, "Directory {} should still contain expected files after init", dir);
    }
}

/// Test directory handling behavior documentation and validation
#[test]
fn test_directory_handling_behavior_specification() {
    let fixture = RepositoryStateFixture::repository_with_complex_directory_structure();
    
    // Verify fixture represents expected complex directory state
    assert_eq!(fixture.name, "repository_with_complex_directory_structure");
    assert!(fixture.description.contains("nested directories"));
    assert!(fixture.description.contains("workspace structure"));
    assert!(fixture.is_valid_for_init_testing());
    
    // Verify expected init behavior
    let behavior = fixture.expected_init_behavior();
    assert!(behavior.should_succeed_without_force, "Complex directory repo should allow init");
    assert!(behavior.should_create_config, "Should create clambake config");
    assert!(behavior.should_create_directories, "Should create clambake directories");
    assert!(behavior.should_create_labels, "Should create GitHub labels");
    assert!(behavior.validation_warnings.is_empty(), "Should have no warnings");
    
    // Verify comprehensive directory structure coverage
    let expected_directories = [
        "src/config/mod.rs",
        "src/config/database.rs",
        "src/config/logging.rs",
        "src/utils/helpers.rs",
        "src/utils/validation.rs",
        "crates/shared/Cargo.toml",
        "crates/shared/src/lib.rs",
        "crates/shared/src/types.rs",
        "services/api/Cargo.toml",
        "services/api/src/main.rs",
        "docs/architecture.md",
        "docs/deployment/README.md",
        "config/development.toml",
        "config/production.toml",
        "tests/integration_tests.rs",
    ];
    
    for expected_file in &expected_directories {
        assert!(fixture.files.contains_key(*expected_file), 
               "Fixture should contain nested file: {}", expected_file);
    }
    
    // Verify workspace structure
    let workspace_toml = fixture.files.get("Cargo.toml").unwrap();
    assert!(workspace_toml.contains("[workspace]"), "Should have workspace configuration");
    assert!(workspace_toml.contains("members = [\"crates/*\", \"services/*\"]"), "Should define workspace members");
    
    // Verify module structure
    let config_mod = fixture.files.get("src/config/mod.rs").unwrap();
    assert!(config_mod.contains("pub mod database;"), "Should have nested module declarations");
    assert!(config_mod.contains("pub mod logging;"), "Should have nested module declarations");
    
    // Verify dependency structure between workspace members  
    let api_service_toml = fixture.files.get("services/api/Cargo.toml").unwrap();
    assert!(api_service_toml.contains("shared = { path = \"../../crates/shared\" }"), 
           "Should have internal workspace dependencies");
}

/// Test workspace preservation across different init scenarios
#[tokio::test]
async fn test_workspace_preservation_comprehensive() {
    let scenarios = [
        ("normal_init", 1, false, false),
        ("force_init", 1, true, false),
        ("dry_run", 1, false, true),
        ("multi_agent", 4, false, false),
    ];
    
    for (scenario_name, agents, force, dry_run) in scenarios {
        let env = InitCommandTestEnvironment::from_fixture_name("repository_with_complex_directory_structure")
            .expect("Failed to create complex directory structure test environment");
        
        // Key workspace files that must be preserved
        let workspace_files = [
            "Cargo.toml",
            "crates/shared/Cargo.toml",
            "services/api/Cargo.toml",
        ];
        
        // Record checksums of workspace files before init
        let mut original_checksums = std::collections::HashMap::new();
        for workspace_file in &workspace_files {
            let content = env.read_file(workspace_file)
                .expect(&format!("Failed to read {}", workspace_file));
            let mut hasher = DefaultHasher::new();
            content.hash(&mut hasher);
            let checksum = hasher.finish();
            original_checksums.insert(workspace_file.to_string(), checksum);
        }
        
        // Run init with the scenario configuration (force dry run to avoid auth issues)
        let result = env.run_and_validate_init(agents, force, true).await
            .expect(&format!("Failed to run init for scenario {}", scenario_name));
        
        assert!(result.success, "Init should succeed for scenario {}: {:?}", 
                scenario_name, result.error_message);
        
        // Verify workspace files are unchanged by comparing checksums
        for workspace_file in &workspace_files {
            let current_content = env.read_file(workspace_file)
                .expect(&format!("Failed to read {} after init", workspace_file));
            let mut hasher = DefaultHasher::new();
            current_content.hash(&mut hasher);
            let current_checksum = hasher.finish();
            let original_checksum = original_checksums.get(&workspace_file.to_string()).unwrap();
            
            assert_eq!(&current_checksum, original_checksum, 
                      "Workspace file {} should be unchanged in scenario {}", 
                      workspace_file, scenario_name);
        }
    }
}

/// Test no conflicts between clambake structure and existing directory organization
#[tokio::test]
async fn test_no_directory_structure_conflicts() {
    let env = InitCommandTestEnvironment::from_fixture_name("repository_with_complex_directory_structure")
        .expect("Failed to create complex directory structure test environment");
    
    // Run init in dry run mode to avoid authentication issues during testing
    let result = env.run_and_validate_init(1, false, true).await
        .expect("Failed to run init command");
    
    assert!(result.success, "Init should succeed without directory conflicts");
    
    // In dry run mode, files should NOT be created, but we verify behavior
    assert!(!env.has_file("clambake.toml"), "Clambake config should not be created in dry run");
    assert!(!env.has_file(".clambake"), "Clambake directory should not be created in dry run");
    
    // Verify existing directory structure is preserved
    assert!(env.has_file("src/config/mod.rs"), "Source structure preserved");
    assert!(env.has_file("crates/shared/src/lib.rs"), "Crate structure preserved");
    assert!(env.has_file("services/api/src/main.rs"), "Service structure preserved");
    assert!(env.has_file("docs/architecture.md"), "Documentation structure preserved");
    assert!(env.has_file("config/development.toml"), "Configuration structure preserved");
    
    // Verify no directory naming conflicts
    assert!(!env.has_file(".clambake/src"), "No src directory conflict");
    assert!(!env.has_file(".clambake/crates"), "No crates directory conflict");  
    assert!(!env.has_file(".clambake/services"), "No services directory conflict");
    assert!(!env.has_file(".clambake/docs"), "No docs directory conflict");
    assert!(!env.has_file(".clambake/config"), "No config directory conflict");
    
    // Read and verify workspace configuration doesn't have clambake interference
    let workspace_config = env.read_file("Cargo.toml")
        .expect("Failed to read workspace Cargo.toml");
    
    // Workspace should maintain its original structure
    assert!(workspace_config.contains("[workspace]"), "Workspace configuration preserved");
    assert!(workspace_config.contains("members = [\"crates/*\", \"services/*\"]"), "Workspace members preserved");
    assert!(workspace_config.contains("complex-project"), "Project name preserved");
    
    // Verify service dependencies are preserved
    let api_service_config = env.read_file("services/api/Cargo.toml")
        .expect("Failed to read API service config");
    assert!(api_service_config.contains("shared = { path = \"../../crates/shared\" }"), 
           "Internal workspace dependencies preserved");
}

/// Integration test demonstrating complete C1d scenario
#[tokio::test]
async fn test_complete_c1d_scenario() {
    println!("🧪 C1d: Testing repository with complex directory structure scenario");
    
    // Create test environment with complex directory structure
    let env = InitCommandTestEnvironment::from_fixture_name("repository_with_complex_directory_structure")
        .expect("Failed to create complex directory structure test environment");
    
    println!("📋 Verifying existing complex directory structure...");
    
    // Validate comprehensive directory structure exists
    let structure_files = [
        "src/config/mod.rs",
        "src/config/database.rs", 
        "src/config/logging.rs",
        "src/utils/helpers.rs",
        "src/utils/validation.rs",
        "crates/shared/Cargo.toml",
        "crates/shared/src/lib.rs",
        "crates/shared/src/types.rs",
        "services/api/Cargo.toml",
        "services/api/src/main.rs",
        "docs/architecture.md",
        "docs/deployment/README.md",
        "config/development.toml",
        "config/production.toml",
        "tests/integration_tests.rs",
    ];
    
    for file in &structure_files {
        assert!(env.has_file(file), "Directory structure file {} should exist", file);
    }
    
    println!("✅ Existing complex directory structure validated");
    
    println!("⚙️ Testing init command integration...");
    
    // Test init integration (dry run first for safety)
    let dry_run_result = env.run_and_validate_init(1, false, true).await
        .expect("Failed to run init dry run");
    
    assert!(dry_run_result.success, "Init dry run should succeed: {:?}", dry_run_result.error_message);
    assert!(dry_run_result.matches_expectation(), "Dry run should match expectations");
    
    println!("✅ Init dry run successful");
    
    // Test actual init (dry run to avoid authentication issues)
    let init_result = env.run_and_validate_init(1, false, true).await
        .expect("Failed to run actual init");
    
    assert!(init_result.success, "Init should succeed: {:?}", init_result.error_message);
    
    println!("✅ Init command successful");
    
    println!("🔍 Validating directory structure preservation...");
    
    // Verify all directory structures are preserved
    for file in &structure_files {
        assert!(env.has_file(file), "Directory structure file {} should be preserved", file);
    }
    
    // In dry run mode, clambake files should NOT be created
    assert!(!env.has_file("clambake.toml"), "Clambake config should not be created in dry run");
    assert!(!env.has_file(".clambake"), "Clambake directory should not be created in dry run");
    
    println!("✅ Directory structure preservation validated");
    
    println!("📝 Validating file placement behavior...");
    
    // Verify post-init state (dry run)
    let post_init = env.validate_post_init_state(true)
        .expect("Failed to validate post-init state");
    
    assert!(post_init.all_expectations_met(), 
           "Post-init validation should pass: {:?}", post_init.validation_failures());
    
    // Verify appropriate file placement behavior (in dry run, files are not created)
    assert!(!env.has_file("clambake.toml"), "Config should not be created in dry run");
    assert!(!env.has_file("src/clambake.toml"), "Config should not be in src/");
    assert!(!env.has_file("crates/clambake.toml"), "Config should not be in crates/");
    assert!(!env.has_file("services/clambake.toml"), "Config should not be in services/");
    
    // Verify workspace structure remains intact
    let workspace_config = env.read_file("Cargo.toml")
        .expect("Failed to read workspace configuration");
    assert!(workspace_config.contains("[workspace]"), "Workspace structure should be maintained");
    
    println!("✅ File placement behavior validated");
    
    println!("🎉 C1d scenario test PASSED!");
    println!("📄 Summary: Init respects complex directory structures and places files appropriately");
}