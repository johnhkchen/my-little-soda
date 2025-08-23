//! Process Lifecycle Integration
//!
//! This module provides a high-level interface that integrates process management,
//! resource monitoring, and cleanup functionality into a cohesive system.

use std::sync::{Arc, Mutex};
use std::time::Duration;
use anyhow::Result;
use tracing::{info, warn, error};

use super::process_manager::{AgentProcessManager, ProcessManagerConfig, ResourceLimits};
use super::resource_monitor::{ResourceMonitor, AlertThresholds, Alert};

/// Comprehensive process lifecycle manager
#[derive(Debug)]
pub struct ProcessLifecycleManager {
    process_manager: Arc<Mutex<AgentProcessManager>>,
    resource_monitor: Arc<Mutex<ResourceMonitor>>,
    config: ProcessLifecycleConfig,
    monitoring_task: Option<tokio::task::JoinHandle<()>>,
}

/// Configuration for process lifecycle management
#[derive(Debug, Clone)]
pub struct ProcessLifecycleConfig {
    pub process_config: ProcessManagerConfig,
    pub alert_thresholds: AlertThresholds,
    pub enable_auto_cleanup: bool,
    pub enable_resource_alerts: bool,
    pub cleanup_failed_after_minutes: u64,
    pub cleanup_completed_after_minutes: u64,
    pub alert_check_interval_secs: u64,
}

impl Default for ProcessLifecycleConfig {
    fn default() -> Self {
        Self {
            process_config: ProcessManagerConfig::default(),
            alert_thresholds: AlertThresholds::default(),
            enable_auto_cleanup: true,
            enable_resource_alerts: true,
            cleanup_failed_after_minutes: 10,
            cleanup_completed_after_minutes: 5,
            alert_check_interval_secs: 30,
        }
    }
}

/// Process lifecycle events for external notification
#[derive(Debug, Clone)]
pub enum LifecycleEvent {
    ProcessSpawned {
        process_id: String,
        agent_id: String,
        issue_number: u64,
    },
    ProcessCompleted {
        process_id: String,
        agent_id: String,
        issue_number: u64,
        runtime_seconds: u64,
    },
    ProcessFailed {
        process_id: String,
        agent_id: String,
        issue_number: u64,
        error: String,
    },
    ProcessTerminated {
        process_id: String,
        reason: String,
    },
    ResourceAlert {
        alert: Alert,
    },
    ProcessCleanedUp {
        process_id: String,
        reason: String,
    },
}

/// Lifecycle event handler trait
pub trait LifecycleEventHandler: Send + Sync {
    fn handle_event(&self, event: LifecycleEvent);
}

/// Default logging event handler
pub struct LoggingEventHandler;

impl LifecycleEventHandler for LoggingEventHandler {
    fn handle_event(&self, event: LifecycleEvent) {
        match event {
            LifecycleEvent::ProcessSpawned { process_id, agent_id, issue_number } => {
                info!(
                    process_id = %process_id,
                    agent_id = %agent_id,
                    issue_number = %issue_number,
                    "Agent process spawned"
                );
            }
            LifecycleEvent::ProcessCompleted { process_id, agent_id, issue_number, runtime_seconds } => {
                info!(
                    process_id = %process_id,
                    agent_id = %agent_id,
                    issue_number = %issue_number,
                    runtime_seconds = %runtime_seconds,
                    "Agent process completed successfully"
                );
            }
            LifecycleEvent::ProcessFailed { process_id, agent_id, issue_number, error } => {
                error!(
                    process_id = %process_id,
                    agent_id = %agent_id,
                    issue_number = %issue_number,
                    error = %error,
                    "Agent process failed"
                );
            }
            LifecycleEvent::ProcessTerminated { process_id, reason } => {
                warn!(
                    process_id = %process_id,
                    reason = %reason,
                    "Agent process terminated"
                );
            }
            LifecycleEvent::ResourceAlert { alert } => {
                match alert.severity {
                    super::resource_monitor::AlertSeverity::Critical |
                    super::resource_monitor::AlertSeverity::Emergency => {
                        error!(
                            alert_id = %alert.id,
                            severity = ?alert.severity,
                            message = %alert.message,
                            "Critical resource alert"
                        );
                    }
                    super::resource_monitor::AlertSeverity::Warning => {
                        warn!(
                            alert_id = %alert.id,
                            severity = ?alert.severity,
                            message = %alert.message,
                            "Resource warning alert"
                        );
                    }
                    super::resource_monitor::AlertSeverity::Info => {
                        info!(
                            alert_id = %alert.id,
                            severity = ?alert.severity,
                            message = %alert.message,
                            "Resource info alert"
                        );
                    }
                }
            }
            LifecycleEvent::ProcessCleanedUp { process_id, reason } => {
                info!(
                    process_id = %process_id,
                    reason = %reason,
                    "Process cleaned up"
                );
            }
        }
    }
}

