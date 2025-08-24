// Clambake Library - Multi-Agent GitHub Orchestration
// This exposes the core components for testing and integration

pub mod agent_lifecycle;
pub mod agents;
pub mod autonomous;
pub mod bundling;
pub mod config;
pub mod database;
pub mod git;
pub mod github;
pub mod http;
pub mod metrics;
pub mod observability;
pub mod priority;
pub mod shutdown;
pub mod telemetry;
pub mod train_schedule;
pub mod workflows;

// Re-export key types for easy access
pub use agent_lifecycle::types::{AgentState, Command, GitCommand, GitHubCommand, PreFlightIssue};
pub use agents::integrator::{CompletedWork, IntegrationResult, WorkIntegrator};
pub use agents::{AgentCoordinator, AgentRouter};
pub use autonomous::integration::IntegrationCoordinator;
pub use autonomous::persistence::StatePersistenceManager;
pub use autonomous::{
    AutonomousCoordinator, AutonomousErrorRecovery, AutonomousEvent, AutonomousWorkflowMachine,
    AutonomousWorkflowState, CoordinationConfig,
};
pub use bundling::types::BundleWindow;
pub use bundling::{BundleManager, BundleResult};
pub use config::{config, init_config, MyLittleSodaConfig};
pub use database::{init_database, shutdown_database};
pub use git::operations::CommitInfo;
pub use git::{Git2Operations, GitOperations};
pub use github::{GitHubClient, GitHubError};
pub use http::RateLimitedHttpClient;
pub use metrics::{IntegrationAttempt, IntegrationOutcome, IntegrationPhase, MetricsTracker};
pub use observability::{create_workflow_span, github_metrics, GitHubApiMetrics, OperationTimer};
pub use priority::Priority;
pub use shutdown::ShutdownCoordinator;
pub use telemetry::{
    create_coordination_span, generate_correlation_id, init_telemetry, shutdown_telemetry,
};
pub use train_schedule::{QueuedBranch, ScheduleStatus, TrainSchedule};
pub use workflows::state_machine::{StateMachine, StateTransition, TransitionResult};
