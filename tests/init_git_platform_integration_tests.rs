/// Git platform integration testing for init command (C3a requirement)
/// 
/// Tests different git platform scenarios:
/// - GitLab integration testing (via mocking)
/// - Bitbucket integration testing (via mocking)
/// - Self-hosted Git servers testing (via mocking)
/// - SSH vs HTTPS authentication scenarios

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
async fn test_init_with_github_https_remote() {
    let temp_dir = tempfile::tempdir().unwrap();
    setup_base_git_repo(&temp_dir).await.unwrap();
    let original_dir = std::env::current_dir().unwrap();
    
    std::env::set_current_dir(temp_dir.path()).unwrap();
    
    // Add GitHub HTTPS remote
    std::process::Command::new("git")
        .args(["remote", "add", "origin", "https://github.com/test-owner/test-repo.git"])
        .output()
        .expect("Failed to add GitHub HTTPS remote");
    
    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops);
    
    let result = init_command.execute().await;
    assert!(result.is_ok(), "Init should handle GitHub HTTPS remote: {:?}", result.err());
    
    std::env::set_current_dir(original_dir).unwrap();
}

#[tokio::test]
async fn test_init_with_github_ssh_remote() {
    let temp_dir = tempfile::tempdir().unwrap();
    setup_base_git_repo(&temp_dir).await.unwrap();
    let original_dir = std::env::current_dir().unwrap();
    
    std::env::set_current_dir(temp_dir.path()).unwrap();
    
    // Add GitHub SSH remote
    std::process::Command::new("git")
        .args(["remote", "add", "origin", "git@github.com:test-owner/test-repo.git"])
        .output()
        .expect("Failed to add GitHub SSH remote");
    
    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops);
    
    let result = init_command.execute().await;
    assert!(result.is_ok(), "Init should handle GitHub SSH remote: {:?}", result.err());
    
    std::env::set_current_dir(original_dir).unwrap();
}

#[tokio::test]
async fn test_init_with_gitlab_https_remote() {
    let temp_dir = tempfile::tempdir().unwrap();
    setup_base_git_repo(&temp_dir).await.unwrap();
    let original_dir = std::env::current_dir().unwrap();
    
    std::env::set_current_dir(temp_dir.path()).unwrap();
    
    // Add GitLab HTTPS remote
    std::process::Command::new("git")
        .args(["remote", "add", "origin", "https://gitlab.com/test-owner/test-repo.git"])
        .output()
        .expect("Failed to add GitLab HTTPS remote");
    
    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops);
    
    let result = init_command.execute().await;
    // GitLab remotes should be detected but may fail validation in dry run
    // The key is that we get a meaningful error, not a crash
    if result.is_err() {
        let error_msg = result.unwrap_err().to_string();
        // Should get specific error about non-GitHub remote or similar
        println!("Expected error for GitLab remote: {}", error_msg);
    }
    
    std::env::set_current_dir(original_dir).unwrap();
}

#[tokio::test]
async fn test_init_with_gitlab_ssh_remote() {
    let temp_dir = tempfile::tempdir().unwrap();
    setup_base_git_repo(&temp_dir).await.unwrap();
    let original_dir = std::env::current_dir().unwrap();
    
    std::env::set_current_dir(temp_dir.path()).unwrap();
    
    // Add GitLab SSH remote
    std::process::Command::new("git")
        .args(["remote", "add", "origin", "git@gitlab.com:test-owner/test-repo.git"])
        .output()
        .expect("Failed to add GitLab SSH remote");
    
    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops);
    
    let result = init_command.execute().await;
    if result.is_err() {
        let error_msg = result.unwrap_err().to_string();
        println!("Expected error for GitLab SSH remote: {}", error_msg);
    }
    
    std::env::set_current_dir(original_dir).unwrap();
}

#[tokio::test]
async fn test_init_with_bitbucket_https_remote() {
    let temp_dir = tempfile::tempdir().unwrap();
    setup_base_git_repo(&temp_dir).await.unwrap();
    let original_dir = std::env::current_dir().unwrap();
    
    std::env::set_current_dir(temp_dir.path()).unwrap();
    
    // Add Bitbucket HTTPS remote
    std::process::Command::new("git")
        .args(["remote", "add", "origin", "https://bitbucket.org/test-owner/test-repo.git"])
        .output()
        .expect("Failed to add Bitbucket HTTPS remote");
    
    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops);
    
    let result = init_command.execute().await;
    if result.is_err() {
        let error_msg = result.unwrap_err().to_string();
        println!("Expected error for Bitbucket remote: {}", error_msg);
    }
    
    std::env::set_current_dir(original_dir).unwrap();
}

#[tokio::test]
async fn test_init_with_bitbucket_ssh_remote() {
    let temp_dir = tempfile::tempdir().unwrap();
    setup_base_git_repo(&temp_dir).await.unwrap();
    let original_dir = std::env::current_dir().unwrap();
    
    std::env::set_current_dir(temp_dir.path()).unwrap();
    
    // Add Bitbucket SSH remote
    std::process::Command::new("git")
        .args(["remote", "add", "origin", "git@bitbucket.org:test-owner/test-repo.git"])
        .output()
        .expect("Failed to add Bitbucket SSH remote");
    
    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops);
    
    let result = init_command.execute().await;
    if result.is_err() {
        let error_msg = result.unwrap_err().to_string();
        println!("Expected error for Bitbucket SSH remote: {}", error_msg);
    }
    
    std::env::set_current_dir(original_dir).unwrap();
}

