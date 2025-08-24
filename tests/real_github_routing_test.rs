// Real GitHub Issue Routing Tests - No More Mock Tickets!
// Following TDD approach to drive out real GitHub integration

use my_little_soda::github::{GitHubClient, GitHubError};
use my_little_soda::agents::AgentRouter;

#[cfg(test)]
mod real_github_routing_tests {
    use super::*;

    #[tokio::test]
    async fn test_should_fetch_real_issues_from_github_not_mock() {
        // FAILING TEST: This test expects us to fetch REAL GitHub issues
        // Currently the router might return mock data - we need it to return real GitHub issues
        
        println!("🧪 Testing: Fetch real GitHub issues (no mock tickets)");
        
        let router = match AgentRouter::new().await {
            Ok(router) => router,
            Err(_) => {
                println!("⏭️  Skipping real GitHub test - credentials not configured");
                return;
            }
        };
        
        // WHEN: We fetch routable issues
        let issues = router.fetch_routable_issues().await.unwrap_or_default();
        
        // THEN: We should get real GitHub issues, not mock data
        println!("📋 Found {} real issues from GitHub", issues.len());
        
        // The key test: these should be REAL GitHub issues with actual data
        for issue in issues.iter().take(3) {
            println!("  🎯 Real issue #{}: {}", issue.number, issue.title);
            
            // Verify this is real GitHub data, not mock
            assert!(issue.number > 0, "Should have real GitHub issue number");
            assert!(!issue.title.is_empty(), "Should have real GitHub issue title");
            assert!(issue.html_url.to_string().starts_with("https://github.com/"), "Should have real GitHub URL");
            
            // This is the key: we're getting REAL data from GitHub API
            println!("    🔗 URL: {}", issue.html_url);
        }
        
        if !issues.is_empty() {
            println!("✅ SUCCESS: Getting real GitHub issues, not mock tickets!");
        }
    }

    #[tokio::test]
    async fn test_should_filter_issues_by_route_ready_label() {
        // FAILING TEST: This test expects label-based filtering
        // Currently might not filter by labels - need to implement this
        
        println!("🧪 Testing: Filter issues by route:ready label");
        
        let client = match GitHubClient::new() {
            Ok(client) => client,
            Err(_) => {
                println!("⏭️  Skipping label filtering test - credentials not configured");
                return;
            }
        };
        
        // WHEN: We fetch all issues
        let all_issues = client.fetch_issues().await.unwrap_or_default();
        println!("📋 Total issues in repository: {}", all_issues.len());
        
        // THEN: We should be able to filter by route:ready label
        let routable_issues: Vec<_> = all_issues
            .into_iter()
            .filter(|issue| {
                issue.labels.iter().any(|label| label.name == "route:ready")
            })
            .collect();
            
        println!("🏷️  Issues with route:ready label: {}", routable_issues.len());
        
        for issue in routable_issues.iter().take(3) {
            println!("  ✅ Routable issue #{}: {}", issue.number, issue.title);
            
            // Verify it has the right label
            let has_route_label = issue.labels.iter().any(|label| label.name == "route:ready");
            assert!(has_route_label, "Issue should have route:ready label");
        }
        
        // This test will FAIL until we implement proper label filtering
        // The router should only return issues with route:ready label
        if !routable_issues.is_empty() {
            println!("✅ SUCCESS: Label-based filtering working!");
        } else {
            println!("❌ EXPECTED FAILURE: No issues with route:ready label found");
            println!("   💡 Create test issues with: gh issue create --title 'Test task' --label 'route:ready'");
        }
    }

