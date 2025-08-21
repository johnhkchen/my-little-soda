pub mod client;
// pub mod retry;

pub use client::{GitHubClient, GitHubError, ConflictAnalysis, ConflictRecoveryData, SafeMergeResult};
// pub use retry::{GitHubRetryHandler, RetryConfig};