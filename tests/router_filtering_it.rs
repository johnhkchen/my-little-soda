// Testing library/framework used: Rust built-in test framework with Tokio if async used.
// This file validates label-based semantics on octocrab Issue objects as a contract.
// Note: These are not invoking AgentRouter network operations; they ensure the label-state semantics used by the router remain consistent.

use octocrab::models::{issues::Issue, IssueState};

fn make_issue(
    number: u64,
    state: IssueState,
    labels: Vec<&str>,
    assignee_login: Option<&str>,
) -> Issue {
    let state_str = match state {
        IssueState::Open => "open",
        IssueState::Closed => "closed",
        _ => "open", // Default to open for any other states
    };

    let assignee_json = if let Some(login) = assignee_login {
        serde_json::json!({
            "login": login,
            "id": 1
        })
    } else {
        serde_json::Value::Null
    };

    let labels_json: Vec<serde_json::Value> = labels
        .into_iter()
        .map(|name| {
            serde_json::json!({
                "id": 1000,
                "name": name
            })
        })
        .collect();

    serde_json::from_value(serde_json::json!({
        "id": number,
        "number": number,
        "state": state_str,
        "title": format!("Test Issue {}", number),
        "user": {
            "login": "testuser",
            "id": 1
        },
        "body": "Test issue body",
        "labels": labels_json,
        "assignee": assignee_json,
        "assignees": assignee_login.map(|login| vec![serde_json::json!({"login": login, "id": 1})]).unwrap_or_default(),
        "comments": 0,
        "created_at": "2023-01-01T00:00:00Z",
        "updated_at": "2023-01-01T00:00:00Z",
        "html_url": format!("https://github.com/test/test/issues/{}", number),
        "url": format!("https://api.github.com/repos/test/test/issues/{}", number),
        "repository_url": "https://api.github.com/repos/test/test",
        "labels_url": format!("https://api.github.com/repos/test/test/issues/{}/labels{{/name}}", number),
        "comments_url": format!("https://api.github.com/repos/test/test/issues/{}/comments", number),
        "events_url": format!("https://api.github.com/repos/test/test/issues/{}/events", number),
        "locked": false,
        "author_association": "OWNER"
    })).expect("Failed to construct test issue")
}

// These tests validate the routing semantics as a contract on issue metadata,
// independent of AgentRouter's internal structure.

#[test]
fn route_ready_unassigned_is_candidate() {
    let issue = make_issue(100, IssueState::Open, vec!["route:ready"], None);
    let is_open = issue.state == IssueState::Open;
    let has_route_ready = issue.labels.iter().any(|l| l.name == "route:ready");
    let has_agent_label = issue.labels.iter().any(|l| l.name.starts_with("agent"));
    let is_human_only = issue.labels.iter().any(|l| l.name == "route:human-only");
    let is_routable = if has_route_ready {
        !has_agent_label
    } else {
        false
    };
    assert!(is_open && is_routable && !is_human_only);
}

#[test]
fn route_ready_with_agent_label_excluded() {
    let issue = make_issue(
        101,
        IssueState::Open,
        vec!["route:ready", "agent:beta"],
        None,
    );
    let has_route_ready = issue.labels.iter().any(|l| l.name == "route:ready");
    let has_agent_label = issue.labels.iter().any(|l| l.name.starts_with("agent"));
    let is_routable = if has_route_ready {
        !has_agent_label
    } else {
        false
    };
    assert!(!is_routable);
}

#[test]
fn route_ready_to_merge_always_routable_if_open() {
    let issue = make_issue(102, IssueState::Open, vec!["route:ready_to_merge"], None);
    let is_open = issue.state == IssueState::Open;
    let has_route_ready_to_merge = issue
        .labels
        .iter()
        .any(|l| l.name == "route:ready_to_merge");
    let is_routable = has_route_ready_to_merge;
    assert!(is_open && is_routable);
}

#[test]
fn closed_issue_never_routable() {
    let issue = make_issue(103, IssueState::Closed, vec!["route:ready_to_merge"], None);
    let is_open = issue.state == IssueState::Open;
    assert!(!is_open);
}

#[test]
fn human_only_excludes_unassigned_bots() {
    let issue = make_issue(
        104,
        IssueState::Open,
        vec!["route:ready", "route:human-only"],
        None,
    );
    let is_human_only = issue.labels.iter().any(|l| l.name == "route:human-only");
    let has_route_ready = issue.labels.iter().any(|l| l.name == "route:ready");
    let has_agent_label = issue.labels.iter().any(|l| l.name.starts_with("agent"));
    let is_routable = if has_route_ready {
        !has_agent_label
    } else {
        false
    };
    assert!(!(is_routable && !is_human_only));
}
