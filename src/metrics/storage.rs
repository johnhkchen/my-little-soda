use super::types::*;
use crate::github::GitHubError;
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Debug)]
pub struct MetricsStorage {
    pub(super) storage_path: PathBuf,
}

impl Default for MetricsStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsStorage {
    pub fn new() -> Self {
        let storage_path = PathBuf::from(".my-little-soda/metrics");
        Self { storage_path }
    }

    pub async fn store_routing_metrics(&self, metrics: RoutingMetrics) -> Result<(), GitHubError> {
        let file_path = self.storage_path.join("routing_metrics.jsonl");
        self.store_jsonl_entry(&file_path, &metrics).await
    }

    pub async fn store_agent_utilization_metrics(
        &self,
        metrics: AgentUtilizationMetrics,
    ) -> Result<(), GitHubError> {
        let file_path = self.storage_path.join("agent_utilization.jsonl");
        self.store_jsonl_entry(&file_path, &metrics).await
    }

    pub async fn store_coordination_decision(
        &self,
        decision: CoordinationDecision,
    ) -> Result<(), GitHubError> {
        let file_path = self.storage_path.join("coordination_decisions.jsonl");
        self.store_jsonl_entry(&file_path, &decision).await
    }

    pub async fn store_performance_bottleneck(
        &self,
        bottleneck: PerformanceBottleneck,
    ) -> Result<(), GitHubError> {
        let file_path = self.storage_path.join("performance_bottlenecks.jsonl");
        self.store_jsonl_entry(&file_path, &bottleneck).await
    }

    pub async fn store_integration_attempt(
        &self,
        attempt: IntegrationAttempt,
    ) -> Result<(), GitHubError> {
        let file_path = self.storage_path.join("integration_attempts.jsonl");
        let attempt_json = serde_json::to_string(&attempt).map_err(|e| {
            GitHubError::NotImplemented(format!("Failed to serialize attempt: {e}"))
        })?;

        let content = format!("{attempt_json}\n");

        match tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .await
        {
            Ok(mut file) => {
                use tokio::io::AsyncWriteExt;
                file.write_all(content.as_bytes())
                    .await
                    .map_err(|e| GitHubError::IoError(e))?;
            }
            Err(e) => {
                return Err(GitHubError::IoError(e));
            }
        }

        Ok(())
    }

    async fn store_jsonl_entry<T: serde::Serialize>(
        &self,
        file_path: &Path,
        entry: &T,
    ) -> Result<(), GitHubError> {
        // Ensure storage directory exists
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| GitHubError::IoError(e))?;
        }

        let entry_json = serde_json::to_string(entry)
            .map_err(|e| GitHubError::NotImplemented(format!("Failed to serialize entry: {e}")))?;

        // Append to JSONL file (one JSON object per line)
        let content = format!("{entry_json}\n");

        // Try to append to existing file, or create if it doesn't exist
        match tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)
            .await
        {
            Ok(mut file) => {
                use tokio::io::AsyncWriteExt;
                file.write_all(content.as_bytes())
                    .await
                    .map_err(|e| GitHubError::IoError(e))?;
            }
            Err(e) => {
                return Err(GitHubError::IoError(e));
            }
        }

        Ok(())
    }

    pub async fn load_integration_attempts(&self) -> Result<Vec<IntegrationAttempt>, GitHubError> {
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

    pub async fn load_routing_metrics(
        &self,
        lookback_hours: Option<u64>,
    ) -> Result<Vec<RoutingMetrics>, GitHubError> {
        let file_path = self.storage_path.join("routing_metrics.jsonl");
        self.load_jsonl_entries(&file_path, lookback_hours).await
    }

    pub async fn load_agent_utilization_metrics(
        &self,
        lookback_hours: Option<u64>,
    ) -> Result<Vec<AgentUtilizationMetrics>, GitHubError> {
        let file_path = self.storage_path.join("agent_utilization.jsonl");
        self.load_jsonl_entries(&file_path, lookback_hours).await
    }

    pub async fn load_coordination_decisions(
        &self,
        lookback_hours: Option<u64>,
    ) -> Result<Vec<CoordinationDecision>, GitHubError> {
        let file_path = self.storage_path.join("coordination_decisions.jsonl");
        self.load_jsonl_entries(&file_path, lookback_hours).await
    }

    pub async fn load_performance_bottlenecks(
        &self,
        lookback_hours: Option<u64>,
    ) -> Result<Vec<PerformanceBottleneck>, GitHubError> {
        let file_path = self.storage_path.join("performance_bottlenecks.jsonl");
        self.load_jsonl_entries(&file_path, lookback_hours).await
    }

    async fn load_jsonl_entries<T: serde::de::DeserializeOwned>(
        &self,
        file_path: &Path,
        lookback_hours: Option<u64>,
    ) -> Result<Vec<T>, GitHubError> {
        match fs::read_to_string(file_path).await {
            Ok(content) => {
                let mut entries = Vec::new();
                for line in content.lines() {
                    if !line.trim().is_empty() {
                        match serde_json::from_str::<T>(line) {
                            Ok(entry) => entries.push(entry),
                            Err(e) => {
                                tracing::warn!("Failed to parse metrics line: {}", e);
                            }
                        }
                    }
                }

                // Filter by lookback period if specified - this is a simple implementation
                // For proper timestamp filtering, we'd need to extract timestamp from each entry
                if lookback_hours.is_some() {
                    // For now, return recent entries (last 1000 for 1 hour lookback)
                    let limit = lookback_hours.unwrap_or(24) * 100; // approximate recent entries
                    if entries.len() > limit as usize {
                        entries = entries.into_iter().rev().take(limit as usize).collect();
                    }
                }

                Ok(entries)
            }
            Err(_) => {
                // File doesn't exist yet, return empty vec
                Ok(Vec::new())
            }
        }
    }
}
