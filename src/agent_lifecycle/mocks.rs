// Mock implementations for testing - no side effects

use anyhow::{anyhow, Result};
use std::cell::RefCell;
use std::collections::HashMap;

use crate::agent_lifecycle::traits::*;
use crate::agent_lifecycle::types::*;

/// Mock Git operations that store expected responses
#[derive(Debug)]
pub struct MockGitOperations {
    pub current_branch: RefCell<String>,
    pub branches: RefCell<HashMap<String, bool>>,
    pub commits_ahead: RefCell<HashMap<String, Vec<String>>>,
    pub commits_behind: RefCell<HashMap<String, u32>>,
    pub merge_conflicts: RefCell<HashMap<String, Vec<String>>>,
    pub is_clean: RefCell<bool>,
    pub status: RefCell<String>,
    pub executed_commands: RefCell<Vec<GitCommand>>,
}

impl Default for MockGitOperations {
    fn default() -> Self {
        Self::new()
    }
}

impl MockGitOperations {
    pub fn new() -> Self {
        Self {
            current_branch: RefCell::new("main".to_string()),
            branches: RefCell::new(HashMap::new()),
            commits_ahead: RefCell::new(HashMap::new()),
            commits_behind: RefCell::new(HashMap::new()),
            merge_conflicts: RefCell::new(HashMap::new()),
            is_clean: RefCell::new(true),
            status: RefCell::new(
                "On branch main\nnothing to commit, working tree clean".to_string(),
            ),
            executed_commands: RefCell::new(Vec::new()),
        }
    }

    pub fn set_current_branch(&self, branch: &str) {
        *self.current_branch.borrow_mut() = branch.to_string();
    }

    pub fn set_commits_ahead(&self, base: &str, commits: Vec<String>) {
        self.commits_ahead
            .borrow_mut()
            .insert(base.to_string(), commits);
    }

    pub fn set_commits_behind(&self, base: &str, count: u32) {
        self.commits_behind
            .borrow_mut()
            .insert(base.to_string(), count);
    }

    pub fn set_branch_exists(&self, branch: &str, exists: bool) {
        self.branches
            .borrow_mut()
            .insert(branch.to_string(), exists);
    }

    pub fn set_merge_conflicts(&self, base: &str, files: Vec<String>) {
        self.merge_conflicts
            .borrow_mut()
            .insert(base.to_string(), files);
    }

    pub fn set_clean(&self, clean: bool) {
        *self.is_clean.borrow_mut() = clean;
    }

    pub fn get_executed_commands(&self) -> Vec<GitCommand> {
        self.executed_commands.borrow().clone()
    }

    pub fn clear_executed_commands(&self) {
        self.executed_commands.borrow_mut().clear();
    }
}

impl GitOperations for MockGitOperations {
    fn get_current_branch(&self) -> Result<String> {
        self.executed_commands
            .borrow_mut()
            .push(GitCommand::GetCurrentBranch);
        Ok(self.current_branch.borrow().clone())
    }

    fn get_commits_ahead(&self, base: &str) -> Result<Vec<String>> {
        self.executed_commands
            .borrow_mut()
            .push(GitCommand::GetCommitsAhead {
                base: base.to_string(),
            });
        Ok(self
            .commits_ahead
            .borrow()
            .get(base)
            .cloned()
            .unwrap_or_default())
    }

    fn get_commits_behind(&self, base: &str) -> Result<u32> {
        self.executed_commands
            .borrow_mut()
            .push(GitCommand::GetCommitsBehind {
                base: base.to_string(),
            });
        Ok(self.commits_behind.borrow().get(base).cloned().unwrap_or(0))
    }

    fn checkout_branch(&self, branch: &str) -> Result<()> {
        self.executed_commands
            .borrow_mut()
            .push(GitCommand::CheckoutBranch {
                branch: branch.to_string(),
            });
        *self.current_branch.borrow_mut() = branch.to_string();
        Ok(())
    }

    fn push(&self, remote: &str, branch: &str) -> Result<()> {
        self.executed_commands.borrow_mut().push(GitCommand::Push {
            remote: remote.to_string(),
            branch: branch.to_string(),
        });
        Ok(())
    }

