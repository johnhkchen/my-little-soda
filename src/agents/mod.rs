// Agent coordination modules for GitHub-native orchestration
// Following VERBOTEN rules: GitHub is source of truth, atomic operations, work preservation

pub mod coordinator;
pub mod integrator;
pub mod process_lifecycle;
pub mod process_manager;
pub mod recovery;
pub mod resource_monitor;
pub mod router;
pub mod routing;
pub mod validation;

pub use coordinator::{Agent, AgentCoordinator, AgentState};
pub use router::AgentRouter;
// Unused integrator and recovery imports removed for code quality
