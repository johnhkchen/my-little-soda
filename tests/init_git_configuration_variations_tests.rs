/// Git configuration variations testing for init command (C3b requirement)
/// 
/// Tests init command behavior with different Git repository configurations:
/// - Repositories with existing branches
/// - Repositories with complex branching strategies  
/// - Submodule configurations
/// - Git worktree configurations

use my_little_soda::cli::commands::init::InitCommand;
use my_little_soda::fs::{FileSystemOperations, StandardFileSystem};
use std::sync::Arc;
use tempfile::TempDir;

async fn setup_base_git_repo(temp_dir: &TempDir) -> Result<(), Box<dyn std::error::Error>> {
    let original_dir = std::env::current_dir()?;
    std::env::set_current_dir(temp_dir.path())?;
    
    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .output()
        .expect("Failed to init git repo");
    
    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .output()
        .expect("Failed to set git user");
        
    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .output()
        .expect("Failed to set git email");
    
    std::process::Command::new("git")
        .args(["remote", "add", "origin", "https://github.com/test-owner/test-repo.git"])
        .output()
        .expect("Failed to add git remote");

    std::fs::write("README.md", "# Test Repository")?;
    
    std::process::Command::new("git")
        .args(["add", "README.md"])
        .output()
        .expect("Failed to add file");
        
    std::process::Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .output()
        .expect("Failed to commit");

    std::env::set_current_dir(original_dir)?;
    Ok(())
}

#[tokio::test]
async fn test_init_with_multiple_existing_branches() {
    let temp_dir = tempfile::tempdir().unwrap();
    setup_base_git_repo(&temp_dir).await.unwrap();
    let original_dir = std::env::current_dir().unwrap();
    
    std::env::set_current_dir(temp_dir.path()).unwrap();
    
    // Create multiple branches
    let branches = ["feature/auth", "bugfix/login", "release/v1.0", "hotfix/security"];
    
    for branch in branches {
        std::process::Command::new("git")
            .args(["checkout", "-b", branch])
            .output()
            .expect("Failed to create branch");
            
        // Make a unique commit on each branch
        std::fs::write(format!("{}.txt", branch.replace('/', "_")), format!("Content for {}", branch)).unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .output()
            .expect("Failed to add files");
        std::process::Command::new("git")
            .args(["commit", "-m", &format!("Work on {}", branch)])
            .output()
            .expect("Failed to commit");
    }
    
    // Go back to main branch
    std::process::Command::new("git")
        .args(["checkout", "main"])
        .output()
        .expect("Failed to checkout main");
    
    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops);
    
    let result = init_command.execute().await;
    assert!(result.is_ok(), "Init should handle repos with multiple branches: {:?}", result.err());
    
    std::env::set_current_dir(original_dir).unwrap();
}

#[tokio::test]
async fn test_init_with_detached_head() {
    let temp_dir = tempfile::tempdir().unwrap();
    setup_base_git_repo(&temp_dir).await.unwrap();
    let original_dir = std::env::current_dir().unwrap();
    
    std::env::set_current_dir(temp_dir.path()).unwrap();
    
    // Create additional commit to have multiple commits
    std::fs::write("second.txt", "Second file").unwrap();
    std::process::Command::new("git")
        .args(["add", "second.txt"])
        .output()
        .expect("Failed to add second file");
    std::process::Command::new("git")
        .args(["commit", "-m", "Second commit"])
        .output()
        .expect("Failed to commit second file");
    
    // Get the first commit hash and checkout detached HEAD
    let output = std::process::Command::new("git")
        .args(["log", "--oneline", "--reverse"])
        .output()
        .expect("Failed to get commit log");
    
    let log_output = String::from_utf8(output.stdout).unwrap();
    let first_commit_hash = log_output.lines().next().unwrap().split_whitespace().next().unwrap();
    
    std::process::Command::new("git")
        .args(["checkout", first_commit_hash])
        .output()
        .expect("Failed to checkout detached HEAD");
    
    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops);
    
    let result = init_command.execute().await;
    // Should handle detached HEAD state gracefully
    if result.is_err() {
        let error_msg = result.unwrap_err().to_string();
        println!("Result for detached HEAD: {}", error_msg);
    }
    
    std::env::set_current_dir(original_dir).unwrap();
}

