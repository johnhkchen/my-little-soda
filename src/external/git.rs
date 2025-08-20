//! Git command abstractions
//! 
//! Provides trait-based abstractions for Git operations, enabling testable
//! Git integrations through dependency injection.

use async_trait::async_trait;
use thiserror::Error;
use super::command::{CommandExecutor, CommandError};
use std::sync::Arc;

pub type BranchName = String;
pub type CommitHash = String;

#[derive(Debug, Error)]
pub enum GitError {
    #[error("Repository not found or not a git repository")]
    RepositoryNotFound,
    #[error("Branch not found: {branch}")]
    BranchNotFound { branch: BranchName },
    #[error("Remote not found: {remote}")]
    RemoteNotFound { remote: String },
    #[error("Merge conflict detected")]
    MergeConflict,
    #[error("Working directory not clean")]
    WorkingDirectoryNotClean,
    #[error("Command execution error: {source}")]
    CommandError {
        #[from]
        source: CommandError,
    },
    #[error("Git command failed: {message}")]
    GitCommandFailed { message: String },
}

#[derive(Debug, Clone)]
pub struct GitStatus {
    pub current_branch: Option<BranchName>,
    pub is_clean: bool,
    pub staged_files: Vec<String>,
    pub unstaged_files: Vec<String>,
    pub untracked_files: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: BranchName,
    pub commit_hash: CommitHash,
    pub is_current: bool,
    pub upstream: Option<String>,
}

/// Trait for Git repository operations
/// 
/// This abstraction enables testing Git operations without actual repository
/// manipulation, while preserving the exact interface used by the application.
#[async_trait]
pub trait GitRepository: Send + Sync {
    /// Get the current branch name
    async fn current_branch(&self) -> Result<BranchName, GitError>;

    /// Get the number of commits in a range (e.g., "main..HEAD")
    async fn commit_count(&self, range: &str) -> Result<u32, GitError>;

    /// Check if a branch exists locally
    async fn branch_exists(&self, branch: &BranchName) -> Result<bool, GitError>;

    /// Check if a remote branch exists
    async fn remote_branch_exists(&self, remote: &str, branch: &BranchName) -> Result<bool, GitError>;

    /// Checkout a branch
    async fn checkout(&self, branch: &BranchName) -> Result<(), GitError>;

    /// Create a new branch from the current HEAD
    async fn create_branch(&self, branch: &BranchName) -> Result<(), GitError>;

    /// Fetch from a remote
    async fn fetch(&self, remote: &str) -> Result<(), GitError>;

    /// Push a branch to remote
    async fn push(&self, remote: &str, branch: &BranchName) -> Result<(), GitError>;

    /// Get repository status
    async fn status(&self) -> Result<GitStatus, GitError>;

    /// Check if one commit is an ancestor of another
    async fn is_ancestor(&self, ancestor: &str, descendant: &str) -> Result<bool, GitError>;

    /// Get the merge base between two commits/branches
    async fn merge_base(&self, commit1: &str, commit2: &str) -> Result<CommitHash, GitError>;

    /// List local branches
    async fn list_branches(&self) -> Result<Vec<BranchInfo>, GitError>;

    /// Delete a local branch
    async fn delete_branch(&self, branch: &BranchName, force: bool) -> Result<(), GitError>;
}

/// Real Git implementation
pub struct GitClient {
    executor: Arc<dyn CommandExecutor>,
}

impl GitClient {
    pub fn new(executor: Arc<dyn CommandExecutor>) -> Self {
        Self { executor }
    }

    async fn execute_git_command(&self, args: &[&str]) -> Result<String, GitError> {
        let output = self.executor.execute("git", args).await?;
        
        if !output.success() {
            return Err(self.classify_git_error(&output.stderr, args));
        }
        
        Ok(output.stdout.trim().to_string())
    }