impl ProcessLifecycleManager {
    /// Create new process lifecycle manager
    pub async fn new(config: ProcessLifecycleConfig) -> Result<Self> {
        let mut process_manager = AgentProcessManager::new(config.process_config.clone());
        process_manager.start_background_tasks().await;
        
        let resource_monitor = ResourceMonitor::new(config.alert_thresholds.clone());

        Ok(Self {
            process_manager: Arc::new(Mutex::new(process_manager)),
            resource_monitor: Arc::new(Mutex::new(resource_monitor)),
            config,
            monitoring_task: None,
        })
    }

    /// Start the integrated monitoring and cleanup system
    pub async fn start(&mut self, event_handler: Arc<dyn LifecycleEventHandler>) -> Result<()> {
        let process_manager = Arc::clone(&self.process_manager);
        let resource_monitor = Arc::clone(&self.resource_monitor);
        let config = self.config.clone();

        self.monitoring_task = Some(tokio::spawn(async move {
            Self::monitoring_loop(process_manager, resource_monitor, config, event_handler).await;
        }));

        info!("Process lifecycle management started");
        Ok(())
    }

    /// Spawn new agent process with full lifecycle management
    pub async fn spawn_agent(
        &self,
        agent_id: &str,
        issue_number: u64,
        branch_name: String,
        limits: Option<ResourceLimits>,
        event_handler: Arc<dyn LifecycleEventHandler>,
    ) -> Result<String> {
        let process_id = {
            let process_manager = self.process_manager.lock().unwrap();
            process_manager.spawn_agent(agent_id, issue_number, branch_name, limits).await?
        };

        // Emit spawn event
        event_handler.handle_event(LifecycleEvent::ProcessSpawned {
            process_id: process_id.clone(),
            agent_id: agent_id.to_string(),
            issue_number,
        });

        Ok(process_id)
    }

    /// Terminate agent process with lifecycle event
    pub fn terminate_agent(
        &self,
        process_id: &str,
        reason: &str,
        event_handler: Arc<dyn LifecycleEventHandler>,
    ) -> Result<()> {
        {
            let process_manager = self.process_manager.lock().unwrap();
            process_manager.terminate_agent_sync(process_id, reason)?;
        }

        // Emit termination event
        event_handler.handle_event(LifecycleEvent::ProcessTerminated {
            process_id: process_id.to_string(),
            reason: reason.to_string(),
        });

        Ok(())
    }

    /// Get comprehensive system status
    pub fn get_system_status(&self) -> SystemStatus {
        let active_processes = {
            let process_manager = self.process_manager.lock().unwrap();
            process_manager.list_active_processes()
        };

        let resource_summary = {
            let process_manager = self.process_manager.lock().unwrap();
            process_manager.get_resource_summary()
        };

        let active_alerts = {
            let resource_monitor = self.resource_monitor.lock().unwrap();
            resource_monitor.get_active_alerts().len()
        };

        let usage_report = {
            let resource_monitor = self.resource_monitor.lock().unwrap();
            resource_monitor.generate_usage_report(&active_processes)
        };

        SystemStatus {
            active_process_count: active_processes.len(),
            resource_summary,
            active_alert_count: active_alerts,
            usage_report,
        }
    }

