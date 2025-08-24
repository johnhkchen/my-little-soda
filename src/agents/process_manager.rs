//! Agent Process Management with Resource Limits and Monitoring
//!
//! This module implements production-grade resource management for Claude Code agent processes,
//! including memory/CPU limits, health monitoring, and automatic cleanup.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::process::{Child, Command};
use tracing::{debug, error, info, warn};

/// Resource limits for agent processes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory usage in MB
    pub max_memory_mb: u64,
    /// Maximum CPU usage percentage (0-100)
    pub max_cpu_percent: f64,
    /// Process timeout in minutes
    pub timeout_minutes: u64,
    /// Maximum number of open file descriptors
    pub max_file_descriptors: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: 512,
            max_cpu_percent: 50.0,
            timeout_minutes: 30,
            max_file_descriptors: 1024,
        }
    }
}

/// Agent process information and status
#[derive(Debug, Clone)]
pub struct AgentProcess {
    pub process_id: String,
    pub system_pid: Option<u32>,
    pub agent_id: String,
    pub issue_number: u64,
    pub branch_name: String,
    pub status: ProcessStatus,
    pub started_at: Instant,
    pub last_activity: Instant,
    pub resource_usage: ResourceUsage,
    pub limits: ResourceLimits,
}

/// Process status tracking
#[derive(Debug, Clone)]
pub enum ProcessStatus {
    Starting,
    Running {
        last_heartbeat: Instant,
    },
    Working {
        task_description: String,
        progress_percent: Option<f32>,
    },
    Completing,
    Completed {
        exit_code: i32,
        completed_at: Instant,
    },
    Failed {
        error: String,
        failed_at: Instant,
    },
    TimedOut {
        duration: Duration,
    },
    Terminated {
        reason: String,
        terminated_at: Instant,
    },
}

/// Resource usage metrics
#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub memory_mb: f64,
    pub cpu_percent: f64,
    pub file_descriptors: u64,
    pub runtime_seconds: f64,
    pub last_updated: Instant,
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            memory_mb: 0.0,
            cpu_percent: 0.0,
            file_descriptors: 0,
            runtime_seconds: 0.0,
            last_updated: Instant::now(),
        }
    }
}

/// Process monitoring and cleanup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessManagerConfig {
    pub claude_code_path: String,
    pub max_concurrent_agents: usize,
    pub monitoring_interval_secs: u64,
    pub cleanup_interval_secs: u64,
    pub default_limits: ResourceLimits,
    pub enable_resource_monitoring: bool,
    pub enable_automatic_cleanup: bool,
}

impl Default for ProcessManagerConfig {
    fn default() -> Self {
        Self {
            claude_code_path: "claude-code".to_string(),
            max_concurrent_agents: 1,
            monitoring_interval_secs: 10,
            cleanup_interval_secs: 60,
            default_limits: ResourceLimits::default(),
            enable_resource_monitoring: true,
            enable_automatic_cleanup: true,
        }
    }
}

/// Agent process manager with resource limits and monitoring
#[derive(Debug)]
pub struct AgentProcessManager {
    processes: Arc<Mutex<HashMap<String, AgentProcess>>>,
    config: ProcessManagerConfig,
    system_processes: Arc<Mutex<HashMap<String, Child>>>,
    monitoring_handle: Option<tokio::task::JoinHandle<()>>,
    cleanup_handle: Option<tokio::task::JoinHandle<()>>,
}

