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

/// Create a test issue with human-only label for testing human-only filtering
pub fn create_human_only_issue() -> Issue {
    let mut base_issue = load_test_issues()[0].clone();
    
    // Modify to be a human-only issue
    base_issue.number = 999;
    base_issue.title = "Human-only task - sensitive security review".to_string();
    
    // Create human-only label by cloning an existing one and modifying it
    let mut human_only_label = base_issue.labels[0].clone();
    human_only_label.name = "route:human-only".to_string();
    
    // Remove agent001 label and add human-only label
    base_issue.labels = base_issue.labels.into_iter()
        .filter(|label| !label.name.starts_with("agent"))
        .collect();
    base_issue.labels.push(human_only_label);
    
    // Clear assignee for unassigned human-only task
    base_issue.assignee = None;
    base_issue.assignees = vec![];
    
    base_issue
}

/// Create a normal routable issue (no human-only label)
pub fn create_normal_routable_issue() -> Issue {
    let mut base_issue = load_test_issues()[0].clone();
    
    // Modify to be a normal routable issue
    base_issue.number = 998;
    base_issue.title = "Normal task - implement feature X".to_string();
    
    // Remove agent labels and assignee to make it available for routing
    base_issue.labels = base_issue.labels.into_iter()
        .filter(|label| !label.name.starts_with("agent"))
        .collect();
    base_issue.assignee = None;
    base_issue.assignees = vec![];
    
    base_issue
}

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

/// Filter issues for bot routing (excludes human-only tasks)
pub fn filter_bot_routable_issues(issues: &[Issue]) -> Vec<&Issue> {
    issues
        .iter()
        .filter(|issue| {
            let is_open = issue.state == octocrab::models::IssueState::Open;
            let has_route_label = issue.labels.iter()
                .any(|label| label.name == "route:ready");
            let has_agent_label = issue.labels.iter()
                .any(|label| label.name.starts_with("agent"));
            let is_human_only = issue.labels.iter()
                .any(|label| label.name == "route:human-only");
            
            is_open && has_route_label && !has_agent_label && !is_human_only
        })
        .collect()
}

/// Filter issues available for humans (includes human-only tasks)
pub fn filter_human_available_issues<'a>(issues: &'a [Issue], current_user: &str) -> Vec<&'a Issue> {
    issues
        .iter()
        .filter(|issue| {
            let is_open = issue.state == octocrab::models::IssueState::Open;
            let has_route_label = issue.labels.iter()
                .any(|label| label.name == "route:ready");
            
            if !is_open || !has_route_label {
                return false;
            }
            
            // Check if this is a human-only task
            let is_human_only = issue.labels.iter()
                .any(|label| label.name == "route:human-only");
            
            // Accept based on assignment status and human-only filtering
            match &issue.assignee {
                None => {
                    // Unassigned tasks: humans can take both normal and human-only tasks
                    true
                },
                Some(assignee) => {
                    // Tasks assigned to current user: allow regardless of human-only status
                    assignee.login == current_user
                }
            }
        })
        .collect()
}

/// Create a completed issue (with route:ready_to_merge label indicating merge-ready)
pub fn create_completed_issue(number: u64, title: &str) -> Issue {
    let mut base_issue = load_test_issues()[0].clone();
    
    base_issue.number = number;
    base_issue.title = title.to_string();
    
    // Create route:ready_to_merge label to indicate completed work
    let mut land_label = base_issue.labels[0].clone();
    land_label.name = "route:ready_to_merge".to_string();
    
    // Remove agent labels and add route:ready_to_merge
    base_issue.labels = base_issue.labels.into_iter()
        .filter(|label| !label.name.starts_with("agent"))
        .collect();
    base_issue.labels.push(land_label);
    
    base_issue
}

/// Create a ready issue (route:ready label but no agent assignment)
pub fn create_ready_issue(number: u64, title: &str) -> Issue {
    let mut base_issue = load_test_issues()[0].clone();
    
    base_issue.number = number;
    base_issue.title = title.to_string();
    
    // Remove agent labels and assignee to make it available
    base_issue.labels = base_issue.labels.into_iter()
        .filter(|label| !label.name.starts_with("agent"))
        .collect();
    base_issue.assignee = None;
    base_issue.assignees = vec![];
    
    base_issue
}