    fn create_branch(&self, name: &str, from: &str) -> Result<()> {
        self.executed_commands
            .borrow_mut()
            .push(GitCommand::CreateBranch {
                name: name.to_string(),
                from: from.to_string(),
            });
        self.branches.borrow_mut().insert(name.to_string(), true);
        Ok(())
    }

    fn delete_branch(&self, name: &str) -> Result<()> {
        self.executed_commands
            .borrow_mut()
            .push(GitCommand::DeleteBranch {
                name: name.to_string(),
            });
        self.branches.borrow_mut().remove(name);
        Ok(())
    }

    fn commit(&self, message: &str) -> Result<()> {
        self.executed_commands
            .borrow_mut()
            .push(GitCommand::Commit {
                message: message.to_string(),
            });
        Ok(())
    }

    fn add_files(&self, files: &[String]) -> Result<()> {
        self.executed_commands.borrow_mut().push(GitCommand::Add {
            files: files.to_vec(),
        });
        Ok(())
    }

    fn get_merge_conflicts(&self, base: &str) -> Result<Vec<String>> {
        self.executed_commands
            .borrow_mut()
            .push(GitCommand::GetMergeConflicts {
                base: base.to_string(),
            });
        Ok(self
            .merge_conflicts
            .borrow()
            .get(base)
            .cloned()
            .unwrap_or_default())
    }

    fn is_clean(&self) -> Result<bool> {
        self.executed_commands
            .borrow_mut()
            .push(GitCommand::IsClean);
        Ok(*self.is_clean.borrow())
    }

    fn get_status(&self) -> Result<String> {
        self.executed_commands
            .borrow_mut()
            .push(GitCommand::GetStatus);
        Ok(self.status.borrow().clone())
    }

    fn branch_exists(&self, branch: &str) -> Result<bool> {
        Ok(self.branches.borrow().get(branch).cloned().unwrap_or(false))
    }

    fn remote_branch_exists(&self, _remote: &str, branch: &str) -> Result<bool> {
        Ok(self
            .branches
            .borrow()
            .get(&format!("origin/{branch}"))
            .cloned()
            .unwrap_or(false))
    }
}

/// Mock GitHub operations that store expected responses
#[derive(Debug)]
pub struct MockGitHubOperations {
    pub issues: RefCell<HashMap<u64, IssueData>>,
    pub labels: RefCell<HashMap<u64, Vec<String>>>,
    pub prs: RefCell<HashMap<u64, PRData>>,
    pub executed_commands: RefCell<Vec<GitHubCommand>>,
    pub owner: String,
    pub repo: String,
}

impl MockGitHubOperations {
    pub fn new(owner: &str, repo: &str) -> Self {
        Self {
            issues: RefCell::new(HashMap::new()),
            labels: RefCell::new(HashMap::new()),
            prs: RefCell::new(HashMap::new()),
            executed_commands: RefCell::new(Vec::new()),
            owner: owner.to_string(),
            repo: repo.to_string(),
        }
    }

    pub fn add_issue(&self, issue: IssueData) {
        let number = issue.number;
        let labels = issue.labels.clone();
        self.issues.borrow_mut().insert(number, issue);
        self.labels.borrow_mut().insert(number, labels);
    }

    pub fn add_label(&self, issue: u64, label: &str) {
        self.labels
            .borrow_mut()
            .entry(issue)
            .or_default()
            .push(label.to_string());
    }

    pub fn remove_label(&self, issue: u64, label: &str) {
        if let Some(labels) = self.labels.borrow_mut().get_mut(&issue) {
            labels.retain(|l| l != label);
        }
    }

    pub fn get_executed_commands(&self) -> Vec<GitHubCommand> {
        self.executed_commands.borrow().clone()
    }

    pub fn clear_executed_commands(&self) {
        self.executed_commands.borrow_mut().clear();
    }

    pub fn has_label(&self, issue: u64, label: &str) -> bool {
        self.labels
            .borrow()
            .get(&issue)
            .map(|labels| labels.contains(&label.to_string()))
            .unwrap_or(false)
    }
}

