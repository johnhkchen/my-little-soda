Testing library/framework used: Rust built-in test framework (cargo test), with optional Tokio #[tokio::test] for async if needed.

These tests focus on:
- Pure logic validation of priority calculation and semantics of routing filters in src/agents/router.rs.
- Integration-style validations that model issue metadata (labels, state, assignee) using octocrab types without invoking networked clients.

If crate::github::GitHubClient and crate::agents::AgentCoordinator support in-memory/test constructors, consider extending the colocated tests in src/agents/router.rs to exercise route_issues_to_agents and pop_* methods end-to-end by injecting test doubles and asserting side-effects like assignment and branch creation.