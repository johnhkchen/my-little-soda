//! Command execution module
//!
//! This module provides the `RealCommandExecutor` implementation that executes
//! agent lifecycle commands against real Git repositories and GitHub APIs.
//!
//! # Architecture
//!
//! The executor handles three types of commands:
//! - **Git commands**: Repository operations (checkout, commit, push, etc.)
//! - **GitHub commands**: API operations (labels, issues, PRs, etc.)
//! - **Print commands**: Console output for user feedback
//!
//! # Design Principles
//!
//! - **Dependency Injection**: Uses trait objects for testability
//! - **Error Propagation**: Comprehensive error handling with context
//! - **GitHub Source of Truth**: Prioritizes GitHub state over local state
//! - **Atomic Operations**: Commands either succeed completely or fail cleanly
//!
//! # Example
//!
//! ```rust,ignore
//! use agent_lifecycle::executor::RealCommandExecutor;
//! use agent_lifecycle::types::{Command, GitCommand};
//!
//! let executor = RealCommandExecutor::new(github_client, git_ops);
//! let result = executor.execute(&Command::Git(GitCommand::Checkout {
//!     branch: "feature-branch".to_string()
//! }))?;
//! ```

use crate::agent_lifecycle::traits::*;
use crate::agent_lifecycle::types::*;
use anyhow::Result;

/// Production implementation of command execution
///
/// The `RealCommandExecutor` executes agent lifecycle commands against real
/// systems - GitHub APIs and local Git repositories. It provides the runtime
/// implementation of the abstract command patterns defined in the agent
/// lifecycle state machine.
///
/// # Type Parameters
///
/// - `G`: GitHub operations trait implementation
/// - `O`: Git operations trait implementation
///
/// # Command Types Supported
///
/// - **Git Commands**: All local repository operations
/// - **GitHub Commands**: All GitHub API operations
/// - **Print Commands**: User-facing status messages
///
/// # Error Handling
///
/// All operations return `Result<CommandResult>` with detailed error context.
/// Failed commands include information for debugging and potential retry.
pub struct RealCommandExecutor<G: GitHubOperations, O: GitOperations> {
    /// GitHub operations for API interactions
    github_ops: G,
    /// Git operations for repository interactions
    git_ops: O,
}

