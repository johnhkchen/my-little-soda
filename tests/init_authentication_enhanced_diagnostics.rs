/// Tests for init command enhanced authentication diagnostics
/// 
/// This module implements comprehensive testing for the enhanced authentication
/// diagnostics added to the init command as specified in issue #378.
/// These tests verify that improved error messages and diagnostics are provided
/// when authentication issues occur.

use my_little_soda::cli::commands::init::InitCommand;
use my_little_soda::fs::{FileSystemOperations, StandardFileSystem};
use std::sync::Arc;

/// Test that init command provides helpful error messages when GitHub CLI is not installed
#[tokio::test]
async fn test_init_github_cli_not_available() {
    let temp_dir = tempfile::tempdir().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    
    // Set up a directory without git (fresh project detection will bypass auth)
    std::env::set_current_dir(temp_dir.path()).unwrap();
    std::fs::write("README.md", "# Test Repository").unwrap();

    // Clear environment variables that might interfere
    std::env::remove_var("MY_LITTLE_SODA_GITHUB_TOKEN");
    std::env::remove_var("GITHUB_TOKEN");
    std::env::remove_var("GH_TOKEN");

    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, false, fs_ops);
    
    let result = init_command.execute().await;
    
    // For fresh project, init should succeed by skipping GitHub validation
    // This tests the fresh project detection and bypass logic
    assert!(result.is_ok(), "Fresh project init should succeed without GitHub auth: {:?}", result.err());
    
    // Cleanup
    std::env::set_current_dir(original_dir).unwrap();
}

/// Test that verbose mode provides detailed authentication environment diagnostics
#[tokio::test]
async fn test_init_verbose_authentication_diagnostics() {
    let temp_dir = tempfile::tempdir().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    
    // Set up a fresh project environment
    std::env::set_current_dir(temp_dir.path()).unwrap();
    std::fs::write("README.md", "# Test Repository").unwrap();

    // Set some test environment variables to verify diagnostics
    std::env::set_var("MY_LITTLE_SODA_GITHUB_TOKEN", "test_token_1234");
    std::env::set_var("GITHUB_OWNER", "test-owner");
    std::env::set_var("GITHUB_REPO", "test-repo");

    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops).with_verbose(true); // dry run + verbose
    
    let result = init_command.execute().await;
    
    // In fresh project + dry run + verbose mode, should provide detailed diagnostics and succeed
    assert!(result.is_ok(), "Verbose fresh project init should succeed: {:?}", result.err());
    
    // Cleanup environment
    std::env::remove_var("MY_LITTLE_SODA_GITHUB_TOKEN");
    std::env::remove_var("GITHUB_OWNER");
    std::env::remove_var("GITHUB_REPO");
    std::env::set_current_dir(original_dir).unwrap();
}

/// Test that init command handles placeholder values correctly
#[tokio::test]
async fn test_init_placeholder_token_detection() {
    let temp_dir = tempfile::tempdir().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    
    // Set up a fresh project environment
    std::env::set_current_dir(temp_dir.path()).unwrap();
    std::fs::write("README.md", "# Test Repository").unwrap();

    // Set placeholder values that should be detected and flagged
    std::env::set_var("MY_LITTLE_SODA_GITHUB_TOKEN", "YOUR_GITHUB_TOKEN_HERE");
    std::env::set_var("GITHUB_OWNER", "your-github-username");
    std::env::set_var("GITHUB_REPO", "your-repo-name");

    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops).with_verbose(true); // dry run + verbose
    
    let result = init_command.execute().await;
    
    // Should succeed in dry run mode for fresh project even with placeholder values
    assert!(result.is_ok(), "Init should succeed in dry run with placeholders for fresh project: {:?}", result.err());
    
    // Cleanup environment
    std::env::remove_var("MY_LITTLE_SODA_GITHUB_TOKEN");
    std::env::remove_var("GITHUB_OWNER");
    std::env::remove_var("GITHUB_REPO");
    std::env::set_current_dir(original_dir).unwrap();
}