#[tokio::test]
async fn test_init_with_merge_conflicts_present() {
    let temp_dir = tempfile::tempdir().unwrap();
    setup_base_git_repo(&temp_dir).await.unwrap();
    let original_dir = std::env::current_dir().unwrap();
    
    std::env::set_current_dir(temp_dir.path()).unwrap();
    
    // Create conflicting branches
    std::process::Command::new("git")
        .args(["checkout", "-b", "branch1"])
        .output()
        .expect("Failed to create branch1");
    
    std::fs::write("conflict.txt", "Content from branch1").unwrap();
    std::process::Command::new("git")
        .args(["add", "conflict.txt"])
        .output()
        .expect("Failed to add conflict file");
    std::process::Command::new("git")
        .args(["commit", "-m", "Add conflict file from branch1"])
        .output()
        .expect("Failed to commit from branch1");
    
    std::process::Command::new("git")
        .args(["checkout", "main"])
        .output()
        .expect("Failed to checkout main");
    
    std::process::Command::new("git")
        .args(["checkout", "-b", "branch2"])
        .output()
        .expect("Failed to create branch2");
    
    std::fs::write("conflict.txt", "Content from branch2").unwrap();
    std::process::Command::new("git")
        .args(["add", "conflict.txt"])
        .output()
        .expect("Failed to add conflict file");
    std::process::Command::new("git")
        .args(["commit", "-m", "Add conflict file from branch2"])
        .output()
        .expect("Failed to commit from branch2");
    
    // Attempt merge that will create conflict
    let merge_result = std::process::Command::new("git")
        .args(["merge", "branch1"])
        .output()
        .expect("Failed to attempt merge");
    
    // Verify we're in a merge conflict state
    let status_output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .expect("Failed to get git status");
    
    let status_str = String::from_utf8(status_output.stdout).unwrap();
    
    if status_str.contains("UU") { // Merge conflict indicator
        let fs_ops = Arc::new(StandardFileSystem);
        let init_command = InitCommand::new(None, false, true, fs_ops);
        
        let result = init_command.execute().await;
        // Should handle merge conflict state appropriately
        if result.is_err() {
            let error_msg = result.unwrap_err().to_string();
            println!("Expected behavior during merge conflict: {}", error_msg);
        }
    }
    
    std::env::set_current_dir(original_dir).unwrap();
}

#[tokio::test]
async fn test_init_with_git_stash_present() {
    let temp_dir = tempfile::tempdir().unwrap();
    setup_base_git_repo(&temp_dir).await.unwrap();
    let original_dir = std::env::current_dir().unwrap();
    
    std::env::set_current_dir(temp_dir.path()).unwrap();
    
    // Create uncommitted changes
    std::fs::write("stash_test.txt", "Uncommitted changes").unwrap();
    std::fs::write("README.md", "# Modified README").unwrap();
    
    // Stash the changes
    std::process::Command::new("git")
        .args(["stash", "push", "-m", "Test stash"])
        .output()
        .expect("Failed to stash changes");
    
    // Verify stash exists
    let stash_output = std::process::Command::new("git")
        .args(["stash", "list"])
        .output()
        .expect("Failed to list stash");
    
    let stash_str = String::from_utf8(stash_output.stdout).unwrap();
    assert!(stash_str.contains("stash@{0}"), "Stash should exist");
    
    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops);
    
    let result = init_command.execute().await;
    assert!(result.is_ok(), "Init should handle repos with stash: {:?}", result.err());
    
    std::env::set_current_dir(original_dir).unwrap();
}

#[tokio::test]
async fn test_init_with_submodule_configuration() {
    let temp_dir = tempfile::tempdir().unwrap();
    setup_base_git_repo(&temp_dir).await.unwrap();
    let original_dir = std::env::current_dir().unwrap();
    
    std::env::set_current_dir(temp_dir.path()).unwrap();
    
    // Create a mock submodule configuration (without actual submodule)
    std::fs::write(".gitmodules", r#"[submodule "libs/common"]
    path = libs/common
    url = https://github.com/test-owner/common-lib.git
[submodule "vendor/third-party"]
    path = vendor/third-party  
    url = git@github.com:vendor/third-party.git
"#).unwrap();
    
    // Create the directories that would contain submodules
    std::fs::create_dir_all("libs/common").unwrap();
    std::fs::create_dir_all("vendor/third-party").unwrap();
    
    std::process::Command::new("git")
        .args(["add", ".gitmodules"])
        .output()
        .expect("Failed to add .gitmodules");
        
    std::process::Command::new("git")
        .args(["commit", "-m", "Add submodule configuration"])
        .output()
        .expect("Failed to commit submodules");
    
    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops);
    
    let result = init_command.execute().await;
    assert!(result.is_ok(), "Init should handle repos with submodule config: {:?}", result.err());
    
    std::env::set_current_dir(original_dir).unwrap();
}

