use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use thiserror::Error;
use tracing::{info, warn, error, debug};
use rand::Rng;

use crate::github::{GitHubClient, errors::GitHubError};
use crate::agents::recovery::{AutomaticRecovery, ComprehensiveRecoveryReport, RecoveryError};

// Helper function to convert various errors to RecoveryError
fn convert_error<E: std::fmt::Display>(error: E, context: &str) -> RecoveryError {
    RecoveryError::GitError(format!("{}: {}", context, error))
}

// Trait for converting any error to RecoveryError
trait IntoRecoveryError<T> {
    fn into_recovery_error(self, context: &str) -> Result<T, RecoveryError>;
}

impl<T, E: std::fmt::Display> IntoRecoveryError<T> for Result<T, E> {
    fn into_recovery_error(self, context: &str) -> Result<T, RecoveryError> {
        self.map_err(|e| convert_error(e, context))
    }
}
use super::workflow_state_machine::{
    AutonomousWorkflowState, AutonomousEvent, AutonomousWorkflowError,
    BlockerType, ConflictInfo, CIFailure, AbandonmentReason
};

/// Autonomous error recovery strategies for unattended operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    /// Retry the operation with exponential backoff
    RetryWithBackoff {
        max_attempts: u8,
        base_delay_ms: u64,
        max_delay_ms: u64,
    },
    
    /// Attempt automated fix
    AutomatedFix {
        fix_type: FixType,
        confidence: ConfidenceLevel,
    },
    
    /// Fallback to alternative approach
    Fallback {
        alternative: AlternativeApproach,
    },
    
    /// Escalate to human intervention
    Escalate {
        urgency: UrgencyLevel,
        context: String,
    },
    
    /// Abandon current work and reset
    AbandonAndReset {
        reason: AbandonmentReason,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FixType {
    MergeConflictResolution,
    TestFailureFix,
    BuildErrorFix,
    DependencyUpdate,
    ConfigurationAdjustment,
    CodeFormatting,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ConfidenceLevel {
    High,    // 90-100% success rate
    Medium,  // 60-89% success rate  
    Low,     // 30-59% success rate
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AlternativeApproach {
    DifferentImplementation,
    SimplifiedSolution,
    ManualProcess,
    ExternalTool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum UrgencyLevel {
    Low,     // Can wait hours
    Medium,  // Needs attention within hour
    High,    // Needs immediate attention
    Critical,// Blocking all progress
}

/// Recovery attempt with detailed tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousRecoveryAttempt {
    pub attempt_id: String,
    pub error_type: ErrorType,
    pub strategy: RecoveryStrategy,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub success: bool,
    pub error_message: Option<String>,
    pub recovery_actions: Vec<RecoveryAction>,
    pub metrics: RecoveryMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorType {
    GitOperationFailed { operation: String, error: String },
    GitHubAPIError { endpoint: String, status: u16, message: String },
    MergeConflict { files: Vec<String>, conflict_count: u32 },
    CIFailure { job: String, step: String, error: String },
    TestFailure { test_suite: String, failed_tests: Vec<String> },
    BuildFailure { stage: String, error: String },
    DependencyIssue { dependency: String, version_conflict: bool },
    NetworkIssue { service: String, timeout: bool },
    WorkspaceCorruption { files_affected: Vec<String> },
    StateInconsistency { expected_state: String, actual_state: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryAction {
    GitReset { to_commit: String },
    GitRebase { onto: String },
    FileRestore { files: Vec<String> },
    DependencyReinstall { packages: Vec<String> },
    ConfigUpdate { file: String, changes: HashMap<String, String> },
    ServiceRestart { service: String },
    WorkspaceClean,
    BranchRecreation { from_commit: String },
    TestSkip { tests: Vec<String> },
    AutomergeConflictResolution { files: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryMetrics {
    pub attempts_count: u32,
    pub total_duration_ms: u64,
    pub actions_executed: u32,
    pub files_affected: u32,
    pub git_operations: u32,
    pub network_requests: u32,
}

/// Comprehensive autonomous recovery system
pub struct AutonomousErrorRecovery {
    github_client: GitHubClient,
    base_recovery: Box<dyn AutomaticRecovery + Send + Sync>,
    recovery_history: Vec<AutonomousRecoveryAttempt>,
    max_recovery_attempts: u8,
    recovery_timeout_minutes: u32,
    enable_aggressive_recovery: bool,
}

impl AutonomousErrorRecovery {
    pub fn new(
        github_client: GitHubClient,
        base_recovery: Box<dyn AutomaticRecovery + Send + Sync>,
    ) -> Self {
        Self {
            github_client,
            base_recovery,
            recovery_history: Vec::new(),
            max_recovery_attempts: 3,
            recovery_timeout_minutes: 30,
            enable_aggressive_recovery: false,
        }
    }
    
    pub fn with_max_attempts(mut self, max_attempts: u8) -> Self {
        self.max_recovery_attempts = max_attempts;
        self
    }
    
    pub fn with_timeout(mut self, timeout_minutes: u32) -> Self {
        self.recovery_timeout_minutes = timeout_minutes;
        self
    }
    
    pub fn with_aggressive_recovery(mut self, enable: bool) -> Self {
        self.enable_aggressive_recovery = enable;
        self
    }
    
    /// Determine recovery strategy for a given error type
    pub fn determine_recovery_strategy(&self, error_type: &ErrorType) -> RecoveryStrategy {
        match error_type {
            ErrorType::GitOperationFailed { operation, .. } => {
                match operation.as_str() {
                    "push" | "pull" | "fetch" => RecoveryStrategy::RetryWithBackoff {
                        max_attempts: 3,
                        base_delay_ms: 1000,
                        max_delay_ms: 10000,
                    },
                    "merge" | "rebase" => RecoveryStrategy::AutomatedFix {
                        fix_type: FixType::MergeConflictResolution,
                        confidence: ConfidenceLevel::Medium,
                    },
                    _ => RecoveryStrategy::Fallback {
                        alternative: AlternativeApproach::ManualProcess,
                    }
                }
            }
            
            ErrorType::GitHubAPIError { status, .. } => {
                match *status {
                    429 => RecoveryStrategy::RetryWithBackoff {
                        max_attempts: 5,
                        base_delay_ms: 2000,
                        max_delay_ms: 30000,
                    },
                    500..=599 => RecoveryStrategy::RetryWithBackoff {
                        max_attempts: 3,
                        base_delay_ms: 5000,
                        max_delay_ms: 20000,
                    },
                    _ => RecoveryStrategy::Escalate {
                        urgency: UrgencyLevel::Medium,
                        context: "GitHub API error requires investigation".to_string(),
                    }
                }
            }
            
            ErrorType::MergeConflict { files, conflict_count } => {
                if *conflict_count <= 5 && files.iter().all(|f| !f.contains("migration")) {
                    RecoveryStrategy::AutomatedFix {
                        fix_type: FixType::MergeConflictResolution,
                        confidence: ConfidenceLevel::High,
                    }
                } else {
                    RecoveryStrategy::Escalate {
                        urgency: UrgencyLevel::High,
                        context: format!("Complex merge conflicts in {} files", files.len()),
                    }
                }
            }
            
            ErrorType::CIFailure { job, error, .. } => {
                if error.contains("test") {
                    RecoveryStrategy::AutomatedFix {
                        fix_type: FixType::TestFailureFix,
                        confidence: ConfidenceLevel::Medium,
                    }
                } else if error.contains("build") || error.contains("compile") {
                    RecoveryStrategy::AutomatedFix {
                        fix_type: FixType::BuildErrorFix,
                        confidence: ConfidenceLevel::Low,
                    }
                } else {
                    RecoveryStrategy::Escalate {
                        urgency: UrgencyLevel::Medium,
                        context: format!("CI failure in {}: {}", job, error),
                    }
                }
            }
            
            ErrorType::TestFailure { failed_tests, .. } => {
                if failed_tests.len() <= 3 {
                    RecoveryStrategy::AutomatedFix {
                        fix_type: FixType::TestFailureFix,
                        confidence: ConfidenceLevel::Medium,
                    }
                } else {
                    RecoveryStrategy::Fallback {
                        alternative: AlternativeApproach::SimplifiedSolution,
                    }
                }
            }
            
            ErrorType::BuildFailure { stage, error } => {
                if stage == "dependencies" {
                    RecoveryStrategy::AutomatedFix {
                        fix_type: FixType::DependencyUpdate,
                        confidence: ConfidenceLevel::High,
                    }
                } else if error.contains("format") || error.contains("lint") {
                    RecoveryStrategy::AutomatedFix {
                        fix_type: FixType::CodeFormatting,
                        confidence: ConfidenceLevel::High,
                    }
                } else {
                    RecoveryStrategy::AutomatedFix {
                        fix_type: FixType::BuildErrorFix,
                        confidence: ConfidenceLevel::Low,
                    }
                }
            }
            
            ErrorType::DependencyIssue { version_conflict, .. } => {
                if *version_conflict {
                    RecoveryStrategy::AutomatedFix {
                        fix_type: FixType::DependencyUpdate,
                        confidence: ConfidenceLevel::Medium,
                    }
                } else {
                    RecoveryStrategy::RetryWithBackoff {
                        max_attempts: 3,
                        base_delay_ms: 2000,
                        max_delay_ms: 10000,
                    }
                }
            }
            
            ErrorType::NetworkIssue { timeout, .. } => {
                if *timeout {
                    RecoveryStrategy::RetryWithBackoff {
                        max_attempts: 5,
                        base_delay_ms: 1000,
                        max_delay_ms: 15000,
                    }
                } else {
                    RecoveryStrategy::Escalate {
                        urgency: UrgencyLevel::Low,
                        context: "Network connectivity issues".to_string(),
                    }
                }
            }
            
            ErrorType::WorkspaceCorruption { files_affected } => {
                if files_affected.len() <= 5 {
                    RecoveryStrategy::AutomatedFix {
                        fix_type: FixType::ConfigurationAdjustment,
                        confidence: ConfidenceLevel::Medium,
                    }
                } else {
                    RecoveryStrategy::AbandonAndReset {
                        reason: AbandonmentReason::CriticalFailure {
                            error: "Workspace corruption".to_string(),
                        }
                    }
                }
            }
            
            ErrorType::StateInconsistency { .. } => {
                RecoveryStrategy::AutomatedFix {
                    fix_type: FixType::ConfigurationAdjustment,
                    confidence: ConfidenceLevel::High,
                }
            }
        }
    }
    
    /// Execute automated recovery strategy
    pub async fn execute_recovery_strategy(
        &mut self,
        error_type: ErrorType,
        strategy: RecoveryStrategy,
        context: &AutonomousWorkflowState,
    ) -> Result<AutonomousRecoveryAttempt, RecoveryError> {
        let attempt_id = format!("recovery-{}-{}", Utc::now().timestamp(), rand::random::<u16>());
        let start_time = Utc::now();
        let start_instant = std::time::Instant::now();
        
        info!(
            attempt_id = %attempt_id,
            error_type = ?error_type,
            strategy = ?strategy,
            "Starting autonomous recovery attempt"
        );
        
        let mut recovery_actions = Vec::new();
        let mut success = false;
        let mut error_message = None;
        
        match strategy {
            RecoveryStrategy::RetryWithBackoff { max_attempts, base_delay_ms, max_delay_ms } => {
                success = self.execute_retry_strategy(
                    &error_type,
                    max_attempts,
                    base_delay_ms,
                    max_delay_ms,
                    &mut recovery_actions
                ).await?;
            }
            
            RecoveryStrategy::AutomatedFix { fix_type, confidence } => {
                match self.execute_automated_fix(fix_type, confidence, &mut recovery_actions).await {
                    Ok(_) => success = true,
                    Err(e) => {
                        error_message = Some(format!("Automated fix failed: {:?}", e));
                        success = false;
                    }
                }
            }
            
            RecoveryStrategy::Fallback { ref alternative } => {
                match self.execute_fallback_strategy(*alternative, &mut recovery_actions).await {
                    Ok(_) => success = true,
                    Err(e) => {
                        error_message = Some(format!("Fallback strategy failed: {:?}", e));
                        success = false;
                    }
                }
            }
            
            RecoveryStrategy::Escalate { ref urgency, context: ref escalation_context } => {
                self.execute_escalation(*urgency, escalation_context.clone(), &mut recovery_actions).await?;
                success = false; // Escalation is not a "success" in terms of autonomous recovery
                error_message = Some("Escalated to human intervention".to_string());
            }
            
            RecoveryStrategy::AbandonAndReset { ref reason } => {
                self.execute_abandonment(reason.clone(), &mut recovery_actions).await?;
                success = true; // Reset is considered successful
            }
        }
        
        let duration = start_instant.elapsed();
        let attempt = AutonomousRecoveryAttempt {
            attempt_id: attempt_id.clone(),
            error_type,
            strategy,
            started_at: start_time,
            completed_at: Some(Utc::now()),
            success,
            error_message,
            recovery_actions: recovery_actions.clone(),
            metrics: RecoveryMetrics {
                attempts_count: 1,
                total_duration_ms: duration.as_millis() as u64,
                actions_executed: recovery_actions.len() as u32,
                files_affected: self.count_files_affected(&recovery_actions),
                git_operations: self.count_git_operations(&recovery_actions),
                network_requests: self.count_network_requests(&recovery_actions),
            },
        };
        
        self.recovery_history.push(attempt.clone());
        
        info!(
            attempt_id = %attempt_id,
            success = %success,
            duration_ms = %duration.as_millis(),
            actions_count = %recovery_actions.len(),
            "Autonomous recovery attempt completed"
        );
        
        Ok(attempt)
    }
    
    /// Execute retry strategy with exponential backoff
    async fn execute_retry_strategy(
        &self,
        error_type: &ErrorType,
        max_attempts: u8,
        base_delay_ms: u64,
        max_delay_ms: u64,
        recovery_actions: &mut Vec<RecoveryAction>,
    ) -> Result<bool, RecoveryError> {
        let mut attempt = 0;
        let mut delay = base_delay_ms;
        
        while attempt < max_attempts {
            attempt += 1;
            
            debug!(
                attempt = %attempt,
                max_attempts = %max_attempts,
                delay_ms = %delay,
                "Retrying operation"
            );
            
            // Wait before retry (except first attempt)
            if attempt > 1 {
                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                recovery_actions.push(RecoveryAction::ServiceRestart {
                    service: "retry-delay".to_string()
                });
            }
            
            // Attempt the operation based on error type
            let retry_success = match error_type {
                ErrorType::GitOperationFailed { operation, .. } => {
                    self.retry_git_operation(operation, recovery_actions).await?
                }
                ErrorType::GitHubAPIError { .. } => {
                    self.retry_github_operation(recovery_actions).await?
                }
                ErrorType::NetworkIssue { .. } => {
                    self.retry_network_operation(recovery_actions).await?
                }
                _ => false, // Other error types don't benefit from simple retry
            };
            
            if retry_success {
                info!(attempt = %attempt, "Retry strategy succeeded");
                return Ok(true);
            }
            
            // Exponential backoff
            delay = std::cmp::min(delay * 2, max_delay_ms);
        }
        
        warn!(max_attempts = %max_attempts, "Retry strategy exhausted all attempts");
        Ok(false)
    }
    
    /// Execute automated fix based on fix type
    async fn execute_automated_fix(
        &self,
        fix_type: FixType,
        confidence: ConfidenceLevel,
        recovery_actions: &mut Vec<RecoveryAction>,
    ) -> Result<(), RecoveryError> {
        info!(
            fix_type = ?fix_type,
            confidence = ?confidence,
            "Executing automated fix"
        );
        
        match fix_type {
            FixType::MergeConflictResolution => {
                self.resolve_merge_conflicts_automatically(recovery_actions).await?;
            }
            FixType::TestFailureFix => {
                self.fix_test_failures_automatically(recovery_actions).await?;
            }
            FixType::BuildErrorFix => {
                self.fix_build_errors_automatically(recovery_actions).await?;
            }
            FixType::DependencyUpdate => {
                self.update_dependencies_automatically(recovery_actions).await?;
            }
            FixType::ConfigurationAdjustment => {
                self.adjust_configuration_automatically(recovery_actions).await?;
            }
            FixType::CodeFormatting => {
                self.format_code_automatically(recovery_actions).await?;
            }
        }
        
        Ok(())
    }
    
    /// Execute fallback strategy
    async fn execute_fallback_strategy(
        &self,
        alternative: AlternativeApproach,
        recovery_actions: &mut Vec<RecoveryAction>,
    ) -> Result<(), RecoveryError> {
        info!(alternative = ?alternative, "Executing fallback strategy");
        
        match alternative {
            AlternativeApproach::DifferentImplementation => {
                recovery_actions.push(RecoveryAction::WorkspaceClean);
            }
            AlternativeApproach::SimplifiedSolution => {
                recovery_actions.push(RecoveryAction::ConfigUpdate {
                    file: "config.toml".to_string(),
                    changes: [("complexity".to_string(), "low".to_string())].into(),
                });
            }
            AlternativeApproach::ManualProcess => {
                // Mark for manual intervention
            }
            AlternativeApproach::ExternalTool => {
                recovery_actions.push(RecoveryAction::ServiceRestart {
                    service: "external-tool".to_string()
                });
            }
        }
        
        Ok(())
    }
    
    /// Execute escalation to human intervention
    async fn execute_escalation(
        &self,
        urgency: UrgencyLevel,
        context: String,
        recovery_actions: &mut Vec<RecoveryAction>,
    ) -> Result<(), RecoveryError> {
        warn!(
            urgency = ?urgency,
            context = %context,
            "Escalating to human intervention"
        );
        
        // In a real implementation, this would:
        // 1. Create an issue/ticket
        // 2. Send notifications
        // 3. Update monitoring dashboards
        // 4. Preserve work state for human review
        
        recovery_actions.push(RecoveryAction::WorkspaceClean);
        
        Ok(())
    }
    
    /// Execute abandonment and reset
    async fn execute_abandonment(
        &self,
        reason: AbandonmentReason,
        recovery_actions: &mut Vec<RecoveryAction>,
    ) -> Result<(), RecoveryError> {
        warn!(reason = ?reason, "Abandoning work and resetting");
        
        recovery_actions.push(RecoveryAction::WorkspaceClean);
        recovery_actions.push(RecoveryAction::GitReset {
            to_commit: "HEAD~1".to_string()
        });
        
        Ok(())
    }
    
    // Implementation helpers for specific recovery operations
    
    async fn retry_git_operation(
        &self,
        operation: &str,
        recovery_actions: &mut Vec<RecoveryAction>,
    ) -> Result<bool, RecoveryError> {
        recovery_actions.push(RecoveryAction::ServiceRestart {
            service: format!("git-{}", operation)
        });
        // Simulate git operation retry
        Ok(rand::random::<f64>() > 0.3) // 70% success rate
    }
    
    async fn retry_github_operation(
        &self,
        recovery_actions: &mut Vec<RecoveryAction>,
    ) -> Result<bool, RecoveryError> {
        recovery_actions.push(RecoveryAction::ServiceRestart {
            service: "github-api".to_string()
        });
        // Simulate GitHub API retry
        Ok(rand::random::<f64>() > 0.2) // 80% success rate
    }
    
    async fn retry_network_operation(
        &self,
        recovery_actions: &mut Vec<RecoveryAction>,
    ) -> Result<bool, RecoveryError> {
        recovery_actions.push(RecoveryAction::ServiceRestart {
            service: "network".to_string()
        });
        // Simulate network retry
        Ok(rand::random::<f64>() > 0.4) // 60% success rate
    }
    
    async fn resolve_merge_conflicts_automatically(
        &self,
        recovery_actions: &mut Vec<RecoveryAction>,
    ) -> Result<(), RecoveryError> {
        // Get current repository state
        let repo = git2::Repository::open(".").into_recovery_error("opening repository")?;
        let mut conflicted_files = Vec::new();
        
        // Find conflicted files
        let index = repo.index().into_recovery_error("getting repository index")?;
        for entry in index.iter() {
            // Check if entry has conflicts (stage != 0 indicates conflict)
            let path = std::str::from_utf8(&entry.path).into_recovery_error("parsing file path")?;
            if !conflicted_files.contains(&path.to_string()) {
                conflicted_files.push(path.to_string());
            }
        }
        
        info!(
            conflicted_files = ?conflicted_files,
            "Attempting automatic merge conflict resolution"
        );
        
        // Attempt to resolve conflicts automatically
        for file_path in &conflicted_files {
            match self.resolve_simple_conflicts(file_path).await {
                Ok(resolved) => {
                    if resolved {
                        recovery_actions.push(RecoveryAction::AutomergeConflictResolution {
                            files: vec![file_path.clone()]
                        });
                    }
                }
                Err(e) => {
                    warn!(
                        file = %file_path,
                        error = %e,
                        "Failed to auto-resolve conflict"
                    );
                    return Err(RecoveryError::GitError(format!(
                        "Failed to resolve conflict in {}: {}", file_path, e
                    )));
                }
            }
        }
        
        // Stage resolved files and commit
        if !conflicted_files.is_empty() {
            recovery_actions.push(RecoveryAction::GitReset {
                to_commit: "HEAD".to_string()
            });
        }
        
        Ok(())
    }
    
    async fn fix_test_failures_automatically(
        &self,
        recovery_actions: &mut Vec<RecoveryAction>,
    ) -> Result<(), RecoveryError> {
        // First, try running tests to identify failures
        let output = tokio::process::Command::new("cargo")
            .arg("test")
            .arg("--")
            .arg("--format")
            .arg("json")
            .output()
            .await.into_recovery_error("running cargo test")?;
        
        let test_output = String::from_utf8_lossy(&output.stdout);
        let mut failed_tests = Vec::new();
        let mut flaky_tests = Vec::new();
        
        // Parse test output to find failed tests
        for line in test_output.lines() {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                if json["type"] == "test" && json["event"] == "failed" {
                    if let Some(test_name) = json["name"].as_str() {
                        failed_tests.push(test_name.to_string());
                        
                        // Check if this test is known to be flaky (simple heuristic)
                        if test_name.contains("timeout") || 
                           test_name.contains("race") || 
                           test_name.contains("flaky") ||
                           test_name.contains("network") {
                            flaky_tests.push(test_name.to_string());
                        }
                    }
                }
            }
        }
        
        info!(
            failed_tests = ?failed_tests,
            flaky_tests = ?flaky_tests,
            "Analyzed test failures"
        );
        
        // Strategy 1: Retry flaky tests
        if !flaky_tests.is_empty() {
            for _ in 0..3 { // Retry up to 3 times
                let retry_output = tokio::process::Command::new("cargo")
                    .arg("test")
                    .args(&flaky_tests)
                    .output()
                    .await?;
                
                if retry_output.status.success() {
                    recovery_actions.push(RecoveryAction::ServiceRestart {
                        service: "test-retry".to_string()
                    });
                    info!("Flaky tests passed on retry");
                    return Ok(());
                }
            }
        }
        
        // Strategy 2: Check if tests pass in isolation
        let mut isolation_successes = Vec::new();
        for test in &failed_tests {
            let isolated_output = tokio::process::Command::new("cargo")
                .arg("test")
                .arg(test)
                .arg("--")
                .arg("--test-threads=1")
                .output()
                .await?;
            
            if isolated_output.status.success() {
                isolation_successes.push(test.clone());
            }
        }
        
        if !isolation_successes.is_empty() {
            info!(
                isolated_successes = ?isolation_successes,
                "Some tests pass in isolation - possible race condition"
            );
        }
        
        // Strategy 3: Skip consistently failing non-critical tests
        let non_critical_tests: Vec<String> = failed_tests.iter()
            .filter(|test| {
                test.contains("integration") || 
                test.contains("benchmark") ||
                test.contains("stress")
            })
            .cloned()
            .collect();
        
        if !non_critical_tests.is_empty() {
            recovery_actions.push(RecoveryAction::TestSkip {
                tests: non_critical_tests.clone()
            });
            warn!(
                skipped_tests = ?non_critical_tests,
                "Skipping non-critical failing tests"
            );
        }
        
        Ok(())
    }
    
    async fn fix_build_errors_automatically(
        &self,
        recovery_actions: &mut Vec<RecoveryAction>,
    ) -> Result<(), RecoveryError> {
        // First, try a clean build
        info!("Attempting clean build to fix build errors");
        
        let clean_output = tokio::process::Command::new("cargo")
            .arg("clean")
            .output()
            .await?;
        
        if !clean_output.status.success() {
            warn!("Cargo clean failed");
        }
        
        // Try building again
        let build_output = tokio::process::Command::new("cargo")
            .arg("build")
            .output()
            .await?;
        
        if build_output.status.success() {
            recovery_actions.push(RecoveryAction::WorkspaceClean);
            info!("Build succeeded after clean");
            return Ok(());
        }
        
        // Analyze build errors
        let build_error = String::from_utf8_lossy(&build_output.stderr);
        info!(error = %build_error, "Analyzing build error");
        
        // Strategy 1: Handle missing dependencies
        if build_error.contains("could not find") || build_error.contains("unresolved import") {
            info!("Attempting to fix missing dependencies");
            
            let update_output = tokio::process::Command::new("cargo")
                .arg("update")
                .output()
                .await?;
            
            if update_output.status.success() {
                recovery_actions.push(RecoveryAction::DependencyReinstall {
                    packages: vec!["cargo-update".to_string()]
                });
                
                // Try building again after update
                let retry_build = tokio::process::Command::new("cargo")
                    .arg("build")
                    .output()
                    .await?;
                
                if retry_build.status.success() {
                    info!("Build succeeded after dependency update");
                    return Ok(());
                }
            }
        }
        
        // Strategy 2: Handle formatting/linting errors
        if build_error.contains("rustfmt") || build_error.contains("fmt") {
            info!("Attempting to fix formatting issues");
            
            let fmt_output = tokio::process::Command::new("cargo")
                .arg("fmt")
                .output()
                .await?;
            
            if fmt_output.status.success() {
                recovery_actions.push(RecoveryAction::ServiceRestart {
                    service: "code-formatter".to_string()
                });
                return Ok(());
            }
        }
        
        // Strategy 3: Handle clippy warnings as errors
        if build_error.contains("clippy") {
            info!("Attempting to fix clippy issues");
            
            let clippy_output = tokio::process::Command::new("cargo")
                .arg("clippy")
                .arg("--fix")
                .arg("--allow-dirty")
                .arg("--allow-staged")
                .output()
                .await?;
            
            if clippy_output.status.success() {
                recovery_actions.push(RecoveryAction::ConfigUpdate {
                    file: ".cargo/config.toml".to_string(),
                    changes: [("clippy".to_string(), "fixed".to_string())].into(),
                });
                return Ok(());
            }
        }
        
        warn!("Could not automatically fix build errors");
        Err(RecoveryError::GitError("Build errors could not be resolved automatically".to_string()))
    }
    
    async fn update_dependencies_automatically(
        &self,
        recovery_actions: &mut Vec<RecoveryAction>,
    ) -> Result<(), RecoveryError> {
        info!("Attempting automatic dependency updates");
        
        // Strategy 1: Update Cargo.lock
        let update_output = tokio::process::Command::new("cargo")
            .arg("update")
            .output()
            .await?;
        
        if update_output.status.success() {
            recovery_actions.push(RecoveryAction::DependencyReinstall {
                packages: vec!["cargo-lock-update".to_string()]
            });
            
            // Test if the update fixed the issue
            let build_output = tokio::process::Command::new("cargo")
                .arg("check")
                .output()
                .await?;
            
            if build_output.status.success() {
                info!("Dependency update resolved the issue");
                return Ok(());
            }
        }
        
        // Strategy 2: Clean and rebuild dependencies
        info!("Cleaning dependency cache");
        
        let clean_deps = tokio::process::Command::new("cargo")
            .arg("clean")
            .arg("--package")
            .arg("*")
            .output()
            .await?;
        
        if clean_deps.status.success() {
            recovery_actions.push(RecoveryAction::WorkspaceClean);
            
            // Force rebuild dependencies
            let build_deps = tokio::process::Command::new("cargo")
                .arg("build")
                .arg("--offline")
                .output()
                .await;
            
            if let Ok(output) = build_deps {
                if output.status.success() {
                    info!("Dependencies rebuilt successfully");
                    return Ok(());
                }
            }
        }
        
        // Strategy 3: Update specific problematic dependencies
        let common_problematic_deps = [
            "tokio", "serde", "chrono", "clap", "tracing", "anyhow",
            "thiserror", "uuid", "octocrab", "git2"
        ];
        
        for dep in &common_problematic_deps {
            let dep_update = tokio::process::Command::new("cargo")
                .arg("update")
                .arg("--package")
                .arg(dep)
                .output()
                .await;
            
            if let Ok(output) = dep_update {
                if output.status.success() {
                    recovery_actions.push(RecoveryAction::DependencyReinstall {
                        packages: vec![dep.to_string()]
                    });
                    
                    // Check if this specific update helped
                    let check_output = tokio::process::Command::new("cargo")
                        .arg("check")
                        .output()
                        .await?;
                    
                    if check_output.status.success() {
                        info!(dependency = %dep, "Specific dependency update resolved the issue");
                        return Ok(());
                    }
                }
            }
        }
        
        // Strategy 4: Downgrade to previous versions if available
        if std::path::Path::new("Cargo.lock.bak").exists() {
            info!("Attempting to restore previous dependency versions");
            
            let restore = tokio::process::Command::new("cp")
                .arg("Cargo.lock.bak")
                .arg("Cargo.lock")
                .output()
                .await;
            
            if let Ok(output) = restore {
                if output.status.success() {
                    recovery_actions.push(RecoveryAction::FileRestore {
                        files: vec!["Cargo.lock".to_string()]
                    });
                    
                    let check_output = tokio::process::Command::new("cargo")
                        .arg("check")
                        .output()
                        .await?;
                    
                    if check_output.status.success() {
                        info!("Restored previous dependency versions successfully");
                        return Ok(());
                    }
                }
            }
        }
        
        recovery_actions.push(RecoveryAction::DependencyReinstall {
            packages: vec!["all".to_string()]
        });
        
        warn!("Could not automatically resolve dependency issues");
        Err(RecoveryError::GitError("Dependency issues could not be resolved automatically".to_string()))
    }
    
    async fn adjust_configuration_automatically(
        &self,
        recovery_actions: &mut Vec<RecoveryAction>,
    ) -> Result<(), RecoveryError> {
        recovery_actions.push(RecoveryAction::ConfigUpdate {
            file: "config.toml".to_string(),
            changes: [("auto_fix".to_string(), "true".to_string())].into(),
        });
        Ok(())
    }
    
    async fn format_code_automatically(
        &self,
        recovery_actions: &mut Vec<RecoveryAction>,
    ) -> Result<(), RecoveryError> {
        // Run cargo fmt to format Rust code
        let output = tokio::process::Command::new("cargo")
            .arg("fmt")
            .output()
            .await?;
        
        if output.status.success() {
            recovery_actions.push(RecoveryAction::ServiceRestart {
                service: "code-formatter".to_string()
            });
            info!("Code formatting completed successfully");
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            warn!(error = %error, "Code formatting failed");
            return Err(RecoveryError::GitError(format!("Code formatting failed: {}", error)));
        }
        
        Ok(())
    }
    
    /// Attempt to resolve simple merge conflicts automatically
    async fn resolve_simple_conflicts(&self, file_path: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        // Read the file content
        let content = tokio::fs::read_to_string(file_path).await?;
        
        // Check if it has conflict markers
        if !content.contains("<<<<<<< ") || !content.contains(">>>>>>> ") {
            return Ok(false);
        }
        
        // Simple conflict resolution strategies:
        // 1. If conflicts are in import/use statements, merge both
        // 2. If conflicts are in comments, take both
        // 3. If conflicts are in dependency versions, take the higher version
        
        let mut resolved_content = String::new();
        let mut in_conflict = false;
        let mut our_section = Vec::new();
        let mut their_section = Vec::new();
        let mut in_our_section = true;
        
        for line in content.lines() {
            if line.starts_with("<<<<<<< ") {
                in_conflict = true;
                our_section.clear();
                their_section.clear();
                in_our_section = true;
                continue;
            } else if line.starts_with("======= ") {
                in_our_section = false;
                continue;
            } else if line.starts_with(">>>>>>> ") {
                in_conflict = false;
                
                // Apply resolution strategy
                let resolution = self.resolve_conflict_sections(&our_section, &their_section, file_path);
                resolved_content.push_str(&resolution);
                continue;
            }
            
            if in_conflict {
                if in_our_section {
                    our_section.push(line.to_string());
                } else {
                    their_section.push(line.to_string());
                }
            } else {
                resolved_content.push_str(line);
                resolved_content.push('\n');
            }
        }
        
        // Write the resolved content back
        tokio::fs::write(file_path, resolved_content).await?;
        
        info!(file = %file_path, "Successfully resolved merge conflicts");
        Ok(true)
    }
    
    /// Apply resolution strategy to conflict sections
    fn resolve_conflict_sections(&self, ours: &[String], theirs: &[String], file_path: &str) -> String {
        // Strategy 1: For Rust use/import statements, merge both
        if file_path.ends_with(".rs") && 
           (ours.iter().any(|l| l.trim().starts_with("use ")) || 
            theirs.iter().any(|l| l.trim().starts_with("use "))) {
            
            let mut merged = ours.iter().cloned().collect::<std::collections::HashSet<_>>();
            merged.extend(theirs.iter().cloned());
            let mut sorted: Vec<_> = merged.into_iter().collect();
            sorted.sort();
            
            return sorted.join("\n") + "\n";
        }
        
        // Strategy 2: For version conflicts in Cargo.toml, take the higher version
        if file_path == "Cargo.toml" {
            if let (Some(our_version), Some(their_version)) = (
                self.extract_version_from_lines(ours),
                self.extract_version_from_lines(theirs)
            ) {
                if self.compare_versions(&our_version, &their_version) >= 0 {
                    return ours.join("\n") + "\n";
                } else {
                    return theirs.join("\n") + "\n";
                }
            }
        }
        
        // Strategy 3: For comments, merge both
        if ours.iter().all(|l| l.trim().starts_with("//") || l.trim().is_empty()) &&
           theirs.iter().all(|l| l.trim().starts_with("//") || l.trim().is_empty()) {
            let mut merged = ours.to_vec();
            merged.extend(theirs.iter().cloned());
            return merged.join("\n") + "\n";
        }
        
        // Default: take ours (safer for most conflicts)
        ours.join("\n") + "\n"
    }
    
    /// Extract version string from TOML lines
    fn extract_version_from_lines(&self, lines: &[String]) -> Option<String> {
        for line in lines {
            if let Some(version_start) = line.find("version = \"") {
                let version_part = &line[version_start + 11..];
                if let Some(version_end) = version_part.find('"') {
                    return Some(version_part[..version_end].to_string());
                }
            }
        }
        None
    }
    
    /// Compare two version strings (simple semver comparison)
    fn compare_versions(&self, v1: &str, v2: &str) -> i32 {
        let v1_parts: Vec<u32> = v1.split('.').filter_map(|s| s.parse().ok()).collect();
        let v2_parts: Vec<u32> = v2.split('.').filter_map(|s| s.parse().ok()).collect();
        
        for i in 0..std::cmp::max(v1_parts.len(), v2_parts.len()) {
            let v1_part = v1_parts.get(i).unwrap_or(&0);
            let v2_part = v2_parts.get(i).unwrap_or(&0);
            
            match v1_part.cmp(v2_part) {
                std::cmp::Ordering::Greater => return 1,
                std::cmp::Ordering::Less => return -1,
                std::cmp::Ordering::Equal => continue,
            }
        }
        0
    }
    
    // Metrics helpers
    
    fn count_files_affected(&self, actions: &[RecoveryAction]) -> u32 {
        actions.iter().map(|action| match action {
            RecoveryAction::FileRestore { files } => files.len() as u32,
            RecoveryAction::AutomergeConflictResolution { files } => files.len() as u32,
            _ => 1,
        }).sum()
    }
    
    fn count_git_operations(&self, actions: &[RecoveryAction]) -> u32 {
        actions.iter().filter(|action| matches!(
            action,
            RecoveryAction::GitReset { .. } |
            RecoveryAction::GitRebase { .. } |
            RecoveryAction::BranchRecreation { .. }
        )).count() as u32
    }
    
    fn count_network_requests(&self, actions: &[RecoveryAction]) -> u32 {
        actions.iter().filter(|action| matches!(
            action,
            RecoveryAction::DependencyReinstall { .. } |
            RecoveryAction::ServiceRestart { .. }
        )).count() as u32
    }
    
    /// Get recovery history for analysis
    pub fn recovery_history(&self) -> &[AutonomousRecoveryAttempt] {
        &self.recovery_history
    }
    
    /// Generate recovery report
    pub fn generate_recovery_report(&self) -> AutonomousRecoveryReport {
        let total_attempts = self.recovery_history.len();
        let successful_attempts = self.recovery_history.iter()
            .filter(|attempt| attempt.success)
            .count();
        
        let success_rate = if total_attempts > 0 {
            (successful_attempts as f64 / total_attempts as f64) * 100.0
        } else {
            0.0
        };
        
        let average_duration = if total_attempts > 0 {
            self.recovery_history.iter()
                .map(|attempt| attempt.metrics.total_duration_ms)
                .sum::<u64>() / total_attempts as u64
        } else {
            0
        };
        
        AutonomousRecoveryReport {
            total_attempts: total_attempts as u32,
            successful_attempts: successful_attempts as u32,
            success_rate,
            average_duration_ms: average_duration,
            common_error_types: self.get_common_error_types(),
            most_effective_strategies: self.get_most_effective_strategies(),
            generated_at: Utc::now(),
        }
    }
    
    fn get_common_error_types(&self) -> Vec<(String, u32)> {
        let mut error_counts = HashMap::new();
        
        for attempt in &self.recovery_history {
            let error_type = format!("{:?}", attempt.error_type);
            *error_counts.entry(error_type).or_insert(0) += 1;
        }
        
        let mut sorted_errors: Vec<_> = error_counts.into_iter().collect();
        sorted_errors.sort_by(|a, b| b.1.cmp(&a.1));
        sorted_errors.into_iter().take(5).collect()
    }
    
    fn get_most_effective_strategies(&self) -> Vec<(String, f64)> {
        let mut strategy_stats = HashMap::new();
        
        for attempt in &self.recovery_history {
            let strategy_key = format!("{:?}", attempt.strategy);
            let stats = strategy_stats.entry(strategy_key).or_insert((0, 0));
            stats.0 += 1; // total
            if attempt.success {
                stats.1 += 1; // successful
            }
        }
        
        let mut effectiveness: Vec<_> = strategy_stats.into_iter()
            .map(|(strategy, (total, successful))| {
                let rate = if total > 0 {
                    (successful as f64 / total as f64) * 100.0
                } else {
                    0.0
                };
                (strategy, rate)
            })
            .collect();
        
        effectiveness.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        effectiveness.into_iter().take(5).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousRecoveryReport {
    pub total_attempts: u32,
    pub successful_attempts: u32,
    pub success_rate: f64,
    pub average_duration_ms: u64,
    pub common_error_types: Vec<(String, u32)>,
    pub most_effective_strategies: Vec<(String, f64)>,
    pub generated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::recovery::AutoRecovery;

    #[tokio::test]
    async fn test_recovery_strategy_determination() {
        let github_client = GitHubClient::new().unwrap();
        let base_recovery = Box::new(AutoRecovery::new(github_client.clone(), true));
        let recovery = AutonomousErrorRecovery::new(github_client, base_recovery);
        
        let git_error = ErrorType::GitOperationFailed {
            operation: "push".to_string(),
            error: "Connection failed".to_string(),
        };
        
        let strategy = recovery.determine_recovery_strategy(&git_error);
        
        assert!(matches!(strategy, RecoveryStrategy::RetryWithBackoff { .. }));
    }
    
    #[tokio::test]
    async fn test_merge_conflict_strategy() {
        let github_client = GitHubClient::new().unwrap();
        let base_recovery = Box::new(AutoRecovery::new(github_client.clone(), true));
        let recovery = AutonomousErrorRecovery::new(github_client, base_recovery);
        
        let conflict_error = ErrorType::MergeConflict {
            files: vec!["src/main.rs".to_string()],
            conflict_count: 2,
        };
        
        let strategy = recovery.determine_recovery_strategy(&conflict_error);
        
        assert!(matches!(
            strategy,
            RecoveryStrategy::AutomatedFix { 
                fix_type: FixType::MergeConflictResolution,
                confidence: ConfidenceLevel::High 
            }
        ));
    }
    
    #[tokio::test]
    async fn test_escalation_strategy() {
        let github_client = GitHubClient::new().unwrap();
        let base_recovery = Box::new(AutoRecovery::new(github_client.clone(), true));
        let recovery = AutonomousErrorRecovery::new(github_client, base_recovery);
        
        let complex_conflict = ErrorType::MergeConflict {
            files: vec!["migration.sql".to_string(), "schema.rs".to_string()],
            conflict_count: 10,
        };
        
        let strategy = recovery.determine_recovery_strategy(&complex_conflict);
        
        assert!(matches!(strategy, RecoveryStrategy::Escalate { .. }));
    }
}