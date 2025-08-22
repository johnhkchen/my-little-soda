use super::types::*;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct MetricsReporter;

impl MetricsReporter {
    pub fn format_metrics_report(metrics: &IntegrationMetrics, detailed: bool) -> String {
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

    pub fn export_metrics_for_monitoring(
        routing_metrics: &[RoutingMetrics],
        utilization_metrics: &[AgentUtilizationMetrics],
        bottlenecks: &[PerformanceBottleneck],
    ) -> HashMap<String, serde_json::Value> {
        let mut export = HashMap::new();
        
        // Export routing metrics
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
        if !utilization_metrics.is_empty() {
            let avg_utilization: f64 = utilization_metrics.iter()
                .map(|m| m.utilization_percentage)
                .sum::<f64>() / utilization_metrics.len() as f64;
            
            export.insert("agent_avg_utilization_percentage".to_string(), serde_json::Value::Number(
                serde_json::Number::from_f64(avg_utilization).unwrap_or(serde_json::Number::from(0))
            ));
        }
        
        // Export bottleneck counts
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
        
        export
    }
}