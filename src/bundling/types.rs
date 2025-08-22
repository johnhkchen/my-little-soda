use chrono::{DateTime, Local, Timelike};
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
        let issues_str = sorted_issues.iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join("_");
            
        format!("bundle/{}__issues_{}", window_str, issues_str)
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
    Failed {
        error: anyhow::Error,
    },
}

/// Information about a bundle branch being created
#[derive(Debug, Clone)]
pub struct BundleBranch {
    pub name: String,
    pub base_branch: String,
    pub included_branches: Vec<String>,
    pub included_issues: Vec<u64>,
}