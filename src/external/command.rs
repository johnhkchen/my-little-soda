//! Base command execution abstraction
//! 
//! Provides the foundational trait for executing external commands, enabling
//! dependency injection for testing.

use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub status_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl CommandOutput {
    pub fn success(&self) -> bool {
        self.status_code == 0
    }
}

#[derive(Debug, Error, Clone)]
pub enum CommandError {
    #[error("Command execution failed: {message}")]
    ExecutionFailed { message: String },
    #[error("Command not found: {command}")]
    CommandNotFound { command: String },
    #[error("Command timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },
    #[error("IO error: {message}")]
    Io { message: String },
}

/// Trait for executing external commands
/// 
/// This abstraction allows the rest of the codebase to execute commands
/// without directly depending on std::process::Command, enabling testing
/// with mock implementations.
#[async_trait]
pub trait CommandExecutor: Send + Sync {
    async fn execute(&self, program: &str, args: &[&str]) -> Result<CommandOutput, CommandError>;
}

/// Real implementation using std::process::Command
pub struct ProcessCommandExecutor;

#[async_trait]
impl CommandExecutor for ProcessCommandExecutor {
    async fn execute(&self, program: &str, args: &[&str]) -> Result<CommandOutput, CommandError> {
        use std::process::Command;
        
        let output = Command::new(program)
            .args(args)
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    CommandError::CommandNotFound {
                        command: program.to_string(),
                    }
                } else {
                    CommandError::Io { message: e.to_string() }
                }
            })?;

        Ok(CommandOutput {
            status_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use mockall::mock;

    // Simple mock for testing
    struct MockCommandExecutor {
        responses: std::collections::HashMap<String, Result<CommandOutput, CommandError>>,
    }

    impl MockCommandExecutor {
        fn new() -> Self {
            Self {
                responses: std::collections::HashMap::new(),
            }
        }

        fn expect_command(mut self, program: &str, args: &[&str], response: Result<CommandOutput, CommandError>) -> Self {
            let key = format!("{} {}", program, args.join(" "));
            self.responses.insert(key, response);
            self
        }
    }

    #[async_trait]
    impl CommandExecutor for MockCommandExecutor {
        async fn execute(&self, program: &str, args: &[&str]) -> Result<CommandOutput, CommandError> {
            let key = format!("{} {}", program, args.join(" "));
            self.responses.get(&key)
                .cloned()
                .unwrap_or(Err(CommandError::CommandNotFound {
                    command: program.to_string(),
                }))
        }
    }

    #[tokio::test]
    async fn test_process_command_executor_success() {
        let executor = ProcessCommandExecutor;
        let result = executor.execute("echo", &["hello"]).await;
        
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.success());
        assert!(output.stdout.contains("hello"));
    }

    #[tokio::test]
    async fn test_process_command_executor_command_not_found() {
        let executor = ProcessCommandExecutor;
        let result = executor.execute("nonexistent_command_xyz", &[]).await;
        
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CommandError::CommandNotFound { .. }));
    }

    #[tokio::test]
    async fn test_mock_command_executor() {
        let mock = MockCommandExecutor::new()
            .expect_command("echo", &["hello"], Ok(CommandOutput {
                status_code: 0,
                stdout: "hello\n".to_string(),
                stderr: String::new(),
            }));

        let result = mock.execute("echo", &["hello"]).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.success());
        assert_eq!(output.stdout, "hello\n");
    }
}