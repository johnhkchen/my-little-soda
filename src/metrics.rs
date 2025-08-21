// Integration Success Tracking and Metrics
// Provides tracking for work integration success rates and performance

use crate::github::GitHubError;
use crate::telemetry::generate_correlation_id;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationAttempt {
    pub correlation_id: String,
    pub issue_number: u64,
    pub agent_id: String,
    pub attempt_time: u64, // Unix timestamp
    pub phase: IntegrationPhase,
    pub outcome: IntegrationOutcome,
    pub duration_seconds: Option<u64>,
    pub error_message: Option<String>,
    pub pr_number: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntegrationPhase {
    WorkCompletion,     // agent completed work, created PR
    MergeReady,         // PR reviewed, ready for merge
    Merged,             // PR merged to main
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntegrationOutcome {
    Success,
    Failed,
    InProgress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetrics {
    pub agent_id: String,
    pub total_attempts: u64,
    pub successful_integrations: u64,
    pub failed_integrations: u64,
    pub average_completion_time_seconds: Option<f64>,
    pub last_activity: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationMetrics {
    pub overall_success_rate: f64,
    pub total_attempts: u64,
    pub successful_integrations: u64,
    pub failed_integrations: u64,
    pub average_time_to_merge_seconds: Option<f64>,
    pub agent_metrics: HashMap<String, AgentMetrics>,
    pub recent_attempts: Vec<IntegrationAttempt>,
}

#[derive(Debug)]
pub struct MetricsTracker {
    storage_path: std::path::PathBuf,
}

impl MetricsTracker {
    pub fn new() -> Self {
        let storage_path = std::path::PathBuf::from(".clambake/metrics");
        Self { storage_path }
    }

    pub async fn track_integration_attempt(
        &self,
        issue_number: u64,
        agent_id: &str,
        phase: IntegrationPhase,
        outcome: IntegrationOutcome,
        duration: Option<Duration>,
        error_message: Option<String>,
        pr_number: Option<u64>,
    ) -> Result<(), GitHubError> {
        let attempt = IntegrationAttempt {
            correlation_id: generate_correlation_id(),
            issue_number,
            agent_id: agent_id.to_string(),
            attempt_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            phase: phase.clone(),
            outcome: outcome.clone(),
            duration_seconds: duration.map(|d| d.as_secs()),
            error_message,
            pr_number,
        };

        // Log structured metrics for telemetry  
        tracing::info!(
            issue.number = issue_number,
            agent.id = agent_id,
            phase = ?phase,
            outcome = ?outcome,
            duration_seconds = attempt.duration_seconds,
            correlation.id = &attempt.correlation_id,
            "Integration attempt tracked"
        );
        
        self.store_attempt(attempt).await?;

        Ok(())
    }

    async fn store_attempt(&self, attempt: IntegrationAttempt) -> Result<(), GitHubError> {
        // Ensure storage directory exists
        if let Some(parent) = self.storage_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                GitHubError::IoError(e)
            })?;
        }

        let file_path = self.storage_path.join("integration_attempts.jsonl");
        let attempt_json = serde_json::to_string(&attempt).map_err(|e| {
            GitHubError::NotImplemented(format!("Failed to serialize attempt: {}", e))
        })?;

        // Append to JSONL file (one JSON object per line)
        let content = format!("{}\n", attempt_json);
        
        // Try to append to existing file, or create if it doesn't exist
        match tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .await
        {
            Ok(mut file) => {
                use tokio::io::AsyncWriteExt;
                file.write_all(content.as_bytes()).await.map_err(|e| {
                    GitHubError::IoError(e)
                })?;
            }
            Err(e) => {
                return Err(GitHubError::IoError(e));
            }
        }

        Ok(())
    }

    pub async fn load_attempts(&self) -> Result<Vec<IntegrationAttempt>, GitHubError> {
        let file_path = self.storage_path.join("integration_attempts.jsonl");
        
        match fs::read_to_string(&file_path).await {
            Ok(content) => {
                let mut attempts = Vec::new();
                for line in content.lines() {
                    if !line.trim().is_empty() {
                        match serde_json::from_str::<IntegrationAttempt>(line) {
                            Ok(attempt) => attempts.push(attempt),
                            Err(e) => {
                                tracing::warn!("Failed to parse metrics line: {}", e);
                            }
                        }
                    }
                }
                Ok(attempts)
            }
            Err(_) => {
                // File doesn't exist yet, return empty vec
                Ok(Vec::new())
            }
        }
    }

    pub async fn calculate_metrics(&self, lookback_hours: Option<u64>) -> Result<IntegrationMetrics, GitHubError> {
        let attempts = self.load_attempts().await?;
        
        // Filter by lookback period if specified
        let cutoff_time = lookback_hours.map(|hours| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() - (hours * 3600)
        });

        let filtered_attempts: Vec<_> = attempts
            .into_iter()
            .filter(|attempt| {
                cutoff_time.map_or(true, |cutoff| attempt.attempt_time >= cutoff)
            })
            .collect();

        let total_attempts = filtered_attempts.len() as u64;
        let successful_integrations = filtered_attempts
            .iter()
            .filter(|a| matches!(a.outcome, IntegrationOutcome::Success))
            .count() as u64;
        let failed_integrations = filtered_attempts
            .iter()
            .filter(|a| matches!(a.outcome, IntegrationOutcome::Failed))
            .count() as u64;

        let overall_success_rate = if total_attempts > 0 {
            successful_integrations as f64 / total_attempts as f64
        } else {
            0.0
        };

        // Calculate average time to merge for successful integrations
        let successful_durations: Vec<u64> = filtered_attempts
            .iter()
            .filter(|a| matches!(a.outcome, IntegrationOutcome::Success))
            .filter_map(|a| a.duration_seconds)
            .collect();

        let average_time_to_merge_seconds = if !successful_durations.is_empty() {
            Some(successful_durations.iter().sum::<u64>() as f64 / successful_durations.len() as f64)
        } else {
            None
        };

        // Calculate per-agent metrics
        let mut agent_stats: HashMap<String, (u64, u64, u64, Vec<u64>, Option<u64>)> = HashMap::new();
        
        for attempt in &filtered_attempts {
            let entry = agent_stats.entry(attempt.agent_id.clone())
                .or_insert((0, 0, 0, Vec::new(), None));
            
            entry.0 += 1; // total attempts
            
            match attempt.outcome {
                IntegrationOutcome::Success => entry.1 += 1,
                IntegrationOutcome::Failed => entry.2 += 1,
                IntegrationOutcome::InProgress => {}
            }
            
            if let Some(duration) = attempt.duration_seconds {
                entry.3.push(duration);
            }
            
            // Update last activity
            entry.4 = Some(match entry.4 {
                Some(last) => last.max(attempt.attempt_time),
                None => attempt.attempt_time,
            });
        }

        let agent_metrics: HashMap<String, AgentMetrics> = agent_stats
            .into_iter()
            .map(|(agent_id, (total, success, failed, durations, last_activity))| {
                let average_completion_time_seconds = if !durations.is_empty() {
                    Some(durations.iter().sum::<u64>() as f64 / durations.len() as f64)
                } else {
                    None
                };

                let metrics = AgentMetrics {
                    agent_id: agent_id.clone(),
                    total_attempts: total,
                    successful_integrations: success,
                    failed_integrations: failed,
                    average_completion_time_seconds,
                    last_activity,
                };

                (agent_id, metrics)
            })
            .collect();

        // Get recent attempts (last 20)
        let mut recent_attempts = filtered_attempts;
        recent_attempts.sort_by(|a, b| b.attempt_time.cmp(&a.attempt_time));
        recent_attempts.truncate(20);

        Ok(IntegrationMetrics {
            overall_success_rate,
            total_attempts,
            successful_integrations,
            failed_integrations,
            average_time_to_merge_seconds,
            agent_metrics,
            recent_attempts,
        })
    }

    pub fn format_metrics_report(&self, metrics: &IntegrationMetrics, detailed: bool) -> String {
        let mut report = String::new();
        
        report.push_str("🔍 INTEGRATION METRICS REPORT\n");
        report.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n");
        
        // Overall metrics
        report.push_str(&format!("📊 OVERALL PERFORMANCE\n"));
        report.push_str(&format!("   Success Rate:     {:.1}% ({}/{} attempts)\n", 
            metrics.overall_success_rate * 100.0,
            metrics.successful_integrations,
            metrics.total_attempts
        ));
        
        if let Some(avg_time) = metrics.average_time_to_merge_seconds {
            let hours = avg_time / 3600.0;
            let minutes = (avg_time % 3600.0) / 60.0;
            report.push_str(&format!("   Avg Time to Merge: {:.1}h {:.0}m\n", hours, minutes));
        }
        
        if metrics.failed_integrations > 0 {
            report.push_str(&format!("   Failed Attempts:   {}\n", metrics.failed_integrations));
        }
        
        report.push_str("\n");

        // Agent performance
        if !metrics.agent_metrics.is_empty() {
            report.push_str("🤖 AGENT PERFORMANCE\n");
            let mut agents: Vec<_> = metrics.agent_metrics.values().collect();
            agents.sort_by(|a, b| {
                b.successful_integrations.cmp(&a.successful_integrations)
                    .then_with(|| {
                        let rate_a = if a.total_attempts > 0 {
                            a.successful_integrations as f64 / a.total_attempts as f64
                        } else { 0.0 };
                        let rate_b = if b.total_attempts > 0 {
                            b.successful_integrations as f64 / b.total_attempts as f64  
                        } else { 0.0 };
                        rate_b.partial_cmp(&rate_a).unwrap_or(std::cmp::Ordering::Equal)
                    })
            });
            
            for agent in agents {
                let success_rate = if agent.total_attempts > 0 {
                    agent.successful_integrations as f64 / agent.total_attempts as f64 * 100.0
                } else {
                    0.0
                };
                
                report.push_str(&format!("   {}: {:.1}% ({}/{} attempts)",
                    agent.agent_id,
                    success_rate,
                    agent.successful_integrations,
                    agent.total_attempts
                ));
                
                if let Some(avg_time) = agent.average_completion_time_seconds {
                    let hours = avg_time / 3600.0;
                    let minutes = (avg_time % 3600.0) / 60.0;
                    report.push_str(&format!(", avg {:.1}h {:.0}m", hours, minutes));
                }
                
                report.push_str("\n");
            }
            report.push_str("\n");
        }

        // Recent activity (if detailed)
        if detailed && !metrics.recent_attempts.is_empty() {
            report.push_str("📋 RECENT INTEGRATION ATTEMPTS\n");
            for attempt in metrics.recent_attempts.iter().take(10) {
                let time_ago = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() - attempt.attempt_time;
                
                let time_str = if time_ago < 3600 {
                    format!("{}m ago", time_ago / 60)
                } else if time_ago < 86400 {
                    format!("{}h ago", time_ago / 3600)
                } else {
                    format!("{}d ago", time_ago / 86400)
                };
                
                let status = match attempt.outcome {
                    IntegrationOutcome::Success => "✅",
                    IntegrationOutcome::Failed => "❌",
                    IntegrationOutcome::InProgress => "🔄",
                };
                
                report.push_str(&format!("   {} Issue #{} ({}) - {} [{}]\n",
                    status,
                    attempt.issue_number,
                    attempt.agent_id,
                    time_str,
                    format!("{:?}", attempt.phase)
                ));
            }
            report.push_str("\n");
        }

        // Health indicators
        report.push_str("🎯 SYSTEM HEALTH\n");
        if metrics.overall_success_rate >= 0.9 {
            report.push_str("   Status: EXCELLENT (>90% success rate)\n");
        } else if metrics.overall_success_rate >= 0.7 {
            report.push_str("   Status: GOOD (70-90% success rate)\n");
        } else if metrics.overall_success_rate >= 0.5 {
            report.push_str("   Status: NEEDS ATTENTION (50-70% success rate)\n");
        } else {
            report.push_str("   Status: CRITICAL (<50% success rate)\n");
        }

        if metrics.total_attempts < 10 {
            report.push_str("   Note: Limited data available (< 10 attempts)\n");
        }

        report
    }
}