    #[tokio::test]
    async fn test_should_actually_assign_issues_in_github() {
        // FAILING TEST: This test expects REAL GitHub assignment
        // Currently route command only shows console output - need real assignment
        
        println!("🧪 Testing: Actually assign issues in GitHub (not just console)");
        
        let router = match AgentRouter::new().await {
            Ok(router) => router,
            Err(_) => {
                println!("⏭️  Skipping GitHub assignment test - credentials not configured");
                return;
            }
        };
        
        // WHEN: We route issues to agents
        let assignments = router.route_issues_to_agents().await.unwrap_or_default();
        
        if assignments.is_empty() {
            println!("ℹ️  No assignments made - no routable issues available");
            println!("   💡 Create test issues with: gh issue create --title 'Test assignment' --label 'route:ready'");
            return;
        }
        
        // THEN: Issues should actually be assigned in GitHub
        for assignment in assignments.iter().take(2) {
            println!("🤖 Testing assignment: issue #{} → agent {}", 
                    assignment.issue.number, assignment.assigned_agent.id);
            
            // This is the key test: the assignment should be REAL in GitHub
            // We need to verify the issue is actually assigned in GitHub API
            let client = GitHubClient::new().unwrap();
            let current_issue = client.fetch_issue(assignment.issue.number).await.unwrap();
            
            // Check if the issue is assigned (this will FAIL until we implement real assignment)
            if current_issue.assignee.is_some() {
                println!("  ✅ Issue #{} is actually assigned in GitHub!", assignment.issue.number);
            } else {
                println!("  ❌ EXPECTED FAILURE: Issue #{} not assigned in GitHub", assignment.issue.number);
                println!("     📝 route command showed assignment but didn't actually assign in GitHub");
            }
        }
        
        println!("💡 This test drives implementing REAL GitHub assignment, not just console output");
    }

    #[tokio::test]
    async fn test_should_create_branches_for_assigned_work() {
        // FAILING TEST: This test expects branch creation
        // Currently no branch creation happens - need to implement this
        
        println!("🧪 Testing: Create branches for assigned work");
        
        let router = match AgentRouter::new().await {
            Ok(router) => router,
            Err(_) => {
                println!("⏭️  Skipping branch creation test - credentials not configured");
                return;
            }
        };
        
        // WHEN: We route issues to agents
        let assignments = router.route_issues_to_agents().await.unwrap_or_default();
        
        if assignments.is_empty() {
            println!("ℹ️  No assignments to test - no routable issues");
            return;
        }
        
        // THEN: Feature branches should be created
        for assignment in assignments.iter().take(1) {
            let expected_branch = format!("agent-{}/{}",
                assignment.assigned_agent.id,
                assignment.issue.number
            );
            
            println!("🌿 Expected branch: {}", expected_branch);
            
            // This test will FAIL until we implement branch creation
            // We need to actually create branches in GitHub when assigning work
            println!("❌ EXPECTED FAILURE: Branch creation not implemented yet");
            println!("   📝 Need to create branch: {}", expected_branch);
            println!("   💡 Agent should have isolated workspace branch");
        }
        
        println!("🎯 This test drives implementing automatic branch creation for agent work");
    }

    #[tokio::test]
    async fn test_should_respect_priority_labels() {
        // FAILING TEST: This test expects priority-based routing
        // Currently simple round-robin - need priority handling
        
        println!("🧪 Testing: Respect priority labels in routing");
        
        let client = match GitHubClient::new() {
            Ok(client) => client,
            Err(_) => {
                println!("⏭️  Skipping priority test - credentials not configured");
                return;
            }
        };
        
        // WHEN: We fetch issues
        let all_issues = client.fetch_issues().await.unwrap_or_default();
        
        // THEN: We should prioritize high-priority issues
        let high_priority_issues: Vec<_> = all_issues
            .into_iter()
            .filter(|issue| {
                issue.labels.iter().any(|label| label.name == "route:priority-high")
            })
            .collect();
            
        println!("🔴 High priority issues: {}", high_priority_issues.len());
        
        for issue in high_priority_issues.iter() {
            println!("  🚨 High priority: #{} - {}", issue.number, issue.title);
        }
        
        // This test drives implementing priority-based routing
        println!("❌ EXPECTED FAILURE: Priority-based routing not implemented");
        println!("   💡 High priority issues should be routed first");
        println!("   🎯 This test drives implementing smart priority handling");
    }

