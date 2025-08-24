//! Resource Usage Reporting and Alerting
//!
//! This module provides comprehensive reporting of agent resource usage patterns
//! and implements alerting when resource limits are approached or exceeded.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};
use tracing::info;

use super::process_manager::{AgentProcess, ResourceSummary, ResourceUsage};

/// Alert severity levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}

/// Resource alert types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceAlert {
    MemoryLimitApproached {
        process_id: String,
        current_mb: f64,
        limit_mb: u64,
        usage_percentage: f32,
    },
    MemoryLimitExceeded {
        process_id: String,
        current_mb: f64,
        limit_mb: u64,
        excess_mb: f64,
    },
    CpuLimitApproached {
        process_id: String,
        current_percent: f64,
        limit_percent: f64,
        usage_percentage: f32,
    },
    CpuLimitExceeded {
        process_id: String,
        current_percent: f64,
        limit_percent: f64,
        excess_percent: f64,
    },
    FileDescriptorLimitApproached {
        process_id: String,
        current_count: u64,
        limit_count: u64,
        usage_percentage: f32,
    },
    ProcessTimeout {
        process_id: String,
        runtime_minutes: u64,
        limit_minutes: u64,
    },
    SystemResourceExhaustion {
        total_memory_mb: f64,
        total_cpu_percent: f64,
        active_process_count: usize,
        max_process_limit: usize,
    },
    ProcessUnresponsive {
        process_id: String,
        last_activity_minutes: u64,
    },
}

/// Alert with metadata
#[derive(Debug, Clone)]
pub struct Alert {
    pub id: String,
    pub severity: AlertSeverity,
    pub alert_type: ResourceAlert,
    pub triggered_at: Instant,
    pub acknowledged: bool,
    pub resolved: bool,
    pub message: String,
}

/// Resource usage trend over time
#[derive(Debug, Clone)]
pub struct ResourceTrend {
    pub timestamp: Instant,
    pub memory_mb: f64,
    pub cpu_percent: f64,
    pub file_descriptors: u64,
}

/// Historical resource usage tracking
#[derive(Debug, Clone)]
pub struct ResourceHistory {
    pub process_id: String,
    pub samples: VecDeque<ResourceTrend>,
    pub max_samples: usize,
}

impl ResourceHistory {
    pub fn new(process_id: String, max_samples: usize) -> Self {
        Self {
            process_id,
            samples: VecDeque::with_capacity(max_samples),
            max_samples,
        }
    }

    pub fn add_sample(&mut self, usage: &ResourceUsage) {
        if self.samples.len() >= self.max_samples {
            self.samples.pop_front();
        }

        self.samples.push_back(ResourceTrend {
            timestamp: usage.last_updated,
            memory_mb: usage.memory_mb,
            cpu_percent: usage.cpu_percent,
            file_descriptors: usage.file_descriptors,
        });
    }

    pub fn get_memory_trend(&self, duration: Duration) -> Vec<f64> {
        let cutoff = Instant::now() - duration;
        self.samples
            .iter()
            .filter(|sample| sample.timestamp >= cutoff)
            .map(|sample| sample.memory_mb)
            .collect()
    }

    pub fn get_cpu_trend(&self, duration: Duration) -> Vec<f64> {
        let cutoff = Instant::now() - duration;
        self.samples
            .iter()
            .filter(|sample| sample.timestamp >= cutoff)
            .map(|sample| sample.cpu_percent)
            .collect()
    }
}

/// Comprehensive resource monitoring and alerting
#[derive(Debug)]
pub struct ResourceMonitor {
    active_alerts: HashMap<String, Alert>,
    resource_histories: HashMap<String, ResourceHistory>,
    alert_thresholds: AlertThresholds,
    max_history_samples: usize,
    alert_cooldown: Duration,
    last_alert_times: HashMap<String, Instant>,
}

/// Alert threshold configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    pub memory_warning_percent: f32,
    pub memory_critical_percent: f32,
    pub cpu_warning_percent: f32,
    pub cpu_critical_percent: f32,
    pub fd_warning_percent: f32,
    pub fd_critical_percent: f32,
    pub unresponsive_minutes: u64,
    pub system_memory_limit_mb: f64,
    pub system_cpu_limit_percent: f64,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            memory_warning_percent: 80.0,
            memory_critical_percent: 95.0,
            cpu_warning_percent: 75.0,
            cpu_critical_percent: 90.0,
            fd_warning_percent: 80.0,
            fd_critical_percent: 95.0,
            unresponsive_minutes: 5,
            system_memory_limit_mb: 4096.0, // 4GB system limit
            system_cpu_limit_percent: 80.0,
        }
    }
}