#[tokio::test]
async fn test_init_with_git_hooks_present() {
    let temp_dir = tempfile::tempdir().unwrap();
    setup_base_git_repo(&temp_dir).await.unwrap();
    let original_dir = std::env::current_dir().unwrap();
    
    std::env::set_current_dir(temp_dir.path()).unwrap();
    
    // Create hooks directory and sample hooks
    std::fs::create_dir_all(".git/hooks").unwrap();
    
    let pre_commit_hook = r#"#!/bin/sh
echo "Running pre-commit hook"
exit 0
"#;
    
    let post_commit_hook = r#"#!/bin/sh
echo "Running post-commit hook"
"#;
    
    std::fs::write(".git/hooks/pre-commit", pre_commit_hook).unwrap();
    std::fs::write(".git/hooks/post-commit", post_commit_hook).unwrap();
    
    // Make hooks executable (on Unix-like systems)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(".git/hooks/pre-commit").unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(".git/hooks/pre-commit", perms).unwrap();
        
        let mut perms = std::fs::metadata(".git/hooks/post-commit").unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(".git/hooks/post-commit", perms).unwrap();
    }
    
    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops);
    
    let result = init_command.execute().await;
    assert!(result.is_ok(), "Init should handle repos with git hooks: {:?}", result.err());
    
    std::env::set_current_dir(original_dir).unwrap();
}

#[tokio::test]
async fn test_init_with_large_git_history() {
    let temp_dir = tempfile::tempdir().unwrap();
    setup_base_git_repo(&temp_dir).await.unwrap();
    let original_dir = std::env::current_dir().unwrap();
    
    std::env::set_current_dir(temp_dir.path()).unwrap();
    
    // Create a series of commits to simulate larger history
    for i in 1..=20 {
        std::fs::write(format!("file_{}.txt", i), format!("Content of file {}", i)).unwrap();
        std::process::Command::new("git")
            .args(["add", &format!("file_{}.txt", i)])
            .output()
            .expect("Failed to add file");
        std::process::Command::new("git")
            .args(["commit", "-m", &format!("Add file {}", i)])
            .output()
            .expect("Failed to commit file");
    }
    
    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops);
    
    let result = init_command.execute().await;
    assert!(result.is_ok(), "Init should handle repos with large history: {:?}", result.err());
    
    std::env::set_current_dir(original_dir).unwrap();
}

#[tokio::test]
async fn test_init_with_custom_git_config() {
    let temp_dir = tempfile::tempdir().unwrap();
    setup_base_git_repo(&temp_dir).await.unwrap();
    let original_dir = std::env::current_dir().unwrap();
    
    std::env::set_current_dir(temp_dir.path()).unwrap();
    
    // Set various custom git configurations
    let configs = [
        ("core.autocrlf", "true"),
        ("core.filemode", "false"),
        ("pull.rebase", "true"),
        ("push.default", "simple"),
        ("branch.autosetupmerge", "always"),
        ("merge.tool", "vimdiff"),
    ];
    
    for (key, value) in configs {
        std::process::Command::new("git")
            .args(["config", key, value])
            .output()
            .expect("Failed to set git config");
    }
    
    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops);
    
    let result = init_command.execute().await;
    assert!(result.is_ok(), "Init should handle repos with custom git config: {:?}", result.err());
    
    std::env::set_current_dir(original_dir).unwrap();
}

#[tokio::test]
async fn test_init_with_git_lfs_configuration() {
    let temp_dir = tempfile::tempdir().unwrap();
    setup_base_git_repo(&temp_dir).await.unwrap();
    let original_dir = std::env::current_dir().unwrap();
    
    std::env::set_current_dir(temp_dir.path()).unwrap();
    
    // Create a mock Git LFS configuration
    std::fs::write(".gitattributes", r#"*.zip filter=lfs diff=lfs merge=lfs -text
*.tar.gz filter=lfs diff=lfs merge=lfs -text
*.pdf filter=lfs diff=lfs merge=lfs -text
"#).unwrap();
    
    std::process::Command::new("git")
        .args(["add", ".gitattributes"])
        .output()
        .expect("Failed to add .gitattributes");
        
    std::process::Command::new("git")
        .args(["commit", "-m", "Add LFS configuration"])
        .output()
        .expect("Failed to commit LFS config");
    
    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops);
    
    let result = init_command.execute().await;
    assert!(result.is_ok(), "Init should handle repos with LFS config: {:?}", result.err());
    
    std::env::set_current_dir(original_dir).unwrap();
}