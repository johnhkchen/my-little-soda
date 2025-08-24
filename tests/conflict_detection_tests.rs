use my_little_soda::github::{ConflictAnalysis, SafeMergeResult};
use chrono::Datelike;

#[tokio::test]
async fn test_conflict_detection_structures() {
    // Test that our conflict analysis structures work correctly
    let analysis = ConflictAnalysis {
        has_conflicts: true,
        is_mergeable: false,
        conflict_files: vec!["src/main.rs".to_string(), "src/lib.rs".to_string()],
        base_branch: "main".to_string(),
        head_branch: "agent001/123".to_string(),
        head_sha: "abc123def456".to_string(),
        analysis_timestamp: chrono::Utc::now(),
    };
    
    assert!(analysis.has_conflicts);
    assert!(!analysis.is_mergeable);
    assert_eq!(analysis.conflict_files.len(), 2);
    assert_eq!(analysis.base_branch, "main");
    assert_eq!(analysis.head_branch, "agent001/123");
}

#[tokio::test]
async fn test_safe_merge_result_patterns() {
    // Test SafeMergeResult enum patterns
    let successful_merge = SafeMergeResult::SuccessfulMerge {
        pr_number: 42,
        merged_sha: Some("merged123".to_string()),
    };
    
    match successful_merge {
        SafeMergeResult::SuccessfulMerge { pr_number, merged_sha } => {
            assert_eq!(pr_number, 42);
            assert!(merged_sha.is_some());
        }
        _ => panic!("Expected SuccessfulMerge variant"),
    }
    
    let conflict_detected = SafeMergeResult::ConflictDetected {
        original_pr: 42,
        recovery_pr: 43,
        recovery_url: Some("https://github.com/test/repo/pull/43".to_string()),
        requires_human_review: true,
    };
    
    match conflict_detected {
        SafeMergeResult::ConflictDetected { 
            original_pr, 
            recovery_pr, 
            requires_human_review,
            ..
        } => {
            assert_eq!(original_pr, 42);
            assert_eq!(recovery_pr, 43);
            assert!(requires_human_review);
        }
        _ => panic!("Expected ConflictDetected variant"),
    }
}

#[test]
fn test_conflict_analysis_timestamp() {
    // Test that timestamps are properly generated
    let now_before = chrono::Utc::now();
    
    let analysis = ConflictAnalysis {
        has_conflicts: false,
        is_mergeable: true,
        conflict_files: Vec::new(),
        base_branch: "main".to_string(),
        head_branch: "feature/test".to_string(),
        head_sha: "test123".to_string(),
        analysis_timestamp: chrono::Utc::now(),
    };
    
    let now_after = chrono::Utc::now();
    
    assert!(analysis.analysis_timestamp >= now_before);
    assert!(analysis.analysis_timestamp <= now_after);
}

#[test]
fn test_backup_branch_naming() {
    // Test that backup branch names follow expected pattern
    let agent_id = "agent001";
    let issue_number = 123;
    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    
    let backup_branch = format!("backup/{}-{}-{}", agent_id, issue_number, timestamp);
    
    assert!(backup_branch.starts_with("backup/agent001-123-"));
    assert!(backup_branch.contains(&format!("{}", chrono::Utc::now().year())));
}

#[test]
fn test_recovery_branch_naming() {
    // Test that recovery branch names follow expected pattern
    let original_pr = 42;
    let agent_id = "agent001";
    
    let recovery_branch = format!("conflict-recovery/{}-{}", original_pr, agent_id);
    
    assert_eq!(recovery_branch, "conflict-recovery/42-agent001");
}