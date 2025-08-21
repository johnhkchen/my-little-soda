// Clambake Library - Multi-Agent GitHub Orchestration
// This exposes the core components for testing and integration

pub mod github;
pub mod agents;
pub mod workflows;
pub mod priority;
pub mod train_schedule;

// Re-export key types for easy access
pub use github::{GitHubClient, GitHubError};
pub use agents::{AgentCoordinator, AgentRouter, WorkIntegrator, CompletedWork, IntegrationResult};
pub use workflows::{StateMachine, StateTransition, TransitionResult};
pub use priority::Priority;
pub use train_schedule::{TrainSchedule, ScheduleStatus, QueuedBranch};