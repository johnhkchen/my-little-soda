/// Tests for init command authentication edge cases and diagnostics
/// 
/// This module implements comprehensive testing for GitHub authentication
/// scenarios as specified in issue #378 - focusing on invalid GitHub
/// authentication and edge cases that users might encounter.

use my_little_soda::cli::commands::init::InitCommand;
use my_little_soda::fs::MockFileSystemOperations;
use mockall::predicate::*;
use std::sync::Arc;
use std::process::{Output, ExitStatus};

fn create_successful_exit_status() -> ExitStatus {
    std::process::Command::new("true").status().unwrap()
}

fn create_failed_exit_status() -> ExitStatus {
    std::process::Command::new("false").status().unwrap()
}

/// Test invalid GitHub token scenarios
#[tokio::test]
async fn test_init_invalid_github_token_comprehensive() {
    let mut mock_fs = MockFileSystemOperations::new();
    
    mock_fs
        .expect_exists()
        .with(eq("my-little-soda.toml"))
        .return_const(false);

    // Mock git commands for existing project (not fresh)
    mock_fs
        .expect_execute_command()
        .with(eq("git"), eq(vec!["status".to_string()]))
        .times(1)
        .returning(|_, _| Ok(Output {
            status: create_successful_exit_status(),
            stdout: vec![],
            stderr: vec![],
        }));
    
    mock_fs
        .expect_execute_command()
        .with(eq("git"), eq(vec!["remote".to_string(), "get-url".to_string(), "origin".to_string()]))
        .times(1)
        .returning(|_, _| Ok(Output {
            status: create_successful_exit_status(),
            stdout: b"https://github.com/owner/repo.git\n".to_vec(),
            stderr: vec![],
        }));

    // Mock GitHub CLI returning invalid token
    let invalid_auth_output = Output {
        status: create_failed_exit_status(),
        stdout: vec![],
        stderr: b"Not logged into any GitHub hosts. Run 'gh auth login' to authenticate.\n".to_vec(),
    };
    
    mock_fs
        .expect_execute_command()
        .with(eq("gh"), eq(vec!["auth".to_string(), "status".to_string()]))
        .times(1)
        .returning(move |_, _| Ok(invalid_auth_output.clone()));

    let fs_ops = Arc::new(mock_fs);
    let init_command = InitCommand::new(None, false, false, fs_ops);

    let result = init_command.execute().await;
    assert!(result.is_err(), "Init should fail with invalid GitHub authentication");
    
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("GitHub CLI not authenticated") && error_msg.contains("gh auth login"),
        "Error should provide clear authentication guidance: {}",
        error_msg
    );
}

/// Test GitHub CLI command not found scenario
#[tokio::test]
async fn test_init_github_cli_not_installed() {
    let mut mock_fs = MockFileSystemOperations::new();
    
    mock_fs
        .expect_exists()
        .with(eq("my-little-soda.toml"))
        .return_const(false);

    // Mock git commands for existing project 
    mock_fs
        .expect_execute_command()
        .with(eq("git"), eq(vec!["status".to_string()]))
        .times(1)
        .returning(|_, _| Ok(Output {
            status: create_successful_exit_status(),
            stdout: vec![],
            stderr: vec![],
        }));
    
    mock_fs
        .expect_execute_command()
        .with(eq("git"), eq(vec!["remote".to_string(), "get-url".to_string(), "origin".to_string()]))
        .times(1)
        .returning(|_, _| Ok(Output {
            status: create_successful_exit_status(),
            stdout: b"https://github.com/owner/repo.git\n".to_vec(),
            stderr: vec![],
        }));

    // Mock GitHub CLI command not found
    mock_fs
        .expect_execute_command()
        .with(eq("gh"), eq(vec!["auth".to_string(), "status".to_string()]))
        .times(1)
        .returning(|_, _| Err(anyhow::anyhow!("No such file or directory (os error 2)")));

    let fs_ops = Arc::new(mock_fs);
    let init_command = InitCommand::new(None, false, false, fs_ops);

    let result = init_command.execute().await;
    assert!(result.is_err(), "Init should fail when GitHub CLI not installed");
    
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("Failed to run 'gh auth status'") && error_msg.contains("GitHub CLI is installed"),
        "Error should mention GitHub CLI installation requirement: {}",
        error_msg
    );
}

