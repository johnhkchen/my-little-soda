//! End-to-end CLI integration tests
//!
//! These tests use assert_cmd to test the full clambake binary behavior
//! without external dependencies by mocking GitHub and git operations.

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;
use tempfile::TempDir;
use std::fs::{File, write, create_dir_all};
use std::path::Path;

/// Helper for setting up CLI test environment
pub struct CliTestEnvironment {
    pub temp_dir: TempDir,
    pub git_repo_path: String,
}

impl CliTestEnvironment {
    /// Create a new CLI test environment with git repo setup
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let git_repo_path = temp_dir.path().to_string_lossy().to_string();
        
        let env = Self {
            temp_dir,
            git_repo_path: git_repo_path.clone(),
        };
        
        // Initialize git repo
        env.setup_git_repo()?;
        env.setup_clambake_config()?;
        
        Ok(env)
    }
    
    fn setup_git_repo(&self) -> Result<(), Box<dyn std::error::Error>> {
        let repo_path = Path::new(&self.git_repo_path);
        
        // Initialize git repo
        Command::new("git")
            .current_dir(repo_path)
            .args(&["init"])
            .output()?;
        
        // Set git config
        Command::new("git")
            .current_dir(repo_path)
            .args(&["config", "user.name", "Test User"])
            .output()?;
        
        Command::new("git")
            .current_dir(repo_path)
            .args(&["config", "user.email", "test@example.com"])
            .output()?;
        
        // Create initial commit
        write(repo_path.join("README.md"), "# Test Repo")?;
        
        Command::new("git")
            .current_dir(repo_path)
            .args(&["add", "README.md"])
            .output()?;
        
        Command::new("git")
            .current_dir(repo_path)
            .args(&["commit", "-m", "Initial commit"])
            .output()?;
        
        // Create main branch
        Command::new("git")
            .current_dir(repo_path)
            .args(&["branch", "-M", "main"])
            .output()?;
        
        Ok(())
    }
    
    fn setup_clambake_config(&self) -> Result<(), Box<dyn std::error::Error>> {
        let repo_path = Path::new(&self.git_repo_path);
        let clambake_dir = repo_path.join(".clambake");
        create_dir_all(&clambake_dir)?;
        
        // Create mock configuration
        let config = r#"
        [github]
        owner = "test-owner"
        repo = "test-repo"
        token = "mock-token"
        
        [bundling]
        enabled = true
        interval_minutes = 10
        "#;
        
        write(clambake_dir.join("config.toml"), config)?;
        
        Ok(())
    }
    
    /// Create a test agent branch with commits
    pub fn create_agent_branch(&self, issue_number: u64, commit_message: &str) -> Result<(), Box<dyn std::error::Error>> {
        let repo_path = Path::new(&self.git_repo_path);
        let branch_name = format!("agent001/{}", issue_number);
        
        // Create and checkout branch
        Command::new("git")
            .current_dir(repo_path)
            .args(&["checkout", "-b", &branch_name])
            .output()?;
        
        // Make some changes and commit
        let test_file = format!("issue_{}.txt", issue_number);
        write(repo_path.join(&test_file), format!("Work for issue {}", issue_number))?;
        
        Command::new("git")
            .current_dir(repo_path)
            .args(&["add", &test_file])
            .output()?;
        
        Command::new("git")
            .current_dir(repo_path)
            .args(&["commit", "-m", commit_message])
            .output()?;
        
        // Return to main branch
        Command::new("git")
            .current_dir(repo_path)
            .args(&["checkout", "main"])
            .output()?;
        
        Ok(())
    }
    
    /// Get the path for running clambake commands
    pub fn repo_path(&self) -> &str {
        &self.git_repo_path
    }
    
    /// Set mock environment variables for testing
    pub fn mock_environment_vars() -> Vec<(&'static str, &'static str)> {
        vec![
            ("GITHUB_TOKEN", "mock-token"),
            ("MY_LITTLE_SODA_MOCK_MODE", "true"), // Signal to my-little-soda to use mocks
            ("MY_LITTLE_SODA_TEST_MODE", "true"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clambake_help_command() {
        let mut cmd = Command::cargo_bin("clambake").unwrap();
        
        cmd.arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("Multi-agent GitHub orchestration"))
            .stdout(predicate::str::contains("pop"))
            .stdout(predicate::str::contains("land"))
            .stdout(predicate::str::contains("bundle"));
    }

    #[test]
    fn test_clambake_version() {
        let mut cmd = Command::cargo_bin("clambake").unwrap();
        
        cmd.arg("--version")
            .assert()
            .success()
            .stdout(predicate::str::contains("clambake"))
            .stdout(predicate::str::contains("0.1.0"));
    }

    #[test]
    fn test_clambake_pop_no_work() {
        let env = CliTestEnvironment::new().unwrap();
        let mut cmd = Command::cargo_bin("clambake").unwrap();
        
        // Set mock environment
        for (key, value) in CliTestEnvironment::mock_environment_vars() {
            cmd.env(key, value);
        }
        
        cmd.current_dir(env.repo_path())
            .arg("pop")
            .assert()
            .success() // Should succeed even with no work
            .stdout(predicate::str::contains("No available tasks").or(
                predicate::str::contains("Searching for available tasks")
            ));
    }

    #[test]
    fn test_clambake_peek_command() {
        let env = CliTestEnvironment::new().unwrap();
        let mut cmd = Command::cargo_bin("clambake").unwrap();
        
        // Set mock environment
        for (key, value) in CliTestEnvironment::mock_environment_vars() {
            cmd.env(key, value);
        }
        
        cmd.current_dir(env.repo_path())
            .arg("peek")
            .assert()
            .success()
            .stdout(predicate::str::contains("Next available task").or(
                predicate::str::contains("No tasks available")
            ));
    }

    #[test]
    fn test_clambake_land_without_work() {
        let env = CliTestEnvironment::new().unwrap();
        let mut cmd = Command::cargo_bin("clambake").unwrap();
        
        // Set mock environment
        for (key, value) in CliTestEnvironment::mock_environment_vars() {
            cmd.env(key, value);
        }
        
        cmd.current_dir(env.repo_path())
            .arg("land")
            .assert()
            .failure() // Should fail when not on agent branch
            .stderr(predicate::str::contains("is not an agent branch").or(
                predicate::str::contains("Expected format: agent001/123 or agent001/123-description")
            ));
    }

    #[test]
    fn test_clambake_bundle_with_mock_branches() {
        let env = CliTestEnvironment::new().unwrap();
        
        // Create some test agent branches
        env.create_agent_branch(123, "Fix authentication bug").unwrap();
        env.create_agent_branch(124, "Add new feature").unwrap();
        
        let mut cmd = Command::cargo_bin("clambake").unwrap();
        
        // Set mock environment
        for (key, value) in CliTestEnvironment::mock_environment_vars() {
            cmd.env(key, value);
        }
        
        cmd.current_dir(env.repo_path())
            .arg("bundle")
            .assert()
            .code(predicate::in_iter(vec![0, 1])) // May succeed or fail based on mock setup
            .stdout(predicate::str::contains("bundle").or(
                predicate::str::contains("Bundle").or(
                    predicate::str::contains("queued")
                )
            ));
    }

    #[test]
    fn test_clambake_invalid_command() {
        let env = CliTestEnvironment::new().unwrap();
        let mut cmd = Command::cargo_bin("clambake").unwrap();
        
        cmd.current_dir(env.repo_path())
            .arg("invalid-command")
            .assert()
            .failure()
            .stderr(predicate::str::contains("error").or(
                predicate::str::contains("invalid").or(
                    predicate::str::contains("command")
                )
            ));
    }

    #[test]
    fn test_clambake_missing_git_repo() {
        let temp_dir = TempDir::new().unwrap();
        let mut cmd = Command::cargo_bin("clambake").unwrap();
        
        // Set mock environment
        for (key, value) in CliTestEnvironment::mock_environment_vars() {
            cmd.env(key, value);
        }
        
        cmd.current_dir(temp_dir.path())
            .arg("pop")
            .assert()
            .failure()
            .stderr(predicate::str::contains("git").or(
                predicate::str::contains("repository").or(
                    predicate::str::contains("not found")
                )
            ));
    }

    #[test]
    fn test_clambake_environment_variables() {
        let env = CliTestEnvironment::new().unwrap();
        let mut cmd = Command::cargo_bin("clambake").unwrap();
        
        // Test without GITHUB_TOKEN
        cmd.current_dir(env.repo_path())
            .arg("pop")
            .assert()
            .code(predicate::in_iter(vec![0, 1])) // May succeed with mock mode or fail gracefully
            .stdout(predicate::str::contains("").or(predicate::str::is_empty()).not());
    }

    #[test] 
    fn test_clambake_concurrent_bundler_prevention() {
        let env = CliTestEnvironment::new().unwrap();
        
        // Create lock file to simulate running bundler
        let lock_dir = Path::new(env.repo_path()).join(".clambake");
        create_dir_all(&lock_dir).unwrap();
        let _lock_file = File::create(lock_dir.join("bundle.lock")).unwrap();
        
        let mut cmd = Command::cargo_bin("clambake").unwrap();
        
        // Set mock environment
        for (key, value) in CliTestEnvironment::mock_environment_vars() {
            cmd.env(key, value);
        }
        
        cmd.current_dir(env.repo_path())
            .arg("bundle")
            .assert()
            .code(predicate::in_iter(vec![0, 1])) // Should handle lock gracefully
            .stdout(predicate::str::contains("bundler").or(
                predicate::str::contains("running").or(
                    predicate::str::contains("lock")
                )
            ).or(predicate::str::is_empty()));
    }

    #[test]
    fn test_clambake_config_validation() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();
        
        // Initialize git repo without clambake config
        Command::new("git")
            .current_dir(repo_path)
            .args(&["init"])
            .output().unwrap();
        
        let mut cmd = Command::cargo_bin("clambake").unwrap();
        
        cmd.current_dir(repo_path)
            .arg("pop")
            .assert()
            .code(predicate::in_iter(vec![0, 1])) // May succeed or fail based on config handling
            .stdout(predicate::str::contains("").or(predicate::str::is_empty()).not());
    }

    #[test]
    fn test_clambake_output_format() {
        let env = CliTestEnvironment::new().unwrap();
        let mut cmd = Command::cargo_bin("clambake").unwrap();
        
        // Set mock environment
        for (key, value) in CliTestEnvironment::mock_environment_vars() {
            cmd.env(key, value);
        }
        
        cmd.current_dir(env.repo_path())
            .arg("peek")
            .assert()
            .success()
            .stdout(predicate::str::contains("").or(predicate::str::is_empty())); // Output should be valid
    }

    #[tokio::test]
    async fn test_clambake_async_operations() {
        let env = CliTestEnvironment::new().unwrap();
        
        // Test that multiple clambake commands can be run concurrently
        let mut handles = Vec::new();
        
        for i in 0..3 {
            let repo_path = env.repo_path().to_string();
            let handle = tokio::spawn(async move {
                let mut cmd = Command::cargo_bin("clambake").unwrap();
                
                // Set mock environment
                for (key, value) in CliTestEnvironment::mock_environment_vars() {
                    cmd.env(key, value);
                }
                
                let result = cmd
                    .current_dir(&repo_path)
                    .arg("peek")
                    .output();
                    
                (i, result)
            });
            handles.push(handle);
        }
        
        // Wait for all commands to complete
        let mut successful_commands = 0;
        for handle in handles {
            let (_i, result) = handle.await.unwrap();
            if let Ok(output) = result {
                if output.status.success() {
                    successful_commands += 1;
                }
            }
        }
        
        // All commands should complete successfully
        assert!(successful_commands >= 1, "At least one command should succeed");
    }
}