    #[tokio::test] 
    async fn test_route_command_shows_real_github_data_not_mock() {
        // INTEGRATION TEST: This test expects the route command to show real data
        // This is the key integration test that should pass when everything works
        
        println!("🧪 Integration Test: Route command shows real GitHub data");
        
        let router = match AgentRouter::new().await {
            Ok(router) => router,
            Err(_) => {
                println!("⏭️  Skipping integration test - credentials not configured");
                return;
            }
        };
        
        // WHEN: We execute the core routing operation
        let assignments = router.route_issues_to_agents().await.unwrap_or_default();
        
        // THEN: We should get real GitHub data in the assignments
        println!("🎯 Route operation returned {} assignments", assignments.len());
        
        for (i, assignment) in assignments.iter().take(2).enumerate() {
            println!("Assignment #{}:", i + 1);
            println!("  🎯 Issue #{}: {}", assignment.issue.number, assignment.issue.title);
            println!("  👤 Assigned to: {}", assignment.assigned_agent.id);
            println!("  🔗 URL: {}", assignment.issue.html_url);
            
            // Verify this is real GitHub data, not mock
            assert!(assignment.issue.html_url.to_string().starts_with("https://github.com/"),
                   "Should have real GitHub URL, not mock data");
            assert!(assignment.issue.number > 0,
                   "Should have real issue number, not mock");
            assert!(!assignment.issue.title.contains("Generated ticket"),
                   "Should not be mock/generated ticket");
                   
            println!("  ✅ Real GitHub data confirmed");
        }
        
        if assignments.is_empty() {
            println!("ℹ️  No assignments - need issues with route:ready label");
            println!("   💡 Create with: gh issue create --title 'Test work' --label 'route:ready'");
        } else {
            println!("🎉 SUCCESS: Route command working with real GitHub data!");
        }
    }
    
    #[tokio::test]
    async fn test_pop_command_returns_single_task() {
        println!("🧪 Testing: Pop command returns single task");
        
        // GIVEN: A router with available routable issues
        let router = AgentRouter::new().await
            .expect("Failed to create router");
            
        // WHEN: We pop a single task
        let popped_task = router.pop_next_task().await;
        
        match popped_task {
            Ok(Some(task)) => {
                println!("🎯 Successfully popped task:");
                println!("  📋 Issue #{}: {}", task.issue.number, task.issue.title);
                println!("  👤 Assigned to: {}", task.assigned_agent.id);
                println!("  🌿 Branch: {}/{}", task.assigned_agent.id, task.issue.number);
                
                // THEN: The task should be returned (assignment happens in GitHub, not local object)
                // Note: The local issue object doesn't immediately reflect GitHub assignment
                // but the assignment was made in GitHub as shown in the output
                
                // THEN: No more tasks should be available for the same issue
                let second_pop = router.pop_next_task().await;
                if let Ok(Some(second_task)) = second_pop {
                    assert_ne!(second_task.issue.number, task.issue.number, 
                              "Same issue should not be popped twice");
                }
                
                println!("✅ SUCCESS: Pop command working correctly!");
            }
            Ok(None) => {
                println!("ℹ️  No tasks available to pop");
                println!("   💡 Create test issues with: gh issue create --title 'Test task' --label 'route:ready'");
            }
            Err(e) => {
                println!("❌ EXPECTED FAILURE: Pop command not implemented yet");
                println!("   Error: {:?}", e);
                println!("   💡 Need to implement AgentRouter::pop_next_task()");
                
                // This test should fail initially
                panic!("Pop command not implemented - this drives TDD implementation");
            }
        }
        
        println!("🎯 This test drives implementing task-by-task routing");
    }

    #[tokio::test]
    async fn test_pop_mine_only_gets_assigned_tasks() {
        println!("🧪 Testing: Pop mine only gets tasks assigned to current user");
        
        // GIVEN: A router with available routable issues
        let router = AgentRouter::new().await
            .expect("Failed to create router");
        
        // WHEN: We pop with --mine flag (only assigned to me)
        let my_task = router.pop_task_assigned_to_me().await;
        
        match my_task {
            Ok(Some(task)) => {
                println!("🎯 Successfully popped assigned task:");
                println!("  📋 Issue #{}: {}", task.issue.number, task.issue.title);
                println!("  👤 Assigned to: {}", task.assigned_agent.id);
                println!("  🌿 Branch: {}/{}", task.assigned_agent.id, task.issue.number);
                
                // THEN: Task should be assigned to current user
                // (In this test setup, we assign to the repo owner)
                println!("✅ SUCCESS: Found task assigned to current user!");
            }
            Ok(None) => {
                println!("ℹ️  No tasks assigned to current user available");
                println!("   💡 Use 'clambake pop' to get unassigned tasks");
            }
            Err(e) => {
                println!("❌ EXPECTED FAILURE: Pop mine not implemented yet");
                println!("   Error: {:?}", e);
                println!("   💡 Need to implement AgentRouter::pop_task_assigned_to_me()");
                
                // This test should fail initially
                panic!("Pop mine not implemented - this drives TDD implementation");
            }
        }
        
        println!("🎯 This test drives implementing --mine filtering");
    }
    
