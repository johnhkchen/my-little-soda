// Core types for the agent lifecycle state machine

/// Agent states in the lifecycle
#[derive(Debug, Clone, PartialEq)]
pub enum AgentState {
    /// Agent has no active assignment
    Idle,
    /// Agent has been assigned to an issue but hasn't committed work
    Assigned {
        agent_id: String,
        issue: u64,
        branch: String,
    },
    /// Agent is actively working (has made commits)
    Working {
        agent_id: String,
        issue: u64,
        branch: String,
        commits_ahead: u32,
    },
    /// Agent has completed work and landed it (freed immediately)
    Landed { issue: u64 },
    /// Work has been included in a bundle PR
    Bundled { issues: Vec<u64>, bundle_pr: u64 },
    /// Work has been integrated into main branch
    Merged { issues: Vec<u64> },
}

/// Issues detected during pre-flight checks
#[derive(Debug, Clone, PartialEq)]
pub enum PreFlightIssue {
    /// No commits found on branch
    NoCommits,
    /// Commits exist but haven't been pushed
    UnpushedCommits { count: u32 },
    /// Branch is behind main
    BehindMain { commits: u32 },
    /// Merge conflicts detected
    MergeConflicts { files: Vec<String> },
    /// Branch is missing
    BranchMissing { branch: String },
    /// GitHub labels don't match expected state
    LabelMismatch {
        expected: Vec<String>,
        actual: Vec<String>,
    },
    /// Issue is in wrong state
    IssueStateMismatch {
        issue: u64,
        expected_labels: Vec<String>,
        actual_labels: Vec<String>,
    },
}

/// Context information for state transitions
#[derive(Debug, Default)]
pub struct TransitionContext {
    pub dry_run: bool,
    pub verbose: bool,
    pub force: bool,
    pub auto_fix: bool,
}

/// Plan for executing a state transition
#[derive(Debug)]
pub struct TransitionPlan {
    pub commands: Vec<Command>,
    pub can_auto_execute: bool,
    pub requires_user_input: bool,
    pub risk_level: RiskLevel,
    pub estimated_duration: Option<std::time::Duration>,
}

/// Plan for recovering from detected issues
#[derive(Debug)]
pub struct RecoveryPlan {
    pub commands: Vec<Command>,
    pub risk_level: RiskLevel,
    pub auto_fixable: bool,
}

/// Risk level for operations
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    /// Auto-execute without prompting
    Safe,
    /// Auto-execute with notification
    Low,
    /// Prompt for confirmation
    Medium,
    /// Require explicit approval
    High,
    /// Block and require manual intervention
    Critical,
}

/// Commands that can be executed
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Command {
    // Git operations
    Git(GitCommand),
    // GitHub operations
    GitHub(GitHubCommand),
    // User communication
    Print(String),
    Warning(String),
    Error(String),
    // Compound operations
    Sequence(Vec<Command>),
    Conditional {
        condition: Condition,
        then_cmd: Box<Command>,
        else_cmd: Option<Box<Command>>,
    },
}

/// Git operations
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GitCommand {
    GetCurrentBranch,
    GetCommitsAhead { base: String },
    GetCommitsBehind { base: String },
    CheckoutBranch { branch: String },
    Push { remote: String, branch: String },
    CreateBranch { name: String, from: String },
    DeleteBranch { name: String },
    Commit { message: String },
    Add { files: Vec<String> },
    GetMergeConflicts { base: String },
    IsClean,
    GetStatus,
}

/// GitHub operations
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GitHubCommand {
    AddLabel {
        issue: u64,
        label: String,
    },
    RemoveLabel {
        issue: u64,
        label: String,
    },
    GetIssue {
        issue: u64,
    },
    GetLabels {
        issue: u64,
    },
    CreatePR {
        title: String,
        body: String,
        head: String,
        base: String,
    },
    MergePR {
        number: u64,
    },
    ClosePR {
        number: u64,
    },
    GetPR {
        number: u64,
    },
}

/// Conditions for conditional commands
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Condition {
    /// Always true
    Always,
    /// Always false
    Never,
    /// Check if branch exists
    BranchExists { branch: String },
    /// Check if issue has label
    IssueHasLabel { issue: u64, label: String },
    /// Check if commits exist
    HasCommits { base: String },
    /// Check if working directory is clean
    IsClean,
}

/// Results from command execution
#[derive(Debug, Clone)]
pub struct CommandResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub data: Option<CommandData>,
}

/// Structured data returned from commands
#[derive(Debug, Clone)]
pub enum CommandData {
    String(String),
    Number(u64),
    StringList(Vec<String>),
    Issue(IssueData),
    PR(PRData),
}

/// Issue data structure
#[derive(Debug, Clone)]
pub struct IssueData {
    pub number: u64,
    pub title: String,
    pub labels: Vec<String>,
    pub state: String,
    pub assignee: Option<String>,
}

/// PR data structure
#[derive(Debug, Clone)]
pub struct PRData {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub head: String,
    pub base: String,
    pub mergeable: Option<bool>,
}

/// Parse agent branch pattern (agent001/123 or agent001/123-description)
pub fn parse_agent_branch(branch: &str) -> Option<(String, u64)> {
    let parts: Vec<&str> = branch.split('/').collect();
    if parts.len() == 2 {
        let agent_id = parts[0];
        let issue_part = parts[1];

        // Handle both formats: "123" and "123-description"
        let issue_number = if let Some(dash_pos) = issue_part.find('-') {
            // New format: "123-description" -> extract "123"
            issue_part[..dash_pos].parse::<u64>()
        } else {
            // Legacy format: "123" -> parse directly
            issue_part.parse::<u64>()
        };

        if let Ok(issue_num) = issue_number {
            return Some((agent_id.to_string(), issue_num));
        }
    }
    None
}

/// Extract agent ID from branch name
pub fn extract_agent_from_branch(branch: &str) -> String {
    parse_agent_branch(branch)
        .map(|(agent, _)| agent)
        .unwrap_or_else(|| "unknown".to_string())
}

/// Validation for agent states
impl AgentState {
    pub fn agent_id(&self) -> Option<&str> {
        match self {
            AgentState::Assigned { agent_id, .. } => Some(agent_id),
            AgentState::Working { agent_id, .. } => Some(agent_id),
            _ => None,
        }
    }

    pub fn issue_number(&self) -> Option<u64> {
        match self {
            AgentState::Assigned { issue, .. } => Some(*issue),
            AgentState::Working { issue, .. } => Some(*issue),
            AgentState::Landed { issue } => Some(*issue),
            _ => None,
        }
    }

    pub fn branch_name(&self) -> Option<&str> {
        match self {
            AgentState::Assigned { branch, .. } => Some(branch),
            AgentState::Working { branch, .. } => Some(branch),
            _ => None,
        }
    }

    pub fn is_busy(&self) -> bool {
        matches!(
            self,
            AgentState::Assigned { .. } | AgentState::Working { .. }
        )
    }

    pub fn is_available(&self) -> bool {
        matches!(self, AgentState::Idle)
    }
}
