// Agent Lifecycle Module - Testable State Machine
// 
// This module implements the complete agent lifecycle with dependency injection
// for comprehensive testing and clear separation of concerns.

pub mod types;
pub mod traits;
pub mod detector;
pub mod state_machine;
pub mod executor;
pub mod commands;

#[cfg(test)]
pub mod mocks;

#[cfg(test)]  
pub mod tests;

// Unused imports removed for code quality
pub use state_machine::{AgentStateMachine, AgentEvent};