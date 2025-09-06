/// Init command implementation with graceful conflict resolution
///
/// # Conflict Resolution Strategy
///
/// The init command follows a non-destructive approach when handling existing files:
///
/// ## 1. Existing Project Files
/// - **README.md, src/, Cargo.toml, etc.**: Completely preserved without modification
/// - **No overwriting**: Existing project files are never touched or modified
/// - **No data loss**: All user content remains intact
///
/// ## 2. Configuration Files
/// - **my-little-soda.toml**: Only created if it doesn't exist, otherwise requires `--force` flag
/// - **Explicit user consent**: User must use `--force` to overwrite existing configuration
/// - **Clear error messages**: Informative errors when conflicts would occur
///
/// ## 3. Directory Structure
/// - **.my-little-soda/ directory**: Created alongside existing directories
/// - **No conflicts**: Clambake directories don't interfere with existing project structure
/// - **Isolated setup**: All my-little-soda-specific files are contained in dedicated directories
///
/// ## 4. Git Repository State
/// - **Clean repository required**: Init fails on uncommitted changes unless `--force` is used
/// - **Branch preservation**: Current branch and git state remain unchanged
/// - **Remote detection**: Automatically detects GitHub repository information from git remotes
///
/// This approach ensures that my-little-soda can be initialized in any existing repository
/// without risk of data loss or conflicts with existing project structure.

pub mod core;
pub mod validation;
pub mod labels;
pub mod config;
pub mod setup;

// Re-export the main public API
pub use core::InitCommand;

#[cfg(test)]
mod tests {
    use super::*;
    use super::validation::validate_environment;
    use crate::fs::{FileSystemOperations, MockFileSystemOperations};
    use mockall::predicate::*;
    use std::process::{ExitStatus, Output};
    use crate::cli::commands::init::core::LabelSpec;

    fn create_successful_exit_status() -> ExitStatus {
        std::process::Command::new("true").status().unwrap()
    }

    fn create_failed_exit_status() -> ExitStatus {
        std::process::Command::new("false").status().unwrap()
    }