impl ResourceMonitor {
    /// Create new resource monitor with configuration
    pub fn new(thresholds: AlertThresholds) -> Self {
        Self {
            active_alerts: HashMap::new(),
            resource_histories: HashMap::new(),
            alert_thresholds: thresholds,
            max_history_samples: 1000, // Keep last 1000 samples
            alert_cooldown: Duration::from_secs(300), // 5 minute cooldown
            last_alert_times: HashMap::new(),
        }
    }

    /// Monitor resource usage and generate alerts
    pub fn monitor_resources(&mut self, processes: &[AgentProcess]) -> Vec<Alert> {
        let mut new_alerts = Vec::new();

        // Monitor individual processes
        for process in processes {
            self.update_resource_history(&process.process_id, &process.resource_usage);

            if let Some(alerts) = self.check_process_alerts(process) {
                new_alerts.extend(alerts);
            }
        }

        // Monitor system-wide resource usage
        let system_summary = self.calculate_system_summary(processes);
        if let Some(system_alert) = self.check_system_alerts(&system_summary) {
            new_alerts.push(system_alert);
        }

        // Clean up resolved alerts
        self.cleanup_resolved_alerts(processes);

        new_alerts
    }

    /// Update resource usage history for a process
    fn update_resource_history(&mut self, process_id: &str, usage: &ResourceUsage) {
        let history = self
            .resource_histories
            .entry(process_id.to_string())
            .or_insert_with(|| {
                ResourceHistory::new(process_id.to_string(), self.max_history_samples)
            });

        history.add_sample(usage);
    }

