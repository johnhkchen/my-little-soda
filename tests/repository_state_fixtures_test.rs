/// Simple integration test for repository state fixtures functionality
/// Tests fixture loading, creation, and basic validation without complex init command integration

use tempfile::TempDir;
use std::fs;

#[path = "fixtures/repository_states.rs"]
mod repository_states;

use repository_states::*;

#[tokio::test]
async fn test_empty_repository_fixture_creation() {
    let fixture = RepositoryStateFixture::empty_repository();
    
    assert_eq!(fixture.name, "empty_repository");
    assert!(fixture.is_valid_for_init_testing());
    assert!(fixture.existing_my_little_soda_config.is_none());
    assert!(fixture.files.contains_key("README.md"));
    assert!(fixture.files.contains_key(".gitignore"));
    
    let behavior = fixture.expected_init_behavior();
    assert!(behavior.should_succeed_without_force);
    assert!(behavior.should_create_config);
    assert!(behavior.validation_warnings.is_empty());
}

#[tokio::test]
async fn test_repository_with_existing_files_fixture() {
    let fixture = RepositoryStateFixture::repository_with_existing_files();
    
    assert_eq!(fixture.name, "repository_with_existing_files");
    assert!(fixture.is_valid_for_init_testing());
    assert!(fixture.existing_my_little_soda_config.is_none());
    assert!(fixture.files.contains_key("Cargo.toml"));
    assert!(fixture.files.contains_key("src/main.rs"));
    assert!(fixture.files.contains_key("src/lib.rs"));
    
    let behavior = fixture.expected_init_behavior();
    assert!(behavior.should_succeed_without_force);
    assert!(behavior.should_create_config);
}

#[tokio::test]
async fn test_partial_initialization_fixture() {
    let fixture = RepositoryStateFixture::repository_with_partial_initialization();
    
    assert_eq!(fixture.name, "repository_with_partial_initialization");
    assert!(fixture.is_valid_for_init_testing());
    assert!(fixture.existing_my_little_soda_config.is_some());
    assert!(fixture.files.contains_key("my-little-soda.toml"));
    assert!(fixture.files.contains_key(".my-little-soda/partial_setup"));
    
    let behavior = fixture.expected_init_behavior();
    assert!(!behavior.should_succeed_without_force); // Should fail without --force
    assert!(!behavior.should_create_config);
    assert!(!behavior.validation_warnings.is_empty());
}

#[tokio::test]
async fn test_conflicts_fixture() {
    let fixture = RepositoryStateFixture::repository_with_conflicts();
    
    assert_eq!(fixture.name, "repository_with_conflicts");
    assert!(fixture.is_valid_for_init_testing());
    assert!(fixture.git_config.uncommitted_changes);
    assert!(!fixture.git_config.conflicted_files.is_empty());
    assert!(fixture.git_config.conflicted_files.contains(&"src/main.rs".to_string()));
    
    let behavior = fixture.expected_init_behavior();
    assert!(!behavior.should_succeed_without_force); // Should fail due to uncommitted changes
    assert!(!behavior.should_create_config);
    assert!(!behavior.validation_warnings.is_empty());
}

#[test]
fn test_all_fixtures_loaded_correctly() {
    let fixtures = RepositoryStateFixture::all_fixtures();
    
    assert_eq!(fixtures.len(), 7);
    
    let names: Vec<String> = fixtures.iter().map(|f| f.name.clone()).collect();
    assert!(names.contains(&"empty_repository".to_string()));
    assert!(names.contains(&"repository_with_existing_files".to_string()));
    assert!(names.contains(&"repository_with_partial_initialization".to_string()));
    assert!(names.contains(&"repository_with_conflicts".to_string()));
    assert!(names.contains(&"repository_with_complex_directory_structure".to_string()));
    assert!(names.contains(&"repository_with_existing_issue_templates".to_string()));
    assert!(names.contains(&"repository_with_existing_cicd_files".to_string()));
    
    // All fixtures should be valid for init testing
    for fixture in &fixtures {
        assert!(fixture.is_valid_for_init_testing(), 
                "Fixture '{}' should be valid for init testing", fixture.name);
    }
}

#[test]
fn test_fixture_loader_functions() {
    // Test loading specific fixture
    let empty_fixture = RepositoryFixtureLoader::load_fixture("empty_repository");
    assert!(empty_fixture.is_some());
    assert_eq!(empty_fixture.unwrap().name, "empty_repository");
    
    // Test loading nonexistent fixture
    let nonexistent = RepositoryFixtureLoader::load_fixture("nonexistent");
    assert!(nonexistent.is_none());
    
    // Test loading all fixtures
    let all_fixtures = RepositoryFixtureLoader::load_all_fixtures();
    assert_eq!(all_fixtures.len(), 7);
    
    // Test loading init-specific fixtures
    let init_fixtures = RepositoryFixtureLoader::load_init_command_fixtures();
    assert!(!init_fixtures.is_empty());
    for fixture in init_fixtures {
        assert!(fixture.is_valid_for_init_testing());
    }
}

