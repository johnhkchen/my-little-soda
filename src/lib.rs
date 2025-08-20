// Clambake Library - Multi-Agent GitHub Orchestration
// This exposes the core components for testing and integration

pub mod github;
pub mod agents;
pub mod workflows;

// Re-export key types for easy access
pub use github::{GitHubClient, GitHubError};
pub use agents::{AgentCoordinator, AgentRouter};
// Future exports: WorkIntegrator, CompletedWork, StateMachine, StateTransition