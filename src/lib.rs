// Clambake Library - Multi-Agent GitHub Orchestration
// This exposes the core components for testing and integration

pub mod github;
pub mod agent_lifecycle;
pub mod agents;
pub mod workflows;
pub mod priority;
pub mod train_schedule;
pub mod telemetry;
pub mod metrics;
pub mod git;
pub mod bundling;
pub mod http;
pub mod observability;
pub mod config;
pub mod shutdown;
pub mod database;
pub mod autonomous;

// Re-export key types for easy access
pub use github::{GitHubClient, GitHubError};
pub use agents::{AgentCoordinator, AgentRouter, WorkIntegrator, CompletedWork, IntegrationResult};
pub use workflows::{StateMachine, StateTransition, TransitionResult};
pub use priority::Priority;
pub use train_schedule::{TrainSchedule, ScheduleStatus, QueuedBranch};
pub use telemetry::{init_telemetry, shutdown_telemetry, generate_correlation_id, create_coordination_span};
pub use agent_lifecycle::{AgentState, PreFlightIssue, Command, GitCommand, GitHubCommand};
pub use metrics::{MetricsTracker, IntegrationAttempt, IntegrationPhase, IntegrationOutcome};
pub use git::{GitOperations, Git2Operations, CommitInfo};
pub use bundling::{BundleManager, BundleResult, BundleWindow};
pub use http::{RateLimitedHttpClient};
pub use observability::{GitHubApiMetrics, github_metrics, create_workflow_span, OperationTimer};
pub use config::{MyLittleSodaConfig, config, init_config};
pub use shutdown::ShutdownCoordinator;
pub use database::{init_database, shutdown_database};
pub use autonomous::{
    AutonomousCoordinator, 
    AutonomousWorkflowMachine, 
    AutonomousWorkflowState, 
    AutonomousEvent,
    AutonomousErrorRecovery,
    StatePersistenceManager,
    IntegrationCoordinator,
    CoordinationConfig,
};