#[tokio::test]
async fn test_temporary_repository_creation() {
    let fixture = RepositoryStateFixture::empty_repository();
    let temp_repo = fixture.create_temp_repository();
    
    assert!(temp_repo.is_ok());
    let temp_dir = temp_repo.unwrap();
    
    // Verify files were created correctly
    let readme_path = temp_dir.path().join("README.md");
    assert!(readme_path.exists());
    
    let readme_content = fs::read_to_string(&readme_path).unwrap();
    assert!(readme_content.contains("# Test Repository"));
    
    let gitignore_path = temp_dir.path().join(".gitignore");
    assert!(gitignore_path.exists());
    
    let gitignore_content = fs::read_to_string(&gitignore_path).unwrap();
    assert!(gitignore_content.contains("target/"));
    assert!(gitignore_content.contains("*.log"));
    
    // Verify git repository was initialized
    let git_dir = temp_dir.path().join(".git");
    assert!(git_dir.exists());
}

#[tokio::test]
async fn test_temp_repo_from_loader() {
    let temp_repo = RepositoryFixtureLoader::create_temp_repository_from_name("empty_repository").unwrap();
    
    assert!(temp_repo.is_some());
    let temp_dir = temp_repo.unwrap();
    
    // Basic validation that temp repo was created
    assert!(temp_dir.path().join("README.md").exists());
    assert!(temp_dir.path().join(".git").exists());
}

#[tokio::test]
async fn test_repository_with_existing_files_temp_creation() {
    let fixture = RepositoryStateFixture::repository_with_existing_files();
    let temp_repo = fixture.create_temp_repository().unwrap();
    
    // Verify Rust project structure was created
    assert!(temp_repo.path().join("Cargo.toml").exists());
    assert!(temp_repo.path().join("src/main.rs").exists());
    assert!(temp_repo.path().join("src/lib.rs").exists());
    
    // Verify content
    let cargo_content = fs::read_to_string(temp_repo.path().join("Cargo.toml")).unwrap();
    assert!(cargo_content.contains("[package]"));
    assert!(cargo_content.contains("existing-project"));
    
    let main_content = fs::read_to_string(temp_repo.path().join("src/main.rs")).unwrap();
    assert!(main_content.contains("Hello, existing world!"));
    
    let lib_content = fs::read_to_string(temp_repo.path().join("src/lib.rs")).unwrap();
    assert!(lib_content.contains("existing_function"));
    assert!(lib_content.contains("#[cfg(test)]"));
}

#[tokio::test]
async fn test_partial_initialization_temp_creation() {
    let fixture = RepositoryStateFixture::repository_with_partial_initialization();
    let temp_repo = fixture.create_temp_repository().unwrap();
    
    // Verify partial clambake setup exists
    assert!(temp_repo.path().join("my-little-soda.toml").exists());
    assert!(temp_repo.path().join(".my-little-soda/partial_setup").exists());
    
    // Verify existing config content
    let config_content = fs::read_to_string(temp_repo.path().join("my-little-soda.toml")).unwrap();
    assert!(config_content.contains("old-owner"));
    assert!(config_content.contains("old-repo"));
    assert!(config_content.contains("tracing_enabled = false"));
}

#[tokio::test]
async fn test_conflicts_temp_creation() {
    let fixture = RepositoryStateFixture::repository_with_conflicts();
    let temp_repo = fixture.create_temp_repository().unwrap();
    
    // Verify conflicted files exist with conflict markers
    let main_rs = fs::read_to_string(temp_repo.path().join("src/main.rs")).unwrap();
    assert!(main_rs.contains("<<<<<<< HEAD"));
    assert!(main_rs.contains("======="));
    assert!(main_rs.contains(">>>>>>> feature-branch"));
    
    let cargo_toml = fs::read_to_string(temp_repo.path().join("Cargo.toml")).unwrap();
    assert!(cargo_toml.contains("<<<<<<< HEAD"));
    assert!(cargo_toml.contains("serde"));
    assert!(cargo_toml.contains("tokio"));
}

#[test]
fn test_fixture_validation_comprehensive() {
    let fixtures = RepositoryStateFixture::all_fixtures();
    
    for fixture in fixtures {
        // Every fixture should have basic properties
        assert!(!fixture.name.is_empty(), "Fixture name should not be empty");
        assert!(!fixture.description.is_empty(), "Fixture description should not be empty");
        assert!(!fixture.files.is_empty(), "Fixture should have at least one file");
        
        // Git config should be valid
        assert!(!fixture.git_config.current_branch.is_empty(), "Should have current branch");
        
        // Expected behavior should be consistent with fixture characteristics
        let behavior = fixture.expected_init_behavior();
        
        match fixture.name.as_str() {
            "empty_repository" | "repository_with_existing_files" | "repository_with_complex_directory_structure" | "repository_with_existing_issue_templates" | "repository_with_existing_cicd_files" => {
                assert!(behavior.should_succeed_without_force, 
                        "Clean repos should succeed without force: {}", fixture.name);
                assert!(behavior.validation_warnings.is_empty(), 
                        "Clean repos should have no warnings: {}", fixture.name);
            },
            "repository_with_partial_initialization" | "repository_with_conflicts" => {
                assert!(!behavior.should_succeed_without_force, 
                        "Problematic repos should fail without force: {}", fixture.name);
                assert!(!behavior.validation_warnings.is_empty(), 
                        "Problematic repos should have warnings: {}", fixture.name);
            },
            _ => panic!("Unknown fixture type: {}", fixture.name),
        }
    }
}