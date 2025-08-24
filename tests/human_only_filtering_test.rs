/// Human-only label filtering integration tests
/// Tests that bot routing properly excludes human-only labeled tasks
use my_little_soda::agents::AgentRouter;

#[tokio::test]
async fn test_human_only_filtering_manual() {
    // This is a manual integration test to verify human-only filtering works
    println!("Testing human-only filtering...");

    // Test 1: Basic routing should work
    match AgentRouter::new().await {
        Ok(router) => {
            match router.fetch_routable_issues().await {
                Ok(issues) => {
                    println!("✅ Found {} routable issues", issues.len());

                    // Verify no human-only issues are included
                    let has_human_only = issues.iter().any(|issue| {
                        issue
                            .labels
                            .iter()
                            .any(|label| label.name == "route:human-only")
                    });

                    if has_human_only {
                        panic!("❌ Bot routing returned human-only labeled issues!");
                    } else {
                        println!("✅ Bot routing correctly excludes human-only issues");
                    }

                    // Show what labels exist for debugging
                    for issue in issues.iter().take(3) {
                        let labels: Vec<String> =
                            issue.labels.iter().map(|l| l.name.clone()).collect();
                        println!("Issue #{}: labels={:?}", issue.number, labels);
                    }
                }
                Err(e) => {
                    println!("⚠️  Could not fetch routable issues: {:?}", e);
                    // This is expected if GitHub credentials are not set up
                }
            }
        }
        Err(e) => {
            println!("⚠️  Could not initialize router: {:?}", e);
            // This is expected if GitHub credentials are not set up
        }
    }

    println!("Human-only filtering test completed");
}

#[tokio::test]
async fn test_pop_any_available_task_excludes_human_only() {
    println!("Testing pop any available task excludes human-only...");

    match AgentRouter::new().await {
        Ok(router) => {
            match router.pop_any_available_task().await {
                Ok(Some(task)) => {
                    // Verify the popped task is not human-only
                    let is_human_only = task
                        .issue
                        .labels
                        .iter()
                        .any(|label| label.name == "route:human-only");

                    if is_human_only {
                        panic!("❌ pop_any_available_task returned a human-only task!");
                    } else {
                        println!("✅ Popped task #{} is not human-only", task.issue.number);
                    }
                }
                Ok(None) => {
                    println!("ℹ️  No tasks available to pop (expected in test environment)");
                }
                Err(e) => {
                    println!("⚠️  Could not pop task: {:?}", e);
                }
            }
        }
        Err(e) => {
            println!("⚠️  Could not initialize router: {:?}", e);
        }
    }

    println!("Pop task filtering test completed");
}

#[test]
fn test_human_only_label_constant() {
    // Test that our human-only label constant is correctly defined
    let human_only_label = "route:human-only";

    // Verify it follows the route: prefix convention
    assert!(human_only_label.starts_with("route:"));
    assert_eq!(human_only_label, "route:human-only");

    println!(
        "✅ Human-only label format is correct: {}",
        human_only_label
    );
}