impl AgentProcessManager {
    /// Create new process manager with configuration
    pub fn new(config: ProcessManagerConfig) -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            config,
            system_processes: Arc::new(Mutex::new(HashMap::new())),
            monitoring_handle: None,
            cleanup_handle: None,
        }
    }

    /// Start background monitoring and cleanup tasks
    pub async fn start_background_tasks(&mut self) {
        if self.config.enable_resource_monitoring {
            let processes = Arc::clone(&self.processes);
            let system_processes = Arc::clone(&self.system_processes);
            let interval = Duration::from_secs(self.config.monitoring_interval_secs);

            self.monitoring_handle = Some(tokio::spawn(async move {
                Self::monitoring_task(processes, system_processes, interval).await;
            }));

            info!(
                "Started resource monitoring task with {}s interval",
                self.config.monitoring_interval_secs
            );
        }

        if self.config.enable_automatic_cleanup {
            let processes = Arc::clone(&self.processes);
            let system_processes = Arc::clone(&self.system_processes);
            let interval = Duration::from_secs(self.config.cleanup_interval_secs);

            self.cleanup_handle = Some(tokio::spawn(async move {
                Self::cleanup_task(processes, system_processes, interval).await;
            }));

            info!(
                "Started automatic cleanup task with {}s interval",
                self.config.cleanup_interval_secs
            );
        }
    }

    /// Spawn new Claude Code agent process with resource limits
    pub async fn spawn_agent(
        &self,
        agent_id: &str,
        issue_number: u64,
        branch_name: String,
        limits: Option<ResourceLimits>,
    ) -> Result<String> {
        let mut processes = self.processes.lock().unwrap();

        // Check concurrent agent limit
        let active_count = processes
            .values()
            .filter(|p| {
                matches!(
                    p.status,
                    ProcessStatus::Starting
                        | ProcessStatus::Running { .. }
                        | ProcessStatus::Working { .. }
                )
            })
            .count();

        if active_count >= self.config.max_concurrent_agents {
            return Err(anyhow!(
                "Maximum concurrent agents ({}) exceeded. Currently running: {}",
                self.config.max_concurrent_agents,
                active_count
            ));
        }

        let process_id = format!("{agent_id}_{issue_number}");
        let limits = limits.unwrap_or_else(|| self.config.default_limits.clone());

        // Create process record
        let agent_process = AgentProcess {
            process_id: process_id.clone(),
            system_pid: None,
            agent_id: agent_id.to_string(),
            issue_number,
            branch_name: branch_name.clone(),
            status: ProcessStatus::Starting,
            started_at: Instant::now(),
            last_activity: Instant::now(),
            resource_usage: ResourceUsage::default(),
            limits: limits.clone(),
        };

        processes.insert(process_id.clone(), agent_process);
        drop(processes);

        // Spawn Claude Code process with resource limits
        let mut command = Command::new(&self.config.claude_code_path);
        command
            .arg("--issue")
            .arg(issue_number.to_string())
            .arg("--branch")
            .arg(&branch_name)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        // Apply memory limits (Linux-specific using systemd-run if available)
        #[cfg(target_os = "linux")]
        {
            if std::process::Command::new("systemd-run")
                .arg("--version")
                .output()
                .is_ok()
            {
                command = Command::new("systemd-run");
                command
                    .arg("--scope")
                    .arg("--user")
                    .arg(format!("--property=MemoryMax={}M", limits.max_memory_mb))
                    .arg(format!(
                        "--property=CPUQuota={}%",
                        limits.max_cpu_percent as u32
                    ))
                    .arg(&self.config.claude_code_path)
                    .arg("--issue")
                    .arg(issue_number.to_string())
                    .arg("--branch")
                    .arg(&branch_name);
            }
        }

        match command.spawn() {
            Ok(child) => {
                let system_pid = child.id();

                // Update process record with PID
                let mut processes = self.processes.lock().unwrap();
                if let Some(process) = processes.get_mut(&process_id) {
                    process.system_pid = system_pid;
                    process.status = ProcessStatus::Running {
                        last_heartbeat: Instant::now(),
                    };
                }
                drop(processes);

                // Store system process handle
                self.system_processes
                    .lock()
                    .unwrap()
                    .insert(process_id.clone(), child);

                info!(
                    agent_id = %agent_id,
                    issue_number = %issue_number,
                    process_id = %process_id,
                    system_pid = ?system_pid,
                    memory_limit_mb = %limits.max_memory_mb,
                    cpu_limit_percent = %limits.max_cpu_percent,
                    timeout_minutes = %limits.timeout_minutes,
                    "Agent process spawned with resource limits"
                );

                Ok(process_id)
            }
            Err(e) => {
                // Clean up failed process record
                self.processes.lock().unwrap().remove(&process_id);

                error!(
                    agent_id = %agent_id,
                    issue_number = %issue_number,
                    error = %e,
                    "Failed to spawn agent process"
                );

                Err(anyhow!("Failed to spawn agent process: {}", e))
            }
        }
    }

    /// Get status of specific agent process
    pub fn get_process_status(&self, process_id: &str) -> Option<AgentProcess> {
        self.processes.lock().unwrap().get(process_id).cloned()
    }

    /// List all active agent processes
    pub fn list_active_processes(&self) -> Vec<AgentProcess> {
        self.processes
            .lock()
            .unwrap()
            .values()
            .filter(|p| {
                !matches!(
                    p.status,
                    ProcessStatus::Completed { .. } | ProcessStatus::Failed { .. }
                )
            })
            .cloned()
            .collect()
    }

    /// Get resource usage summary
    pub fn get_resource_summary(&self) -> ResourceSummary {
        let processes = self.processes.lock().unwrap();
        let active_processes: Vec<_> = processes
            .values()
            .filter(|p| {
                matches!(
                    p.status,
                    ProcessStatus::Running { .. } | ProcessStatus::Working { .. }
                )
            })
            .collect();

        let total_memory_mb = active_processes
            .iter()
            .map(|p| p.resource_usage.memory_mb)
            .sum();

        let total_cpu_percent = active_processes
            .iter()
            .map(|p| p.resource_usage.cpu_percent)
            .sum();

        let total_file_descriptors = active_processes
            .iter()
            .map(|p| p.resource_usage.file_descriptors)
            .sum();

        ResourceSummary {
            active_process_count: active_processes.len(),
            total_memory_mb,
            total_cpu_percent,
            total_file_descriptors,
            max_concurrent_limit: self.config.max_concurrent_agents,
        }
    }

    /// Terminate agent process gracefully (simplified version)
    pub fn terminate_agent_sync(&self, process_id: &str, reason: &str) -> Result<()> {
        // Remove from system processes tracking
        {
            let mut system_processes = self.system_processes.lock().unwrap();
            if let Some(child) = system_processes.remove(process_id) {
                if let Some(pid) = child.id() {
                    info!(
                        process_id = %process_id,
                        system_pid = %pid,
                        reason = %reason,
                        "Terminating agent process"
                    );

                    // Attempt to kill the process using system commands
                    #[cfg(unix)]
                    {
                        use std::process::Command as StdCommand;
                        let _ = StdCommand::new("kill")
                            .arg("-TERM")
                            .arg(pid.to_string())
                            .output();
                    }

                    // For non-Unix systems or as fallback, we'll rely on the drop handler
                }
            }
        }

        // Update process status
        let mut processes = self.processes.lock().unwrap();
        if let Some(process) = processes.get_mut(process_id) {
            process.status = ProcessStatus::Terminated {
                reason: reason.to_string(),
                terminated_at: Instant::now(),
            };
        }

        Ok(())
    }

    /// Background monitoring task for resource usage and health
    async fn monitoring_task(
        processes: Arc<Mutex<HashMap<String, AgentProcess>>>,
        system_processes: Arc<Mutex<HashMap<String, Child>>>,
        interval: Duration,
    ) {
        let mut monitoring_interval = tokio::time::interval(interval);

        loop {
            monitoring_interval.tick().await;

            let process_ids: Vec<String> = {
                let processes = processes.lock().unwrap();
                processes.keys().cloned().collect()
            };

            for process_id in process_ids {
                if let Err(e) = Self::monitor_single_process(
                    Arc::clone(&processes),
                    Arc::clone(&system_processes),
                    &process_id,
                )
                .await
                {
                    debug!(
                        process_id = %process_id,
                        error = %e,
                        "Error monitoring process (may have completed)"
                    );
                }
            }
        }
    }

    /// Monitor individual process resource usage
    async fn monitor_single_process(
        processes: Arc<Mutex<HashMap<String, AgentProcess>>>,
        _system_processes: Arc<Mutex<HashMap<String, Child>>>,
        process_id: &str,
    ) -> Result<()> {
        let (system_pid, limits) = {
            let processes = processes.lock().unwrap();
            if let Some(process) = processes.get(process_id) {
                if let Some(pid) = process.system_pid {
                    (pid, process.limits.clone())
                } else {
                    return Ok(()); // Process not started yet
                }
            } else {
                return Ok(()); // Process not found
            }
        };

        // Get process resource usage (Linux-specific via /proc)
        #[cfg(target_os = "linux")]
        {
            let memory_mb = Self::get_process_memory_mb(system_pid)?;
            let cpu_percent = Self::get_process_cpu_percent(system_pid)?;
            let file_descriptors = Self::get_process_file_descriptors(system_pid)?;

            // Update resource usage
            {
                let mut processes = processes.lock().unwrap();
                if let Some(process) = processes.get_mut(process_id) {
                    process.resource_usage = ResourceUsage {
                        memory_mb,
                        cpu_percent,
                        file_descriptors,
                        runtime_seconds: process.started_at.elapsed().as_secs_f64(),
                        last_updated: Instant::now(),
                    };
                    process.last_activity = Instant::now();

                    // Check resource limits
                    if memory_mb > limits.max_memory_mb as f64 {
                        warn!(
                            process_id = %process_id,
                            system_pid = %system_pid,
                            memory_mb = %memory_mb,
                            limit_mb = %limits.max_memory_mb,
                            "Process exceeds memory limit"
                        );

                        process.status = ProcessStatus::Failed {
                            error: format!(
                                "Memory limit exceeded: {:.1}MB > {}MB",
                                memory_mb, limits.max_memory_mb
                            ),
                            failed_at: Instant::now(),
                        };
                    }

                    if cpu_percent > limits.max_cpu_percent {
                        warn!(
                            process_id = %process_id,
                            system_pid = %system_pid,
                            cpu_percent = %cpu_percent,
                            limit_percent = %limits.max_cpu_percent,
                            "Process exceeds CPU limit"
                        );
                    }

                    // Check timeout
                    if process.started_at.elapsed()
                        > Duration::from_secs(limits.timeout_minutes * 60)
                    {
                        warn!(
                            process_id = %process_id,
                            system_pid = %system_pid,
                            runtime_minutes = %process.started_at.elapsed().as_secs() / 60,
                            limit_minutes = %limits.timeout_minutes,
                            "Process exceeded timeout"
                        );

                        process.status = ProcessStatus::TimedOut {
                            duration: process.started_at.elapsed(),
                        };
                    }
                }
            }
        }

        Ok(())
    }

    /// Background cleanup task for completed/failed processes
    async fn cleanup_task(
        processes: Arc<Mutex<HashMap<String, AgentProcess>>>,
        system_processes: Arc<Mutex<HashMap<String, Child>>>,
        interval: Duration,
    ) {
        let mut cleanup_interval = tokio::time::interval(interval);

        loop {
            cleanup_interval.tick().await;

            let cleanup_candidates: Vec<String> = {
                let processes = processes.lock().unwrap();
                processes
                    .iter()
                    .filter_map(|(id, process)| {
                        match &process.status {
                            ProcessStatus::Completed { completed_at, .. } => {
                                if completed_at.elapsed() > Duration::from_secs(300) {
                                    // 5 minutes
                                    Some(id.clone())
                                } else {
                                    None
                                }
                            }
                            ProcessStatus::Failed { failed_at, .. } => {
                                if failed_at.elapsed() > Duration::from_secs(600) {
                                    // 10 minutes
                                    Some(id.clone())
                                } else {
                                    None
                                }
                            }
                            ProcessStatus::Terminated { terminated_at, .. } => {
                                if terminated_at.elapsed() > Duration::from_secs(300) {
                                    // 5 minutes
                                    Some(id.clone())
                                } else {
                                    None
                                }
                            }
                            ProcessStatus::TimedOut { .. } => Some(id.clone()),
                            _ => None,
                        }
                    })
                    .collect()
            };

            for process_id in cleanup_candidates {
                info!(
                    process_id = %process_id,
                    "Cleaning up completed/failed process"
                );

                // Remove from tracking
                processes.lock().unwrap().remove(&process_id);
                system_processes.lock().unwrap().remove(&process_id);
            }
        }
    }

    /// Get process memory usage in MB (Linux-specific)
    #[cfg(target_os = "linux")]
    fn get_process_memory_mb(pid: u32) -> Result<f64> {
        let status_path = format!("/proc/{pid}/status");
        let content = std::fs::read_to_string(&status_path)
            .map_err(|_| anyhow!("Process {} not found", pid))?;

        for line in content.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let kb = parts[1].parse::<f64>()?;
                    return Ok(kb / 1024.0); // Convert KB to MB
                }
            }
        }

        Ok(0.0)
    }

    /// Get process CPU usage percentage (Linux-specific)
    #[cfg(target_os = "linux")]
    fn get_process_cpu_percent(pid: u32) -> Result<f64> {
        // This is a simplified implementation
        // In production, you'd want to track CPU over time intervals
        let stat_path = format!("/proc/{pid}/stat");
        let content = std::fs::read_to_string(&stat_path)
            .map_err(|_| anyhow!("Process {} not found", pid))?;

        let parts: Vec<&str> = content.split_whitespace().collect();
        if parts.len() > 13 {
            let _utime: u64 = parts[13].parse().unwrap_or(0);
            let _stime: u64 = parts[14].parse().unwrap_or(0);
            // TODO: Calculate actual CPU percentage using time deltas
            return Ok(0.0); // Placeholder
        }

        Ok(0.0)
    }

    /// Get process file descriptor count (Linux-specific)
    #[cfg(target_os = "linux")]
    fn get_process_file_descriptors(pid: u32) -> Result<u64> {
        let fd_dir = format!("/proc/{pid}/fd");
        match std::fs::read_dir(&fd_dir) {
            Ok(entries) => Ok(entries.count() as u64),
            Err(_) => Ok(0), // Process may have exited
        }
    }

    // Stub implementations for non-Linux platforms
    #[cfg(not(target_os = "linux"))]
    fn get_process_memory_mb(_pid: u32) -> Result<f64> {
        Ok(0.0)
    }

    #[cfg(not(target_os = "linux"))]
    fn get_process_cpu_percent(_pid: u32) -> Result<f64> {
        Ok(0.0)
    }

    #[cfg(not(target_os = "linux"))]
    fn get_process_file_descriptors(_pid: u32) -> Result<u64> {
        Ok(0)
    }
}

/// Resource usage summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSummary {
    pub active_process_count: usize,
    pub total_memory_mb: f64,
    pub total_cpu_percent: f64,
    pub total_file_descriptors: u64,
    pub max_concurrent_limit: usize,
}

impl Drop for AgentProcessManager {
    fn drop(&mut self) {
        // Terminate background tasks
        if let Some(handle) = self.monitoring_handle.take() {
            handle.abort();
        }
        if let Some(handle) = self.cleanup_handle.take() {
            handle.abort();
        }
    }
}