impl<G: GitHubOperations, O: GitOperations> RealCommandExecutor<G, O> {
    /// Create new real command executor with GitHub and Git operations
    ///
    /// # Arguments
    ///
    /// - `github_ops`: Implementation of GitHub API operations
    /// - `git_ops`: Implementation of Git repository operations
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let executor = RealCommandExecutor::new(
    ///     GitHubClient::new()?,
    ///     Git2Operations::new()?
    /// );
    /// ```
    pub fn new(github_ops: G, git_ops: O) -> Self {
        Self {
            github_ops,
            git_ops,
        }
    }
}

impl<G: GitHubOperations, O: GitOperations> CommandExecutor for RealCommandExecutor<G, O> {
    /// Execute a single command of any type
    ///
    /// This is the main entry point for command execution. It dispatches
    /// commands to appropriate handlers based on command type.
    ///
    /// # Arguments
    ///
    /// - `command`: The command to execute (Git, GitHub, or Print)
    ///
    /// # Returns
    ///
    /// - `CommandResult` containing execution status, output, and any errors
    ///
    /// # Command Types
    ///
    /// - **Git Commands**: Delegated to `execute_git_command`
    /// - **GitHub Commands**: Delegated to `execute_github_command`
    /// - **Print Commands**: Executed inline with console output
    ///
    /// # Errors
    ///
    /// Returns error if the underlying operation fails. Error context
    /// includes command type and specific failure information.
    fn execute(&self, command: &Command) -> Result<CommandResult> {
        match command {
            Command::Git(git_cmd) => self.execute_git_command(git_cmd),
            Command::GitHub(github_cmd) => self.execute_github_command(github_cmd),
            Command::Print(message) => {
                println!("{message}");
                Ok(CommandResult {
                    success: true,
                    output: message.clone(),
                    error: None,
                    data: None,
                })
            }
            Command::Warning(message) => {
                eprintln!("WARNING: {message}");
                Ok(CommandResult {
                    success: true,
                    output: message.clone(),
                    error: None,
                    data: None,
                })
            }
            Command::Error(message) => {
                eprintln!("ERROR: {message}");
                Ok(CommandResult {
                    success: false,
                    output: String::new(),
                    error: Some(message.clone()),
                    data: None,
                })
            }
            Command::Sequence(commands) => self.execute_sequence(commands).map(|results| {
                let success = results.iter().all(|r| r.success);
                let output = results
                    .iter()
                    .map(|r| r.output.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");

                CommandResult {
                    success,
                    output,
                    error: results.iter().find_map(|r| r.error.as_ref()).cloned(),
                    data: None,
                }
            }),
            Command::Conditional {
                condition,
                then_cmd,
                else_cmd,
            } => {
                let condition_result = self.evaluate_condition(condition)?;
                if condition_result {
                    self.execute(then_cmd)
                } else if let Some(else_command) = else_cmd {
                    self.execute(else_command)
                } else {
                    Ok(CommandResult {
                        success: true,
                        output: "Condition was false, no else branch".to_string(),
                        error: None,
                        data: None,
                    })
                }
            }
        }
    }

    /// Execute commands with rollback on failure
    fn execute_atomic(&self, commands: &[Command]) -> Result<Vec<CommandResult>> {
        // For now, just execute in sequence
        // A full implementation would support rollback operations
        self.execute_sequence(commands)
    }
}

impl<G: GitHubOperations, O: GitOperations> RealCommandExecutor<G, O> {
    /// Execute Git command
    fn execute_git_command(&self, git_cmd: &GitCommand) -> Result<CommandResult> {
        match git_cmd {
            GitCommand::GetCurrentBranch => match self.git_ops.get_current_branch() {
                Ok(branch) => Ok(CommandResult {
                    success: true,
                    output: branch.clone(),
                    error: None,
                    data: Some(CommandData::String(branch)),
                }),
                Err(e) => Ok(CommandResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                    data: None,
                }),
            },
            GitCommand::GetCommitsAhead { base } => match self.git_ops.get_commits_ahead(base) {
                Ok(commits) => Ok(CommandResult {
                    success: true,
                    output: format!("{} commits ahead", commits.len()),
                    error: None,
                    data: Some(CommandData::StringList(commits)),
                }),
                Err(e) => Ok(CommandResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                    data: None,
                }),
            },
            GitCommand::GetCommitsBehind { base } => match self.git_ops.get_commits_behind(base) {
                Ok(count) => Ok(CommandResult {
                    success: true,
                    output: format!("{count} commits behind"),
                    error: None,
                    data: Some(CommandData::Number(count as u64)),
                }),
                Err(e) => Ok(CommandResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                    data: None,
                }),
            },
            GitCommand::CheckoutBranch { branch } => match self.git_ops.checkout_branch(branch) {
                Ok(()) => Ok(CommandResult {
                    success: true,
                    output: format!("Checked out branch: {branch}"),
                    error: None,
                    data: None,
                }),
                Err(e) => Ok(CommandResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                    data: None,
                }),
            },
            GitCommand::Push { remote, branch } => match self.git_ops.push(remote, branch) {
                Ok(()) => Ok(CommandResult {
                    success: true,
                    output: format!("Pushed {branch} to {remote}"),
                    error: None,
                    data: None,
                }),
                Err(e) => Ok(CommandResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                    data: None,
                }),
            },
            GitCommand::CreateBranch { name, from } => {
                match self.git_ops.create_branch(name, from) {
                    Ok(()) => Ok(CommandResult {
                        success: true,
                        output: format!("Created branch {name} from {from}"),
                        error: None,
                        data: None,
                    }),
                    Err(e) => Ok(CommandResult {
                        success: false,
                        output: String::new(),
                        error: Some(e.to_string()),
                        data: None,
                    }),
                }
            }
            GitCommand::DeleteBranch { name } => match self.git_ops.delete_branch(name) {
                Ok(()) => Ok(CommandResult {
                    success: true,
                    output: format!("Deleted branch: {name}"),
                    error: None,
                    data: None,
                }),
                Err(e) => Ok(CommandResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                    data: None,
                }),
            },
            GitCommand::Commit { message } => match self.git_ops.commit(message) {
                Ok(()) => Ok(CommandResult {
                    success: true,
                    output: format!("Committed: {message}"),
                    error: None,
                    data: None,
                }),
                Err(e) => Ok(CommandResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                    data: None,
                }),
            },
            GitCommand::Add { files } => match self.git_ops.add_files(files) {
                Ok(()) => Ok(CommandResult {
                    success: true,
                    output: format!("Added {} files", files.len()),
                    error: None,
                    data: None,
                }),
                Err(e) => Ok(CommandResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                    data: None,
                }),
            },
            GitCommand::GetMergeConflicts { base } => {
                match self.git_ops.get_merge_conflicts(base) {
                    Ok(conflicts) => Ok(CommandResult {
                        success: true,
                        output: format!("{} merge conflicts", conflicts.len()),
                        error: None,
                        data: Some(CommandData::StringList(conflicts)),
                    }),
                    Err(e) => Ok(CommandResult {
                        success: false,
                        output: String::new(),
                        error: Some(e.to_string()),
                        data: None,
                    }),
                }
            }
            GitCommand::IsClean => match self.git_ops.is_clean() {
                Ok(clean) => Ok(CommandResult {
                    success: true,
                    output: if clean { "clean" } else { "dirty" }.to_string(),
                    error: None,
                    data: Some(CommandData::String(clean.to_string())),
                }),
                Err(e) => Ok(CommandResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                    data: None,
                }),
            },
            GitCommand::GetStatus => match self.git_ops.get_status() {
                Ok(status) => Ok(CommandResult {
                    success: true,
                    output: status.clone(),
                    error: None,
                    data: Some(CommandData::String(status)),
                }),
                Err(e) => Ok(CommandResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                    data: None,
                }),
            },
        }
    }

    /// Execute GitHub command
    fn execute_github_command(&self, github_cmd: &GitHubCommand) -> Result<CommandResult> {
        match github_cmd {
            GitHubCommand::AddLabel { issue, label } => {
                match self.github_ops.add_label(*issue, label) {
                    Ok(()) => Ok(CommandResult {
                        success: true,
                        output: format!("Added label '{label}' to issue #{issue}"),
                        error: None,
                        data: None,
                    }),
                    Err(e) => Ok(CommandResult {
                        success: false,
                        output: String::new(),
                        error: Some(e.to_string()),
                        data: None,
                    }),
                }
            }
            GitHubCommand::RemoveLabel { issue, label } => {
                match self.github_ops.remove_label(*issue, label) {
                    Ok(()) => Ok(CommandResult {
                        success: true,
                        output: format!("Removed label '{label}' from issue #{issue}"),
                        error: None,
                        data: None,
                    }),
                    Err(e) => Ok(CommandResult {
                        success: false,
                        output: String::new(),
                        error: Some(e.to_string()),
                        data: None,
                    }),
                }
            }
            GitHubCommand::GetIssue { issue } => match self.github_ops.get_issue(*issue) {
                Ok(issue_data) => Ok(CommandResult {
                    success: true,
                    output: format!("Issue #{}: {}", issue_data.number, issue_data.title),
                    error: None,
                    data: Some(CommandData::Issue(issue_data)),
                }),
                Err(e) => Ok(CommandResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                    data: None,
                }),
            },
            GitHubCommand::GetLabels { issue } => match self.github_ops.get_labels(*issue) {
                Ok(labels) => Ok(CommandResult {
                    success: true,
                    output: format!("Labels: {}", labels.join(", ")),
                    error: None,
                    data: Some(CommandData::StringList(labels)),
                }),
                Err(e) => Ok(CommandResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                    data: None,
                }),
            },
            GitHubCommand::CreatePR {
                title,
                body,
                head,
                base,
            } => match self.github_ops.create_pr(title, body, head, base) {
                Ok(pr_url) => Ok(CommandResult {
                    success: true,
                    output: format!("Created PR: {pr_url}"),
                    error: None,
                    data: Some(CommandData::String(pr_url)),
                }),
                Err(e) => Ok(CommandResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                    data: None,
                }),
            },
            GitHubCommand::MergePR { number } => match self.github_ops.merge_pr(*number) {
                Ok(()) => Ok(CommandResult {
                    success: true,
                    output: format!("Merged PR #{number}"),
                    error: None,
                    data: None,
                }),
                Err(e) => Ok(CommandResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                    data: None,
                }),
            },
            GitHubCommand::ClosePR { number } => match self.github_ops.close_pr(*number) {
                Ok(()) => Ok(CommandResult {
                    success: true,
                    output: format!("Closed PR #{number}"),
                    error: None,
                    data: None,
                }),
                Err(e) => Ok(CommandResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                    data: None,
                }),
            },
            GitHubCommand::GetPR { number } => match self.github_ops.get_pr(*number) {
                Ok(pr_data) => Ok(CommandResult {
                    success: true,
                    output: format!("PR #{}: {}", pr_data.number, pr_data.title),
                    error: None,
                    data: Some(CommandData::PR(pr_data)),
                }),
                Err(e) => Ok(CommandResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                    data: None,
                }),
            },
        }
    }

    /// Evaluate condition for conditional commands
    fn evaluate_condition(&self, condition: &Condition) -> Result<bool> {
        match condition {
            Condition::Always => Ok(true),
            Condition::Never => Ok(false),
            Condition::BranchExists { branch } => self.git_ops.branch_exists(branch),
            Condition::IssueHasLabel { issue, label } => {
                self.github_ops.issue_has_label(*issue, label)
            }
            Condition::HasCommits { base } => {
                let commits = self.git_ops.get_commits_ahead(base)?;
                Ok(!commits.is_empty())
            }
            Condition::IsClean => self.git_ops.is_clean(),
        }
    }
}

impl<G: GitHubOperations, O: GitOperations> ConditionEvaluator for RealCommandExecutor<G, O> {
    /// Evaluate a condition
    fn evaluate(&self, condition: &Condition) -> Result<bool> {
        self.evaluate_condition(condition)
    }
}