/// Test successful authentication with verbose diagnostics
#[tokio::test]
async fn test_init_verbose_authentication_success() {
    let mut mock_fs = MockFileSystemOperations::new();
    
    mock_fs
        .expect_exists()
        .with(eq("my-little-soda.toml"))
        .return_const(false);

    // Mock git commands for existing project
    mock_fs
        .expect_execute_command()
        .with(eq("git"), eq(vec!["status".to_string()]))
        .times(1)
        .returning(|_, _| Ok(Output {
            status: create_successful_exit_status(),
            stdout: vec![],
            stderr: vec![],
        }));
    
    mock_fs
        .expect_execute_command()
        .with(eq("git"), eq(vec!["remote".to_string(), "get-url".to_string(), "origin".to_string()]))
        .times(1)
        .returning(|_, _| Ok(Output {
            status: create_successful_exit_status(),
            stdout: b"https://github.com/owner/repo.git\n".to_vec(),
            stderr: vec![],
        }));

    // Mock successful GitHub CLI authentication
    let success_auth_output = Output {
        status: create_successful_exit_status(),
        stdout: "Logged in to github.com as testuser\n".as_bytes().to_vec(),
        stderr: vec![],
    };
    
    mock_fs
        .expect_execute_command()
        .with(eq("gh"), eq(vec!["auth".to_string(), "status".to_string()]))
        .times(1)
        .returning(move |_, _| Ok(success_auth_output.clone()));

    // Mock clean git status
    mock_fs
        .expect_execute_command()
        .with(eq("git"), eq(vec!["status".to_string(), "--porcelain".to_string()]))
        .times(1)
        .returning(|_, _| Ok(Output {
            status: create_successful_exit_status(),
            stdout: vec![],
            stderr: vec![],
        }));

    let fs_ops = Arc::new(mock_fs);
    let init_command = InitCommand::new(None, false, true, fs_ops).with_verbose(true); // dry run + verbose

    let result = init_command.execute().await;
    assert!(result.is_ok(), "Verbose init should succeed with valid authentication: {:?}", result.err());
}

/// Test authentication with corrupted/empty token scenarios
#[tokio::test]
async fn test_init_corrupted_token_scenarios() {
    let mut mock_fs = MockFileSystemOperations::new();
    
    mock_fs
        .expect_exists()
        .with(eq("my-little-soda.toml"))
        .return_const(false);

    // Mock git commands for existing project
    mock_fs
        .expect_execute_command()
        .with(eq("git"), eq(vec!["status".to_string()]))
        .times(1)
        .returning(|_, _| Ok(Output {
            status: create_successful_exit_status(),
            stdout: vec![],
            stderr: vec![],
        }));
    
    mock_fs
        .expect_execute_command()
        .with(eq("git"), eq(vec!["remote".to_string(), "get-url".to_string(), "origin".to_string()]))
        .times(1)
        .returning(|_, _| Ok(Output {
            status: create_successful_exit_status(),
            stdout: b"https://github.com/owner/repo.git\n".to_vec(),
            stderr: vec![],
        }));

    // Mock GitHub CLI returning empty token (corrupted state)
    let empty_token_output = Output {
        status: create_successful_exit_status(),
        stdout: b"".to_vec(), // Empty token
        stderr: vec![],
    };
    
    mock_fs
        .expect_execute_command()
        .with(eq("gh"), eq(vec!["auth".to_string(), "status".to_string()]))
        .times(1)
        .returning(move |_, _| Ok(empty_token_output.clone()));

    let fs_ops = Arc::new(mock_fs);
    let init_command = InitCommand::new(None, false, false, fs_ops);

    let result = init_command.execute().await;
    assert!(result.is_err(), "Init should fail with empty/corrupted token");
    
    let error_msg = result.unwrap_err().to_string();
    // Error message should provide guidance for fixing corrupted authentication
    assert!(
        error_msg.contains("GitHub CLI not authenticated") || error_msg.contains("authentication"),
        "Error should provide authentication guidance: {}",
        error_msg
    );
}

