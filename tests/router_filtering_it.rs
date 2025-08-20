// Testing library/framework used: Rust built-in test framework with Tokio if async used.
// This file validates label-based semantics on octocrab Issue objects as a contract.
// Note: These are not invoking AgentRouter network operations; they ensure the label-state semantics used by the router remain consistent.

use octocrab::models::{issues::Issue, IssueState, Label, User};

fn make_issue(
    number: u64,
    state: IssueState,
    labels: Vec<&str>,
    assignee_login: Option<&str>,
) -> Issue {
    Issue {
        number,
        state,
        labels: labels
            .into_iter()
            .map(|name| Label {
                id: None,
                node_id: None,
                url: None,
                name: name.to_string(),
                description: None,
                color: None,
                default: None,
            })
            .collect(),
        assignee: assignee_login.map(|login| User {
            login: login.to_string(),
            id: 0u64.into(),
            node_id: None,
            avatar_url: None,
            gravatar_id: None,
            url: None,
            html_url: None,
            followers_url: None,
            following_url: None,
            gists_url: None,
            starred_url: None,
            subscriptions_url: None,
            organizations_url: None,
            repos_url: None,
            events_url: None,
            received_events_url: None,
            site_admin: None,
            name: None,
            company: None,
            blog: None,
            location: None,
            email: None,
            hireable: None,
            bio: None,
            twitter_username: None,
            public_repos: None,
            public_gists: None,
            followers: None,
            following: None,
            created_at: None,
            updated_at: None,
            suspended_at: None,
        }),
        // Unused fields defaulted
        id: 0u64.into(),
        node_id: None,
        url: None,
        repository_url: None,
        labels_url: None,
        comments_url: None,
        events_url: None,
        html_url: None,
        number_from_url: None,
        title: None,
        user: None,
        body: None,
        closed_at: None,
        created_at: None,
        updated_at: None,
        locked: None,
        active_lock_reason: None,
        comments: None,
        pull_request: None,
        milestone: None,
        assignees: vec![],
        author_association: None,
        state_reason: None,
        reactions: None,
        timeline_url: None,
        performed_via_github_app: None,
        repository: None,
        draft: None,
    }
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
    let is_routable = if has_route_ready { !has_agent_label } else { false };
    assert!(is_open && is_routable && !is_human_only);
}

#[test]
fn route_ready_with_agent_label_excluded() {
    let issue = make_issue(101, IssueState::Open, vec!["route:ready", "agent:beta"], None);
    let has_route_ready = issue.labels.iter().any(|l| l.name == "route:ready");
    let has_agent_label = issue.labels.iter().any(|l| l.name.starts_with("agent"));
    let is_routable = if has_route_ready { !has_agent_label } else { false };
    assert!(!is_routable);
}

#[test]
fn route_land_always_routable_if_open() {
    let issue = make_issue(102, IssueState::Open, vec!["route:land"], None);
    let is_open = issue.state == IssueState::Open;
    let has_route_land = issue.labels.iter().any(|l| l.name == "route:land");
    let is_routable = if has_route_land { true } else { false };
    assert!(is_open && is_routable);
}

#[test]
fn closed_issue_never_routable() {
    let issue = make_issue(103, IssueState::Closed, vec!["route:land"], None);
    let is_open = issue.state == IssueState::Open;
    assert!(!is_open);
}

#[test]
fn human_only_excludes_unassigned_bots() {
    let issue = make_issue(104, IssueState::Open, vec!["route:ready", "route:human-only"], None);
    let is_human_only = issue.labels.iter().any(|l| l.name == "route:human-only");
    let has_route_ready = issue.labels.iter().any(|l| l.name == "route:ready");
    let has_agent_label = issue.labels.iter().any(|l| l.name.starts_with("agent"));
    let is_routable = if has_route_ready { !has_agent_label } else { false };
    assert!(!(is_routable && !is_human_only));
}