use crate::github::GitHubError;
use super::types::*;
use super::storage::MetricsStorage;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct BottleneckDetector {
    storage: MetricsStorage,
}

impl BottleneckDetector {
    pub fn new() -> Self {
        Self {
            storage: MetricsStorage::new(),
        }
    }

    pub async fn detect_and_store_bottlenecks(&self) -> Result<Vec<PerformanceBottleneck>, GitHubError> {
        let mut bottlenecks = Vec::new();

        // Check recent routing metrics for latency bottlenecks
        if let Ok(routing_metrics) = self.storage.load_routing_metrics(Some(1)).await {
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
        if let Ok(utilization_metrics) = self.storage.load_agent_utilization_metrics(Some(1)).await {
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
            self.storage.store_performance_bottleneck(bottleneck.clone()).await?;
        }

        Ok(bottlenecks)
    }

    pub fn detect_routing_bottlenecks(
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
}