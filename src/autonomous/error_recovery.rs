use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use thiserror::Error;
use tracing::{info, warn, error, debug};
use rand::Rng;

use crate::github::{GitHubClient, errors::GitHubError};
use crate::agents::recovery::{AutomaticRecovery, ComprehensiveRecoveryReport, RecoveryError};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FixType {
    MergeConflictResolution,
    TestFailureFix,
    BuildErrorFix,
    DependencyUpdate,
    ConfigurationAdjustment,
    CodeFormatting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfidenceLevel {
    High,    // 90-100% success rate
    Medium,  // 60-89% success rate  
    Low,     // 30-59% success rate
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlternativeApproach {
    DifferentImplementation,
    SimplifiedSolution,
    ManualProcess,
    ExternalTool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        let attempt_id = format!("recovery-{}-{}", Utc::now().timestamp(), rand::thread_rng().gen::<u16>());
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
            
            RecoveryStrategy::Fallback { alternative } => {
                match self.execute_fallback_strategy(alternative, &mut recovery_actions).await {
                    Ok(_) => success = true,
                    Err(e) => {
                        error_message = Some(format!("Fallback strategy failed: {:?}", e));
                        success = false;
                    }
                }
            }
            
            RecoveryStrategy::Escalate { urgency, context: ref escalation_context } => {
                self.execute_escalation(urgency, escalation_context.clone(), &mut recovery_actions).await?;
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
        Ok(rand::thread_rng().gen::<f64>() > 0.3) // 70% success rate
    }
    
    async fn retry_github_operation(
        &self,
        recovery_actions: &mut Vec<RecoveryAction>,
    ) -> Result<bool, RecoveryError> {
        recovery_actions.push(RecoveryAction::ServiceRestart {
            service: "github-api".to_string()
        });
        // Simulate GitHub API retry
        Ok(rand::thread_rng().gen::<f64>() > 0.2) // 80% success rate
    }
    
    async fn retry_network_operation(
        &self,
        recovery_actions: &mut Vec<RecoveryAction>,
    ) -> Result<bool, RecoveryError> {
        recovery_actions.push(RecoveryAction::ServiceRestart {
            service: "network".to_string()
        });
        // Simulate network retry
        Ok(rand::thread_rng().gen::<f64>() > 0.4) // 60% success rate
    }
    
    async fn resolve_merge_conflicts_automatically(
        &self,
        recovery_actions: &mut Vec<RecoveryAction>,
    ) -> Result<(), RecoveryError> {
        recovery_actions.push(RecoveryAction::AutomergeConflictResolution {
            files: vec!["src/main.rs".to_string(), "Cargo.toml".to_string()]
        });
        Ok(())
    }
    
    async fn fix_test_failures_automatically(
        &self,
        recovery_actions: &mut Vec<RecoveryAction>,
    ) -> Result<(), RecoveryError> {
        recovery_actions.push(RecoveryAction::TestSkip {
            tests: vec!["flaky_test".to_string()]
        });
        Ok(())
    }
    
    async fn fix_build_errors_automatically(
        &self,
        recovery_actions: &mut Vec<RecoveryAction>,
    ) -> Result<(), RecoveryError> {
        recovery_actions.push(RecoveryAction::DependencyReinstall {
            packages: vec!["build-tools".to_string()]
        });
        Ok(())
    }
    
    async fn update_dependencies_automatically(
        &self,
        recovery_actions: &mut Vec<RecoveryAction>,
    ) -> Result<(), RecoveryError> {
        recovery_actions.push(RecoveryAction::DependencyReinstall {
            packages: vec!["all".to_string()]
        });
        Ok(())
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
        recovery_actions.push(RecoveryAction::ServiceRestart {
            service: "code-formatter".to_string()
        });
        Ok(())
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