//! Tests for src/agents/coordinator.rs
//! Testing library/framework: Rust built-in test framework with Tokio async runtime (#[tokio::test]).
//! These tests are integration tests that use the real GitHubClient.

use my_little_soda::agents::coordinator::{AgentCoordinator, AgentState};

#[tokio::test]
async fn coordinator_can_be_created() {
    // Basic smoke test to ensure the coordinator can be instantiated
    // This requires valid GitHub credentials in the environment
    if std::env::var("GITHUB_TOKEN").is_ok() {
        let _coord = AgentCoordinator::new().await.expect("coordinator new");
    } else {
        // Skip test if no GitHub token available
        println!("Skipping test - GITHUB_TOKEN not available");
    }
}

#[tokio::test]
async fn coordinator_agent_state_can_be_created() {
    // Test that AgentState enum variants can be created
    let _available = AgentState::Available;
    let _assigned = AgentState::Assigned("http://github.com/test/test/issues/1".to_string());
    let _working = AgentState::Working("http://github.com/test/test/issues/1".to_string());
}
