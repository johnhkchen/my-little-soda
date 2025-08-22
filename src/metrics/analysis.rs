use crate::github::GitHubError;
use super::types::*;
use super::storage::MetricsStorage;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct MetricsAnalyzer {
    storage: MetricsStorage,
}

impl MetricsAnalyzer {
    pub fn new() -> Self {
        Self {
            storage: MetricsStorage::new(),
        }
    }

    pub async fn calculate_metrics(&self, lookback_hours: Option<u64>) -> Result<IntegrationMetrics, GitHubError> {
        let attempts = self.storage.load_integration_attempts().await?;
        
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

    pub async fn format_performance_report(&self, lookback_hours: Option<u64>) -> Result<String, GitHubError> {
        let performance_reporter = super::performance::PerformanceReporter::new();
        performance_reporter.format_performance_report(lookback_hours).await
    }

    pub fn format_metrics_report(&self, metrics: &IntegrationMetrics, detailed: bool) -> String {
        super::reports::MetricsReporter::format_metrics_report(metrics, detailed)
    }

    pub async fn export_metrics_for_monitoring(&self, lookback_hours: Option<u64>) -> Result<HashMap<String, serde_json::Value>, GitHubError> {
        let routing_metrics = self.storage.load_routing_metrics(lookback_hours).await?;
        let utilization_metrics = self.storage.load_agent_utilization_metrics(lookback_hours).await?;
        let bottlenecks = self.storage.load_performance_bottlenecks(lookback_hours).await?;
        
        Ok(super::reports::MetricsReporter::export_metrics_for_monitoring(
            &routing_metrics, 
            &utilization_metrics, 
            &bottlenecks
        ))
    }
}