impl GitHubOperations for MockGitHubOperations {
    fn add_label(&self, issue: u64, label: &str) -> Result<()> {
        self.executed_commands
            .borrow_mut()
            .push(GitHubCommand::AddLabel {
                issue,
                label: label.to_string(),
            });

        self.labels
            .borrow_mut()
            .entry(issue)
            .or_default()
            .push(label.to_string());
        Ok(())
    }

    fn remove_label(&self, issue: u64, label: &str) -> Result<()> {
        self.executed_commands
            .borrow_mut()
            .push(GitHubCommand::RemoveLabel {
                issue,
                label: label.to_string(),
            });

        if let Some(labels) = self.labels.borrow_mut().get_mut(&issue) {
            labels.retain(|l| l != label);
        }
        Ok(())
    }

    fn get_issue(&self, issue: u64) -> Result<IssueData> {
        self.executed_commands
            .borrow_mut()
            .push(GitHubCommand::GetIssue { issue });

        self.issues
            .borrow()
            .get(&issue)
            .cloned()
            .ok_or_else(|| anyhow!("Issue #{} not found", issue))
    }

    fn get_labels(&self, issue: u64) -> Result<Vec<String>> {
        self.executed_commands
            .borrow_mut()
            .push(GitHubCommand::GetLabels { issue });

        Ok(self
            .labels
            .borrow()
            .get(&issue)
            .cloned()
            .unwrap_or_default())
    }

    fn create_pr(&self, title: &str, body: &str, head: &str, base: &str) -> Result<String> {
        self.executed_commands
            .borrow_mut()
            .push(GitHubCommand::CreatePR {
                title: title.to_string(),
                body: body.to_string(),
                head: head.to_string(),
                base: base.to_string(),
            });

        let pr_number = (self.prs.borrow().len() + 1) as u64;
        let pr_url = format!(
            "https://github.com/{}/{}/pull/{}",
            self.owner, self.repo, pr_number
        );

        let pr = PRData {
            number: pr_number,
            title: title.to_string(),
            state: "open".to_string(),
            head: head.to_string(),
            base: base.to_string(),
            mergeable: Some(true),
        };

        self.prs.borrow_mut().insert(pr_number, pr);
        Ok(pr_url)
    }

    fn merge_pr(&self, number: u64) -> Result<()> {
        self.executed_commands
            .borrow_mut()
            .push(GitHubCommand::MergePR { number });

        if let Some(pr) = self.prs.borrow_mut().get_mut(&number) {
            pr.state = "closed".to_string();
        }
        Ok(())
    }

    fn close_pr(&self, number: u64) -> Result<()> {
        self.executed_commands
            .borrow_mut()
            .push(GitHubCommand::ClosePR { number });

        if let Some(pr) = self.prs.borrow_mut().get_mut(&number) {
            pr.state = "closed".to_string();
        }
        Ok(())
    }

    fn get_pr(&self, number: u64) -> Result<PRData> {
        self.executed_commands
            .borrow_mut()
            .push(GitHubCommand::GetPR { number });

        self.prs
            .borrow()
            .get(&number)
            .cloned()
            .ok_or_else(|| anyhow!("PR #{} not found", number))
    }

    fn issue_has_label(&self, issue: u64, label: &str) -> Result<bool> {
        Ok(self
            .labels
            .borrow()
            .get(&issue)
            .map(|labels| labels.contains(&label.to_string()))
            .unwrap_or(false))
    }

    fn get_issues_with_label(&self, label: &str) -> Result<Vec<u64>> {
        let issues = self
            .labels
            .borrow()
            .iter()
            .filter_map(|(issue, labels)| {
                if labels.contains(&label.to_string()) {
                    Some(*issue)
                } else {
                    None
                }
            })
            .collect();
        Ok(issues)
    }
}

/// Mock command executor that collects commands instead of executing them
#[derive(Debug)]
pub struct MockCommandExecutor {
    pub executed_commands: RefCell<Vec<Command>>,
    pub responses: HashMap<Command, CommandResult>,
    pub git: MockGitOperations,
    pub github: MockGitHubOperations,
}

