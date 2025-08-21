// Integration Success Tracking and Metrics
// Provides tracking for work integration success rates and performance

use crate::github::GitHubError;
use crate::telemetry::generate_correlation_id;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
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
pub struct RoutingMetrics {
    pub correlation_id: String,
    pub routing_start_time: u64, // Unix timestamp
    pub routing_duration_ms: u64,
    pub issues_evaluated: u64,
    pub agents_available: u64,
    pub decision_outcome: RoutingDecision,
    pub bottlenecks_detected: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoutingDecision {
    TaskAssigned { issue_number: u64, agent_id: String },
    NoTasksAvailable,
    NoAgentsAvailable,
    FilteredOut { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentUtilizationMetrics {
    pub agent_id: String,
    pub timestamp: u64,
    pub current_capacity: u32,
    pub max_capacity: u32,
    pub utilization_percentage: f64,
    pub active_issues: Vec<u64>,
    pub state: String, // Available, Working, etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationDecision {
    pub correlation_id: String,
    pub timestamp: u64,
    pub operation: String, // "assign_agent", "route_issues", "pop_task", etc.
    pub agent_id: Option<String>,
    pub issue_number: Option<u64>,
    pub decision_rationale: String,
    pub execution_time_ms: u64,
    pub success: bool,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceBottleneck {
    pub detected_at: u64,
    pub bottleneck_type: BottleneckType,
    pub severity: BottleneckSeverity,
    pub description: String,
    pub suggested_action: String,
    pub metrics: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BottleneckType {
    RoutingLatency,
    AgentCapacity,
    GitHubApiRate,
    WorkCompletion,
    DecisionTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BottleneckSeverity {
    Low,
    Medium,
    High,
    Critical,
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

    pub async fn track_routing_metrics(
        &self,
        correlation_id: String,
        routing_start: Instant,
        issues_evaluated: u64,
        agents_available: u64,
        decision_outcome: RoutingDecision,
    ) -> Result<(), GitHubError> {
        let routing_duration_ms = routing_start.elapsed().as_millis() as u64;
        
        let metrics = RoutingMetrics {
            correlation_id: correlation_id.clone(),
            routing_start_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            routing_duration_ms,
            issues_evaluated,
            agents_available,
            decision_outcome: decision_outcome.clone(),
            bottlenecks_detected: self.detect_routing_bottlenecks(
                routing_duration_ms,
                issues_evaluated,
                agents_available,
                &decision_outcome
            ),
        };

        // Log structured metrics for telemetry  
        tracing::info!(
            correlation.id = &correlation_id,
            routing.duration_ms = routing_duration_ms,
            routing.issues_evaluated = issues_evaluated,
            routing.agents_available = agents_available,
            routing.decision = ?decision_outcome,
            "Routing metrics tracked"
        );

        self.store_routing_metrics(metrics).await?;
        Ok(())
    }

    pub async fn track_agent_utilization(
        &self,
        agent_id: &str,
        current_capacity: u32,
        max_capacity: u32,
        active_issues: Vec<u64>,
        state: &str,
    ) -> Result<(), GitHubError> {
        let utilization_percentage = if max_capacity > 0 {
            (current_capacity as f64 / max_capacity as f64) * 100.0
        } else {
            0.0
        };

        let metrics = AgentUtilizationMetrics {
            agent_id: agent_id.to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            current_capacity,
            max_capacity,
            utilization_percentage,
            active_issues: active_issues.clone(),
            state: state.to_string(),
        };

        // Log structured metrics for telemetry  
        tracing::info!(
            agent.id = agent_id,
            agent.utilization_percentage = utilization_percentage,
            agent.current_capacity = current_capacity,
            agent.max_capacity = max_capacity,
            agent.state = state,
            agent.active_issues = ?active_issues,
            "Agent utilization tracked"
        );

        self.store_agent_utilization_metrics(metrics).await?;
        Ok(())
    }

    pub async fn track_coordination_decision(
        &self,
        correlation_id: String,
        operation: &str,
        agent_id: Option<&str>,
        issue_number: Option<u64>,
        decision_rationale: &str,
        execution_start: Instant,
        success: bool,
        metadata: HashMap<String, String>,
    ) -> Result<(), GitHubError> {
        let execution_time_ms = execution_start.elapsed().as_millis() as u64;

        let decision = CoordinationDecision {
            correlation_id: correlation_id.clone(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            operation: operation.to_string(),
            agent_id: agent_id.map(|s| s.to_string()),
            issue_number,
            decision_rationale: decision_rationale.to_string(),
            execution_time_ms,
            success,
            metadata: metadata.clone(),
        };

        // Log structured metrics for telemetry  
        tracing::info!(
            correlation.id = &correlation_id,
            coordination.operation = operation,
            coordination.agent_id = agent_id,
            coordination.issue_number = issue_number,
            coordination.execution_time_ms = execution_time_ms,
            coordination.success = success,
            coordination.rationale = decision_rationale,
            "Coordination decision tracked"
        );

        self.store_coordination_decision(decision).await?;
        Ok(())
    }

    pub async fn detect_and_store_bottlenecks(&self) -> Result<Vec<PerformanceBottleneck>, GitHubError> {
        let mut bottlenecks = Vec::new();

        // Check recent routing metrics for latency bottlenecks
        if let Ok(routing_metrics) = self.load_routing_metrics(Some(1)).await {
            for metric in routing_metrics {
                if metric.routing_duration_ms > 2000 { // > 2 seconds
                    let severity = if metric.routing_duration_ms > 10000 {
                        BottleneckSeverity::Critical
                    } else if metric.routing_duration_ms > 5000 {
                        BottleneckSeverity::High
                    } else {
                        BottleneckSeverity::Medium
                    };

                    let mut metrics_map = HashMap::new();
                    metrics_map.insert("routing_duration_ms".to_string(), metric.routing_duration_ms as f64);
                    metrics_map.insert("issues_evaluated".to_string(), metric.issues_evaluated as f64);

                    bottlenecks.push(PerformanceBottleneck {
                        detected_at: SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        bottleneck_type: BottleneckType::RoutingLatency,
                        severity,
                        description: format!(
                            "Routing operation took {}ms (target: <2000ms)",
                            metric.routing_duration_ms
                        ),
                        suggested_action: "Consider optimizing GitHub API calls or filtering logic".to_string(),
                        metrics: metrics_map,
                    });
                }
            }
        }

        // Check agent utilization for capacity bottlenecks
        if let Ok(utilization_metrics) = self.load_agent_utilization_metrics(Some(1)).await {
            let mut utilization_by_agent = HashMap::new();
            for metric in utilization_metrics {
                utilization_by_agent.insert(metric.agent_id.clone(), metric.utilization_percentage);
            }

            let high_utilization_agents: Vec<_> = utilization_by_agent
                .iter()
                .filter(|(_, utilization)| **utilization > 80.0)
                .collect();

            if !high_utilization_agents.is_empty() {
                let mut metrics_map = HashMap::new();
                for (agent_id, utilization) in high_utilization_agents.iter() {
                    metrics_map.insert(format!("{}_utilization", agent_id), **utilization);
                }

                bottlenecks.push(PerformanceBottleneck {
                    detected_at: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    bottleneck_type: BottleneckType::AgentCapacity,
                    severity: BottleneckSeverity::High,
                    description: format!(
                        "High agent utilization detected: {} agents >80%",
                        high_utilization_agents.len()
                    ),
                    suggested_action: "Consider adding more agent capacity or optimizing task distribution".to_string(),
                    metrics: metrics_map,
                });
            }
        }

        // Store detected bottlenecks
        for bottleneck in &bottlenecks {
            self.store_performance_bottleneck(bottleneck.clone()).await?;
        }

        Ok(bottlenecks)
    }

    fn detect_routing_bottlenecks(
        &self,
        routing_duration_ms: u64,
        issues_evaluated: u64,
        agents_available: u64,
        decision_outcome: &RoutingDecision,
    ) -> Vec<String> {
        let mut bottlenecks = Vec::new();

        if routing_duration_ms > 2000 {
            bottlenecks.push("routing_latency_high".to_string());
        }

        if agents_available == 0 {
            bottlenecks.push("no_agents_available".to_string());
        }

        if issues_evaluated == 0 {
            bottlenecks.push("no_issues_to_evaluate".to_string());
        }

        match decision_outcome {
            RoutingDecision::NoTasksAvailable => {
                bottlenecks.push("no_routable_tasks".to_string());
            }
            RoutingDecision::NoAgentsAvailable => {
                bottlenecks.push("agent_capacity_exhausted".to_string());
            }
            RoutingDecision::FilteredOut { reason } => {
                bottlenecks.push(format!("filtered_out_{}", reason.replace(" ", "_").to_lowercase()));
            }
            _ => {}
        }

        bottlenecks
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

    async fn store_routing_metrics(&self, metrics: RoutingMetrics) -> Result<(), GitHubError> {
        let file_path = self.storage_path.join("routing_metrics.jsonl");
        self.store_jsonl_entry(&file_path, &metrics).await
    }

    async fn store_agent_utilization_metrics(&self, metrics: AgentUtilizationMetrics) -> Result<(), GitHubError> {
        let file_path = self.storage_path.join("agent_utilization.jsonl");
        self.store_jsonl_entry(&file_path, &metrics).await
    }

    async fn store_coordination_decision(&self, decision: CoordinationDecision) -> Result<(), GitHubError> {
        let file_path = self.storage_path.join("coordination_decisions.jsonl");
        self.store_jsonl_entry(&file_path, &decision).await
    }

    async fn store_performance_bottleneck(&self, bottleneck: PerformanceBottleneck) -> Result<(), GitHubError> {
        let file_path = self.storage_path.join("performance_bottlenecks.jsonl");
        self.store_jsonl_entry(&file_path, &bottleneck).await
    }

    async fn store_jsonl_entry<T: serde::Serialize>(&self, file_path: &std::path::Path, entry: &T) -> Result<(), GitHubError> {
        // Ensure storage directory exists
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                GitHubError::IoError(e)
            })?;
        }

        let entry_json = serde_json::to_string(entry).map_err(|e| {
            GitHubError::NotImplemented(format!("Failed to serialize entry: {}", e))
        })?;

        // Append to JSONL file (one JSON object per line)
        let content = format!("{}\n", entry_json);
        
        // Try to append to existing file, or create if it doesn't exist
        match tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)
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

    pub async fn load_routing_metrics(&self, lookback_hours: Option<u64>) -> Result<Vec<RoutingMetrics>, GitHubError> {
        let file_path = self.storage_path.join("routing_metrics.jsonl");
        self.load_jsonl_entries(&file_path, lookback_hours).await
    }

    pub async fn load_agent_utilization_metrics(&self, lookback_hours: Option<u64>) -> Result<Vec<AgentUtilizationMetrics>, GitHubError> {
        let file_path = self.storage_path.join("agent_utilization.jsonl");
        self.load_jsonl_entries(&file_path, lookback_hours).await
    }

    pub async fn load_coordination_decisions(&self, lookback_hours: Option<u64>) -> Result<Vec<CoordinationDecision>, GitHubError> {
        let file_path = self.storage_path.join("coordination_decisions.jsonl");
        self.load_jsonl_entries(&file_path, lookback_hours).await
    }

    pub async fn load_performance_bottlenecks(&self, lookback_hours: Option<u64>) -> Result<Vec<PerformanceBottleneck>, GitHubError> {
        let file_path = self.storage_path.join("performance_bottlenecks.jsonl");
        self.load_jsonl_entries(&file_path, lookback_hours).await
    }

    async fn load_jsonl_entries<T: serde::de::DeserializeOwned>(&self, file_path: &std::path::Path, lookback_hours: Option<u64>) -> Result<Vec<T>, GitHubError> {
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

    pub async fn format_performance_report(&self, lookback_hours: Option<u64>) -> Result<String, GitHubError> {
        let mut report = String::new();
        
        report.push_str("⚡ CLAMBAKE PERFORMANCE METRICS REPORT\n");
        report.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n");

        // Routing Performance
        let routing_metrics = self.load_routing_metrics(lookback_hours).await?;
        if !routing_metrics.is_empty() {
            report.push_str("🚀 ROUTING PERFORMANCE\n");
            
            let avg_routing_time: f64 = routing_metrics.iter()
                .map(|m| m.routing_duration_ms as f64)
                .sum::<f64>() / routing_metrics.len() as f64;
            
            let max_routing_time = routing_metrics.iter()
                .map(|m| m.routing_duration_ms)
                .max()
                .unwrap_or(0);
            
            let routing_success_rate = routing_metrics.iter()
                .filter(|m| matches!(m.decision_outcome, RoutingDecision::TaskAssigned { .. }))
                .count() as f64 / routing_metrics.len() as f64 * 100.0;

            report.push_str(&format!("   Average Routing Time: {:.0}ms (target: <2000ms)\n", avg_routing_time));
            report.push_str(&format!("   Max Routing Time:     {}ms\n", max_routing_time));
            report.push_str(&format!("   Success Rate:         {:.1}%\n", routing_success_rate));
            
            let slow_operations = routing_metrics.iter()
                .filter(|m| m.routing_duration_ms > 2000)
                .count();
            if slow_operations > 0 {
                report.push_str(&format!("   ⚠️  Slow Operations:    {} (>2s)\n", slow_operations));
            }
            
            report.push_str("\n");
        }

        // Agent Utilization
        let utilization_metrics = self.load_agent_utilization_metrics(lookback_hours).await?;
        if !utilization_metrics.is_empty() {
            report.push_str("🤖 AGENT UTILIZATION\n");
            
            let mut agent_stats = HashMap::new();
            for metric in &utilization_metrics {
                let entry = agent_stats.entry(metric.agent_id.clone())
                    .or_insert((Vec::new(), metric.max_capacity));
                entry.0.push(metric.utilization_percentage);
            }
            
            for (agent_id, (utilizations, max_capacity)) in agent_stats {
                let avg_utilization = utilizations.iter().sum::<f64>() / utilizations.len() as f64;
                let max_utilization = utilizations.iter().fold(0.0f64, |a, &b| a.max(b));
                
                report.push_str(&format!("   {}: {:.1}% avg, {:.1}% max (capacity: {})\n",
                    agent_id, avg_utilization, max_utilization, max_capacity));
            }
            report.push_str("\n");
        }

        // Coordination Decisions
        let decisions = self.load_coordination_decisions(lookback_hours).await?;
        if !decisions.is_empty() {
            report.push_str("🎯 COORDINATION DECISIONS\n");
            
            let total_decisions = decisions.len();
            let successful_decisions = decisions.iter()
                .filter(|d| d.success)
                .count();
            let success_rate = successful_decisions as f64 / total_decisions as f64 * 100.0;
            
            let avg_execution_time: f64 = decisions.iter()
                .map(|d| d.execution_time_ms as f64)
                .sum::<f64>() / decisions.len() as f64;
            
            report.push_str(&format!("   Total Decisions:   {}\n", total_decisions));
            report.push_str(&format!("   Success Rate:      {:.1}%\n", success_rate));
            report.push_str(&format!("   Avg Execution:     {:.0}ms\n", avg_execution_time));
            
            // Operation breakdown
            let mut operation_counts = HashMap::new();
            for decision in &decisions {
                *operation_counts.entry(decision.operation.clone()).or_insert(0) += 1;
            }
            
            if !operation_counts.is_empty() {
                report.push_str("   Operations:\n");
                for (operation, count) in operation_counts {
                    report.push_str(&format!("     • {}: {}\n", operation, count));
                }
            }
            
            report.push_str("\n");
        }

        // Performance Bottlenecks
        let bottlenecks = self.load_performance_bottlenecks(lookback_hours).await?;
        if !bottlenecks.is_empty() {
            report.push_str("🚨 PERFORMANCE BOTTLENECKS\n");
            
            let critical_count = bottlenecks.iter()
                .filter(|b| matches!(b.severity, BottleneckSeverity::Critical))
                .count();
            let high_count = bottlenecks.iter()
                .filter(|b| matches!(b.severity, BottleneckSeverity::High))
                .count();
            
            if critical_count > 0 {
                report.push_str(&format!("   🔴 Critical: {}\n", critical_count));
            }
            if high_count > 0 {
                report.push_str(&format!("   🟡 High:     {}\n", high_count));
            }
            
            // Show recent bottlenecks
            let mut recent_bottlenecks = bottlenecks.clone();
            recent_bottlenecks.sort_by(|a, b| b.detected_at.cmp(&a.detected_at));
            recent_bottlenecks.truncate(5);
            
            for bottleneck in recent_bottlenecks {
                let severity_icon = match bottleneck.severity {
                    BottleneckSeverity::Critical => "🔴",
                    BottleneckSeverity::High => "🟡",
                    BottleneckSeverity::Medium => "🟠",
                    BottleneckSeverity::Low => "🟢",
                };
                
                report.push_str(&format!("   {} {}\n", severity_icon, bottleneck.description));
                report.push_str(&format!("     → {}\n", bottleneck.suggested_action));
            }
            
            report.push_str("\n");
        }

        // System Health Summary
        report.push_str("📊 SYSTEM HEALTH SUMMARY\n");
        
        let routing_health = if routing_metrics.is_empty() {
            "No data".to_string()
        } else {
            let avg_time: f64 = routing_metrics.iter()
                .map(|m| m.routing_duration_ms as f64)
                .sum::<f64>() / routing_metrics.len() as f64;
            
            if avg_time < 1000.0 {
                "🟢 EXCELLENT (<1s avg)".to_string()
            } else if avg_time < 2000.0 {
                "🟡 GOOD (<2s avg)".to_string()
            } else {
                "🔴 NEEDS ATTENTION (>2s avg)".to_string()
            }
        };
        
        report.push_str(&format!("   Routing Performance: {}\n", routing_health));
        
        if !bottlenecks.is_empty() {
            let critical_bottlenecks = bottlenecks.iter()
                .filter(|b| matches!(b.severity, BottleneckSeverity::Critical))
                .count();
            
            if critical_bottlenecks > 0 {
                report.push_str("   System Status: 🔴 CRITICAL ISSUES DETECTED\n");
                report.push_str("   Action Required: Address critical bottlenecks immediately\n");
            } else {
                report.push_str("   System Status: 🟡 MONITORING REQUIRED\n");
                report.push_str("   Action Required: Review and optimize performance\n");
            }
        } else {
            report.push_str("   System Status: 🟢 HEALTHY\n");
        }

        Ok(report)
    }

    pub async fn export_metrics_for_monitoring(&self, lookback_hours: Option<u64>) -> Result<HashMap<String, serde_json::Value>, GitHubError> {
        let mut export = HashMap::new();
        
        // Export routing metrics
        let routing_metrics = self.load_routing_metrics(lookback_hours).await?;
        if !routing_metrics.is_empty() {
            let avg_routing_time: f64 = routing_metrics.iter()
                .map(|m| m.routing_duration_ms as f64)
                .sum::<f64>() / routing_metrics.len() as f64;
            
            export.insert("routing_avg_duration_ms".to_string(), serde_json::Value::Number(
                serde_json::Number::from_f64(avg_routing_time).unwrap_or(serde_json::Number::from(0))
            ));
            
            let success_rate = routing_metrics.iter()
                .filter(|m| matches!(m.decision_outcome, RoutingDecision::TaskAssigned { .. }))
                .count() as f64 / routing_metrics.len() as f64;
            
            export.insert("routing_success_rate".to_string(), serde_json::Value::Number(
                serde_json::Number::from_f64(success_rate).unwrap_or(serde_json::Number::from(0))
            ));
        }
        
        // Export agent utilization
        let utilization_metrics = self.load_agent_utilization_metrics(lookback_hours).await?;
        if !utilization_metrics.is_empty() {
            let avg_utilization: f64 = utilization_metrics.iter()
                .map(|m| m.utilization_percentage)
                .sum::<f64>() / utilization_metrics.len() as f64;
            
            export.insert("agent_avg_utilization_percentage".to_string(), serde_json::Value::Number(
                serde_json::Number::from_f64(avg_utilization).unwrap_or(serde_json::Number::from(0))
            ));
        }
        
        // Export bottleneck counts
        let bottlenecks = self.load_performance_bottlenecks(lookback_hours).await?;
        export.insert("bottlenecks_critical".to_string(), serde_json::Value::Number(
            serde_json::Number::from(bottlenecks.iter().filter(|b| matches!(b.severity, BottleneckSeverity::Critical)).count())
        ));
        export.insert("bottlenecks_high".to_string(), serde_json::Value::Number(
            serde_json::Number::from(bottlenecks.iter().filter(|b| matches!(b.severity, BottleneckSeverity::High)).count())
        ));
        
        // Add timestamp
        export.insert("timestamp".to_string(), serde_json::Value::Number(
            serde_json::Number::from(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs())
        ));
        
        Ok(export)
    }
}