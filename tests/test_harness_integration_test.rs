/// Integration tests for the test harness
use std::process::Command;

mod fixtures;
use fixtures::test_harness::*;
use fixtures::test_harness::helpers::*;

#[test]
fn test_basic_harness_creation() {
    let harness = simple_harness().unwrap();
    assert!(harness.path().exists());
    assert!(harness.path().is_dir());
}

#[test]
fn test_file_creation_and_isolation() {
    let harness = simple_harness().unwrap();
    
    let file_path = harness.create_file("test.txt", "Hello, test harness!").unwrap();
    assert!(file_path.exists());
    
    let content = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, "Hello, test harness!");
    
    let relative = harness.relative_path(&file_path).unwrap();
    assert_eq!(relative.to_string_lossy(), "test.txt");
}

#[test]
fn test_nested_directory_structure() {
    let harness = simple_harness().unwrap();
    
    harness.create_dir("src/modules").unwrap();
    let nested_file = harness.create_file("src/modules/test.rs", "// Nested Rust file").unwrap();
    
    assert!(nested_file.exists());
    let content = std::fs::read_to_string(&nested_file).unwrap();
    assert_eq!(content, "// Nested Rust file");
}

#[test]
fn test_git_repository_initialization() {
    let harness = git_harness().unwrap();
    
    let git_dir = harness.path().join(".git");
    assert!(git_dir.exists());
    assert!(git_dir.is_dir());
    
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    
    assert!(output.status.success());
}

#[test]
fn test_rust_project_harness() {
    let harness = rust_project_harness("test-project").unwrap();
    
    assert!(harness.path().join("Cargo.toml").exists());
    assert!(harness.path().join("src").exists());
    assert!(harness.path().join("src/main.rs").exists());
    assert!(harness.path().join("README.md").exists());
    assert!(harness.path().join(".gitignore").exists());
    assert!(harness.path().join(".git").exists());
    
    let cargo_content = std::fs::read_to_string(harness.path().join("Cargo.toml")).unwrap();
    assert!(cargo_content.contains("test-project"));
    assert!(cargo_content.contains("anyhow"));
    
    let output = Command::new("git")
        .args(["log", "--oneline"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    
    assert!(output.status.success());
    let log_output = String::from_utf8_lossy(&output.stdout);
    assert!(log_output.contains("Initial commit"));
}

#[test]
fn test_builder_pattern_comprehensive() {
    let harness = TestHarnessBuilder::new()
        .with_rust_project("builder-project")
        .with_git()
        .with_git_remote("origin", "https://github.com/test-owner/builder-repo.git")
        .with_initial_commit("Test builder commit")
        .build()
        .unwrap();
    
    assert!(harness.path().join("Cargo.toml").exists());
    assert!(harness.path().join(".git").exists());
    
    let output = Command::new("git")
        .args(["remote", "-v"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    
    let remote_output = String::from_utf8_lossy(&output.stdout);
    assert!(remote_output.contains("https://github.com/test-owner/builder-repo.git"));
    
    let log_output = Command::new("git")
        .args(["log", "--oneline", "-1"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    
    let log_content = String::from_utf8_lossy(&log_output.stdout);
    assert!(log_content.contains("Test builder commit"));
}

#[test]
fn test_parallel_harness_isolation() {
    let harnesses = parallel_harnesses(3).unwrap();
    assert_eq!(harnesses.len(), 3);
    
    for (i, harness) in harnesses.iter().enumerate() {
        let test_file = format!("isolation_test_{}.txt", i);
        let unique_content = format!("This is harness {}", i);
        
        harness.create_file(&test_file, &unique_content).unwrap();
        
        let file_path = harness.path().join(&test_file);
        assert!(file_path.exists());
        
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, unique_content);
        
        for (j, other_harness) in harnesses.iter().enumerate() {
            if i != j {
                let other_file_path = other_harness.path().join(&test_file);
                assert!(!other_file_path.exists(), "File should not exist in harness {}", j);
            }
        }
    }
}

#[test]
fn test_harness_cleanup_on_drop() {
    let temp_path = {
        let harness = simple_harness().unwrap();
        let path = harness.path().to_path_buf();
        
        harness.create_file("temp_file.txt", "This should be cleaned up").unwrap();
        assert!(path.join("temp_file.txt").exists());
        
        path
    };
    
    assert!(!temp_path.exists(), "Temporary directory should be cleaned up when harness is dropped");
}

#[test]
fn test_git_commit_functionality() {
    let harness = git_harness().unwrap();
    
    harness.create_file("test_commit.txt", "File for commit test").unwrap();
    harness.commit_all("Add test file").unwrap();
    
    let output = Command::new("git")
        .args(["log", "--oneline", "-1"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    
    let log_output = String::from_utf8_lossy(&output.stdout);
    assert!(log_output.contains("Add test file"));
    
    let status_output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    
    let status_content = String::from_utf8_lossy(&status_output.stdout);
    assert!(status_content.is_empty(), "Repository should be clean after commit");
}

#[test]
fn test_isolation_verification() {
    let harness = simple_harness().unwrap();
    
    harness.verify_isolation().unwrap();
    
    harness.create_file("isolation_check.txt", "isolation test").unwrap();
    harness.verify_isolation().unwrap();
}

#[test]
fn test_multiple_git_operations() {
    let harness = git_harness().unwrap();
    
    harness.add_git_remote("upstream", "https://github.com/upstream/repo.git").unwrap();
    
    let output = Command::new("git")
        .args(["remote", "-v"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    
    let remote_output = String::from_utf8_lossy(&output.stdout);
    assert!(remote_output.contains("upstream"));
    assert!(remote_output.contains("https://github.com/upstream/repo.git"));
    
    harness.create_file("multi_test.txt", "Multiple git operations test").unwrap();
    harness.commit_all("Test multiple git operations").unwrap();
    
    let log_output = Command::new("git")
        .args(["log", "--oneline"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    
    let log_content = String::from_utf8_lossy(&log_output.stdout);
    assert!(log_content.contains("Test multiple git operations"));
}

#[test]
fn test_error_handling() {
    let harness = simple_harness().unwrap();
    
    let result = harness.create_file("", "empty filename");
    assert!(result.is_err(), "Should fail with empty filename");
    
    let result = Command::new("git")
        .args(["status"])
        .current_dir(harness.path())
        .output();
    
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(!output.status.success(), "Git status should fail in non-git directory");
}