pub mod client;
pub mod issues;
pub mod pulls;
pub mod branches;
pub mod comments;
pub mod types;
pub mod errors;
// pub mod retry;

pub use client::{GitHubClient, GitHubOps};
pub use errors::GitHubError;
pub use types::{ConflictAnalysis, ConflictRecoveryData, SafeMergeResult};
pub use issues::IssueHandler;
pub use pulls::{PullRequestHandler, PullRequestStatus};
pub use branches::{BranchHandler, BranchInfo, BranchComparison};
pub use comments::CommentHandler;
// pub use retry::{GitHubRetryHandler, RetryConfig};