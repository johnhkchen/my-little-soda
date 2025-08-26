/// Real file system integration tests using temporary directories
/// Tests actual file creation, directory structure validation, and Git operations
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;
use anyhow::Result;

mod fixtures;
use fixtures::test_harness::*;
use fixtures::test_harness::helpers::*;
use fixtures::repository_states::*;

#[test]
fn test_real_file_creation_with_content_validation() {
    let harness = simple_harness().unwrap();
    
    let file_content = "# Test File\n\nThis is a real file with actual content.\n\n## Features\n- Feature 1\n- Feature 2\n";
    let file_path = harness.create_file("docs/README.md", file_content).unwrap();
    
    assert!(file_path.exists());
    assert!(file_path.is_file());
    
    let actual_content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(actual_content, file_content);
    
    let parent_dir = file_path.parent().unwrap();
    assert!(parent_dir.exists());
    assert!(parent_dir.is_dir());
    assert_eq!(parent_dir.file_name().unwrap(), "docs");
}

#[test]
fn test_complex_directory_structure_creation() {
    let harness = simple_harness().unwrap();
    
    let structure = vec![
        ("src/main.rs", "fn main() { println!(\"Hello, world!\"); }"),
        ("src/lib.rs", "pub mod utils;\npub mod config;"),
        ("src/utils/mod.rs", "pub fn helper() -> String { \"helper\".to_string() }"),
        ("src/config/mod.rs", "pub struct Config { pub debug: bool }"),
        ("tests/integration_test.rs", "#[test]\nfn test_integration() { assert!(true); }"),
        ("docs/api/README.md", "# API Documentation"),
        ("examples/basic.rs", "fn main() { println!(\"Example\"); }"),
        (".github/workflows/ci.yml", "name: CI\non: [push, pull_request]"),
    ];
    
    for (path, content) in &structure {
        harness.create_file(path, content).unwrap();
    }
    
    for (path, expected_content) in &structure {
        let full_path = harness.path().join(path);
        assert!(full_path.exists(), "File should exist: {}", path);
        
        let actual_content = fs::read_to_string(&full_path)
            .unwrap_or_else(|_| panic!("Failed to read file: {}", path));
        assert_eq!(actual_content, *expected_content);
        
        if let Some(parent) = full_path.parent() {
            assert!(parent.exists(), "Parent directory should exist: {}", parent.display());
            assert!(parent.is_dir(), "Parent should be directory: {}", parent.display());
        }
    }
}

#[test]
fn test_file_system_state_validation() {
    let harness = rust_project_harness("fs-validation-test").unwrap();
    
    let expected_files = vec![
        "Cargo.toml",
        "src/main.rs", 
        "README.md",
        ".gitignore",
    ];
    
    for file in &expected_files {
        let file_path = harness.path().join(file);
        assert!(file_path.exists(), "Expected file should exist: {}", file);
        assert!(file_path.is_file(), "Path should be a file: {}", file);
        
        let metadata = fs::metadata(&file_path).unwrap();
        assert!(metadata.len() > 0, "File should not be empty: {}", file);
    }
    
    let expected_dirs = vec![
        "src",
        ".git",
    ];
    
    for dir in &expected_dirs {
        let dir_path = harness.path().join(dir);
        assert!(dir_path.exists(), "Expected directory should exist: {}", dir);
        assert!(dir_path.is_dir(), "Path should be a directory: {}", dir);
    }
}

#[test]
fn test_generated_file_contents_validation() {
    let harness = rust_project_harness("content-validation").unwrap();
    
    let cargo_content = fs::read_to_string(harness.path().join("Cargo.toml")).unwrap();
    assert!(cargo_content.contains("content-validation"));
    assert!(cargo_content.contains("edition = \"2021\""));
    assert!(cargo_content.contains("anyhow"));
    
    let main_content = fs::read_to_string(harness.path().join("src/main.rs")).unwrap();
    assert!(main_content.contains("fn main()"));
    assert!(main_content.contains("Hello, world!"));
    
    let readme_content = fs::read_to_string(harness.path().join("README.md")).unwrap();
    assert!(readme_content.contains("content-validation"));
    assert!(readme_content.contains("A test Rust project"));
    
    let gitignore_content = fs::read_to_string(harness.path().join(".gitignore")).unwrap();
    assert!(gitignore_content.contains("target/"));
    assert!(gitignore_content.contains("*.log"));
}

