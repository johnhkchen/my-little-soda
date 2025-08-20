//! External tool abstractions
//! 
//! This module provides trait-based abstractions for external CLI tools like GitHub CLI
//! and Git, enabling testable code through dependency injection and mock implementations.
//! 
//! Following the white_magic.md patterns, this separates pure logic (decision making)
//! from impure effects (external command execution).

pub mod github;
pub mod git;
pub mod command;

pub use github::{GitHubOperations, GitHubClient, GitHubError};
pub use git::{GitRepository, GitClient, GitError};
pub use command::{CommandExecutor, ProcessCommandExecutor, CommandError, CommandOutput};