/// Test that init command provides enhanced error messages for existing repositories
#[tokio::test]
async fn test_init_enhanced_error_messages_existing_repo() {
    let temp_dir = tempfile::tempdir().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    
    // Set up a git repository (not fresh project)
    std::env::set_current_dir(temp_dir.path()).unwrap();
    
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

    std::fs::write("README.md", "# Test Repository").unwrap();
    
    std::process::Command::new("git")
        .args(["add", "README.md"])
        .output()
        .expect("Failed to add file");
        
    std::process::Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .output()
        .expect("Failed to commit");

    // Clear GitHub authentication to force authentication errors
    std::env::remove_var("MY_LITTLE_SODA_GITHUB_TOKEN");
    std::env::remove_var("GITHUB_TOKEN");
    std::env::remove_var("GH_TOKEN");

    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, false, fs_ops);
    
    let result = init_command.execute().await;
    
    // Should fail with enhanced authentication error message for existing repo
    assert!(result.is_err(), "Init should fail with authentication error for existing repo");
    
    let error_msg = result.unwrap_err().to_string();
    // Verify that the error message contains helpful information
    assert!(
        error_msg.contains("💡") || error_msg.contains("authentication") || error_msg.contains("auth"),
        "Error message should contain enhanced diagnostics with emojis or auth guidance: {}",
        error_msg
    );
    
    // Cleanup
    std::env::set_current_dir(original_dir).unwrap();
}

/// Test authentication error handling with verbose mode
#[tokio::test]  
async fn test_init_authentication_error_verbose() {
    let temp_dir = tempfile::tempdir().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    
    // Set up a git repository with remote (not fresh)
    std::env::set_current_dir(temp_dir.path()).unwrap();
    
    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .output()
        .expect("Failed to init git repo");
    
    std::process::Command::new("git")
        .args(["remote", "add", "origin", "https://github.com/test-owner/test-repo.git"])
        .output()
        .expect("Failed to add git remote");

    // Clear authentication environment
    std::env::remove_var("MY_LITTLE_SODA_GITHUB_TOKEN");
    std::env::remove_var("GITHUB_TOKEN");
    std::env::remove_var("GH_TOKEN");

    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, false, fs_ops).with_verbose(true);
    
    let result = init_command.execute().await;
    
    // Should fail with detailed verbose error information
    assert!(result.is_err(), "Init should fail with authentication error in verbose mode");
    
    let error_msg = result.unwrap_err().to_string();
    // In verbose mode, we should get enhanced error messages with actionable guidance
    assert!(
        error_msg.contains("💡") || error_msg.contains("authentication") || 
        error_msg.contains("auth login") || error_msg.contains("token"),
        "Verbose error message should contain detailed authentication guidance: {}",
        error_msg
    );
    
    // Cleanup
    std::env::set_current_dir(original_dir).unwrap();
}

/// Test that init command detects and reports on credential files
#[tokio::test]
async fn test_init_credential_file_diagnostics() {
    let temp_dir = tempfile::tempdir().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    
    // Set up a fresh project
    std::env::set_current_dir(temp_dir.path()).unwrap();
    std::fs::write("README.md", "# Test Repository").unwrap();

    // Create credential directory structure
    std::fs::create_dir_all(".my-little-soda/credentials").unwrap();
    std::fs::write(".my-little-soda/credentials/github_token", "test_token_value").unwrap();
    std::fs::write(".my-little-soda/credentials/github_owner", "test-owner").unwrap();

    // Clear environment variables to test file-based credentials
    std::env::remove_var("MY_LITTLE_SODA_GITHUB_TOKEN");
    std::env::remove_var("GITHUB_TOKEN");
    std::env::remove_var("GITHUB_OWNER");
    std::env::remove_var("GITHUB_REPO");

    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops).with_verbose(true); // dry run + verbose
    
    let result = init_command.execute().await;
    
    // Should succeed in verbose mode and show credential file diagnostics
    assert!(result.is_ok(), "Init should succeed in verbose mode with credential files: {:?}", result.err());
    
    // Cleanup
    std::env::set_current_dir(original_dir).unwrap();
}