/// Analysis result for merge conflicts
#[derive(Debug, Clone)]
pub struct ConflictAnalysis {
    pub has_conflicts: bool,
    pub is_mergeable: bool,
    pub conflict_files: Vec<String>,
    pub base_branch: String,
    pub head_branch: String,
    pub head_sha: String,
    pub analysis_timestamp: chrono::DateTime<chrono::Utc>,
}

/// Data structure for conflict recovery operations
#[derive(Debug, Clone)]
pub struct ConflictRecoveryData {
    pub agent_id: String,
    pub issue_number: u64,
    pub original_pr_number: u64,
    pub conflict_analysis: ConflictAnalysis,
    pub backup_branch: String,
    pub recovery_timestamp: chrono::DateTime<chrono::Utc>,
}

/// Result of a safe merge operation
#[derive(Debug)]
pub enum SafeMergeResult {
    SuccessfulMerge {
        pr_number: u64,
        merged_sha: Option<String>,
    },
    ConflictDetected {
        original_pr: u64,
        recovery_pr: u64,
        recovery_url: Option<String>,
        requires_human_review: bool,
    },
    MergeFailed {
        error: String,
        recovery_pr: u64,
        work_preserved: bool,
    },
}