    /// Check for alerts related to individual process
    fn check_process_alerts(&mut self, process: &AgentProcess) -> Option<Vec<Alert>> {
        let mut alerts = Vec::new();
        let process_id = &process.process_id;

        // Skip alerting if in cooldown period
        if let Some(&last_alert) = self.last_alert_times.get(process_id) {
            if last_alert.elapsed() < self.alert_cooldown {
                return None;
            }
        }

        // Memory alerts
        let memory_usage_percent =
            (process.resource_usage.memory_mb / process.limits.max_memory_mb as f64 * 100.0) as f32;

        if memory_usage_percent >= self.alert_thresholds.memory_critical_percent {
            let alert = self.create_alert(
                AlertSeverity::Critical,
                ResourceAlert::MemoryLimitExceeded {
                    process_id: process_id.clone(),
                    current_mb: process.resource_usage.memory_mb,
                    limit_mb: process.limits.max_memory_mb,
                    excess_mb: process.resource_usage.memory_mb
                        - process.limits.max_memory_mb as f64,
                },
                format!(
                    "Process {} exceeded memory limit: {:.1}MB > {}MB ({:.1}%)",
                    process_id,
                    process.resource_usage.memory_mb,
                    process.limits.max_memory_mb,
                    memory_usage_percent
                ),
            );
            alerts.push(alert);
        } else if memory_usage_percent >= self.alert_thresholds.memory_warning_percent {
            let alert = self.create_alert(
                AlertSeverity::Warning,
                ResourceAlert::MemoryLimitApproached {
                    process_id: process_id.clone(),
                    current_mb: process.resource_usage.memory_mb,
                    limit_mb: process.limits.max_memory_mb,
                    usage_percentage: memory_usage_percent,
                },
                format!(
                    "Process {} approaching memory limit: {:.1}MB ({:.1}% of {}MB)",
                    process_id,
                    process.resource_usage.memory_mb,
                    memory_usage_percent,
                    process.limits.max_memory_mb
                ),
            );
            alerts.push(alert);
        }

        // CPU alerts
        let cpu_usage_percent =
            (process.resource_usage.cpu_percent / process.limits.max_cpu_percent * 100.0) as f32;

        if cpu_usage_percent >= self.alert_thresholds.cpu_critical_percent {
            let alert = self.create_alert(
                AlertSeverity::Critical,
                ResourceAlert::CpuLimitExceeded {
                    process_id: process_id.clone(),
                    current_percent: process.resource_usage.cpu_percent,
                    limit_percent: process.limits.max_cpu_percent,
                    excess_percent: process.resource_usage.cpu_percent
                        - process.limits.max_cpu_percent,
                },
                format!(
                    "Process {} exceeded CPU limit: {:.1}% > {:.1}% ({:.1}% of limit)",
                    process_id,
                    process.resource_usage.cpu_percent,
                    process.limits.max_cpu_percent,
                    cpu_usage_percent
                ),
            );
            alerts.push(alert);
        } else if cpu_usage_percent >= self.alert_thresholds.cpu_warning_percent {
            let alert = self.create_alert(
                AlertSeverity::Warning,
                ResourceAlert::CpuLimitApproached {
                    process_id: process_id.clone(),
                    current_percent: process.resource_usage.cpu_percent,
                    limit_percent: process.limits.max_cpu_percent,
                    usage_percentage: cpu_usage_percent,
                },
                format!(
                    "Process {} approaching CPU limit: {:.1}% ({:.1}% of {:.1}%)",
                    process_id,
                    process.resource_usage.cpu_percent,
                    cpu_usage_percent,
                    process.limits.max_cpu_percent
                ),
            );
            alerts.push(alert);
        }

        // File descriptor alerts
        let fd_usage_percent = (process.resource_usage.file_descriptors as f64
            / process.limits.max_file_descriptors as f64
            * 100.0) as f32;

        if fd_usage_percent >= self.alert_thresholds.fd_critical_percent {
            let alert = self.create_alert(
                AlertSeverity::Warning,
                ResourceAlert::FileDescriptorLimitApproached {
                    process_id: process_id.clone(),
                    current_count: process.resource_usage.file_descriptors,
                    limit_count: process.limits.max_file_descriptors,
                    usage_percentage: fd_usage_percent,
                },
                format!(
                    "Process {} approaching file descriptor limit: {} ({:.1}% of {})",
                    process_id,
                    process.resource_usage.file_descriptors,
                    fd_usage_percent,
                    process.limits.max_file_descriptors
                ),
            );
            alerts.push(alert);
        }

        // Process timeout alerts
        let runtime_minutes = process.started_at.elapsed().as_secs() / 60;
        if runtime_minutes >= process.limits.timeout_minutes {
            let alert = self.create_alert(
                AlertSeverity::Critical,
                ResourceAlert::ProcessTimeout {
                    process_id: process_id.clone(),
                    runtime_minutes,
                    limit_minutes: process.limits.timeout_minutes,
                },
                format!(
                    "Process {} exceeded timeout: {}min > {}min",
                    process_id, runtime_minutes, process.limits.timeout_minutes
                ),
            );
            alerts.push(alert);
        }

        // Process unresponsive alerts
        let inactivity_minutes = process.last_activity.elapsed().as_secs() / 60;
        if inactivity_minutes >= self.alert_thresholds.unresponsive_minutes {
            let alert = self.create_alert(
                AlertSeverity::Warning,
                ResourceAlert::ProcessUnresponsive {
                    process_id: process_id.clone(),
                    last_activity_minutes: inactivity_minutes,
                },
                format!(
                    "Process {process_id} appears unresponsive: no activity for {inactivity_minutes}min"
                ),
            );
            alerts.push(alert);
        }

        if !alerts.is_empty() {
            self.last_alert_times
                .insert(process_id.clone(), Instant::now());
            Some(alerts)
        } else {
            None
        }
    }

    /// Check for system-wide resource alerts
    fn check_system_alerts(&self, summary: &ResourceSummary) -> Option<Alert> {
        if summary.total_memory_mb > self.alert_thresholds.system_memory_limit_mb
            || summary.total_cpu_percent > self.alert_thresholds.system_cpu_limit_percent
            || summary.active_process_count >= summary.max_concurrent_limit
        {
            Some(self.create_alert(
                AlertSeverity::Critical,
                ResourceAlert::SystemResourceExhaustion {
                    total_memory_mb: summary.total_memory_mb,
                    total_cpu_percent: summary.total_cpu_percent,
                    active_process_count: summary.active_process_count,
                    max_process_limit: summary.max_concurrent_limit,
                },
                format!(
                    "System resources exhausted: Memory={:.1}MB, CPU={:.1}%, Processes={}/{}",
                    summary.total_memory_mb,
                    summary.total_cpu_percent,
                    summary.active_process_count,
                    summary.max_concurrent_limit
                ),
            ))
        } else {
            None
        }
    }

    /// Create new alert with unique ID
    fn create_alert(
        &self,
        severity: AlertSeverity,
        alert_type: ResourceAlert,
        message: String,
    ) -> Alert {
        let alert_id = format!("{:?}_{}", alert_type, Instant::now().elapsed().as_nanos());

        Alert {
            id: alert_id,
            severity,
            alert_type,
            triggered_at: Instant::now(),
            acknowledged: false,
            resolved: false,
            message,
        }
    }

