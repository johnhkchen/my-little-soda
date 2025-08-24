use super::storage::MetricsStorage;
use super::types::*;
use crate::github::GitHubError;
use std::collections::HashMap;

pub struct PerformanceReporter {
    storage: MetricsStorage,
}

impl Default for PerformanceReporter {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceReporter {
    pub fn new() -> Self {
        Self {
            storage: MetricsStorage::new(),
        }
    }

    pub async fn format_performance_report(
        &self,
        lookback_hours: Option<u64>,
    ) -> Result<String, GitHubError> {
        let mut report = String::new();

        report.push_str("⚡ MY LITTLE SODA PERFORMANCE METRICS REPORT\n");
        report.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n");

        // Routing Performance
        let routing_metrics = self.storage.load_routing_metrics(lookback_hours).await?;
        if !routing_metrics.is_empty() {
            report.push_str("🚀 ROUTING PERFORMANCE\n");

            let avg_routing_time: f64 = routing_metrics
                .iter()
                .map(|m| m.routing_duration_ms as f64)
                .sum::<f64>()
                / routing_metrics.len() as f64;

            let max_routing_time = routing_metrics
                .iter()
                .map(|m| m.routing_duration_ms)
                .max()
                .unwrap_or(0);

            let routing_success_rate = routing_metrics
                .iter()
                .filter(|m| matches!(m.decision_outcome, RoutingDecision::TaskAssigned { .. }))
                .count() as f64
                / routing_metrics.len() as f64
                * 100.0;

            report.push_str(&format!(
                "   Average Routing Time: {avg_routing_time:.0}ms (target: <2000ms)\n"
            ));
            report.push_str(&format!("   Max Routing Time:     {max_routing_time}ms\n"));
            report.push_str(&format!(
                "   Success Rate:         {routing_success_rate:.1}%\n"
            ));

            let slow_operations = routing_metrics
                .iter()
                .filter(|m| m.routing_duration_ms > 2000)
                .count();
            if slow_operations > 0 {
                report.push_str(&format!(
                    "   ⚠️  Slow Operations:    {slow_operations} (>2s)\n"
                ));
            }

            report.push('\n');
        }

        // Agent Utilization
        let utilization_metrics = self
            .storage
            .load_agent_utilization_metrics(lookback_hours)
            .await?;
        if !utilization_metrics.is_empty() {
            report.push_str("🤖 AGENT UTILIZATION\n");

            let mut agent_stats = HashMap::new();
            for metric in &utilization_metrics {
                let entry = agent_stats
                    .entry(metric.agent_id.clone())
                    .or_insert((Vec::new(), metric.max_capacity));
                entry.0.push(metric.utilization_percentage);
            }

            for (agent_id, (utilizations, max_capacity)) in agent_stats {
                let avg_utilization = utilizations.iter().sum::<f64>() / utilizations.len() as f64;
                let max_utilization = utilizations.iter().fold(0.0f64, |a, &b| a.max(b));

                report.push_str(&format!("   {agent_id}: {avg_utilization:.1}% avg, {max_utilization:.1}% max (capacity: {max_capacity})\n"));
            }
            report.push('\n');
        }

        // Coordination Decisions
        let decisions = self
            .storage
            .load_coordination_decisions(lookback_hours)
            .await?;
        if !decisions.is_empty() {
            report.push_str("🎯 COORDINATION DECISIONS\n");

            let total_decisions = decisions.len();
            let successful_decisions = decisions.iter().filter(|d| d.success).count();
            let success_rate = successful_decisions as f64 / total_decisions as f64 * 100.0;

            let avg_execution_time: f64 = decisions
                .iter()
                .map(|d| d.execution_time_ms as f64)
                .sum::<f64>()
                / decisions.len() as f64;

            report.push_str(&format!("   Total Decisions:   {total_decisions}\n"));
            report.push_str(&format!("   Success Rate:      {success_rate:.1}%\n"));
            report.push_str(&format!(
                "   Avg Execution:     {avg_execution_time:.0}ms\n"
            ));

            // Operation breakdown
            let mut operation_counts = HashMap::new();
            for decision in &decisions {
                *operation_counts
                    .entry(decision.operation.clone())
                    .or_insert(0) += 1;
            }

            if !operation_counts.is_empty() {
                report.push_str("   Operations:\n");
                for (operation, count) in operation_counts {
                    report.push_str(&format!("     • {operation}: {count}\n"));
                }
            }

            report.push('\n');
        }

        // Performance Bottlenecks
        let bottlenecks = self
            .storage
            .load_performance_bottlenecks(lookback_hours)
            .await?;
        if !bottlenecks.is_empty() {
            report.push_str("🚨 PERFORMANCE BOTTLENECKS\n");

            let critical_count = bottlenecks
                .iter()
                .filter(|b| matches!(b.severity, BottleneckSeverity::Critical))
                .count();
            let high_count = bottlenecks
                .iter()
                .filter(|b| matches!(b.severity, BottleneckSeverity::High))
                .count();

            if critical_count > 0 {
                report.push_str(&format!("   🔴 Critical: {critical_count}\n"));
            }
            if high_count > 0 {
                report.push_str(&format!("   🟡 High:     {high_count}\n"));
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

                report.push_str(&format!(
                    "   {} {}\n",
                    severity_icon, bottleneck.description
                ));
                report.push_str(&format!("     → {}\n", bottleneck.suggested_action));
            }

            report.push('\n');
        }

        // System Health Summary
        report.push_str("📊 SYSTEM HEALTH SUMMARY\n");

        let routing_health = if routing_metrics.is_empty() {
            "No data".to_string()
        } else {
            let avg_time: f64 = routing_metrics
                .iter()
                .map(|m| m.routing_duration_ms as f64)
                .sum::<f64>()
                / routing_metrics.len() as f64;

            if avg_time < 1000.0 {
                "🟢 EXCELLENT (<1s avg)".to_string()
            } else if avg_time < 2000.0 {
                "🟡 GOOD (<2s avg)".to_string()
            } else {
                "🔴 NEEDS ATTENTION (>2s avg)".to_string()
            }
        };

        report.push_str(&format!("   Routing Performance: {routing_health}\n"));

        if !bottlenecks.is_empty() {
            let critical_bottlenecks = bottlenecks
                .iter()
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
}
