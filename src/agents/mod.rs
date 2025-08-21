// Agent coordination modules for GitHub-native orchestration
// Following VERBOTEN rules: GitHub is source of truth, atomic operations, work preservation

pub mod coordinator;
pub mod router;
pub mod integrator;

pub use coordinator::{AgentCoordinator, Agent, AgentState};
pub use router::AgentRouter;
pub use integrator::{WorkIntegrator, CompletedWork, IntegrationResult};