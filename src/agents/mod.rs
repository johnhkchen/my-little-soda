// Agent coordination modules for GitHub-native orchestration
// Following VERBOTEN rules: GitHub is source of truth, atomic operations, work preservation

pub mod coordinator;
pub mod router;
pub mod integrator;
pub mod routing;
pub mod validation;
pub mod recovery;
pub mod process_manager;
pub mod resource_monitor;
pub mod process_lifecycle;

pub use coordinator::{AgentCoordinator, Agent, AgentState};
pub use router::AgentRouter;
pub use integrator::{WorkIntegrator, CompletedWork, IntegrationResult};
pub use validation::{StateValidator, StateValidation, ValidationReport, SystemValidationReport};
pub use recovery::{AutoRecovery, AutomaticRecovery, RecoveryAction, RecoveryAttempt, ComprehensiveRecoveryReport};
pub use process_manager::{AgentProcessManager, AgentProcess, ProcessStatus, ResourceLimits, ResourceUsage, ResourceSummary, ProcessManagerConfig};
pub use resource_monitor::{ResourceMonitor, Alert, AlertSeverity, ResourceAlert, AlertThresholds, ResourceUsageReport, ProcessResourceReport};
pub use process_lifecycle::{ProcessLifecycleManager, ProcessLifecycleConfig, LifecycleEvent, LifecycleEventHandler, LoggingEventHandler, SystemStatus};