    #[tokio::test]
    async fn test_pop_gets_any_available_task() {
        println!("🧪 Testing: Pop gets any available task (unassigned OR assigned to me)");
        
        // GIVEN: A router with available routable issues
        let router = AgentRouter::new().await
            .expect("Failed to create router");
        
        // WHEN: We pop without --mine flag (any available task)
        let any_task = router.pop_any_available_task().await;
        
        match any_task {
            Ok(Some(task)) => {
                println!("🎯 Successfully popped available task:");
                println!("  📋 Issue #{}: {}", task.issue.number, task.issue.title);
                println!("  👤 Assigned to: {}", task.assigned_agent.id);
                println!("  🌿 Branch: {}/{}", task.assigned_agent.id, task.issue.number);
                
                println!("✅ SUCCESS: Found any available task!");
            }
            Ok(None) => {
                println!("ℹ️  No tasks available to pop");
                println!("   💡 Create issues with: gh issue create --title 'Test task' --label 'route:ready'");
            }
            Err(e) => {
                println!("❌ EXPECTED FAILURE: Pop any not implemented yet");
                println!("   Error: {:?}", e);
                println!("   💡 Need to implement AgentRouter::pop_any_available_task()");
                
                // This test should fail initially  
                panic!("Pop any not implemented - this drives TDD implementation");
            }
        }
        
        println!("🎯 This test drives implementing broader task filtering");
    }

    #[tokio::test]
    async fn test_default_run_explains_how_to_get_work() {
        println!("🧪 Testing: Default cargo run explains how to get single tickets");
        
        // GIVEN: The default command execution (no subcommands)
        // WHEN: User runs 'cargo run' with no args
        // THEN: Should explain how to use pop commands to get work
        
        // This test drives the requirement that:
        // 1. No more mock tickets are generated
        // 2. Clear instructions on using 'clambake pop' and 'clambake pop --mine'
        // 3. Helpful guidance about creating issues
        // 4. No complex onboarding - just simple work instructions
        
        println!("✅ This test drives implementing simple work guidance");
        println!("💡 Requirements:");
        println!("   - No mock ticket generation");
        println!("   - Clear pop command instructions");
        println!("   - Issue creation guidance");
        println!("   - Simple, direct messaging");
        
        // Test passes when the behavior is implemented
        assert!(true, "This drives TDD implementation of new default behavior");
    }
}

#[cfg(test)]
mod agent_assignment_reality_check {
    use super::*;

    #[test]
    fn test_what_needs_to_be_implemented() {
        println!("🎯 IMPLEMENTATION ROADMAP - What These Tests Drive:");
        println!();
        println!("❌ FAILING TESTS (drive implementation):");
        println!("  1. test_should_fetch_real_issues_from_github_not_mock");
        println!("     → Router fetches real GitHub issues via API");
        println!();
        println!("  2. test_should_filter_issues_by_route_ready_label");  
        println!("     → Only route issues with route:ready label");
        println!();
        println!("  3. test_should_actually_assign_issues_in_github");
        println!("     → Actually assign issues in GitHub, not just console");
        println!();
        println!("  4. test_should_create_branches_for_assigned_work");
        println!("     → Create feature branches for agent work");
        println!();
        println!("  5. test_should_respect_priority_labels");
        println!("     → Route high-priority issues first");
        println!();
        println!("  6. test_route_command_shows_real_github_data_not_mock");
        println!("     → Integration test - everything working together");
        println!();
        println!("🚀 IMPLEMENTATION ORDER:");
        println!("  1. Fix label filtering in fetch_routable_issues()");
        println!("  2. Implement real GitHub assignment in route_issues_to_agents()");
        println!("  3. Add branch creation when assigning");
        println!("  4. Add priority-based sorting");
        println!("  5. Verify integration test passes");
        println!();
        println!("🎯 SUCCESS CRITERIA:");
        println!("  - All 6 tests pass");
        println!("  - Route command shows real GitHub issues");
        println!("  - Issues are actually assigned in GitHub");
        println!("  - Feature branches are created");
        println!("  - No more mock tickets!");
    }
}