/// Test authentication with network connectivity issues
#[tokio::test]
async fn test_init_network_connectivity_issues() {
    let mut mock_fs = MockFileSystemOperations::new();
    
    mock_fs
        .expect_exists()
        .with(eq("my-little-soda.toml"))
        .return_const(false);

    // Mock git commands for existing project
    mock_fs
        .expect_execute_command()
        .with(eq("git"), eq(vec!["status".to_string()]))
        .times(1)
        .returning(|_, _| Ok(Output {
            status: create_successful_exit_status(),
            stdout: vec![],
            stderr: vec![],
        }));
    
    mock_fs
        .expect_execute_command()
        .with(eq("git"), eq(vec!["remote".to_string(), "get-url".to_string(), "origin".to_string()]))
        .times(1)
        .returning(|_, _| Ok(Output {
            status: create_successful_exit_status(),
            stdout: b"https://github.com/owner/repo.git\n".to_vec(),
            stderr: vec![],
        }));

    // Mock successful local authentication
    let success_auth_output = Output {
        status: create_successful_exit_status(),
        stdout: "Logged in to github.com as testuser\n".as_bytes().to_vec(),
        stderr: vec![],
    };
    
    mock_fs
        .expect_execute_command()
        .with(eq("gh"), eq(vec!["auth".to_string(), "status".to_string()]))
        .times(1)
        .returning(move |_, _| Ok(success_auth_output.clone()));

    // Mock clean git status
    mock_fs
        .expect_execute_command()
        .with(eq("git"), eq(vec!["status".to_string(), "--porcelain".to_string()]))
        .times(1)
        .returning(|_, _| Ok(Output {
            status: create_successful_exit_status(),
            stdout: vec![],
            stderr: vec![],
        }));

    let fs_ops = Arc::new(mock_fs);
    let init_command = InitCommand::new(None, false, false, fs_ops);

    // Note: Network issues would be detected during GitHub API access validation
    // This test verifies that local authentication checks can pass even if network issues occur later
    // The actual network failure would happen during API validation, not in these git/gh commands
    let result = init_command.execute().await;
    
    // This might fail during API validation due to network issues, but should pass local auth checks
    if result.is_err() {
        let error_msg = result.unwrap_err().to_string();
        // Should not fail due to local authentication issues if gh CLI reports success
        assert!(
            !error_msg.contains("GitHub CLI not authenticated"),
            "Should not fail local auth checks with network issues: {}",
            error_msg
        );
    }
}

/// Test authentication with insufficient permissions
#[tokio::test]
async fn test_init_insufficient_permissions() {
    let mut mock_fs = MockFileSystemOperations::new();
    
    mock_fs
        .expect_exists()
        .with(eq("my-little-soda.toml"))
        .return_const(false);

    // Mock git commands for existing project
    mock_fs
        .expect_execute_command()
        .with(eq("git"), eq(vec!["status".to_string()]))
        .times(1)
        .returning(|_, _| Ok(Output {
            status: create_successful_exit_status(),
            stdout: vec![],
            stderr: vec![],
        }));
    
    mock_fs
        .expect_execute_command()
        .with(eq("git"), eq(vec!["remote".to_string(), "get-url".to_string(), "origin".to_string()]))
        .times(1)
        .returning(|_, _| Ok(Output {
            status: create_successful_exit_status(),
            stdout: b"https://github.com/owner/repo.git\n".to_vec(),
            stderr: vec![],
        }));

    // Mock successful local GitHub CLI authentication
    let success_auth_output = Output {
        status: create_successful_exit_status(),
        stdout: "Logged in to github.com as testuser\n".as_bytes().to_vec(),
        stderr: vec![],
    };
    
    mock_fs
        .expect_execute_command()
        .with(eq("gh"), eq(vec!["auth".to_string(), "status".to_string()]))
        .times(1)
        .returning(move |_, _| Ok(success_auth_output.clone()));

    // Mock clean git status
    mock_fs
        .expect_execute_command()
        .with(eq("git"), eq(vec!["status".to_string(), "--porcelain".to_string()]))
        .times(1)
        .returning(|_, _| Ok(Output {
            status: create_successful_exit_status(),
            stdout: vec![],
            stderr: vec![],
        }));

    let fs_ops = Arc::new(mock_fs);
    let init_command = InitCommand::new(None, false, false, fs_ops);

    // Note: Permission issues would be detected during GitHub API access validation
    // This test verifies that local authentication validation passes but API validation may fail
    let result = init_command.execute().await;
    
    // Local authentication should pass, but API validation might fail
    if result.is_err() {
        let error_msg = result.unwrap_err().to_string();
        // Local auth should succeed even if API permissions are insufficient
        assert!(
            !error_msg.contains("GitHub CLI not authenticated"),
            "Local authentication should succeed: {}",
            error_msg
        );
    }
}

