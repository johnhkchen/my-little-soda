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

#[cfg(test)]
mod tests {
    use super::*;
    
    // Helper function to create test issues using fixture approach
    fn create_test_issue(number: u64, title: &str, label_names: Vec<&str>) -> octocrab::models::issues::Issue {
        // Load the base issue fixture
        let json_data = include_str!("../../../tests/fixtures/github_issues.json");
        let mut base_issue: octocrab::models::issues::Issue = serde_json::from_str(json_data)
            .expect("Failed to parse test fixture JSON");
            
        base_issue.number = number;
        base_issue.title = title.to_string();
        
        // Create labels from names by cloning and modifying existing label
        if !base_issue.labels.is_empty() {
            let template_label = &base_issue.labels[0];
            base_issue.labels = label_names
                .into_iter()
                .enumerate()
                .map(|(i, name)| {
                    let mut label = template_label.clone();
                    label.name = name.to_string();
                    label.id = octocrab::models::LabelId(i as u64 + 1000);
                    label
                })
                .collect();
        }
        
        base_issue.assignee = None;
        base_issue.assignees = vec![];
        
        base_issue
    }

    #[test]
    fn test_real_a_series_priority_bug_lexicographic_sorting() {
        // This test recreates the exact bug scenario described in issue #324
        // using the real GitHub issue titles and priorities
        println!("🧪 Testing: Real A-series lexicographic sorting bug");
        
        let routing_decisions = RoutingDecisions::new();

        // Create the exact A-series issues from the GitHub repository
        // All have route:priority-very-high labels (Priority::VeryHigh = 4)
        let test_issues = vec![
            create_test_issue(294, "A2d - Optimize release build settings", vec!["route:priority-very-high", "route:ready"]),
            create_test_issue(293, "A2c - Configure Windows x64 builds", vec!["route:priority-very-high", "route:ready"]), 
            create_test_issue(292, "A2b - Configure macOS builds (Intel and ARM)", vec!["route:priority-very-high", "route:ready"]),
            create_test_issue(291, "A2a - Implement Linux x86_64 binary builds", vec!["route:priority-very-high", "route:ready"]),
            create_test_issue(290, "A1c - Set up build matrix for multiple platforms", vec!["route:priority-very-high", "route:ready"]),
            create_test_issue(289, "A1b - Configure version tag trigger conditions", vec!["route:priority-very-high", "route:ready"]),
            create_test_issue(288, "A1a - Create GitHub Actions workflow file structure", vec!["route:priority-very-high", "route:ready"]),
        ];
        
        // Verify all issues have the same priority level
        for issue in &test_issues {
            let priority = routing_decisions.get_issue_priority(issue);
            assert_eq!(priority, 4, "All A-series issues should have priority 4 (very-high)");
        }

        let mut sorted_issues = test_issues.clone();
        routing_decisions.sort_issues_by_priority(&mut sorted_issues);
        
        // Print debug info to see what's happening
        println!("🔍 Sorted order:");
        for (i, issue) in sorted_issues.iter().enumerate() {
            println!("  {}. #{}: {}", i+1, issue.number, issue.title);
        }
        
        // Expected lexicographic order when priority is equal:
        // 1. A1a - Create GitHub Actions workflow file structure (#288)
        // 2. A1b - Configure version tag trigger conditions (#289)
        // 3. A1c - Set up build matrix for multiple platforms (#290)
        // 4. A2a - Implement Linux x86_64 binary builds (#291)
        // 5. A2b - Configure macOS builds (Intel and ARM) (#292)
        // 6. A2c - Configure Windows x64 builds (#293)
        // 7. A2d - Optimize release build settings (#294)
        
        // THE BUG: Currently A2d is selected first instead of A1a
        assert_eq!(sorted_issues[0].title, "A1a - Create GitHub Actions workflow file structure");
        assert_eq!(sorted_issues[0].number, 288);
        
        assert_eq!(sorted_issues[1].title, "A1b - Configure version tag trigger conditions");
        assert_eq!(sorted_issues[1].number, 289);
        
        assert_eq!(sorted_issues[2].title, "A1c - Set up build matrix for multiple platforms");
        assert_eq!(sorted_issues[2].number, 290);
        
        assert_eq!(sorted_issues[3].title, "A2a - Implement Linux x86_64 binary builds");
        assert_eq!(sorted_issues[3].number, 291);
        
        assert_eq!(sorted_issues[4].title, "A2b - Configure macOS builds (Intel and ARM)");
        assert_eq!(sorted_issues[4].number, 292);
        
        assert_eq!(sorted_issues[5].title, "A2c - Configure Windows x64 builds");
        assert_eq!(sorted_issues[5].number, 293);
        
        assert_eq!(sorted_issues[6].title, "A2d - Optimize release build settings");
        assert_eq!(sorted_issues[6].number, 294);

        println!("✅ SUCCESS: A-series issues sorted correctly lexicographically!");
    }

