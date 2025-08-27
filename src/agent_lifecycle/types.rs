// Core types for the agent lifecycle state machine

/// Agent states in the lifecycle
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
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
}

/// Issues detected during pre-flight checks
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum PreFlightIssue {
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
}













/// Parse agent branch pattern (agent001/123 or agent001/123-description)
#[allow(dead_code)]
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


/// Validation for agent states
impl AgentState {
    #[allow(dead_code)]
    pub fn agent_id(&self) -> Option<&str> {
        match self {
            AgentState::Assigned { agent_id, .. } => Some(agent_id),
            AgentState::Working { agent_id, .. } => Some(agent_id),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn issue_number(&self) -> Option<u64> {
        match self {
            AgentState::Assigned { issue, .. } => Some(*issue),
            AgentState::Working { issue, .. } => Some(*issue),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn branch_name(&self) -> Option<&str> {
        match self {
            AgentState::Assigned { branch, .. } => Some(branch),
            AgentState::Working { branch, .. } => Some(branch),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn is_busy(&self) -> bool {
        matches!(
            self,
            AgentState::Assigned { .. } | AgentState::Working { .. }
        )
    }

    #[allow(dead_code)]
    pub fn is_available(&self) -> bool {
        matches!(self, AgentState::Idle)
    }
}
