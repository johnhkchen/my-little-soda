use serde::{Serialize, Deserialize};
use std::collections::HashMap;

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