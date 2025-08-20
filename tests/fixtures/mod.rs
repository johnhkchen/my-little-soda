/// Test fixtures with real GitHub API response data for consistent testing
use octocrab::models::issues::Issue;

/// Load test issues from cached GitHub API responses
pub fn load_test_issues() -> Vec<Issue> {
    let json_data = include_str!("github_issues.json");
    let issue: Issue = serde_json::from_str(json_data).expect("Failed to parse test fixture JSON");
    vec![issue]
}

/// Get specific test issues by scenario
pub fn get_agent001_working_issue() -> Issue {
    load_test_issues().into_iter()
        .find(|issue| issue.number == 1 && 
              issue.labels.iter().any(|l| l.name == "agent001"))
        .expect("Agent001 working issue not found in fixtures")
}

// Future: These will be implemented when we expand to multi-agent and human-only filtering
// For now, focusing on single agent001 work management

/// Filter issues by criteria (mirrors the production logic)
pub fn filter_agent001_ongoing_work(issues: &[Issue]) -> Vec<&Issue> {
    issues
        .iter()
        .filter(|issue| {
            let is_open = issue.state == octocrab::models::IssueState::Open;
            let has_agent001_label = issue.labels.iter()
                .any(|label| label.name == "agent001");
            let has_route_label = issue.labels.iter()
                .any(|label| label.name == "route:ready");
            
            is_open && has_agent001_label && has_route_label
        })
        .collect()
}

// Future filters for multi-agent and human-only scenarios
// These will be implemented in later phases

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixture_loading() {
        let issues = load_test_issues();
        assert_eq!(issues.len(), 1);
        
        // Verify the real GitHub issue #1 has expected properties for agent001
        let agent001_issue = &issues[0];
        assert_eq!(agent001_issue.number, 1);
        assert_eq!(agent001_issue.title, "Real GitHub Issue Routing Implementation");
        assert!(agent001_issue.labels.iter().any(|l| l.name == "agent001"));
        assert!(agent001_issue.labels.iter().any(|l| l.name == "route:ready"));
        assert!(agent001_issue.assignee.is_some());
        assert_eq!(agent001_issue.assignee.as_ref().unwrap().login, "johnhkchen");
    }
    
    #[test]
    fn test_agent001_ongoing_work_filter() {
        let issues = load_test_issues();
        let ongoing = filter_agent001_ongoing_work(&issues);
        
        assert_eq!(ongoing.len(), 1);
        assert_eq!(ongoing[0].number, 1);
    }
    
    // Future: Test multi-agent and human-only filtering
    // For now, focusing on agent001 work management
}