/// Filter issues that should be assignable (excludes completed work)
pub fn filter_assignable_issues(issues: &[Issue]) -> Vec<&Issue> {
    issues
        .iter()
        .filter(|issue| {
            let is_open = issue.state == octocrab::models::IssueState::Open;
            let has_route_ready = issue.labels.iter()
                .any(|label| label.name == "route:ready");
            let has_route_land = issue.labels.iter()
                .any(|label| label.name == "route:ready_to_merge");
            let has_agent_label = issue.labels.iter()
                .any(|label| label.name.starts_with("agent"));
            
            // Issue is assignable if it's open, has route:ready, 
            // but NOT route:ready_to_merge (completed) and NOT already assigned to agent
            is_open && has_route_ready && !has_route_land && !has_agent_label
        })
        .collect()
}

/// Create an issue with specific labels and assignment (for testing)
pub fn create_issue_with_labels(number: u64, title: &str, label_names: Vec<String>, assignee_login: Option<String>) -> Issue {
    let mut base_issue = load_test_issues()[0].clone();
    
    base_issue.number = number;
    base_issue.title = title.to_string();
    
    // Create labels from names
    base_issue.labels = label_names.into_iter().enumerate().map(|(i, name)| {
        let mut label = base_issue.labels[0].clone();
        label.name = name;
        label.id = octocrab::models::LabelId(i as u64 + 1000); // Unique IDs
        label
    }).collect();
    
    // Set assignee
    if let Some(login) = assignee_login {
        let mut assignee = base_issue.assignee.as_ref().unwrap().clone();
        assignee.login = login;
        base_issue.assignee = Some(assignee.clone());
        base_issue.assignees = vec![assignee];
    } else {
        base_issue.assignee = None;
        base_issue.assignees = vec![];
    }
    
    base_issue
}
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
    
    #[test]
    fn test_human_only_issue_creation() {
        let human_only_issue = create_human_only_issue();
        
        assert_eq!(human_only_issue.number, 999);
        assert!(human_only_issue.labels.iter().any(|l| l.name == "route:human-only"));
        assert!(human_only_issue.labels.iter().any(|l| l.name == "route:ready"));
        assert!(!human_only_issue.labels.iter().any(|l| l.name.starts_with("agent")));
        assert!(human_only_issue.assignee.is_none());
    }
    
    #[test] 
    fn test_normal_routable_issue_creation() {
        let normal_issue = create_normal_routable_issue();
        
        assert_eq!(normal_issue.number, 998);
        assert!(normal_issue.labels.iter().any(|l| l.name == "route:ready"));
        assert!(!normal_issue.labels.iter().any(|l| l.name == "route:human-only"));
        assert!(!normal_issue.labels.iter().any(|l| l.name.starts_with("agent")));
        assert!(normal_issue.assignee.is_none());
    }
    
    #[test]
    fn test_bot_filtering_excludes_human_only() {
        let human_only_issue = create_human_only_issue();
        let normal_issue = create_normal_routable_issue();
        let issues = vec![human_only_issue, normal_issue];
        
        let bot_routable = filter_bot_routable_issues(&issues);
        
        // Only normal issue should be available for bots
        assert_eq!(bot_routable.len(), 1);
        assert_eq!(bot_routable[0].number, 998);
        assert!(!bot_routable[0].labels.iter().any(|l| l.name == "route:human-only"));
    }
    
    #[test]
    fn test_human_filtering_includes_all_available() {
        let human_only_issue = create_human_only_issue();
        let normal_issue = create_normal_routable_issue();
        let issues = vec![human_only_issue, normal_issue];
        
        let human_available = filter_human_available_issues(&issues, "testuser");
        
        // Both issues should be available for humans
        assert_eq!(human_available.len(), 2);
        assert!(human_available.iter().any(|i| i.number == 999)); // human-only
        assert!(human_available.iter().any(|i| i.number == 998)); // normal
    }
}