impl Default for MockCommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl MockCommandExecutor {
    pub fn new() -> Self {
        Self {
            executed_commands: RefCell::new(Vec::new()),
            responses: HashMap::new(),
            git: MockGitOperations::new(),
            github: MockGitHubOperations::new("owner", "repo"),
        }
    }

    pub fn with_git_github(git: MockGitOperations, github: MockGitHubOperations) -> Self {
        Self {
            executed_commands: RefCell::new(Vec::new()),
            responses: HashMap::new(),
            git,
            github,
        }
    }

    pub fn get_executed_commands(&self) -> Vec<Command> {
        self.executed_commands.borrow().clone()
    }

    pub fn clear_executed_commands(&self) {
        self.executed_commands.borrow_mut().clear();
    }

    pub fn set_response(&mut self, command: Command, result: CommandResult) {
        self.responses.insert(command, result);
    }
}

impl CommandExecutor for MockCommandExecutor {
    fn execute(&self, command: &Command) -> Result<CommandResult> {
        self.executed_commands.borrow_mut().push(command.clone());

        // If we have a pre-set response, use it
        if let Some(result) = self.responses.get(command) {
            return Ok(result.clone());
        }

        // Otherwise, simulate the command
        match command {
            Command::Git(git_cmd) => {
                match git_cmd {
                    GitCommand::GetCurrentBranch => {
                        let branch = self.git.get_current_branch()?;
                        Ok(CommandResult {
                            success: true,
                            output: branch.clone(),
                            error: None,
                            data: Some(CommandData::String(branch)),
                        })
                    }
                    GitCommand::CheckoutBranch { branch } => {
                        self.git.checkout_branch(branch)?;
                        Ok(CommandResult {
                            success: true,
                            output: format!("Switched to branch '{branch}'"),
                            error: None,
                            data: None,
                        })
                    }
                    // Add other Git command implementations as needed
                    _ => Ok(CommandResult {
                        success: true,
                        output: format!("Mock executed: {git_cmd:?}"),
                        error: None,
                        data: None,
                    }),
                }
            }
            Command::GitHub(github_cmd) => {
                match github_cmd {
                    GitHubCommand::AddLabel { issue, label } => {
                        self.github.add_label(*issue, label);
                        Ok(CommandResult {
                            success: true,
                            output: format!("Added label '{label}' to issue #{issue}"),
                            error: None,
                            data: None,
                        })
                    }
                    GitHubCommand::RemoveLabel { issue, label } => {
                        self.github.remove_label(*issue, label);
                        Ok(CommandResult {
                            success: true,
                            output: format!("Removed label '{label}' from issue #{issue}"),
                            error: None,
                            data: None,
                        })
                    }
                    // Add other GitHub command implementations as needed
                    _ => Ok(CommandResult {
                        success: true,
                        output: format!("Mock executed: {github_cmd:?}"),
                        error: None,
                        data: None,
                    }),
                }
            }
            Command::Print(msg) => Ok(CommandResult {
                success: true,
                output: msg.clone(),
                error: None,
                data: None,
            }),
            Command::Warning(msg) => Ok(CommandResult {
                success: true,
                output: format!("Warning: {msg}"),
                error: None,
                data: None,
            }),
            Command::Error(msg) => Ok(CommandResult {
                success: false,
                output: "".to_string(),
                error: Some(msg.clone()),
                data: None,
            }),
            Command::Sequence(commands) => {
                let results = self.execute_sequence(commands)?;
                let all_success = results.iter().all(|r| r.success);
                Ok(CommandResult {
                    success: all_success,
                    output: format!("Executed {} commands", commands.len()),
                    error: None,
                    data: None,
                })
            }
            Command::Conditional {
                condition: _,
                then_cmd,
                else_cmd: _,
            } => {
                // For mock, just execute the then command
                self.execute(then_cmd)
            }
        }
    }

    fn execute_atomic(&self, commands: &[Command]) -> Result<Vec<CommandResult>> {
        // For mock, just execute sequence
        self.execute_sequence(commands)
    }
}
