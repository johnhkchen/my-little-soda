use crate::github::GitHubError;
use crate::telemetry::generate_correlation_id;
use super::types::*;
use super::storage::MetricsStorage;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};

#[derive(Debug)]
pub struct MetricsTracker {
    pub(super) storage: MetricsStorage,
}

impl MetricsTracker {
    pub fn new() -> Self {
        Self {
            storage: MetricsStorage::new(),
        }
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

        self.storage.store_routing_metrics(metrics).await?;
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

        self.storage.store_agent_utilization_metrics(metrics).await?;
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

        self.storage.store_coordination_decision(decision).await?;
        Ok(())
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
        
        self.storage.store_integration_attempt(attempt).await?;

        Ok(())
    }

    pub async fn detect_and_store_bottlenecks(&self) -> Result<Vec<PerformanceBottleneck>, GitHubError> {
        let detector = super::bottleneck::BottleneckDetector::new();
        detector.detect_and_store_bottlenecks().await
    }

    fn detect_routing_bottlenecks(
        &self,
        routing_duration_ms: u64,
        issues_evaluated: u64,
        agents_available: u64,
        decision_outcome: &RoutingDecision,
    ) -> Vec<String> {
        super::bottleneck::BottleneckDetector::detect_routing_bottlenecks(
            routing_duration_ms,
            issues_evaluated,
            agents_available,
            decision_outcome,
        )
    }
}