    /// Main monitoring loop for resource alerts and cleanup
    async fn monitoring_loop(
        process_manager: Arc<Mutex<AgentProcessManager>>,
        resource_monitor: Arc<Mutex<ResourceMonitor>>,
        config: ProcessLifecycleConfig,
        event_handler: Arc<dyn LifecycleEventHandler>,
    ) {
        let mut interval = tokio::time::interval(Duration::from_secs(config.alert_check_interval_secs));

        loop {
            interval.tick().await;

            // Get current processes
            let active_processes = {
                let pm = process_manager.lock().unwrap();
                pm.list_active_processes()
            };

            // Check for resource alerts
            if config.enable_resource_alerts {
                let alerts = {
                    let mut rm = resource_monitor.lock().unwrap();
                    rm.monitor_resources(&active_processes)
                };

                for alert in alerts {
                    event_handler.handle_event(LifecycleEvent::ResourceAlert { alert });
                }
            }

            // Perform automatic cleanup
            if config.enable_auto_cleanup {
                Self::perform_cleanup(
                    &process_manager,
                    &active_processes,
                    &config,
                    &event_handler,
                ).await;
            }
        }
    }

    /// Perform automatic cleanup of completed/failed processes
    async fn perform_cleanup(
        process_manager: &Arc<Mutex<AgentProcessManager>>,
        processes: &[super::process_manager::AgentProcess],
        config: &ProcessLifecycleConfig,
        event_handler: &Arc<dyn LifecycleEventHandler>,
    ) {
        // Collect cleanup candidates first
        let mut cleanup_tasks = Vec::new();
        
        for process in processes {
            let should_cleanup = match &process.status {
                super::process_manager::ProcessStatus::Completed { completed_at, .. } => {
                    completed_at.elapsed().as_secs() > config.cleanup_completed_after_minutes * 60
                }
                super::process_manager::ProcessStatus::Failed { failed_at, .. } => {
                    failed_at.elapsed().as_secs() > config.cleanup_failed_after_minutes * 60
                }
                super::process_manager::ProcessStatus::TimedOut { .. } => true,
                _ => false,
            };

            if should_cleanup {
                let reason = match &process.status {
                    super::process_manager::ProcessStatus::Completed { .. } => "Completed process cleanup",
                    super::process_manager::ProcessStatus::Failed { .. } => "Failed process cleanup",
                    super::process_manager::ProcessStatus::TimedOut { .. } => "Timed out process cleanup",
                    _ => "Unknown cleanup reason",
                };

                cleanup_tasks.push((process.process_id.clone(), reason.to_string()));
            }
        }

        // Now perform cleanup
        for (process_id, reason) in cleanup_tasks {
            let result = {
                let pm = process_manager.lock().unwrap();
                pm.terminate_agent_sync(&process_id, &reason)
            };
            
            if let Err(e) = result {
                error!(
                    process_id = %process_id,
                    error = %e,
                    "Failed to cleanup process"
                );
            } else {
                event_handler.handle_event(LifecycleEvent::ProcessCleanedUp {
                    process_id: process_id.clone(),
                    reason: reason.clone(),
                });
            }
        }
    }
}

/// System status summary
#[derive(Debug, Clone)]
pub struct SystemStatus {
    pub active_process_count: usize,
    pub resource_summary: super::process_manager::ResourceSummary,
    pub active_alert_count: usize,
    pub usage_report: super::resource_monitor::ResourceUsageReport,
}

impl Drop for ProcessLifecycleManager {
    fn drop(&mut self) {
        if let Some(handle) = self.monitoring_task.take() {
            handle.abort();
        }
    }
}