//! Bundle management for Clambake
//! 
//! Implements deterministic bundling windows with proper Git operations and conflict handling.

pub mod bundler;
pub mod git_ops;
pub mod types;

pub use bundler::BundleManager;
pub use git_ops::{GitOperations, ConflictStrategy};
pub use types::{BundleWindow, BundleResult, BundleBranch};