    fn classify_git_error(&self, stderr: &str, args: &[&str]) -> GitError {
        if stderr.contains("not a git repository") {
            GitError::RepositoryNotFound
        } else if stderr.contains("not found") && args.contains(&"checkout") {
            GitError::BranchNotFound {
                branch: args.get(2).unwrap_or(&"unknown").to_string(),
            }
        } else if stderr.contains("not found") && args.contains(&"remote") {
            GitError::RemoteNotFound {
                remote: args.get(2).unwrap_or(&"unknown").to_string(),
            }
        } else if stderr.contains("merge conflict") || stderr.contains("CONFLICT") {
            GitError::MergeConflict
        } else if stderr.contains("working tree clean") {
            GitError::WorkingDirectoryNotClean
        } else {
            GitError::GitCommandFailed {
                message: stderr.to_string(),
            }
        }
    }

    fn parse_status_output(&self, output: &str) -> GitStatus {
        let mut staged_files = Vec::new();
        let mut unstaged_files = Vec::new();
        let mut untracked_files = Vec::new();

        for line in output.lines() {
            if line.len() < 3 {
                continue;
            }

            let status_chars: Vec<char> = line.chars().collect();
            let filename = &line[3..];

            match (status_chars[0], status_chars[1]) {
                ('A', _) | ('M', _) | ('D', _) | ('R', _) | ('C', _) => {
                    staged_files.push(filename.to_string());
                }
                (_, 'M') | (_, 'D') => {
                    unstaged_files.push(filename.to_string());
                }
                ('?', '?') => {
                    untracked_files.push(filename.to_string());
                }
                _ => {}
            }
        }

        let is_clean = staged_files.is_empty() && unstaged_files.is_empty() && untracked_files.is_empty();

        GitStatus {
            current_branch: None, // Will be set by the caller
            is_clean,
            staged_files,
            unstaged_files,
            untracked_files,
        }
    }
}

#[async_trait]
impl GitRepository for GitClient {
    async fn current_branch(&self) -> Result<BranchName, GitError> {
        // Try the newer command first
        if let Ok(branch) = self.execute_git_command(&["branch", "--show-current"]).await {
            if !branch.is_empty() {
                return Ok(branch);
            }
        }

        // Fallback to the older method
        let output = self.execute_git_command(&["rev-parse", "--abbrev-ref", "HEAD"]).await?;
        
        if output == "HEAD" {
            return Err(GitError::GitCommandFailed {
                message: "HEAD is detached".to_string(),
            });
        }

        Ok(output)
    }

    async fn commit_count(&self, range: &str) -> Result<u32, GitError> {
        let output = self.execute_git_command(&["rev-list", "--count", range]).await?;
        
        output.parse().map_err(|e| GitError::GitCommandFailed {
            message: format!("Failed to parse commit count '{}': {}", output, e),
        })
    }

    async fn branch_exists(&self, branch: &BranchName) -> Result<bool, GitError> {
        let result = self.execute_git_command(&["show-ref", "--verify", "--quiet", &format!("refs/heads/{}", branch)]).await;
        
        match result {
            Ok(_) => Ok(true),
            Err(GitError::GitCommandFailed { .. }) => Ok(false),
            Err(GitError::CommandError { source: CommandError::ExecutionFailed { .. } }) => Ok(false),
            Err(e) => Err(e),
        }
    }

    async fn remote_branch_exists(&self, remote: &str, branch: &BranchName) -> Result<bool, GitError> {
        let result = self.execute_git_command(&["show-ref", "--verify", "--quiet", &format!("refs/remotes/{}/{}", remote, branch)]).await;
        
        match result {
            Ok(_) => Ok(true),
            Err(GitError::GitCommandFailed { .. }) => Ok(false),
            Err(GitError::CommandError { source: CommandError::ExecutionFailed { .. } }) => Ok(false),
            Err(e) => Err(e),
        }
    }

    async fn checkout(&self, branch: &BranchName) -> Result<(), GitError> {
        self.execute_git_command(&["checkout", branch]).await?;
        Ok(())
    }

