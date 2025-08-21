// Traits for dependency injection - separating concerns for testability

use crate::agent_lifecycle::types::*;
use anyhow::Result;

/// Git operations interface
pub trait GitOperations {
    /// Get current branch name
    fn get_current_branch(&self) -> Result<String>;
    
    /// Get commits ahead of base branch
    fn get_commits_ahead(&self, base: &str) -> Result<Vec<String>>;
    
    /// Get commits behind base branch
    fn get_commits_behind(&self, base: &str) -> Result<u32>;
    
    /// Switch to a branch
    fn checkout_branch(&self, branch: &str) -> Result<()>;
    
    /// Push branch to remote
    fn push(&self, remote: &str, branch: &str) -> Result<()>;
    
    /// Create new branch from base
    fn create_branch(&self, name: &str, from: &str) -> Result<()>;
    
    /// Delete branch
    fn delete_branch(&self, name: &str) -> Result<()>;
    
    /// Commit with message
    fn commit(&self, message: &str) -> Result<()>;
    
    /// Add files to staging
    fn add_files(&self, files: &[String]) -> Result<()>;
    
    /// Check for merge conflicts with base
    fn get_merge_conflicts(&self, base: &str) -> Result<Vec<String>>;
    
    /// Check if working directory is clean
    fn is_clean(&self) -> Result<bool>;
    
    /// Get git status
    fn get_status(&self) -> Result<String>;
    
    /// Check if branch exists locally
    fn branch_exists(&self, branch: &str) -> Result<bool>;
    
    /// Check if branch exists on remote
    fn remote_branch_exists(&self, remote: &str, branch: &str) -> Result<bool>;
}

/// GitHub operations interface  
pub trait GitHubOperations {
    /// Add label to issue
    fn add_label(&self, issue: u64, label: &str) -> Result<()>;
    
    /// Remove label from issue
    fn remove_label(&self, issue: u64, label: &str) -> Result<()>;
    
    /// Get issue data
    fn get_issue(&self, issue: u64) -> Result<IssueData>;
    
    /// Get labels for issue
    fn get_labels(&self, issue: u64) -> Result<Vec<String>>;
    
    /// Create pull request
    fn create_pr(&self, title: &str, body: &str, head: &str, base: &str) -> Result<String>;
    
    /// Merge pull request
    fn merge_pr(&self, number: u64) -> Result<()>;
    
    /// Close pull request
    fn close_pr(&self, number: u64) -> Result<()>;
    
    /// Get pull request data
    fn get_pr(&self, number: u64) -> Result<PRData>;
    
    /// Check if issue has specific label
    fn issue_has_label(&self, issue: u64, label: &str) -> Result<bool>;
    
    /// Get all issues with specific label
    fn get_issues_with_label(&self, label: &str) -> Result<Vec<u64>>;
}

/// Command execution interface
pub trait CommandExecutor {
    /// Execute a single command
    fn execute(&self, command: &Command) -> Result<CommandResult>;
    
    /// Execute multiple commands in sequence
    fn execute_sequence(&self, commands: &[Command]) -> Result<Vec<CommandResult>> {
        let mut results = Vec::new();
        for command in commands {
            let result = self.execute(command)?;
            let success = result.success;
            results.push(result);
            if !success {
                break; // Stop on first failure
            }
        }
        Ok(results)
    }
    
    /// Execute commands with rollback on failure
    fn execute_atomic(&self, commands: &[Command]) -> Result<Vec<CommandResult>>;
}

/// State detection interface
pub trait StateDetector {
    /// Detect current agent state
    fn detect_current_state(&self, agent_id: &str) -> Result<AgentState>;
    
    /// Detect pre-flight issues
    fn detect_pre_flight_issues(&self, agent_id: &str) -> Result<Vec<PreFlightIssue>>;
    
    /// Validate state consistency
    fn validate_state(&self, expected: &AgentState, actual: &AgentState) -> Result<bool>;
}

/// State machine interface
pub trait StateMachine {
    /// Plan transition from one state to another
    fn plan_transition(
        &self,
        from: AgentState,
        to: AgentState,
        context: &TransitionContext,
    ) -> Result<TransitionPlan>;
    
    /// Plan recovery from detected issues
    fn plan_recovery(&self, issues: Vec<PreFlightIssue>) -> Result<RecoveryPlan>;
    
    /// Validate that a transition is legal
    fn validate_transition(&self, from: &AgentState, to: &AgentState) -> Result<()>;
    
    /// Get all possible transitions from a state
    fn get_possible_transitions(&self, from: &AgentState) -> Vec<AgentState>;
}

/// Combined operations interface for convenience
pub trait AgentLifecycleOperations: GitOperations + GitHubOperations {
    /// Get repository owner
    fn owner(&self) -> &str;
    
    /// Get repository name
    fn repo(&self) -> &str;
    
    /// Get agent branch name for issue
    fn agent_branch_name(&self, agent_id: &str, issue: u64) -> String {
        format!("{}/{}", agent_id, issue)
    }
}

// Helper trait for condition evaluation
pub trait ConditionEvaluator {
    /// Evaluate a condition
    fn evaluate(&self, condition: &Condition) -> Result<bool>;
}