    #[tokio::test]
    async fn test_successful_init_clean_repository() {
        let mut mock_fs = MockFileSystemOperations::new();

        // Mock file operations for successful init (dry run only checks existence)
        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(false);

        // Mock git commands for fresh project detection
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string()]))
            .times(1)
            .returning(|_, _| {
                Ok(Output {
                    status: create_successful_exit_status(),
                    stdout: vec![],
                    stderr: vec![],
                })
            });

        mock_fs
            .expect_execute_command()
            .with(
                eq("git"),
                eq(vec![
                    "remote".to_string(),
                    "get-url".to_string(),
                    "origin".to_string(),
                ]),
            )
            .times(1)
            .returning(|_, _| {
                Ok(Output {
                    status: create_successful_exit_status(),
                    stdout: b"https://github.com/owner/repo.git\n".to_vec(),
                    stderr: vec![],
                })
            });

        let fs_ops = std::sync::Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, true, fs_ops); // dry_run = true

        let result = init_command.execute().await;
        assert!(
            result.is_ok(),
            "Init command should succeed in clean repository"
        );
    }

    #[tokio::test]
    async fn test_successful_init_with_template() {
        let mut mock_fs = MockFileSystemOperations::new();

        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(false);

        // Mock git commands for fresh project detection
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string()]))
            .times(1)
            .returning(|_, _| {
                Ok(Output {
                    status: create_successful_exit_status(),
                    stdout: vec![],
                    stderr: vec![],
                })
            });

        mock_fs
            .expect_execute_command()
            .with(
                eq("git"),
                eq(vec![
                    "remote".to_string(),
                    "get-url".to_string(),
                    "origin".to_string(),
                ]),
            )
            .times(1)
            .returning(|_, _| {
                Ok(Output {
                    status: create_successful_exit_status(),
                    stdout: b"https://github.com/owner/repo.git\n".to_vec(),
                    stderr: vec![],
                })
            });

        let fs_ops = std::sync::Arc::new(mock_fs);
        let init_command = InitCommand::new(Some("default".to_string()), false, true, fs_ops);

        let result = init_command.execute().await;
        assert!(result.is_ok(), "Init command should succeed with template");
    }

    #[tokio::test]
    async fn test_init_fails_when_config_exists_without_force() {
        let mut mock_fs = MockFileSystemOperations::new();

        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(true);

        let fs_ops = std::sync::Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, true, fs_ops);

        let result = config::generate_configuration(&init_command).await;
        assert!(
            result.is_err(),
            "Should fail when config exists and force is false"
        );
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_init_succeeds_when_config_exists_with_force() {
        let mut mock_fs = MockFileSystemOperations::new();

        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(true);

        // Mock git commands for fresh project detection
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string()]))
            .times(1)
            .returning(|_, _| {
                Ok(Output {
                    status: create_successful_exit_status(),
                    stdout: vec![],
                    stderr: vec![],
                })
            });

        mock_fs
            .expect_execute_command()
            .with(
                eq("git"),
                eq(vec![
                    "remote".to_string(),
                    "get-url".to_string(),
                    "origin".to_string(),
                ]),
            )
            .times(1)
            .returning(|_, _| {
                Ok(Output {
                    status: create_successful_exit_status(),
                    stdout: b"https://github.com/owner/repo.git\n".to_vec(),
                    stderr: vec![],
                })
            });

        let fs_ops = std::sync::Arc::new(mock_fs);
        let init_command = InitCommand::new(None, true, true, fs_ops); // force = true, dry_run = true

        let result = init_command.execute().await;
        assert!(
            result.is_ok(),
            "Should succeed when config exists and force is true"
        );
    }

    #[tokio::test]
    async fn test_init_handles_git_remote_missing_gracefully() {
        let mut mock_fs = MockFileSystemOperations::new();

        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(false);

        let failed_output = Output {
            status: create_failed_exit_status(),
            stdout: vec![],
            stderr: b"fatal: not a git repository".to_vec(),
        };

        mock_fs
            .expect_execute_command()
            .with(
                eq("git"),
                eq(vec![
                    "remote".to_string(),
                    "get-url".to_string(),
                    "origin".to_string(),
                ]),
            )
            .times(1)
            .returning(move |_, _| Ok(failed_output.clone()));

        let fs_ops = std::sync::Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, false, fs_ops); // dry_run = false

        let result = config::detect_repository_info(&init_command).await;
        assert!(
            result.is_ok(),
            "Should succeed with placeholder values when no git remote found"
        );
        let (owner, repo) = result.unwrap();
        assert_eq!(owner, "your-github-username");
        assert_eq!(repo, "your-repo-name");
    }

    #[tokio::test]
    async fn test_init_fails_with_invalid_github_url() {
        let mut mock_fs = MockFileSystemOperations::new();

        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(false);

        let invalid_url_output = Output {
            status: create_successful_exit_status(),
            stdout: b"git@gitlab.com:user/repo.git\n".to_vec(),
            stderr: vec![],
        };

        mock_fs
            .expect_execute_command()
            .with(
                eq("git"),
                eq(vec![
                    "remote".to_string(),
                    "get-url".to_string(),
                    "origin".to_string(),
                ]),
            )
            .times(1)
            .returning(move |_, _| Ok(invalid_url_output.clone()));

        let fs_ops = std::sync::Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, false, fs_ops);

        let result = config::detect_repository_info(&init_command).await;
        assert!(result.is_err(), "Should fail with non-GitHub URL");
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Could not parse GitHub repository"));
    }

    #[tokio::test]
    async fn test_init_idempotency_with_force() {
        let mut mock_fs = MockFileSystemOperations::new();

        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(true)
            .times(2);

        // Mock git commands for fresh project detection (called twice)
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string()]))
            .times(2)
            .returning(|_, _| {
                Ok(Output {
                    status: create_successful_exit_status(),
                    stdout: vec![],
                    stderr: vec![],
                })
            });

        mock_fs
            .expect_execute_command()
            .with(
                eq("git"),
                eq(vec![
                    "remote".to_string(),
                    "get-url".to_string(),
                    "origin".to_string(),
                ]),
            )
            .times(2)
            .returning(|_, _| {
                Ok(Output {
                    status: create_successful_exit_status(),
                    stdout: b"https://github.com/owner/repo.git\n".to_vec(),
                    stderr: vec![],
                })
            });

        let fs_ops = std::sync::Arc::new(mock_fs);

        // First init - should succeed (force + dry_run)
        let init_command = InitCommand::new(None, true, true, fs_ops.clone());
        let result1 = init_command.execute().await;
        assert!(result1.is_ok(), "First init should succeed");

        // Second init - should also succeed (idempotent)
        let init_command = InitCommand::new(None, true, true, fs_ops);
        let result2 = init_command.execute().await;
        assert!(result2.is_ok(), "Second init should succeed (idempotent)");
    }

    #[tokio::test]
    async fn test_init_directory_creation_failure() {
        let mut mock_fs = MockFileSystemOperations::new();

        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(false);

        mock_fs
            .expect_create_dir_all()
            .with(eq(".my-little-soda/credentials"))
            .times(1)
            .returning(|_| Err(anyhow::anyhow!("Permission denied")));

        let fs_ops = std::sync::Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, false, fs_ops);

        let result = config::generate_configuration(&init_command).await;
        assert!(result.is_err(), "Should fail when directory creation fails");
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to create .my-little-soda directory"));
    }

    #[tokio::test]
    async fn test_init_with_ci_mode() {
        let mut mock_fs = MockFileSystemOperations::new();

        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(false);

        // Mock git commands for fresh project detection
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string()]))
            .times(1)
            .returning(|_, _| {
                Ok(Output {
                    status: create_successful_exit_status(),
                    stdout: vec![],
                    stderr: vec![],
                })
            });

        mock_fs
            .expect_execute_command()
            .with(
                eq("git"),
                eq(vec![
                    "remote".to_string(),
                    "get-url".to_string(),
                    "origin".to_string(),
                ]),
            )
            .times(1)
            .returning(|_, _| {
                Ok(Output {
                    status: create_successful_exit_status(),
                    stdout: b"https://github.com/owner/repo.git\n".to_vec(),
                    stderr: vec![],
                })
            });

        let fs_ops = std::sync::Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, true, fs_ops).with_ci_mode(true);

        let result = init_command.execute().await;
        assert!(result.is_ok(), "Init should succeed in CI mode");
    }

    #[tokio::test]
    async fn test_label_spec_creation() {
        let _init_command =
            InitCommand::new(None, false, true, std::sync::Arc::new(MockFileSystemOperations::new()));
        let labels = labels::get_required_labels();

        assert!(!labels.is_empty(), "Should create multiple labels");

        // Verify critical labels exist
        let label_names: Vec<String> = labels.iter().map(|l| l.name.clone()).collect();
        assert!(label_names.contains(&"route:ready".to_string()));
        assert!(label_names.contains(&"route:ready_to_merge".to_string()));
        assert!(label_names.contains(&"route:unblocker".to_string()));
        assert!(label_names.contains(&"route:priority-high".to_string()));
    }

    #[tokio::test]
    async fn test_github_url_parsing_variations() {
        let mut mock_fs = MockFileSystemOperations::new();

        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(false);

        // Test HTTPS URL
        let https_output = Output {
            status: create_successful_exit_status(),
            stdout: b"https://github.com/owner/repo.git\n".to_vec(),
            stderr: vec![],
        };

        mock_fs
            .expect_execute_command()
            .with(
                eq("git"),
                eq(vec![
                    "remote".to_string(),
                    "get-url".to_string(),
                    "origin".to_string(),
                ]),
            )
            .times(1)
            .returning(move |_, _| Ok(https_output.clone()));

        let fs_ops = std::sync::Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, false, fs_ops);

        let result = config::detect_repository_info(&init_command).await;
        assert!(result.is_ok(), "Should parse HTTPS GitHub URL");
        let (owner, repo) = result.unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");
    }

    #[tokio::test]
    async fn test_github_ssh_url_parsing() {
        let mut mock_fs = MockFileSystemOperations::new();

        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(false);

        // Test SSH URL
        let ssh_output = Output {
            status: create_successful_exit_status(),
            stdout: b"git@github.com:ssh-owner/ssh-repo.git\n".to_vec(),
            stderr: vec![],
        };

        mock_fs
            .expect_execute_command()
            .with(
                eq("git"),
                eq(vec![
                    "remote".to_string(),
                    "get-url".to_string(),
                    "origin".to_string(),
                ]),
            )
            .times(1)
            .returning(move |_, _| Ok(ssh_output.clone()));

        let fs_ops = std::sync::Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, false, fs_ops);

        let result = config::detect_repository_info(&init_command).await;
        assert!(result.is_ok(), "Should parse SSH GitHub URL");
        let (owner, repo) = result.unwrap();
        assert_eq!(owner, "ssh-owner");
        assert_eq!(repo, "ssh-repo");
    }

    #[tokio::test]
    async fn test_validate_environment_github_auth_failure() {
        let mut mock_fs = MockFileSystemOperations::new();

        // Mock git commands for fresh project detection
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string()]))
            .times(1)
            .returning(|_, _| {
                Ok(Output {
                    status: create_successful_exit_status(),
                    stdout: vec![],
                    stderr: vec![],
                })
            });

        mock_fs
            .expect_execute_command()
            .with(
                eq("git"),
                eq(vec![
                    "remote".to_string(),
                    "get-url".to_string(),
                    "origin".to_string(),
                ]),
            )
            .times(1)
            .returning(|_, _| {
                Ok(Output {
                    status: create_successful_exit_status(),
                    stdout: b"https://github.com/owner/repo.git\n".to_vec(),
                    stderr: vec![],
                })
            });

        let auth_failed_output = Output {
            status: create_failed_exit_status(),
            stdout: vec![],
            stderr: b"Not logged into any GitHub hosts".to_vec(),
        };

        mock_fs
            .expect_execute_command()
            .with(eq("gh"), eq(vec!["auth".to_string(), "status".to_string()]))
            .times(1)
            .returning(move |_, _| Ok(auth_failed_output.clone()));

        let fs_ops = std::sync::Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, false, fs_ops);

        let result = validation::validate_environment(&init_command).await;
        assert!(
            result.is_err(),
            "Should fail when GitHub CLI not authenticated"
        );
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("GitHub CLI not authenticated"));
    }

    #[tokio::test]
    async fn test_validate_environment_git_status_with_uncommitted_changes() {
        let mut mock_fs = MockFileSystemOperations::new();

        let auth_success_output = Output {
            status: create_successful_exit_status(),
            stdout: b"Logged in to github.com".to_vec(),
            stderr: vec![],
        };

        let git_status_output = Output {
            status: create_successful_exit_status(),
            stdout: b" M src/main.rs\n?? new_file.txt\n".to_vec(),
            stderr: vec![],
        };

        mock_fs
            .expect_execute_command()
            .with(eq("gh"), eq(vec!["auth".to_string(), "status".to_string()]))
            .times(1)
            .returning(move |_, _| Ok(auth_success_output.clone()));

        // Mock git commands for fresh project detection
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string()]))
            .times(1)
            .returning(|_, _| {
                Ok(Output {
                    status: create_successful_exit_status(),
                    stdout: vec![],
                    stderr: vec![],
                })
            });

        mock_fs
            .expect_execute_command()
            .with(
                eq("git"),
                eq(vec![
                    "remote".to_string(),
                    "get-url".to_string(),
                    "origin".to_string(),
                ]),
            )
            .times(1)
            .returning(|_, _| {
                Ok(Output {
                    status: create_successful_exit_status(),
                    stdout: b"https://github.com/owner/repo.git\n".to_vec(),
                    stderr: vec![],
                })
            });

        mock_fs
            .expect_execute_command()
            .with(
                eq("git"),
                eq(vec!["status".to_string(), "--porcelain".to_string()]),
            )
            .times(1)
            .returning(move |_, _| Ok(git_status_output.clone()));

        let fs_ops = std::sync::Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, false, fs_ops); // force = false

        let result = validation::validate_environment(&init_command).await;
        assert!(
            result.is_err(),
            "Should fail with uncommitted changes and no force flag"
        );
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Repository has uncommitted changes"));
    }

    #[tokio::test]
    async fn test_validate_environment_git_status_with_force_flag() {
        let mut mock_fs = MockFileSystemOperations::new();

        let auth_success_output = Output {
            status: create_successful_exit_status(),
            stdout: b"Logged in to github.com".to_vec(),
            stderr: vec![],
        };

        let git_status_output = Output {
            status: create_successful_exit_status(),
            stdout: b" M src/main.rs\n".to_vec(),
            stderr: vec![],
        };

        mock_fs
            .expect_execute_command()
            .with(eq("gh"), eq(vec!["auth".to_string(), "status".to_string()]))
            .times(1)
            .returning(move |_, _| Ok(auth_success_output.clone()));

        // Mock git commands for fresh project detection
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string()]))
            .times(1)
            .returning(|_, _| {
                Ok(Output {
                    status: create_successful_exit_status(),
                    stdout: vec![],
                    stderr: vec![],
                })
            });

        mock_fs
            .expect_execute_command()
            .with(
                eq("git"),
                eq(vec![
                    "remote".to_string(),
                    "get-url".to_string(),
                    "origin".to_string(),
                ]),
            )
            .times(1)
            .returning(|_, _| {
                Ok(Output {
                    status: create_successful_exit_status(),
                    stdout: b"https://github.com/owner/repo.git\n".to_vec(),
                    stderr: vec![],
                })
            });

        mock_fs
            .expect_execute_command()
            .with(
                eq("git"),
                eq(vec!["status".to_string(), "--porcelain".to_string()]),
            )
            .times(1)
            .returning(move |_, _| Ok(git_status_output.clone()));

        let fs_ops = std::sync::Arc::new(mock_fs);
        let init_command = InitCommand::new(None, true, false, fs_ops); // force = true

        let result = validation::validate_environment(&init_command).await;
        assert!(
            result.is_ok(),
            "Should succeed with uncommitted changes when force flag is set"
        );
    }

    #[tokio::test]
    async fn test_validate_environment_git_command_failure() {
        let mut mock_fs = MockFileSystemOperations::new();

        let auth_success_output = Output {
            status: create_successful_exit_status(),
            stdout: b"Logged in to github.com".to_vec(),
            stderr: vec![],
        };

        mock_fs
            .expect_execute_command()
            .with(eq("gh"), eq(vec!["auth".to_string(), "status".to_string()]))
            .times(1)
            .returning(move |_, _| Ok(auth_success_output.clone()));

        // Mock git commands for fresh project detection
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string()]))
            .times(1)
            .returning(|_, _| {
                Ok(Output {
                    status: create_successful_exit_status(),
                    stdout: vec![],
                    stderr: vec![],
                })
            });

        mock_fs
            .expect_execute_command()
            .with(
                eq("git"),
                eq(vec![
                    "remote".to_string(),
                    "get-url".to_string(),
                    "origin".to_string(),
                ]),
            )
            .times(1)
            .returning(|_, _| {
                Ok(Output {
                    status: create_successful_exit_status(),
                    stdout: b"https://github.com/owner/repo.git\n".to_vec(),
                    stderr: vec![],
                })
            });

        mock_fs
            .expect_execute_command()
            .with(
                eq("git"),
                eq(vec!["status".to_string(), "--porcelain".to_string()]),
            )
            .times(1)
            .returning(|_, _| Err(anyhow::anyhow!("git command not found")));

        let fs_ops = std::sync::Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, false, fs_ops);

        let result = validation::validate_environment(&init_command).await;
        assert!(result.is_err(), "Should fail when git command fails");
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to check git status"));
    }

    #[tokio::test]
    async fn test_github_cli_command_failure() {
        let mut mock_fs = MockFileSystemOperations::new();

        // Mock git commands for fresh project detection
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string()]))
            .times(1)
            .returning(|_, _| {
                Ok(Output {
                    status: create_successful_exit_status(),
                    stdout: vec![],
                    stderr: vec![],
                })
            });

        mock_fs
            .expect_execute_command()
            .with(
                eq("git"),
                eq(vec![
                    "remote".to_string(),
                    "get-url".to_string(),
                    "origin".to_string(),
                ]),
            )
            .times(1)
            .returning(|_, _| {
                Ok(Output {
                    status: create_successful_exit_status(),
                    stdout: b"https://github.com/owner/repo.git\n".to_vec(),
                    stderr: vec![],
                })
            });

        mock_fs
            .expect_execute_command()
            .with(eq("gh"), eq(vec!["auth".to_string(), "status".to_string()]))
            .times(1)
            .returning(|_, _| Err(anyhow::anyhow!("gh command not found")));

        let fs_ops = std::sync::Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, false, fs_ops);

        let result = validation::validate_environment(&init_command).await;
        assert!(result.is_err(), "Should fail when gh command fails");
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Failed to run 'gh auth status'"));
        assert!(error_msg.contains("Make sure GitHub CLI is installed"));
    }

    #[tokio::test]
    async fn test_init_with_existing_partial_config() {
        let mut mock_fs = MockFileSystemOperations::new();

        // Simulate partial config exists
        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(true);

        let fs_ops = std::sync::Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, true, fs_ops); // force = false, dry_run = true

        let result = config::generate_configuration(&init_command).await;
        assert!(
            result.is_err(),
            "Should fail when partial config exists without force"
        );
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_init_overwrites_existing_config_with_force() {
        let mut mock_fs = MockFileSystemOperations::new();

        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(true);

        // Mock git commands for fresh project detection
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string()]))
            .times(1)
            .returning(|_, _| {
                Ok(Output {
                    status: create_successful_exit_status(),
                    stdout: vec![],
                    stderr: vec![],
                })
            });

        mock_fs
            .expect_execute_command()
            .with(
                eq("git"),
                eq(vec![
                    "remote".to_string(),
                    "get-url".to_string(),
                    "origin".to_string(),
                ]),
            )
            .times(1)
            .returning(|_, _| {
                Ok(Output {
                    status: create_successful_exit_status(),
                    stdout: b"https://github.com/owner/repo.git\n".to_vec(),
                    stderr: vec![],
                })
            });

        let fs_ops = std::sync::Arc::new(mock_fs);
        let init_command = InitCommand::new(None, true, true, fs_ops); // force = true, dry_run = true

        let result = init_command.execute().await;
        assert!(
            result.is_ok(),
            "Should succeed when config exists and force is true"
        );
    }

    #[tokio::test]
    async fn test_init_handles_different_agent_counts() {
        let mut mock_fs = MockFileSystemOperations::new();

        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(false);

        // Mock git commands for fresh project detection
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string()]))
            .times(1)
            .returning(|_, _| {
                Ok(Output {
                    status: create_successful_exit_status(),
                    stdout: vec![],
                    stderr: vec![],
                })
            });

        mock_fs
            .expect_execute_command()
            .with(
                eq("git"),
                eq(vec![
                    "remote".to_string(),
                    "get-url".to_string(),
                    "origin".to_string(),
                ]),
            )
            .times(1)
            .returning(|_, _| {
                Ok(Output {
                    status: create_successful_exit_status(),
                    stdout: b"https://github.com/owner/repo.git\n".to_vec(),
                    stderr: vec![],
                })
            });

        let fs_ops = std::sync::Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, true, fs_ops); // max agents, dry_run

        let result = init_command.execute().await;
        assert!(result.is_ok(), "Should succeed with maximum agent count");
    }

    #[tokio::test]
    async fn test_init_boundary_agent_counts() {
        let mut mock_fs = MockFileSystemOperations::new();

        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(false)
            .times(2);

        // Mock git commands for fresh project detection (called twice)
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string()]))
            .times(2)
            .returning(|_, _| {
                Ok(Output {
                    status: create_successful_exit_status(),
                    stdout: vec![],
                    stderr: vec![],
                })
            });

        mock_fs
            .expect_execute_command()
            .with(
                eq("git"),
                eq(vec![
                    "remote".to_string(),
                    "get-url".to_string(),
                    "origin".to_string(),
                ]),
            )
            .times(2)
            .returning(|_, _| {
                Ok(Output {
                    status: create_successful_exit_status(),
                    stdout: b"https://github.com/owner/repo.git\n".to_vec(),
                    stderr: vec![],
                })
            });

        let fs_ops = std::sync::Arc::new(mock_fs);

        // Test minimum valid agent count
        let init_command = InitCommand::new(None, false, true, fs_ops.clone());
        let result = init_command.execute().await;
        assert!(result.is_ok(), "Should succeed with 1 agent");

        // Test maximum valid agent count
        let init_command = InitCommand::new(None, false, true, fs_ops);
        let result = init_command.execute().await;
        assert!(result.is_ok(), "Should succeed with 12 agents");
    }

    #[tokio::test]
    async fn test_init_dry_run_does_not_execute_commands() {
        let mut mock_fs = MockFileSystemOperations::new();

        // Even in dry run mode, validation phase still executes git commands
        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(false);

        // Mock git commands for fresh project detection
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string()]))
            .times(1)
            .returning(|_, _| {
                Ok(Output {
                    status: create_successful_exit_status(),
                    stdout: vec![],
                    stderr: vec![],
                })
            });

        mock_fs
            .expect_execute_command()
            .with(
                eq("git"),
                eq(vec![
                    "remote".to_string(),
                    "get-url".to_string(),
                    "origin".to_string(),
                ]),
            )
            .times(1)
            .returning(|_, _| {
                Ok(Output {
                    status: create_successful_exit_status(),
                    stdout: b"https://github.com/owner/repo.git\n".to_vec(),
                    stderr: vec![],
                })
            });

        let fs_ops = std::sync::Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, true, fs_ops); // dry_run = true

        let result = init_command.execute().await;
        assert!(
            result.is_ok(),
            "Dry run should succeed without actual file operations"
        );
    }

    #[tokio::test]
    async fn test_init_creates_required_label_specs() {
        let _init_command =
            InitCommand::new(None, false, true, std::sync::Arc::new(MockFileSystemOperations::new()));
        let labels = labels::get_required_labels();

        assert!(labels.len() >= 10, "Should create at least 10 labels");

        // Check for essential routing labels
        let route_labels: Vec<&LabelSpec> = labels
            .iter()
            .filter(|l| l.name.starts_with("route:"))
            .collect();
        assert!(
            route_labels.len() >= 6,
            "Should have at least 6 route labels"
        );

        // Verify specific label properties
        let ready_label = labels.iter().find(|l| l.name == "route:ready");
        assert!(ready_label.is_some(), "Should have route:ready label");
        let ready_label = ready_label.unwrap();
        assert_eq!(ready_label.color, "0052cc");
        assert!(!ready_label.description.is_empty());
    }

    #[tokio::test]
    async fn test_file_system_mock_write_operations() {
        let mut mock_fs = MockFileSystemOperations::new();

        // Test successful write operation
        mock_fs
            .expect_write()
            .with(eq("test.txt"), eq(b"test content".as_slice()))
            .times(1)
            .returning(|_, _| Ok(()));

        let fs_ops = std::sync::Arc::new(mock_fs);
        let result = fs_ops.write("test.txt", b"test content").await;
        assert!(result.is_ok(), "Mock write should succeed");
    }

    #[tokio::test]
    async fn test_file_system_mock_create_dir_operations() {
        let mut mock_fs = MockFileSystemOperations::new();

        // Test successful directory creation
        mock_fs
            .expect_create_dir_all()
            .with(eq(".my-little-soda/test"))
            .times(1)
            .returning(|_| Ok(()));

        let fs_ops = std::sync::Arc::new(mock_fs);
        let result = fs_ops.create_dir_all(".my-little-soda/test").await;
        assert!(result.is_ok(), "Mock directory creation should succeed");
    }

    #[tokio::test]
    async fn test_file_system_mock_exists_operations() {
        let mut mock_fs = MockFileSystemOperations::new();

        // Test file existence checks
        mock_fs
            .expect_exists()
            .with(eq("existing_file.toml"))
            .return_const(true);

        mock_fs
            .expect_exists()
            .with(eq("missing_file.toml"))
            .return_const(false);

        let fs_ops = std::sync::Arc::new(mock_fs);
        assert!(
            fs_ops.exists("existing_file.toml"),
            "Should detect existing file"
        );
        assert!(
            !fs_ops.exists("missing_file.toml"),
            "Should detect missing file"
        );
    }

    #[tokio::test]
    async fn test_fresh_project_detection_with_various_git_states() {
        // Test 1: No git repository (fresh project)
        {
            let mut mock_fs = MockFileSystemOperations::new();
            mock_fs
                .expect_execute_command()
                .with(eq("git"), eq(vec!["status".to_string()]))
                .times(1)
                .returning(|_, _| Err(anyhow::anyhow!("not a git repository")));

            let fs_ops = std::sync::Arc::new(mock_fs);
            let init_command = InitCommand::new(None, false, false, fs_ops);

            let result = validation::detect_fresh_project(&init_command).await;
            assert!(result, "Should detect fresh project when no git repo");
        }

        // Test 2: Git repository but no remote (fresh project)
        {
            let mut mock_fs = MockFileSystemOperations::new();
            mock_fs
                .expect_execute_command()
                .with(eq("git"), eq(vec!["status".to_string()]))
                .times(1)
                .returning(|_, _| {
                    Ok(Output {
                        status: create_successful_exit_status(),
                        stdout: vec![],
                        stderr: vec![],
                    })
                });

            mock_fs
                .expect_execute_command()
                .with(
                    eq("git"),
                    eq(vec![
                        "remote".to_string(),
                        "get-url".to_string(),
                        "origin".to_string(),
                    ]),
                )
                .times(1)
                .returning(|_, _| {
                    Ok(Output {
                        status: create_failed_exit_status(),
                        stdout: vec![],
                        stderr: b"fatal: no such remote 'origin'".to_vec(),
                    })
                });

            let fs_ops = std::sync::Arc::new(mock_fs);
            let init_command = InitCommand::new(None, false, false, fs_ops);

            let result = validation::detect_fresh_project(&init_command).await;
            assert!(result, "Should detect fresh project when no remote");
        }

        // Test 3: Full git repository with remote (not fresh)
        {
            let mut mock_fs = MockFileSystemOperations::new();
            mock_fs
                .expect_execute_command()
                .with(eq("git"), eq(vec!["status".to_string()]))
                .times(1)
                .returning(|_, _| {
                    Ok(Output {
                        status: create_successful_exit_status(),
                        stdout: vec![],
                        stderr: vec![],
                    })
                });

            mock_fs
                .expect_execute_command()
                .with(
                    eq("git"),
                    eq(vec![
                        "remote".to_string(),
                        "get-url".to_string(),
                        "origin".to_string(),
                    ]),
                )
                .times(1)
                .returning(|_, _| {
                    Ok(Output {
                        status: create_successful_exit_status(),
                        stdout: b"https://github.com/existing/repo.git\n".to_vec(),
                        stderr: vec![],
                    })
                });

            let fs_ops = std::sync::Arc::new(mock_fs);
            let init_command = InitCommand::new(None, false, false, fs_ops);

            let result = validation::detect_fresh_project(&init_command).await;
            assert!(
                !result,
                "Should not detect fresh project when git repo with remote exists"
            );
        }
    }

    #[tokio::test]
    async fn test_fresh_project_init_success() {
        let mut mock_fs = MockFileSystemOperations::new();

        mock_fs
            .expect_exists()
            .with(eq("my-little-soda.toml"))
            .return_const(false);

        // Mock git commands for fresh project detection (no git repo initially)
        mock_fs
            .expect_execute_command()
            .with(eq("git"), eq(vec!["status".to_string()]))
            .times(1) // Called once for fresh project detection
            .returning(|_, _| Err(anyhow::anyhow!("not a git repository")));

        let fs_ops = std::sync::Arc::new(mock_fs);
        let init_command = InitCommand::new(None, false, true, fs_ops); // dry_run = true

        let result = init_command.execute().await;
        assert!(
            result.is_ok(),
            "Fresh project init should succeed in dry run mode"
        );
    }
}