    async fn create_branch(&self, branch: &BranchName) -> Result<(), GitError> {
        self.execute_git_command(&["checkout", "-b", branch]).await?;
        Ok(())
    }

    async fn fetch(&self, remote: &str) -> Result<(), GitError> {
        self.execute_git_command(&["fetch", remote]).await?;
        Ok(())
    }

    async fn push(&self, remote: &str, branch: &BranchName) -> Result<(), GitError> {
        self.execute_git_command(&["push", remote, branch]).await?;
        Ok(())
    }

    async fn status(&self) -> Result<GitStatus, GitError> {
        let status_output = self.execute_git_command(&["status", "--porcelain"]).await?;
        let current_branch = self.current_branch().await.ok();

        let mut status = self.parse_status_output(&status_output);
        status.current_branch = current_branch;

        Ok(status)
    }

    async fn is_ancestor(&self, ancestor: &str, descendant: &str) -> Result<bool, GitError> {
        let result = self.execute_git_command(&["merge-base", "--is-ancestor", ancestor, descendant]).await;
        
        match result {
            Ok(_) => Ok(true),
            Err(GitError::GitCommandFailed { .. }) => Ok(false),
            Err(GitError::CommandError { source: CommandError::ExecutionFailed { .. } }) => Ok(false),
            Err(e) => Err(e),
        }
    }

    async fn merge_base(&self, commit1: &str, commit2: &str) -> Result<CommitHash, GitError> {
        self.execute_git_command(&["merge-base", commit1, commit2]).await
    }

    async fn list_branches(&self) -> Result<Vec<BranchInfo>, GitError> {
        let output = self.execute_git_command(&["branch", "-v", "--format=%(refname:short) %(objectname:short) %(HEAD)"]).await?;
        
        let mut branches = Vec::new();
        for line in output.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let name = parts[0].to_string();
                let commit_hash = parts[1].to_string();
                let is_current = parts.get(2) == Some(&"*");

                branches.push(BranchInfo {
                    name,
                    commit_hash,
                    is_current,
                    upstream: None, // Could be enhanced to parse upstream info
                });
            }
        }

        Ok(branches)
    }

    async fn delete_branch(&self, branch: &BranchName, force: bool) -> Result<(), GitError> {
        let flag = if force { "-D" } else { "-d" };
        self.execute_git_command(&["branch", flag, branch]).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use mockall::mock;
    use super::super::command::{CommandOutput, CommandError};

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
    async fn test_current_branch_success() {
        let mock_executor = MockCommandExecutor::new()
            .expect_command("git", &["branch", "--show-current"], 
                Ok(CommandOutput {
                    status_code: 0,
                    stdout: "main\n".to_string(),
                    stderr: String::new(),
                }));

        let client = GitClient::new(Arc::new(mock_executor));
        let result = client.current_branch().await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "main");
    }

    #[tokio::test]
    async fn test_branch_exists_true() {
        let mock_executor = MockCommandExecutor::new()
            .expect_command("git", &["show-ref", "--verify", "--quiet", "refs/heads/feature-branch"], 
                Ok(CommandOutput {
                    status_code: 0,
                    stdout: String::new(),
                    stderr: String::new(),
                }));

        let client = GitClient::new(Arc::new(mock_executor));
        let result = client.branch_exists(&"feature-branch".to_string()).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
    }

    #[tokio::test]
    async fn test_branch_exists_false() {
        let mock_executor = MockCommandExecutor::new()
            .expect_command("git", &["show-ref", "--verify", "--quiet", "refs/heads/nonexistent"], 
                Ok(CommandOutput {
                    status_code: 1,
                    stdout: String::new(),
                    stderr: "fatal: Failed to resolve 'refs/heads/nonexistent' as a valid ref.".to_string(),
                }));

        let client = GitClient::new(Arc::new(mock_executor));
        let result = client.branch_exists(&"nonexistent".to_string()).await;

        // This should return Ok(false) rather than an error
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);
    }
}