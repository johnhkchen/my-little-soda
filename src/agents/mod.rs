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
// Unused integrator and recovery imports removed for code quality