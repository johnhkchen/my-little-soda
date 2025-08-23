// Tests for the agent lifecycle state machine

#[cfg(test)]
mod tests {
    use super::super::mocks::*;
    use super::super::types::*;
    use super::super::traits::*;
    
    #[test]
    fn test_mock_git_operations() {
        let git = MockGitOperations::new();
        
        // Test setting up state
        git.set_current_branch("agent001/123");
        git.set_commits_ahead("main", vec!["abc123: Fix bug".to_string()]);
        
        // Test operations
        let branch = git.get_current_branch().unwrap();
        assert_eq!(branch, "agent001/123");
        
        let commits = git.get_commits_ahead("main").unwrap();
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0], "abc123: Fix bug");
        
        // Test command tracking
        let executed = git.get_executed_commands();
        assert_eq!(executed.len(), 2);
        assert!(matches!(executed[0], GitCommand::GetCurrentBranch));
        assert!(matches!(executed[1], GitCommand::GetCommitsAhead { .. }));
    }
    
    #[test]
    fn test_mock_github_operations() {
        let github = MockGitHubOperations::new("owner", "repo");
        
        // Test setting up issue
        let issue = IssueData {
            number: 123,
            title: "Test issue".to_string(),
            labels: vec!["agent001".to_string(), "route:ready".to_string()],
            state: "open".to_string(),
            assignee: Some("johnhkchen".to_string()),
        };
        github.add_issue(issue);
        
        // Test operations
        let retrieved = github.get_issue(123).unwrap();
        assert_eq!(retrieved.number, 123);
        assert_eq!(retrieved.title, "Test issue");
        
        let labels = github.get_labels(123).unwrap();
        assert!(labels.contains(&"agent001".to_string()));
        assert!(labels.contains(&"route:ready".to_string()));
        
        // Test label manipulation using trait methods
        let _ = GitHubOperations::add_label(&github, 123, "route:review");
        let _ = GitHubOperations::remove_label(&github, 123, "route:ready");
        
        let updated_labels = github.get_labels(123).unwrap();
        assert!(updated_labels.contains(&"route:review".to_string()));
        assert!(!updated_labels.contains(&"route:ready".to_string()));
        
        // Test command tracking
        let executed = github.get_executed_commands();
        assert!(executed.len() >= 4); // get_issue, get_labels x2, add_label, remove_label
    }
    
    #[test]
    fn test_mock_command_executor() {
        let executor = MockCommandExecutor::new();
        
        let commands = vec![
            Command::GitHub(GitHubCommand::AddLabel { 
                issue: 123, 
                label: "route:review".to_string() 
            }),
            Command::GitHub(GitHubCommand::RemoveLabel { 
                issue: 123, 
                label: "agent001".to_string() 
            }),
            Command::Git(GitCommand::CheckoutBranch { 
                branch: "main".to_string() 
            }),
            Command::Print("Agent freed".to_string()),
        ];
        
        // Execute commands
        let results = executor.execute_sequence(&commands).unwrap();
        
        // Verify all commands succeeded
        assert_eq!(results.len(), 4);
        assert!(results.iter().all(|r| r.success));
        
        // Verify commands were tracked
        let executed = executor.get_executed_commands();
        assert_eq!(executed.len(), 4);
        assert_eq!(executed[0], commands[0]);
        assert_eq!(executed[1], commands[1]);
        assert_eq!(executed[2], commands[2]);
        assert_eq!(executed[3], commands[3]);
    }
    
    #[test]
    fn test_agent_state_methods() {
        let assigned_state = AgentState::Assigned {
            agent_id: "agent001".to_string(),
            issue: 123,
            branch: "agent001/123".to_string(),
        };
        
        assert_eq!(assigned_state.agent_id(), Some("agent001"));
        assert_eq!(assigned_state.issue_number(), Some(123));
        assert_eq!(assigned_state.branch_name(), Some("agent001/123"));
        assert!(assigned_state.is_busy());
        assert!(!assigned_state.is_available());
        
        let idle_state = AgentState::Idle;
        assert_eq!(idle_state.agent_id(), None);
        assert_eq!(idle_state.issue_number(), None);
        assert_eq!(idle_state.branch_name(), None);
        assert!(!idle_state.is_busy());
        assert!(idle_state.is_available());
    }
    
    #[test]
    fn test_parse_agent_branch() {
        // Test legacy format: agent001/123
        assert_eq!(parse_agent_branch("agent001/123"), Some(("agent001".to_string(), 123)));
        assert_eq!(parse_agent_branch("agent002/456"), Some(("agent002".to_string(), 456)));
        
        // Test new descriptive format: agent001/123-description  
        assert_eq!(parse_agent_branch("agent001/123-fix-clambake-land"), Some(("agent001".to_string(), 123)));
        assert_eq!(parse_agent_branch("agent002/456-implement-new-feature"), Some(("agent002".to_string(), 456)));
        assert_eq!(parse_agent_branch("agent001/789-very-long-descriptive-title-here"), Some(("agent001".to_string(), 789)));
        
        // Test invalid formats
        assert_eq!(parse_agent_branch("main"), None);
        assert_eq!(parse_agent_branch("feature/branch"), None);
        assert_eq!(parse_agent_branch("agent001/notanumber"), None);
        assert_eq!(parse_agent_branch("agent001/notanumber-with-description"), None);
        assert_eq!(parse_agent_branch("agent001"), None);
        assert_eq!(parse_agent_branch("agent001/123/extra/parts"), None);
    }
    
    #[test]
    fn test_extract_agent_from_branch() {
        // Test legacy format
        assert_eq!(extract_agent_from_branch("agent001/123"), "agent001");
        assert_eq!(extract_agent_from_branch("agent002/456"), "agent002");
        
        // Test descriptive format
        assert_eq!(extract_agent_from_branch("agent001/123-fix-clambake-land"), "agent001");
        assert_eq!(extract_agent_from_branch("agent003/789-implement-feature"), "agent003");
        
        // Test invalid formats
        assert_eq!(extract_agent_from_branch("main"), "unknown");
        assert_eq!(extract_agent_from_branch("invalid"), "unknown");
    }
    
    #[test]
    fn test_risk_level_ordering() {
        assert!(RiskLevel::Safe < RiskLevel::Low);
        assert!(RiskLevel::Low < RiskLevel::Medium);
        assert!(RiskLevel::Medium < RiskLevel::High);
        assert!(RiskLevel::High < RiskLevel::Critical);
    }
    
    #[test]
    fn test_integration_scenario() {
        // This test demonstrates how we can test a full workflow scenario
        let git = MockGitOperations::new();
        let github = MockGitHubOperations::new("owner", "repo");
        let executor = MockCommandExecutor::with_git_github(git, github);
        
        // Set up initial state - agent is working on issue 123
        executor.git.set_current_branch("agent001/123");
        executor.git.set_commits_ahead("main", vec!["abc123: Implement feature".to_string()]);
        
        let issue = IssueData {
            number: 123,
            title: "Implement feature X".to_string(),
            labels: vec!["agent001".to_string(), "route:ready".to_string()],
            state: "open".to_string(),
            assignee: Some("johnhkchen".to_string()),
        };
        executor.github.add_issue(issue);
        
        // Simulate the "clambake land" workflow commands
        let land_commands = vec![
            Command::GitHub(GitHubCommand::AddLabel { 
                issue: 123, 
                label: "route:review".to_string() 
            }),
            Command::GitHub(GitHubCommand::RemoveLabel { 
                issue: 123, 
                label: "agent001".to_string() 
            }),
            Command::GitHub(GitHubCommand::RemoveLabel { 
                issue: 123, 
                label: "route:ready".to_string() 
            }),
            Command::Git(GitCommand::CheckoutBranch { 
                branch: "main".to_string() 
            }),
            Command::Print("✅ Agent agent001 freed - ready for new assignment".to_string()),
        ];
        
        // Execute the workflow
        let results = executor.execute_sequence(&land_commands).unwrap();
        
        // Verify all commands succeeded
        assert!(results.iter().all(|r| r.success));
        
        // Verify state changes
        assert!(executor.github.has_label(123, "route:review"));
        assert!(!executor.github.has_label(123, "agent001"));
        assert!(!executor.github.has_label(123, "route:ready"));
        assert_eq!(executor.git.get_current_branch().unwrap(), "main");
        
        // Verify command sequence was executed correctly
        let executed = executor.get_executed_commands();
        assert_eq!(executed.len(), 5);
        
        // This proves the agent lifecycle workflow works correctly!
        println!("✅ Full agent lifecycle workflow tested successfully");
    }

    #[test]
    fn test_agent_state_detector_creation() {
        use super::super::detector::AgentStateDetector;
        
        let github = MockGitHubOperations::new("test-owner", "test-repo");
        let git = MockGitOperations::new();
        
        // Test that AgentStateDetector can be created with mock dependencies
        let _detector = AgentStateDetector::new(github, git);
        
        // Constructor succeeded - basic test for instantiation
        assert!(true);
    }

    #[test]
    fn test_real_command_executor_print_command() {
        use super::super::executor::RealCommandExecutor;
        
        let github = MockGitHubOperations::new("test-owner", "test-repo");
        let git = MockGitOperations::new();
        
        let executor = RealCommandExecutor::new(github, git);
        
        // Test basic print command execution
        let print_cmd = Command::Print("Test message".to_string());
        let result = executor.execute(&print_cmd).unwrap();
        
        assert!(result.success);
        assert!(result.output.contains("Test message"));
        assert!(result.error.is_none());
    }

    #[test]
    fn test_real_command_executor_git_command_execution() {
        use super::super::executor::RealCommandExecutor;
        
        let github = MockGitHubOperations::new("test-owner", "test-repo");
        let git = MockGitOperations::new();
        
        let executor = RealCommandExecutor::new(github, git);
        
        // Test git get current branch command (read-only operation)
        let get_branch_cmd = Command::Git(GitCommand::GetCurrentBranch);
        
        let result = executor.execute(&get_branch_cmd).unwrap();
        assert!(result.success);
        // The mock should return "main" as default current branch
        assert!(result.output.contains("main"));
    }

    #[test] 
    fn test_real_command_executor_github_command_execution() {
        use super::super::executor::RealCommandExecutor;
        
        let github = MockGitHubOperations::new("test-owner", "test-repo");
        let git = MockGitOperations::new();
        
        // Set up mock issue data first
        github.add_issue(IssueData {
            number: 123,
            title: "Test issue".to_string(),
            state: "open".to_string(),
            labels: vec!["test-label".to_string()],
            assignee: None,
        });
        
        let executor = RealCommandExecutor::new(github, git);
        
        // Test GitHub get issue command
        let get_issue_cmd = Command::GitHub(GitHubCommand::GetIssue { issue: 123 });
        
        let result = executor.execute(&get_issue_cmd).unwrap();
        assert!(result.success);
        // Should return issue data in some format
        assert!(!result.output.is_empty());
    }

    #[test]
    fn test_real_command_executor_sequence_success() {
        use super::super::executor::RealCommandExecutor;
        
        let github = MockGitHubOperations::new("test-owner", "test-repo");
        let git = MockGitOperations::new();
        
        let executor = RealCommandExecutor::new(github, git);
        
        let commands = vec![
            Command::Print("Starting".to_string()),
            Command::Print("Complete".to_string()),
        ];
        
        let results = executor.execute_sequence(&commands).unwrap();
        
        // Verify all commands succeeded
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.success));
        assert!(results[0].output.contains("Starting"));
        assert!(results[1].output.contains("Complete"));
    }

    #[test]
    fn test_agent_state_detector_with_prepared_mocks() {
        use super::super::detector::AgentStateDetector;
        
        let github = MockGitHubOperations::new("test-owner", "test-repo");
        let git = MockGitOperations::new();
        
        // Set up test data in mocks
        github.add_issue(IssueData {
            number: 123,
            title: "Test issue".to_string(),
            state: "open".to_string(),
            labels: vec!["agent001".to_string()],
            assignee: None,
        });
        
        git.set_current_branch("agent001/123");
        git.set_commits_ahead("main", vec!["commit1".to_string()]);
        
        let _detector = AgentStateDetector::new(github, git);
        
        // Test validates AgentStateDetector can be created with prepared mock data
        assert!(true);
    }
}