#[test]
fn test_real_git_operations_integration() {
    let harness = git_harness().unwrap();
    
    let output = Command::new("git")
        .args(["status"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    
    harness.create_file("git_test.txt", "Git integration test content").unwrap();
    
    let status_output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    let status_text = String::from_utf8_lossy(&status_output.stdout);
    assert!(status_text.contains("git_test.txt"));
    
    harness.commit_all("Add git test file").unwrap();
    
    let log_output = Command::new("git")
        .args(["log", "--oneline", "-1"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    let log_text = String::from_utf8_lossy(&log_output.stdout);
    assert!(log_text.contains("Add git test file"));
    
    let clean_status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    let clean_text = String::from_utf8_lossy(&clean_status.stdout);
    assert!(clean_text.is_empty());
}

#[test]
fn test_git_remote_and_branch_operations() {
    let harness = git_harness().unwrap();
    
    harness.add_git_remote("origin", "https://github.com/test-owner/test-repo.git").unwrap();
    harness.add_git_remote("upstream", "https://github.com/upstream/test-repo.git").unwrap();
    
    let remotes_output = Command::new("git")
        .args(["remote", "-v"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    let remotes_text = String::from_utf8_lossy(&remotes_output.stdout);
    assert!(remotes_text.contains("origin"));
    assert!(remotes_text.contains("upstream"));
    assert!(remotes_text.contains("https://github.com/test-owner/test-repo.git"));
    assert!(remotes_text.contains("https://github.com/upstream/test-repo.git"));
    
    let branch_output = Command::new("git")
        .args(["branch"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    let branch_text = String::from_utf8_lossy(&branch_output.stdout);
    assert!(branch_text.contains("main") || branch_text.contains("master"));
}

#[test]
fn test_multiple_commits_and_history() {
    let harness = git_harness().unwrap();
    
    let commits = vec![
        ("first.txt", "First file content", "Add first file"),
        ("second.txt", "Second file content", "Add second file"),
        ("third.txt", "Third file content", "Add third file"),
    ];
    
    for (filename, content, commit_msg) in &commits {
        harness.create_file(filename, content).unwrap();
        harness.commit_all(commit_msg).unwrap();
    }
    
    let log_output = Command::new("git")
        .args(["log", "--oneline"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    let log_text = String::from_utf8_lossy(&log_output.stdout);
    
    for (_, _, commit_msg) in &commits {
        assert!(log_text.contains(commit_msg));
    }
    
    let commit_count = log_text.lines().count();
    assert_eq!(commit_count, 3);
}

#[test]
fn test_repository_fixture_file_system_integration() {
    let fixture = RepositoryStateFixture::repository_with_existing_files();
    let temp_repo = fixture.create_temp_repository().unwrap();
    
    assert!(temp_repo.path().join("Cargo.toml").exists());
    assert!(temp_repo.path().join("src/main.rs").exists());
    assert!(temp_repo.path().join("src/lib.rs").exists());
    
    let cargo_content = fs::read_to_string(temp_repo.path().join("Cargo.toml")).unwrap();
    assert!(cargo_content.contains("existing-project"));
    
    let lib_content = fs::read_to_string(temp_repo.path().join("src/lib.rs")).unwrap();
    assert!(lib_content.contains("existing_function"));
    assert!(lib_content.contains("#[test]"));
    
    let git_status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(temp_repo.path())
        .output()
        .unwrap();
    assert!(git_status.status.success());
}

#[test]
fn test_conflicted_repository_file_system_state() {
    let fixture = RepositoryStateFixture::repository_with_conflicts();
    let temp_repo = fixture.create_temp_repository().unwrap();
    
    let main_rs_path = temp_repo.path().join("src/main.rs");
    assert!(main_rs_path.exists());
    
    let main_content = fs::read_to_string(&main_rs_path).unwrap();
    assert!(main_content.contains("<<<<<<< HEAD"));
    assert!(main_content.contains("======="));
    assert!(main_content.contains(">>>>>>> feature-branch"));
    
    let cargo_path = temp_repo.path().join("Cargo.toml");
    let cargo_content = fs::read_to_string(&cargo_path).unwrap();
    assert!(cargo_content.contains("<<<<<<< HEAD"));
    assert!(cargo_content.contains("serde"));
    assert!(cargo_content.contains("tokio"));
}

#[test]
fn test_file_permissions_and_attributes() {
    let harness = simple_harness().unwrap();
    
    harness.create_file("executable.sh", "#!/bin/bash\necho 'Hello World'").unwrap();
    harness.create_file("data.json", "{\"test\": true}").unwrap();
    
    let executable_path = harness.path().join("executable.sh");
    let data_path = harness.path().join("data.json");
    
    assert!(executable_path.exists());
    assert!(data_path.exists());
    
    let exe_metadata = fs::metadata(&executable_path).unwrap();
    let data_metadata = fs::metadata(&data_path).unwrap();
    
    assert!(exe_metadata.is_file());
    assert!(data_metadata.is_file());
    assert!(exe_metadata.len() > 0);
    assert!(data_metadata.len() > 0);
}

#[test]
fn test_large_file_creation_and_validation() {
    let harness = simple_harness().unwrap();
    
    let large_content = "0123456789\n".repeat(1000); // ~11KB file
    harness.create_file("large_file.txt", &large_content).unwrap();
    
    let large_path = harness.path().join("large_file.txt");
    assert!(large_path.exists());
    
    let metadata = fs::metadata(&large_path).unwrap();
    assert!(metadata.len() > 10000); // Should be > 10KB
    
    let actual_content = fs::read_to_string(&large_path).unwrap();
    assert_eq!(actual_content, large_content);
    assert_eq!(actual_content.lines().count(), 1000);
}

#[test]
fn test_binary_file_handling() {
    let harness = simple_harness().unwrap();
    
    let binary_data: Vec<u8> = (0..=255).collect();
    let binary_path = harness.path().join("binary_file.bin");
    
    fs::write(&binary_path, &binary_data).unwrap();
    
    assert!(binary_path.exists());
    let read_data = fs::read(&binary_path).unwrap();
    assert_eq!(read_data, binary_data);
    assert_eq!(read_data.len(), 256);
}

#[test]
fn test_proper_cleanup_verification() {
    let temp_paths: Vec<_> = (0..3).map(|i| {
        let harness = simple_harness().unwrap();
        let path = harness.path().to_path_buf();
        
        harness.create_file(&format!("cleanup_test_{}.txt", i), "cleanup test").unwrap();
        assert!(path.join(&format!("cleanup_test_{}.txt", i)).exists());
        
        path
    }).collect();
    
    for path in temp_paths {
        assert!(!path.exists(), "Temporary directory should be cleaned up: {}", path.display());
    }
}

#[test]
fn test_isolation_between_test_environments() {
    let harness1 = rust_project_harness("isolation-test-1").unwrap();
    let harness2 = rust_project_harness("isolation-test-2").unwrap();
    
    harness1.create_file("unique_file_1.txt", "content from harness 1").unwrap();
    harness2.create_file("unique_file_2.txt", "content from harness 2").unwrap();
    
    assert!(harness1.path().join("unique_file_1.txt").exists());
    assert!(!harness1.path().join("unique_file_2.txt").exists());
    
    assert!(harness2.path().join("unique_file_2.txt").exists());
    assert!(!harness2.path().join("unique_file_1.txt").exists());
    
    assert_ne!(harness1.path(), harness2.path());
}

#[test]
fn test_concurrent_file_operations() {
    let harness = simple_harness().unwrap();
    
    let files_data = vec![
        ("concurrent_1.txt", "Concurrent file 1 content"),
        ("concurrent_2.txt", "Concurrent file 2 content"),
        ("concurrent_3.txt", "Concurrent file 3 content"),
    ];
    
    for (filename, content) in &files_data {
        harness.create_file(filename, content).unwrap();
    }
    
    for (filename, expected_content) in &files_data {
        let file_path = harness.path().join(filename);
        assert!(file_path.exists());
        
        let actual_content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(actual_content, *expected_content);
    }
}

#[test]
fn test_git_operations_in_test_environment() {
    let harness = TestHarnessBuilder::new()
        .with_rust_project("git-ops-test")
        .with_git()
        .with_git_remote("origin", "https://github.com/test/git-ops-test.git")
        .with_initial_commit("Initial test commit")
        .build()
        .unwrap();
    
    harness.create_file("feature.rs", "pub fn new_feature() {}").unwrap();
    
    let diff_output = Command::new("git")
        .args(["diff", "--name-only"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    let diff_text = String::from_utf8_lossy(&diff_output.stdout);
    assert!(diff_text.contains("feature.rs"));
    
    harness.commit_all("Add new feature").unwrap();
    
    let show_output = Command::new("git")
        .args(["show", "--name-only", "HEAD"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    let show_text = String::from_utf8_lossy(&show_output.stdout);
    assert!(show_text.contains("Add new feature"));
    assert!(show_text.contains("feature.rs"));
}