    /// Calculate system resource summary
    fn calculate_system_summary(&self, processes: &[AgentProcess]) -> ResourceSummary {
        let active_processes: Vec<_> = processes
            .iter()
            .filter(|p| {
                matches!(
                    p.status,
                    super::process_manager::ProcessStatus::Running { .. }
                        | super::process_manager::ProcessStatus::Working { .. }
                )
            })
            .collect();

        ResourceSummary {
            active_process_count: active_processes.len(),
            total_memory_mb: active_processes
                .iter()
                .map(|p| p.resource_usage.memory_mb)
                .sum(),
            total_cpu_percent: active_processes
                .iter()
                .map(|p| p.resource_usage.cpu_percent)
                .sum(),
            total_file_descriptors: active_processes
                .iter()
                .map(|p| p.resource_usage.file_descriptors)
                .sum(),
            max_concurrent_limit: 5, // This should come from config
        }
    }

    /// Clean up resolved alerts
    fn cleanup_resolved_alerts(&mut self, _processes: &[AgentProcess]) {
        // Remove alerts for processes that no longer exist or are completed
        // This is a simplified implementation
        let cutoff = Instant::now() - Duration::from_secs(3600); // 1 hour

        self.active_alerts
            .retain(|_, alert| !alert.resolved && alert.triggered_at > cutoff);
    }

    /// Get resource usage report
    pub fn generate_usage_report(&self, processes: &[AgentProcess]) -> ResourceUsageReport {
        let summary = self.calculate_system_summary(processes);
        let active_alert_count = self.active_alerts.len();

        let process_reports: Vec<ProcessResourceReport> = processes
            .iter()
            .map(|process| {
                let history = self.resource_histories.get(&process.process_id);
                let memory_trend = history
                    .map(|h| h.get_memory_trend(Duration::from_secs(3600)))
                    .unwrap_or_default();
                let cpu_trend = history
                    .map(|h| h.get_cpu_trend(Duration::from_secs(3600)))
                    .unwrap_or_default();

                ProcessResourceReport {
                    process_id: process.process_id.clone(),
                    agent_id: process.agent_id.clone(),
                    issue_number: process.issue_number,
                    runtime_seconds: process.started_at.elapsed().as_secs(),
                    current_usage: process.resource_usage.clone(),
                    limits: process.limits.clone(),
                    memory_trend_1h: memory_trend,
                    cpu_trend_1h: cpu_trend,
                    status: process.status.clone(),
                }
            })
            .collect();

        ResourceUsageReport {
            generated_at: Instant::now(),
            system_summary: summary,
            active_alert_count,
            process_reports,
        }
    }

    /// Get active alerts
    pub fn get_active_alerts(&self) -> Vec<&Alert> {
        self.active_alerts.values().collect()
    }

    /// Acknowledge alert
    pub fn acknowledge_alert(&mut self, alert_id: &str) -> Result<()> {
        if let Some(alert) = self.active_alerts.get_mut(alert_id) {
            alert.acknowledged = true;
            info!(alert_id = %alert_id, "Alert acknowledged");
            Ok(())
        } else {
            Err(anyhow!("Alert {} not found", alert_id))
        }
    }

    /// Resolve alert
    pub fn resolve_alert(&mut self, alert_id: &str) -> Result<()> {
        if let Some(alert) = self.active_alerts.get_mut(alert_id) {
            alert.resolved = true;
            info!(alert_id = %alert_id, "Alert resolved");
            Ok(())
        } else {
            Err(anyhow!("Alert {} not found", alert_id))
        }
    }
}

/// Comprehensive resource usage report
#[derive(Debug, Clone)]
pub struct ResourceUsageReport {
    pub generated_at: Instant,
    pub system_summary: ResourceSummary,
    pub active_alert_count: usize,
    pub process_reports: Vec<ProcessResourceReport>,
}

/// Individual process resource report
#[derive(Debug, Clone)]
pub struct ProcessResourceReport {
    pub process_id: String,
    pub agent_id: String,
    pub issue_number: u64,
    pub runtime_seconds: u64,
    pub current_usage: ResourceUsage,
    pub limits: super::process_manager::ResourceLimits,
    pub memory_trend_1h: Vec<f64>,
    pub cpu_trend_1h: Vec<f64>,
    pub status: super::process_manager::ProcessStatus,
}
