# Testable Agent State Machine Architecture

## Design Principles

1. **Separation of Concerns**: Detection, decision, command generation, and execution are separate layers
2. **Pure Functions**: All logic functions are pure and testable without side effects
3. **Dependency Injection**: All external operations (Git, GitHub, filesystem) are injected
4. **Command Pattern**: Operations are expressed as commands that can be collected, validated, or executed
5. **Composable Recovery**: Each failure mode has a specific recovery strategy

## Architecture Layers

```rust
// Layer 1: State Detection (Pure Functions)
pub struct AgentStateDetector;

impl AgentStateDetector {
    /// Detect current agent state from Git and GitHub
    pub fn detect_current_state(
        git: &dyn GitOperations,
        github: &dyn GitHubOperations,
        agent_id: &str
    ) -> Result<AgentState> {
        // Pure function - only reads, never modifies
    }
    
    /// Detect all pre-flight issues that need resolution
    pub fn detect_pre_flight_issues(
        git: &dyn GitOperations,
        github: &dyn GitHubOperations,
        agent_id: &str
    ) -> Vec<PreFlightIssue> {
        // Pure function - returns list of detected issues
    }
}

// Layer 2: Decision Making (Pure Functions)
pub struct AgentStateMachine;

impl AgentStateMachine {
    /// Plan the sequence of commands needed for a state transition
    pub fn plan_transition(
        &self,
        from: AgentState,
        to: AgentState,
        context: &TransitionContext
    ) -> TransitionPlan {
        // Pure function - returns commands needed, no execution
    }
    
    /// Plan recovery actions for detected issues
    pub fn plan_recovery(&self, issues: Vec<PreFlightIssue>) -> RecoveryPlan {
        // Pure function - returns recovery commands
    }
    
    /// Validate that a transition is legal
    pub fn validate_transition(&self, from: AgentState, to: AgentState) -> Result<()> {
        // Pure function - validation only
    }
}

// Layer 3: Command Generation (Pure Functions)
#[derive(Debug, Clone, PartialEq)]
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
    Conditional { condition: Condition, then_cmd: Box<Command>, else_cmd: Option<Box<Command>> }
}

#[derive(Debug, Clone, PartialEq)]
pub enum GitCommand {
    GetCurrentBranch,
    GetCommitsAhead { base: String },
    CheckoutBranch { branch: String },
    Push { remote: String, branch: String },
    CreateBranch { name: String, from: String },
    DeleteBranch { name: String },
    Commit { message: String },
    Add { files: Vec<String> },
}

#[derive(Debug, Clone, PartialEq)]  
pub enum GitHubCommand {
    AddLabel { issue: u64, label: String },
    RemoveLabel { issue: u64, label: String },
    GetIssue { issue: u64 },
    CreatePR { title: String, body: String, head: String, base: String },
    MergePR { number: u64 },
    ClosePR { number: u64 },
}

// Layer 4: Command Execution (Side Effects)
pub trait CommandExecutor {
    fn execute(&self, command: &Command) -> Result<CommandResult>;
}

pub struct RealCommandExecutor {
    git: Box<dyn GitOperations>,
    github: Box<dyn GitHubOperations>,
}

pub struct MockCommandExecutor {
    pub executed_commands: RefCell<Vec<Command>>,
    pub responses: HashMap<Command, CommandResult>,
}

// Supporting Types
#[derive(Debug, Clone, PartialEq)]
pub enum AgentState {
    Idle,
    Assigned { agent_id: String, issue: u64, branch: String },
    Working { agent_id: String, issue: u64, branch: String, commits_ahead: u32 },
    Landed { issue: u64 }, // Agent already freed
    Bundled { issues: Vec<u64>, bundle_pr: u64 },
    Merged { issues: Vec<u64> },
}

#[derive(Debug, Clone)]
pub enum PreFlightIssue {
    NoCommits,
    UnpushedCommits { count: u32 },
    BehindMain { commits: u32 },
    MergeConflicts { files: Vec<String> },
    BranchMissing { branch: String },
    LabelMismatch { expected: Vec<String>, actual: Vec<String> },
}

#[derive(Debug)]
pub struct TransitionPlan {
    pub commands: Vec<Command>,
    pub can_auto_execute: bool,
    pub requires_user_input: bool,
    pub risk_level: RiskLevel,
}

#[derive(Debug)]
pub enum RiskLevel {
    Safe,        // Auto-execute without prompting
    Low,         // Auto-execute with notification
    Medium,      // Prompt for confirmation
    High,        // Require explicit approval
    Critical,    // Block and require manual intervention
}
```

