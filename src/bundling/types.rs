use chrono::{DateTime, Local, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a bundling time window (10-minute intervals)
#[derive(Debug, Clone)]
pub struct BundleWindow {
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
}

impl BundleWindow {
    /// Create a bundle window for the current departure time
    pub fn current() -> Self {
        let now = Local::now();
        let current_minute = now.minute();

        // Round down to nearest 10-minute mark
        let window_minute = (current_minute / 10) * 10;
        let start = now
            .with_minute(window_minute)
            .unwrap()
            .with_second(0)
            .unwrap()
            .with_nanosecond(0)
            .unwrap();
        let end = start + chrono::Duration::minutes(10);

        Self { start, end }
    }

    /// Generate deterministic bundle branch name for this window
    pub fn bundle_branch_name(&self, issues: &[u64]) -> String {
        let mut sorted_issues = issues.to_vec();
        sorted_issues.sort_unstable();

        let window_str = self.start.format("%Y%m%d_%H%M");
        let issues_str = sorted_issues
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join("_");

        format!("bundle/{window_str}__issues_{issues_str}")
    }
}

/// Result of a bundling operation
#[derive(Debug)]
pub enum BundleResult {
    /// Successful bundle with single PR
    Success {
        pr_number: u64,
        bundle_branch: String,
    },
    /// Conflicts detected, fell back to individual PRs
    ConflictFallback {
        individual_prs: HashMap<String, u64>, // branch_name -> pr_number
    },
    /// Bundle creation failed
    Failed { error: anyhow::Error },
}

/// Information about a bundle branch being created
#[derive(Debug, Clone)]
#[allow(dead_code)] // Architectural design - future bundling feature
pub struct BundleBranch {
    pub name: String,
    pub base_branch: String,
    pub included_branches: Vec<String>,
    pub included_issues: Vec<u64>,
}

/// Comprehensive error types for bundling operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BundleErrorType {
    GitOperation {
        operation: String,
        details: String,
    },
    GitHubApi {
        status_code: Option<u16>,
        message: String,
        retry_after: Option<u64>,
    },
    ConflictResolution {
        conflicted_files: Vec<String>,
        branches: Vec<String>,
    },
    NetworkTimeout {
        operation: String,
        duration_ms: u64,
    },
    PermissionDenied {
        resource: String,
        required_permission: String,
    },
    RateLimit {
        limit_type: String,
        reset_time: DateTime<Utc>,
        remaining: u32,
    },
    PartialFailure {
        completed_operations: Vec<String>,
        failed_operations: Vec<String>,
    },
}

/// Recovery strategy for failed bundling operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    Retry { max_attempts: u32, backoff_ms: u64 },
    Fallback { to_operation: String },
    Abort { cleanup_required: bool },
    Manual { instructions: String },
}

/// Audit trail entry for bundling operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleAuditEntry {
    pub timestamp: DateTime<Utc>,
    pub operation: String,
    pub branch_name: Option<String>,
    pub affected_issues: Vec<u64>,
    pub status: BundleOperationStatus,
    pub error: Option<BundleErrorType>,
    pub recovery_action: Option<RecoveryStrategy>,
    pub execution_time_ms: u64,
    pub correlation_id: String,
}

/// Status of a bundling operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BundleOperationStatus {
    Started,
    InProgress { progress_percent: u8 },
    Completed,
    Failed,
    Recovered,
    Aborted,
}

/// Bundling state for recovery purposes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleState {
    pub bundle_branch: String,
    pub target_branches: Vec<String>,
    pub completed_branches: Vec<String>,
    pub failed_branches: Vec<String>,
    pub current_operation: Option<String>,
    pub audit_trail: Vec<BundleAuditEntry>,
    pub recovery_data: Option<RecoveryData>,
}

/// Recovery data for resuming failed operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryData {
    pub last_successful_commit: Option<String>,
    pub cleanup_commands: Vec<String>,
    pub rollback_branch: Option<String>,
    pub temp_files: Vec<String>,
}
