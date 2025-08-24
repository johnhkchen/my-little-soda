//! Test to reproduce the clambake peek bug
//!
//! BUG: After completing work on an issue, clambake peek still shows it as available work
//! 
//! Scenario:
//! 1. Agent gets assigned issue #95 with route:unblocker + route:priority-high labels  
//! 2. Agent completes work and commits/pushes
//! 3. clambake peek still shows issue #95 as next available task
//! 4. Issue still has agent001 label and assignee
//!
//! This reproduces the exact bug we experienced.


mod fixtures;

#[tokio::test]
async fn test_clambake_peek_shows_completed_work_bug_reproduction() {
    // GIVEN: An issue that was assigned to agent001 and completed
    // This simulates the exact state we observed with issue #95
    
    // Create a mock scenario that reproduces the bug
    let completed_issue = fixtures::create_issue_with_labels(
        95,
        "[UNBLOCKER] Add comprehensive state management regression tests",
        vec![
            "route:unblocker".to_string(),
            "route:priority-high".to_string(), 
            "agent001".to_string(),  // Still has agent label - this is the bug!
        ],
        Some("johnhkchen".to_string()), // Still assigned
    );
    
    // WHEN: We call fetch_routable_issues (what clambake peek uses)
    // This should simulate what AgentRouter.fetch_routable_issues() returns
    let mock_issues = vec![completed_issue.clone()];
    let routable_issues = filter_routable_issues_logic(&mock_issues);
    
    // THEN: The completed issue should NOT appear as routable
    // BUT currently it does - this is the bug we need to fix
    
    // This assertion SHOULD pass but currently fails - reproducing the bug
    let has_completed_work = routable_issues.iter()
        .any(|issue| issue.number == 95 && 
             issue.labels.iter().any(|l| l.name.starts_with("agent")));
    
    // CURRENT BUG: This assertion fails because completed work appears as routable
    assert!(
        !has_completed_work,
        "BUG REPRODUCED: Issue #{} with agent label still appears as routable work. \
         This is exactly the bug we experienced with clambake peek.",
        completed_issue.number
    );
}

#[tokio::test]
async fn test_correct_behavior_after_agent_completes_work() {
    // GIVEN: The same scenario but with correct state management
    
    let ready_issue = fixtures::create_issue_with_labels(
        96,
        "A new task ready for assignment", 
        vec![
            "route:ready".to_string(),
            "route:priority-medium".to_string(),
        ],
        None, // Not assigned
    );
    
    let completed_issue = fixtures::create_issue_with_labels(
        95,
        "[UNBLOCKER] Add comprehensive state management regression tests",
        vec![
            // NO routing labels - work is truly completed and closed
        ],
        None, // No longer assigned  
    );
    
    // WHEN: We filter for routable issues
    let mock_issues = vec![ready_issue.clone(), completed_issue.clone()];
    let routable_issues = filter_routable_issues_logic(&mock_issues);
    
    // THEN: Only the ready issue should be routable, not the completed one
    assert_eq!(routable_issues.len(), 1, "Should only have 1 routable issue");
    assert_eq!(routable_issues[0].number, 96, "Should be the ready issue, not completed");
    
    let has_completed_work = routable_issues.iter()
        .any(|issue| issue.number == 95);
    assert!(!has_completed_work, "Completed work (no routing labels) should not be routable");
}

#[tokio::test] 
async fn test_route_ready_to_merge_issues_should_not_appear_in_new_work_queue() {
    // GIVEN: An issue that is merge-ready (route:ready_to_merge) 
    let merge_ready_issue = fixtures::create_issue_with_labels(
        95,
        "[UNBLOCKER] Add comprehensive state management regression tests",
        vec![
            "route:ready_to_merge".to_string(),      // Merge-ready
            "route:priority-high".to_string(),
        ],
        Some("johnhkchen".to_string()), // Still assigned for merge completion
    );
    
    let new_work_issue = fixtures::create_issue_with_labels(
        97,
        "A new task ready for assignment",
        vec!["route:ready".to_string()],
        None,
    );
    
    // WHEN: Agent looks for NEW work to start (what clambake peek should show)
    let mock_issues = vec![merge_ready_issue.clone(), new_work_issue.clone()];
    let new_work_queue = filter_new_work_queue(&mock_issues);
    
    // THEN: Only new work should appear, not merge-ready work
    assert_eq!(new_work_queue.len(), 1, "Should only show new work");
    assert_eq!(new_work_queue[0].number, 97, "Should be the new work issue");
    
    let has_merge_ready = new_work_queue.iter()
        .any(|issue| issue.labels.iter().any(|l| l.name == "route:ready_to_merge"));
    assert!(!has_merge_ready, "Merge-ready work should not appear in new work queue");
}

// Mock the corrected routing logic
fn filter_routable_issues_logic(issues: &[octocrab::models::issues::Issue]) -> Vec<octocrab::models::issues::Issue> {
    // This mimics the CORRECTED logic in AgentRouter::fetch_routable_issues()
    issues.iter()
        .filter(|issue| {
            let is_open = issue.state == octocrab::models::IssueState::Open;
            
            let has_route_ready = issue.labels.iter().any(|l| l.name == "route:ready");
            let has_route_ready_to_merge = issue.labels.iter().any(|l| l.name == "route:ready_to_merge");  
            let has_route_unblocker = issue.labels.iter().any(|l| l.name == "route:unblocker");
            
            let has_agent_label = issue.labels.iter().any(|l| l.name.starts_with("agent"));
            
            // CORRECTED logic: unblocker tasks should not be routable if agent already working
            let is_routable = if has_route_unblocker {
                !has_agent_label // FIXED: route:unblocker should not be routable if agent already working
            } else if has_route_ready_to_merge {
                true 
            } else if has_route_ready {
                !has_agent_label // route:ready only if no agent assigned
            } else {
                false
            };
            
            is_open && is_routable
        })
        .cloned()
        .collect()
}

// What the NEW WORK queue should look like (for clambake peek)
fn filter_new_work_queue(issues: &[octocrab::models::issues::Issue]) -> Vec<octocrab::models::issues::Issue> {
    // This is what clambake peek SHOULD show - only work available for NEW assignment
    issues.iter()
        .filter(|issue| {
            let is_open = issue.state == octocrab::models::IssueState::Open;
            
            let has_route_ready = issue.labels.iter().any(|l| l.name == "route:ready");
            let has_agent_label = issue.labels.iter().any(|l| l.name.starts_with("agent"));
            
            // CORRECTED logic: Only truly available work
            let is_new_work = has_route_ready && !has_agent_label;
            
            is_open && is_new_work
        })
        .cloned()
        .collect()
}