## Implementation Strategy

### Phase 1: Core Infrastructure

```rust
// Trait definitions for external operations
pub trait GitOperations {
    fn get_current_branch(&self) -> Result<String>;
    fn get_commits_ahead(&self, base: &str) -> Result<Vec<String>>;
    fn checkout_branch(&self, branch: &str) -> Result<()>;
    fn push(&self, remote: &str, branch: &str) -> Result<()>;
    fn create_branch(&self, name: &str, from: &str) -> Result<()>;
    fn delete_branch(&self, name: &str) -> Result<()>;
    fn commit(&self, message: &str) -> Result<()>;
    fn add_files(&self, files: &[String]) -> Result<()>;
    fn get_merge_conflicts(&self, base: &str) -> Result<Vec<String>>;
}

pub trait GitHubOperations {
    fn add_label(&self, issue: u64, label: &str) -> Result<()>;
    fn remove_label(&self, issue: u64, label: &str) -> Result<()>;
    fn get_issue(&self, issue: u64) -> Result<Issue>;
    fn create_pr(&self, title: &str, body: &str, head: &str, base: &str) -> Result<String>;
    fn merge_pr(&self, number: u64) -> Result<()>;
    fn get_labels(&self, issue: u64) -> Result<Vec<String>>;
}

// Real implementations that call actual Git/GitHub
pub struct RealGitOperations;
pub struct RealGitHubOperations { client: GitHubClient };

// Mock implementations for testing  
pub struct MockGitOperations { 
    pub responses: HashMap<String, String> 
};
pub struct MockGitHubOperations { 
    pub labels: HashMap<u64, Vec<String>>,
    pub issues: HashMap<u64, Issue>,
};
```

### Phase 2: State Detection Logic

```rust
impl AgentStateDetector {
    pub fn detect_current_state(
        git: &dyn GitOperations,
        github: &dyn GitHubOperations,
        agent_id: &str
    ) -> Result<AgentState> {
        // 1. Check current Git branch
        let current_branch = git.get_current_branch()?;
        
        // 2. Parse agent branch pattern (agent001/123)
        if let Some((branch_agent, issue_num)) = parse_agent_branch(&current_branch) {
            if branch_agent != agent_id {
                return Ok(AgentState::Idle); // Wrong agent
            }
            
            // 3. Check if issue has agent label
            let labels = github.get_labels(issue_num)?;
            let has_agent_label = labels.contains(&agent_id.to_string());
            let has_review_label = labels.contains(&"route:review".to_string());
            
            // 4. Check commits ahead of main
            let commits = git.get_commits_ahead("main")?;
            
            // 5. Determine state based on combination
            match (has_agent_label, has_review_label, commits.is_empty()) {
                (true, false, true) => Ok(AgentState::Assigned { 
                    agent_id: agent_id.to_string(), 
                    issue: issue_num, 
                    branch: current_branch 
                }),
                (true, false, false) => Ok(AgentState::Working { 
                    agent_id: agent_id.to_string(), 
                    issue: issue_num, 
                    branch: current_branch,
                    commits_ahead: commits.len() as u32
                }),
                (false, true, false) => Ok(AgentState::Landed { issue: issue_num }),
                _ => Ok(AgentState::Idle), // Inconsistent state, treat as idle
            }
        } else {
            Ok(AgentState::Idle)
        }
    }
    
    pub fn detect_pre_flight_issues(
        git: &dyn GitOperations,
        github: &dyn GitHubOperations,
        agent_id: &str
    ) -> Vec<PreFlightIssue> {
        let mut issues = Vec::new();
        
        // Check for commits
        if let Ok(commits) = git.get_commits_ahead("main") {
            if commits.is_empty() {
                issues.push(PreFlightIssue::NoCommits);
            }
        }
        
        // Check for unpushed commits
        if let Ok(unpushed) = git.get_commits_ahead(&format!("origin/{}", git.get_current_branch().unwrap_or_default())) {
            if !unpushed.is_empty() {
                issues.push(PreFlightIssue::UnpushedCommits { count: unpushed.len() as u32 });
            }
        }
        
        // Check for merge conflicts
        if let Ok(conflicts) = git.get_merge_conflicts("main") {
            if !conflicts.is_empty() {
                issues.push(PreFlightIssue::MergeConflicts { files: conflicts });
            }
        }
        
        issues
    }
}
```

