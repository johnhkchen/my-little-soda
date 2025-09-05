// Tests for the agent lifecycle state machine

#[cfg(test)]
mod tests {
    use super::super::state_machine::{AgentEvent, AgentStateMachine};
    use super::super::types::{parse_agent_branch, AgentState};

    #[test]
    fn test_agent_state_machine_creation() {
        let machine = AgentStateMachine::new("agent001".to_string());
        assert_eq!(machine.agent_id(), "agent001");
        assert_eq!(machine.current_issue, None);
        assert_eq!(machine.current_branch, None);
        assert_eq!(machine.commits_ahead, 0);
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
        assert_eq!(
            parse_agent_branch("agent001/123"),
            Some(("agent001".to_string(), 123))
        );
        assert_eq!(
            parse_agent_branch("agent002/456"),
            Some(("agent002".to_string(), 456))
        );
        assert_eq!(
            parse_agent_branch("agent001/123-fix-clambake-land"),
            Some(("agent001".to_string(), 123))
        );
        assert_eq!(
            parse_agent_branch("agent002/456-implement-new-feature"),
            Some(("agent002".to_string(), 456))
        );
        assert_eq!(
            parse_agent_branch("agent001/789-very-long-descriptive-title-here"),
            Some(("agent001".to_string(), 789))
        );
        assert_eq!(parse_agent_branch("main"), None);
        assert_eq!(parse_agent_branch("feature/branch"), None);
        assert_eq!(parse_agent_branch("agent001/notanumber"), None);
        assert_eq!(
            parse_agent_branch("agent001/notanumber-with-description"),
            None
        );
        assert_eq!(parse_agent_branch("agent001"), None);
        assert_eq!(parse_agent_branch("agent001/123/extra/parts"), None);
    }

    #[test]
    fn test_agent_events() {
        let assign_event = AgentEvent::Assign {
            agent_id: "agent001".to_string(),
            issue: 123,
            branch: "agent001/123".to_string(),
        };

        let start_work_event = AgentEvent::StartWork { commits_ahead: 2 };

        let complete_work_event = AgentEvent::CompleteWork;

        let bundle_event = AgentEvent::Bundle {
            bundle_pr: 456,
            issues: vec![123, 124],
        };

        let merge_event = AgentEvent::Merge;
        let abandon_event = AgentEvent::Abandon;
        let force_reset_event = AgentEvent::ForceReset;

        // Test that events can be created and are distinct
        assert_ne!(assign_event, start_work_event);
        assert_ne!(complete_work_event, bundle_event);
        assert_ne!(merge_event, abandon_event);
        assert_ne!(abandon_event, force_reset_event);
    }

    #[test]
    fn test_agent_state_machine_basic_workflow() {
        let machine = AgentStateMachine::new("agent001".to_string());

        // Initial state
        assert_eq!(machine.current_issue, None);
        assert_eq!(machine.current_branch, None);
        assert_eq!(machine.commits_ahead, 0);
        assert!(machine.bundle_issues.is_empty());
        assert_eq!(machine.bundle_pr, None);
    }
}
