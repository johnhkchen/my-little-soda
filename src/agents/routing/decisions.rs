use crate::priority::Priority;
use octocrab::models::issues::Issue;

#[derive(Debug)]
pub struct RoutingDecisions;

impl Default for RoutingDecisions {
    fn default() -> Self {
        Self::new()
    }
}

impl RoutingDecisions {
    pub fn new() -> Self {
        Self
    }

    pub fn get_issue_priority(&self, issue: &Issue) -> u32 {
        let label_names: Vec<&str> = issue
            .labels
            .iter()
            .map(|label| label.name.as_str())
            .collect();
        Priority::from_labels(&label_names).value()
    }

    pub fn sort_issues_by_priority(&self, issues: &mut Vec<Issue>) {
        issues.sort_by(|a, b| {
            let a_priority = self.get_issue_priority(a);
            let b_priority = self.get_issue_priority(b);

            // Primary sort: Priority (high to low)
            match b_priority.cmp(&a_priority) {
                std::cmp::Ordering::Equal => {
                    // Secondary sort: Title lexicographically (for consistent ordering within priority)
                    a.title.cmp(&b.title)
                }
                other => other,
            }
        });
    }

    pub fn is_route_ready_to_merge_task(&self, issue: &Issue) -> bool {
        issue
            .labels
            .iter()
            .any(|label| label.name == "route:ready_to_merge")
    }

    pub fn is_route_ready_task(&self, issue: &Issue) -> bool {
        issue.labels.iter().any(|label| label.name == "route:ready")
    }

    pub fn is_route_unblocker_task(&self, issue: &Issue) -> bool {
        issue
            .labels
            .iter()
            .any(|label| label.name == "route:unblocker")
    }

    pub fn should_skip_assignment(&self, issue: &Issue) -> bool {
        self.is_route_ready_to_merge_task(issue)
    }

    pub fn is_unassigned(&self, issue: &Issue) -> bool {
        issue.assignee.is_none()
    }

    pub fn is_assigned_to_user(&self, issue: &Issue, username: &str) -> bool {
        issue
            .assignee
            .as_ref()
            .map(|assignee| assignee.login == username)
            .unwrap_or(false)
    }
}
