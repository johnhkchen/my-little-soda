//! Git operations module
//!
//! This module provides a trait-based interface for git operations,
//! replacing shell-based git commands with proper libgit2 bindings.

pub mod operations;

pub use operations::{Git2Operations, GitOperations};
