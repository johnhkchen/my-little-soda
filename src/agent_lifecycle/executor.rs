// Command executor implementations

use crate::agent_lifecycle::types::*;
use crate::agent_lifecycle::traits::*;
use anyhow::Result;

/// Real command executor that performs actual Git and GitHub operations
pub struct RealCommandExecutor<G: GitHubOperations, O: GitOperations> {
    github_ops: G,
    git_ops: O,
}

impl<G: GitHubOperations, O: GitOperations> RealCommandExecutor<G, O> {
    /// Create new real command executor
    pub fn new(github_ops: G, git_ops: O) -> Self {
        Self {
            github_ops,
            git_ops,
        }
    }
}

impl<G: GitHubOperations, O: GitOperations> CommandExecutor for RealCommandExecutor<G, O> {
    /// Execute a single command
    fn execute(&self, command: &Command) -> Result<CommandResult> {
        match command {
            Command::Git(git_cmd) => self.execute_git_command(git_cmd),
            Command::GitHub(github_cmd) => self.execute_github_command(github_cmd),
            Command::Print(message) => {
                println!("{}", message);
                Ok(CommandResult {
                    success: true,
                    output: message.clone(),
                    error: None,
                    data: None,
                })
            }
            Command::Warning(message) => {
                eprintln!("WARNING: {}", message);
                Ok(CommandResult {
                    success: true,
                    output: message.clone(),
                    error: None,
                    data: None,
                })
            }
            Command::Error(message) => {
                eprintln!("ERROR: {}", message);
                Ok(CommandResult {
                    success: false,
                    output: String::new(),
                    error: Some(message.clone()),
                    data: None,
                })
            }
            Command::Sequence(commands) => {
                self.execute_sequence(commands)
                    .map(|results| {
                        let success = results.iter().all(|r| r.success);
                        let output = results.iter()
                            .map(|r| r.output.as_str())
                            .collect::<Vec<_>>()
                            .join("\n");
                        
                        CommandResult {
                            success,
                            output,
                            error: results.iter()
                                .find_map(|r| r.error.as_ref())
                                .cloned(),
                            data: None,
                        }
                    })
            }
            Command::Conditional { condition, then_cmd, else_cmd } => {
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
            GitCommand::GetCurrentBranch => {
                match self.git_ops.get_current_branch() {
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
                }
            }
            GitCommand::GetCommitsAhead { base } => {
                match self.git_ops.get_commits_ahead(base) {
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
                }
            }
            GitCommand::GetCommitsBehind { base } => {
                match self.git_ops.get_commits_behind(base) {
                    Ok(count) => Ok(CommandResult {
                        success: true,
                        output: format!("{} commits behind", count),
                        error: None,
                        data: Some(CommandData::Number(count as u64)),
                    }),
                    Err(e) => Ok(CommandResult {
                        success: false,
                        output: String::new(),
                        error: Some(e.to_string()),
                        data: None,
                    }),
                }
            }
            GitCommand::CheckoutBranch { branch } => {
                match self.git_ops.checkout_branch(branch) {
                    Ok(()) => Ok(CommandResult {
                        success: true,
                        output: format!("Checked out branch: {}", branch),
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
            GitCommand::Push { remote, branch } => {
                match self.git_ops.push(remote, branch) {
                    Ok(()) => Ok(CommandResult {
                        success: true,
                        output: format!("Pushed {} to {}", branch, remote),
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
            GitCommand::CreateBranch { name, from } => {
                match self.git_ops.create_branch(name, from) {
                    Ok(()) => Ok(CommandResult {
                        success: true,
                        output: format!("Created branch {} from {}", name, from),
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
            GitCommand::DeleteBranch { name } => {
                match self.git_ops.delete_branch(name) {
                    Ok(()) => Ok(CommandResult {
                        success: true,
                        output: format!("Deleted branch: {}", name),
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
            GitCommand::Commit { message } => {
                match self.git_ops.commit(message) {
                    Ok(()) => Ok(CommandResult {
                        success: true,
                        output: format!("Committed: {}", message),
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
            GitCommand::Add { files } => {
                match self.git_ops.add_files(files) {
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
                }
            }
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
            GitCommand::IsClean => {
                match self.git_ops.is_clean() {
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
                }
            }
            GitCommand::GetStatus => {
                match self.git_ops.get_status() {
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
                }
            }
        }
    }
    
    /// Execute GitHub command
    fn execute_github_command(&self, github_cmd: &GitHubCommand) -> Result<CommandResult> {
        match github_cmd {
            GitHubCommand::AddLabel { issue, label } => {
                match self.github_ops.add_label(*issue, label) {
                    Ok(()) => Ok(CommandResult {
                        success: true,
                        output: format!("Added label '{}' to issue #{}", label, issue),
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
                        output: format!("Removed label '{}' from issue #{}", label, issue),
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
            GitHubCommand::GetIssue { issue } => {
                match self.github_ops.get_issue(*issue) {
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
                }
            }
            GitHubCommand::GetLabels { issue } => {
                match self.github_ops.get_labels(*issue) {
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
                }
            }
            GitHubCommand::CreatePR { title, body, head, base } => {
                match self.github_ops.create_pr(title, body, head, base) {
                    Ok(pr_url) => Ok(CommandResult {
                        success: true,
                        output: format!("Created PR: {}", pr_url),
                        error: None,
                        data: Some(CommandData::String(pr_url)),
                    }),
                    Err(e) => Ok(CommandResult {
                        success: false,
                        output: String::new(),
                        error: Some(e.to_string()),
                        data: None,
                    }),
                }
            }
            GitHubCommand::MergePR { number } => {
                match self.github_ops.merge_pr(*number) {
                    Ok(()) => Ok(CommandResult {
                        success: true,
                        output: format!("Merged PR #{}", number),
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
            GitHubCommand::ClosePR { number } => {
                match self.github_ops.close_pr(*number) {
                    Ok(()) => Ok(CommandResult {
                        success: true,
                        output: format!("Closed PR #{}", number),
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
            GitHubCommand::GetPR { number } => {
                match self.github_ops.get_pr(*number) {
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
                }
            }
        }
    }
    
    /// Evaluate condition for conditional commands
    fn evaluate_condition(&self, condition: &Condition) -> Result<bool> {
        match condition {
            Condition::Always => Ok(true),
            Condition::Never => Ok(false),
            Condition::BranchExists { branch } => {
                self.git_ops.branch_exists(branch)
            }
            Condition::IssueHasLabel { issue, label } => {
                self.github_ops.issue_has_label(*issue, label)
            }
            Condition::HasCommits { base } => {
                let commits = self.git_ops.get_commits_ahead(base)?;
                Ok(!commits.is_empty())
            }
            Condition::IsClean => {
                self.git_ops.is_clean()
            }
        }
    }
}

impl<G: GitHubOperations, O: GitOperations> ConditionEvaluator for RealCommandExecutor<G, O> {
    /// Evaluate a condition
    fn evaluate(&self, condition: &Condition) -> Result<bool> {
        self.evaluate_condition(condition)
    }
}