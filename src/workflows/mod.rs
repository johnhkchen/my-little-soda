// Workflow orchestration modules
// Following VERBOTEN rules: Atomic operations, GitHub source of truth

pub mod state_machine;

pub use state_machine::{StateMachine, StateTransition, TransitionResult};