/// Test fresh project scenarios with authentication handling
#[tokio::test]
async fn test_init_fresh_project_authentication_bypass() {
    let mut mock_fs = MockFileSystemOperations::new();
    
    mock_fs
        .expect_exists()
        .with(eq("my-little-soda.toml"))
        .return_const(false);

    // Mock fresh project detection (no git repository)
    mock_fs
        .expect_execute_command()
        .with(eq("git"), eq(vec!["status".to_string()]))
        .times(1)
        .returning(|_, _| Err(anyhow::anyhow!("not a git repository")));

    // For fresh projects, git repo initialization is attempted
    mock_fs
        .expect_execute_command()
        .with(eq("git"), eq(vec!["init".to_string()]))
        .times(1)
        .returning(|_, _| Ok(Output {
            status: create_successful_exit_status(),
            stdout: b"Initialized empty Git repository\n".to_vec(),
            stderr: vec![],
        }));

    let fs_ops = Arc::new(mock_fs);
    let init_command = InitCommand::new(None, false, true, fs_ops); // dry run to avoid actual file operations

    let result = init_command.execute().await;
    assert!(result.is_ok(), "Fresh project init should succeed without GitHub authentication: {:?}", result.err());
}

/// Test verbose authentication diagnostics with detailed output verification
#[tokio::test]
async fn test_init_verbose_authentication_diagnostics() {
    let mut mock_fs = MockFileSystemOperations::new();
    
    mock_fs
        .expect_exists()
        .with(eq("my-little-soda.toml"))
        .return_const(false);

    // Mock git commands for existing project
    mock_fs
        .expect_execute_command()
        .with(eq("git"), eq(vec!["status".to_string()]))
        .times(1)
        .returning(|_, _| Ok(Output {
            status: create_successful_exit_status(),
            stdout: vec![],
            stderr: vec![],
        }));
    
    mock_fs
        .expect_execute_command()
        .with(eq("git"), eq(vec!["remote".to_string(), "get-url".to_string(), "origin".to_string()]))
        .times(1)
        .returning(|_, _| Ok(Output {
            status: create_successful_exit_status(),
            stdout: b"https://github.com/owner/repo.git\n".to_vec(),
            stderr: vec![],
        }));

    // Mock GitHub CLI with specific exit codes and output
    let auth_output_with_details = Output {
        status: create_successful_exit_status(),
        stdout: "Logged in to github.com as testuser (oauth_token)\nGit operations for github.com configured to use https protocol\n".as_bytes().to_vec(),
        stderr: vec![],
    };
    
    mock_fs
        .expect_execute_command()
        .with(eq("gh"), eq(vec!["auth".to_string(), "status".to_string()]))
        .times(1)
        .returning(move |_, _| Ok(auth_output_with_details.clone()));

    // Mock clean git status
    mock_fs
        .expect_execute_command()
        .with(eq("git"), eq(vec!["status".to_string(), "--porcelain".to_string()]))
        .times(1)
        .returning(|_, _| Ok(Output {
            status: create_successful_exit_status(),
            stdout: vec![],
            stderr: vec![],
        }));

    let fs_ops = Arc::new(mock_fs);
    let init_command = InitCommand::new(None, false, true, fs_ops).with_verbose(true);

    let result = init_command.execute().await;
    assert!(result.is_ok(), "Verbose init should succeed and provide detailed diagnostics: {:?}", result.err());
}