    #[test] 
    fn test_priority_levels_override_lexicographic() {
        // This test ensures higher priority levels always come before lower ones,
        // regardless of lexicographic order
        let routing_decisions = RoutingDecisions::new();
        
        let test_issues = vec![
            create_test_issue(1, "Z9z - Low priority", vec!["route:priority-low"]),
            create_test_issue(2, "A1a - Very high priority", vec!["route:priority-very-high"]),
            create_test_issue(3, "M5m - Medium priority", vec!["route:priority-medium"]),
            create_test_issue(4, "B2b - High priority", vec!["route:priority-high"]),
            create_test_issue(5, "Unblocker task", vec!["route:unblocker"]),
            create_test_issue(6, "Ready to merge", vec!["route:ready_to_merge"]),
        ];
        
        let mut sorted_issues = test_issues.clone();
        routing_decisions.sort_issues_by_priority(&mut sorted_issues);
        
        // Expected priority order (Priority values in parentheses):
        assert_eq!(sorted_issues[0].title, "Unblocker task");          // Priority 200
        assert_eq!(sorted_issues[1].title, "Ready to merge");          // Priority 100  
        assert_eq!(sorted_issues[2].title, "A1a - Very high priority"); // Priority 4
        assert_eq!(sorted_issues[3].title, "B2b - High priority");      // Priority 3
        assert_eq!(sorted_issues[4].title, "M5m - Medium priority");    // Priority 2
        assert_eq!(sorted_issues[5].title, "Z9z - Low priority");       // Priority 1

        println!("✅ SUCCESS: Priority levels correctly override lexicographic ordering!");
    }
    
    #[test]
    fn test_same_priority_lexicographic_with_mixed_titles() {
        // Test lexicographic sorting within the same priority with varied titles
        let routing_decisions = RoutingDecisions::new();
        
        let test_issues = vec![
            create_test_issue(999, "Z9z - Should be last task", vec!["route:priority-very-high"]),
            create_test_issue(001, "A0a - Should be first alphabetically", vec!["route:priority-very-high"]),
            create_test_issue(288, "A1a - Create GitHub Actions workflow file structure", vec!["route:priority-very-high"]),
            create_test_issue(294, "A2d - Optimize release build settings", vec!["route:priority-very-high"]),
        ];
        
        let mut sorted_issues = test_issues.clone();
        routing_decisions.sort_issues_by_priority(&mut sorted_issues);
        
        // Should be sorted lexicographically by title when priority is equal
        assert_eq!(sorted_issues[0].title, "A0a - Should be first alphabetically");
        assert_eq!(sorted_issues[1].title, "A1a - Create GitHub Actions workflow file structure");
        assert_eq!(sorted_issues[2].title, "A2d - Optimize release build settings");
        assert_eq!(sorted_issues[3].title, "Z9z - Should be last task");

        println!("✅ SUCCESS: Mixed title lexicographic sorting works correctly!");
    }
    
    #[test]
    fn test_reproduce_real_github_bug_a3d_before_a1a() {
        // This test reproduces the exact bug we see in the real GitHub queue
        // A3d (#298) is being selected before A1a (#288) despite lexicographic ordering
        println!("🧪 Testing: Reproduce real GitHub A3d before A1a bug");
        
        let routing_decisions = RoutingDecisions::new();

        // Create the exact scenario from GitHub: all very-high priority issues
        let test_issues = vec![
            create_test_issue(298, "A3d - Test complete release pipeline", vec!["route:priority-very-high", "route:ready"]),
            create_test_issue(297, "A3c - Set up release notes generation", vec!["route:priority-very-high", "route:ready"]),
            create_test_issue(296, "A3b - Configure binary asset uploads", vec!["route:priority-very-high", "route:ready"]),
            create_test_issue(295, "A3a - Implement automated release creation", vec!["route:priority-very-high", "route:ready"]),
            create_test_issue(294, "A2d - Optimize release build settings", vec!["route:priority-very-high", "route:ready"]),
            create_test_issue(293, "A2c - Configure Windows x64 builds", vec!["route:priority-very-high", "route:ready"]),
            create_test_issue(292, "A2b - Configure macOS builds (Intel and ARM)", vec!["route:priority-very-high", "route:ready"]),
            create_test_issue(291, "A2a - Implement Linux x86_64 binary builds", vec!["route:priority-very-high", "route:ready"]),
            create_test_issue(290, "A1c - Set up build matrix for multiple platforms", vec!["route:priority-very-high", "route:ready"]),
            create_test_issue(289, "A1b - Configure version tag trigger conditions", vec!["route:priority-very-high", "route:ready"]),
            create_test_issue(288, "A1a - Create GitHub Actions workflow file structure", vec!["route:priority-very-high", "route:ready"]),
        ];
        
        let mut sorted_issues = test_issues.clone();
        routing_decisions.sort_issues_by_priority(&mut sorted_issues);
        
        // Print debug info
        println!("🔍 Sorted order:");
        for (i, issue) in sorted_issues.iter().enumerate() {
            println!("  {}. #{}: {}", i+1, issue.number, issue.title);
        }
        
        // Expected: A1a should be first, then A1b, A1c, A2a, A2b, A2c, A2d, A3a, A3b, A3c, A3d
        assert_eq!(sorted_issues[0].title, "A1a - Create GitHub Actions workflow file structure");
        assert_eq!(sorted_issues[0].number, 288);
        
        assert_eq!(sorted_issues[1].title, "A1b - Configure version tag trigger conditions");
        assert_eq!(sorted_issues[1].number, 289);
        
        assert_eq!(sorted_issues[2].title, "A1c - Set up build matrix for multiple platforms");
        assert_eq!(sorted_issues[2].number, 290);
        
        // A3d should be last, not first!
        assert_eq!(sorted_issues[10].title, "A3d - Test complete release pipeline");
        assert_eq!(sorted_issues[10].number, 298);

        println!("✅ SUCCESS: Real GitHub ordering reproduced correctly in unit test!");
    }
}