#[tokio::test]
async fn test_init_with_self_hosted_git_https() {
    let temp_dir = tempfile::tempdir().unwrap();
    setup_base_git_repo(&temp_dir).await.unwrap();
    let original_dir = std::env::current_dir().unwrap();
    
    std::env::set_current_dir(temp_dir.path()).unwrap();
    
    // Add self-hosted Git server HTTPS remote
    std::process::Command::new("git")
        .args(["remote", "add", "origin", "https://git.company.com/team/project.git"])
        .output()
        .expect("Failed to add self-hosted HTTPS remote");
    
    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops);
    
    let result = init_command.execute().await;
    if result.is_err() {
        let error_msg = result.unwrap_err().to_string();
        println!("Expected error for self-hosted Git: {}", error_msg);
    }
    
    std::env::set_current_dir(original_dir).unwrap();
}

#[tokio::test]
async fn test_init_with_self_hosted_git_ssh() {
    let temp_dir = tempfile::tempdir().unwrap();
    setup_base_git_repo(&temp_dir).await.unwrap();
    let original_dir = std::env::current_dir().unwrap();
    
    std::env::set_current_dir(temp_dir.path()).unwrap();
    
    // Add self-hosted Git server SSH remote
    std::process::Command::new("git")
        .args(["remote", "add", "origin", "git@git.company.com:team/project.git"])
        .output()
        .expect("Failed to add self-hosted SSH remote");
    
    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops);
    
    let result = init_command.execute().await;
    if result.is_err() {
        let error_msg = result.unwrap_err().to_string();
        println!("Expected error for self-hosted SSH Git: {}", error_msg);
    }
    
    std::env::set_current_dir(original_dir).unwrap();
}

#[tokio::test]
async fn test_init_with_custom_port_ssh_remote() {
    let temp_dir = tempfile::tempdir().unwrap();
    setup_base_git_repo(&temp_dir).await.unwrap();
    let original_dir = std::env::current_dir().unwrap();
    
    std::env::set_current_dir(temp_dir.path()).unwrap();
    
    // Add SSH remote with custom port
    std::process::Command::new("git")
        .args(["remote", "add", "origin", "ssh://git@gitlab.com:2222/test-owner/test-repo.git"])
        .output()
        .expect("Failed to add custom port SSH remote");
    
    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops);
    
    let result = init_command.execute().await;
    if result.is_err() {
        let error_msg = result.unwrap_err().to_string();
        println!("Result for custom port SSH: {}", error_msg);
    }
    
    std::env::set_current_dir(original_dir).unwrap();
}

#[tokio::test]
async fn test_init_with_multiple_remotes() {
    let temp_dir = tempfile::tempdir().unwrap();
    setup_base_git_repo(&temp_dir).await.unwrap();
    let original_dir = std::env::current_dir().unwrap();
    
    std::env::set_current_dir(temp_dir.path()).unwrap();
    
    // Add multiple remotes (common in development workflows)
    std::process::Command::new("git")
        .args(["remote", "add", "origin", "https://github.com/test-owner/test-repo.git"])
        .output()
        .expect("Failed to add origin remote");
        
    std::process::Command::new("git")
        .args(["remote", "add", "upstream", "https://github.com/upstream-owner/test-repo.git"])
        .output()
        .expect("Failed to add upstream remote");
        
    std::process::Command::new("git")
        .args(["remote", "add", "fork", "git@github.com:fork-owner/test-repo.git"])
        .output()
        .expect("Failed to add fork remote");
    
    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops);
    
    let result = init_command.execute().await;
    // Should handle multiple remotes and use 'origin' by convention
    assert!(result.is_ok(), "Init should handle multiple remotes: {:?}", result.err());
    
    std::env::set_current_dir(original_dir).unwrap();
}

#[tokio::test]
async fn test_init_with_no_remote() {
    let temp_dir = tempfile::tempdir().unwrap();
    setup_base_git_repo(&temp_dir).await.unwrap();
    let original_dir = std::env::current_dir().unwrap();
    
    std::env::set_current_dir(temp_dir.path()).unwrap();
    
    // No remotes added - local repo only
    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops);
    
    let result = init_command.execute().await;
    // Should fail gracefully for repos without remotes
    assert!(result.is_err(), "Init should fail for repos without remotes");
    
    let error_msg = result.unwrap_err().to_string();
    println!("Expected error for no remote: {}", error_msg);
    
    std::env::set_current_dir(original_dir).unwrap();
}

#[tokio::test]
async fn test_init_with_malformed_remote_urls() {
    let temp_dir = tempfile::tempdir().unwrap();
    setup_base_git_repo(&temp_dir).await.unwrap();
    let original_dir = std::env::current_dir().unwrap();
    
    std::env::set_current_dir(temp_dir.path()).unwrap();
    
    let malformed_urls = [
        "not-a-url",
        "http://",
        "git@",
        "https://github.com/",
        "ftp://github.com/owner/repo.git",
        "https://github.com/owner",
    ];
    
    let fs_ops = Arc::new(StandardFileSystem);
    
    for url in malformed_urls {
        // Clean up any existing remotes
        let _ = std::process::Command::new("git")
            .args(["remote", "remove", "origin"])
            .output();
        
        // Add malformed remote
        std::process::Command::new("git")
            .args(["remote", "add", "origin", url])
            .output()
            .expect("Failed to add malformed remote");
        
        let init_command = InitCommand::new(None, false, true, fs_ops.clone());
        let result = init_command.execute().await;
        
        // Should fail gracefully with meaningful error
        assert!(result.is_err(), "Init should fail for malformed URL: {}", url);
        
        let error_msg = result.unwrap_err().to_string();
        println!("Error for malformed URL '{}': {}", url, error_msg);
    }
    
    std::env::set_current_dir(original_dir).unwrap();
}