### Phase 3: Decision Logic

```rust
impl AgentStateMachine {
    pub fn plan_transition(
        &self,
        from: AgentState,
        to: AgentState,
        context: &TransitionContext
    ) -> TransitionPlan {
        match (from, to) {
            (AgentState::Working { issue, agent_id, .. }, AgentState::Landed { .. }) => {
                let mut commands = Vec::new();
                
                // 1. Add route:review label
                commands.push(Command::GitHub(GitHubCommand::AddLabel { 
                    issue, 
                    label: "route:review".to_string() 
                }));
                
                // 2. Remove agent label (frees agent)
                commands.push(Command::GitHub(GitHubCommand::RemoveLabel { 
                    issue, 
                    label: agent_id 
                }));
                
                // 3. Remove route:ready label
                commands.push(Command::GitHub(GitHubCommand::RemoveLabel { 
                    issue, 
                    label: "route:ready".to_string() 
                }));
                
                // 4. Switch back to main
                commands.push(Command::Git(GitCommand::CheckoutBranch { 
                    branch: "main".to_string() 
                }));
                
                // 5. Success message
                commands.push(Command::Print(format!(
                    "✅ Agent {} freed - ready for new assignment", agent_id
                )));
                
                TransitionPlan {
                    commands,
                    can_auto_execute: true,
                    requires_user_input: false,
                    risk_level: RiskLevel::Safe,
                }
            }
            _ => TransitionPlan {
                commands: vec![Command::Error("Invalid transition".to_string())],
                can_auto_execute: false,
                requires_user_input: true,
                risk_level: RiskLevel::Critical,
            }
        }
    }
    
    pub fn plan_recovery(&self, issues: Vec<PreFlightIssue>) -> RecoveryPlan {
        let mut commands = Vec::new();
        let mut risk_level = RiskLevel::Safe;
        
        for issue in issues {
            match issue {
                PreFlightIssue::NoCommits => {
                    commands.push(Command::Warning(
                        "⚠️  No commits detected. Did you forget to commit your work?".to_string()
                    ));
                    risk_level = RiskLevel::High;
                }
                PreFlightIssue::UnpushedCommits { count } => {
                    commands.push(Command::Print(format!(
                        "🔄 Auto-pushing {} unpushed commits...", count
                    )));
                    commands.push(Command::Git(GitCommand::Push { 
                        remote: "origin".to_string(), 
                        branch: "HEAD".to_string() 
                    }));
                    risk_level = risk_level.max(RiskLevel::Low);
                }
                PreFlightIssue::MergeConflicts { files } => {
                    commands.push(Command::Error(format!(
                        "❌ Merge conflicts detected in: {}\nPlease resolve conflicts before landing.",
                        files.join(", ")
                    )));
                    risk_level = RiskLevel::Critical;
                }
                _ => {}
            }
        }
        
        RecoveryPlan { commands, risk_level }
    }
}
```

## Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_detect_working_state() {
        let mut git = MockGitOperations::new();
        git.set_current_branch("agent001/123");
        git.set_commits_ahead("main", vec!["abc123: Fix bug".to_string()]);
        
        let mut github = MockGitHubOperations::new();
        github.add_label(123, "agent001");
        
        let state = AgentStateDetector::detect_current_state(&git, &github, "agent001").unwrap();
        
        assert_eq!(state, AgentState::Working {
            agent_id: "agent001".to_string(),
            issue: 123,
            branch: "agent001/123".to_string(),
            commits_ahead: 1,
        });
    }
    
    #[test]
    fn test_land_transition_plan() {
        let state_machine = AgentStateMachine;
        
        let from = AgentState::Working {
            agent_id: "agent001".to_string(),
            issue: 123,
            branch: "agent001/123".to_string(),
            commits_ahead: 2,
        };
        
        let to = AgentState::Landed { issue: 123 };
        
        let plan = state_machine.plan_transition(from, to, &TransitionContext::default());
        
        assert_eq!(plan.commands.len(), 5);
        assert!(plan.can_auto_execute);
        assert_eq!(plan.risk_level, RiskLevel::Safe);
        
        // Verify specific commands
        assert!(matches!(plan.commands[0], Command::GitHub(GitHubCommand::AddLabel { 
            issue: 123, 
            label: ref l 
        }) if l == "route:review"));
        
        assert!(matches!(plan.commands[1], Command::GitHub(GitHubCommand::RemoveLabel { 
            issue: 123, 
            label: ref l 
        }) if l == "agent001"));
    }
    
    #[test]
    fn test_pre_flight_recovery() {
        let state_machine = AgentStateMachine;
        
        let issues = vec![
            PreFlightIssue::UnpushedCommits { count: 2 },
            PreFlightIssue::MergeConflicts { files: vec!["src/main.rs".to_string()] }
        ];
        
        let recovery = state_machine.plan_recovery(issues);
        
        assert_eq!(recovery.risk_level, RiskLevel::Critical);
        assert!(recovery.commands.iter().any(|cmd| matches!(cmd, Command::Error(_))));
    }
    
    #[test]  
    fn test_full_workflow_integration() {
        let mut executor = MockCommandExecutor::new();
        
        // Set up initial state
        let mut git = MockGitOperations::new();
        git.set_current_branch("agent001/123");
        git.set_commits_ahead("main", vec!["abc123: Implement feature".to_string()]);
        
        let mut github = MockGitHubOperations::new(); 
        github.add_label(123, "agent001");
        github.add_label(123, "route:ready");
        
        // Detect current state
        let current_state = AgentStateDetector::detect_current_state(&git, &github, "agent001").unwrap();
        
        // Plan transition to landed
        let target_state = AgentState::Landed { issue: 123 };
        let state_machine = AgentStateMachine;
        let plan = state_machine.plan_transition(current_state, target_state, &TransitionContext::default());
        
        // Execute plan
        for command in &plan.commands {
            executor.execute(command).unwrap();
        }
        
        // Verify all expected commands were executed
        let executed = executor.get_executed_commands();
        assert_eq!(executed.len(), 5);
        
        // Verify agent was freed (agent001 label removed)
        assert!(executed.iter().any(|cmd| matches!(cmd, 
            Command::GitHub(GitHubCommand::RemoveLabel { issue: 123, label: ref l }) 
            if l == "agent001"
        )));
    }
}
```

This architecture provides:
1. **Complete testability** - No side effects in logic
2. **Clear separation** - Detection, decision, execution are separate
3. **Comprehensive coverage** - Every failure mode can be tested
4. **Flexible execution** - Can emit commands or execute them
5. **Recovery strategies** - Explicit handling for each failure mode

The current bug would be caught immediately by tests since the logic would be explicit and the commands would be verified.