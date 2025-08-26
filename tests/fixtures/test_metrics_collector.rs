/// Performance metrics collection system for init command tests
/// 
/// This module provides real-time performance monitoring and metrics collection
/// during test execution, including memory usage, CPU usage, and timing measurements.

use super::test_result_reporting::TestPerformanceMetrics;
use anyhow::Result;
use std::time::{Instant, Duration};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

/// Real-time performance metrics collector
pub struct TestMetricsCollector {
    test_name: String,
    start_time: Option<Instant>,
    setup_start: Option<Instant>,
    setup_duration: Option<Duration>,
    teardown_start: Option<Instant>,
    teardown_duration: Option<Duration>,
    validation_start: Option<Instant>,
    validation_duration: Option<Duration>,
    memory_samples: Arc<Mutex<Vec<f64>>>,
    cpu_samples: Arc<Mutex<Vec<f64>>>,
    monitoring_active: Arc<Mutex<bool>>,
    monitoring_handle: Option<thread::JoinHandle<()>>,
}

impl TestMetricsCollector {
    /// Create a new metrics collector for a test
    pub fn new(test_name: String) -> Self {
        Self {
            test_name,
            start_time: None,
            setup_start: None,
            setup_duration: None,
            teardown_start: None,
            teardown_duration: None,
            validation_start: None,
            validation_duration: None,
            memory_samples: Arc::new(Mutex::new(Vec::new())),
            cpu_samples: Arc::new(Mutex::new(Vec::new())),
            monitoring_active: Arc::new(Mutex::new(false)),
            monitoring_handle: None,
        }
    }
    
    /// Start overall test timing and performance monitoring
    pub fn start_test(&mut self) -> Result<()> {
        self.start_time = Some(Instant::now());
        self.start_monitoring()?;
        Ok(())
    }
    
    /// Mark the beginning of test setup phase
    pub fn start_setup(&mut self) {
        self.setup_start = Some(Instant::now());
    }
    
    /// Mark the end of test setup phase
    pub fn end_setup(&mut self) {
        if let Some(start) = self.setup_start {
            self.setup_duration = Some(start.elapsed());
        }
    }
    
    /// Mark the beginning of validation phase
    pub fn start_validation(&mut self) {
        self.validation_start = Some(Instant::now());
    }
    
    /// Mark the end of validation phase
    pub fn end_validation(&mut self) {
        if let Some(start) = self.validation_start {
            self.validation_duration = Some(start.elapsed());
        }
    }
    
    /// Mark the beginning of teardown phase
    pub fn start_teardown(&mut self) {
        self.teardown_start = Some(Instant::now());
    }
    
    /// Mark the end of teardown phase
    pub fn end_teardown(&mut self) {
        if let Some(start) = self.teardown_start {
            self.teardown_duration = Some(start.elapsed());
        }
    }
    
    /// Stop monitoring and collect final metrics
    pub fn stop_test(&mut self) -> Result<TestPerformanceMetrics> {
        self.stop_monitoring()?;
        
        let execution_time = self.start_time
            .map(|start| start.elapsed())
            .unwrap_or_else(|| Duration::new(0, 0));
        
        let memory_samples = self.memory_samples.lock().unwrap();
        let cpu_samples = self.cpu_samples.lock().unwrap();
        
        let memory_usage_mb = if !memory_samples.is_empty() {
            Some(memory_samples.iter().sum::<f64>() / memory_samples.len() as f64)
        } else {
            None
        };
        
        let cpu_usage_percent = if !cpu_samples.is_empty() {
            Some(cpu_samples.iter().sum::<f64>() / cpu_samples.len() as f64)
        } else {
            None
        };
        
        Ok(TestPerformanceMetrics {
            test_name: self.test_name.clone(),
            execution_time,
            memory_usage_mb,
            cpu_usage_percent,
            setup_time: self.setup_duration.unwrap_or_else(|| Duration::new(0, 0)),
            teardown_time: self.teardown_duration.unwrap_or_else(|| Duration::new(0, 0)),
            validation_time: self.validation_duration.unwrap_or_else(|| Duration::new(0, 0)),
        })
    }
    
    /// Start background monitoring of system resources
    fn start_monitoring(&mut self) -> Result<()> {
        *self.monitoring_active.lock().unwrap() = true;
        
        let memory_samples = self.memory_samples.clone();
        let cpu_samples = self.cpu_samples.clone();
        let monitoring_active = self.monitoring_active.clone();
        
        self.monitoring_handle = Some(thread::spawn(move || {
            while *monitoring_active.lock().unwrap() {
                if let Ok(memory_usage) = Self::get_process_memory_usage() {
                    memory_samples.lock().unwrap().push(memory_usage);
                }
                
                if let Ok(cpu_usage) = Self::get_process_cpu_usage() {
                    cpu_samples.lock().unwrap().push(cpu_usage);
                }
                
                thread::sleep(Duration::from_millis(100));
            }
        }));
        
        Ok(())
    }
    
    /// Stop background monitoring
    fn stop_monitoring(&mut self) -> Result<()> {
        *self.monitoring_active.lock().unwrap() = false;
        
        if let Some(handle) = self.monitoring_handle.take() {
            handle.join().map_err(|_| anyhow::anyhow!("Failed to join monitoring thread"))?;
        }
        
        Ok(())
    }
    
    /// Get current process memory usage in MB
    fn get_process_memory_usage() -> Result<f64> {
        #[cfg(target_os = "linux")]
        {
            let pid = std::process::id();
            let status_path = format!("/proc/{}/status", pid);
            
            if let Ok(content) = std::fs::read_to_string(&status_path) {
                for line in content.lines() {
                    if line.starts_with("VmRSS:") {
                        if let Some(kb_str) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = kb_str.parse::<f64>() {
                                return Ok(kb / 1024.0); // Convert KB to MB
                            }
                        }
                    }
                }
            }
        }
        
        #[cfg(target_os = "macos")]
        {
            let output = Command::new("ps")
                .args(&["-o", "rss=", "-p", &std::process::id().to_string()])
                .output()?;
            
            if output.status.success() {
                let rss_str = String::from_utf8_lossy(&output.stdout).trim();
                if let Ok(rss_kb) = rss_str.parse::<f64>() {
                    return Ok(rss_kb / 1024.0); // Convert KB to MB
                }
            }
        }
        
        // Fallback: return 0 if we can't measure memory
        Ok(0.0)
    }
    
    /// Get current process CPU usage percentage
    fn get_process_cpu_usage() -> Result<f64> {
        #[cfg(unix)]
        {
            let pid = std::process::id();
            let output = Command::new("ps")
                .args(&["-o", "pcpu=", "-p", &pid.to_string()])
                .output()?;
            
            if output.status.success() {
                let cpu_output = String::from_utf8_lossy(&output.stdout);
                let cpu_str = cpu_output.trim();
                if let Ok(cpu_percent) = cpu_str.parse::<f64>() {
                    return Ok(cpu_percent);
                }
            }
        }
        
        // Fallback: return 0 if we can't measure CPU
        Ok(0.0)
    }
}

impl Drop for TestMetricsCollector {
    fn drop(&mut self) {
        let _ = self.stop_monitoring();
    }
}

/// Simplified metrics collector for basic timing measurements
pub struct SimpleTestTimer {
    test_name: String,
    start_time: Instant,
}

impl SimpleTestTimer {
    pub fn new(test_name: String) -> Self {
        Self {
            test_name,
            start_time: Instant::now(),
        }
    }
    
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
    
    pub fn finish(self) -> (String, Duration) {
        let test_name = self.test_name.clone();
        let duration = self.elapsed();
        (test_name, duration)
    }
}

/// Batch metrics collection for multiple tests
pub struct BatchMetricsCollector {
    test_metrics: Vec<TestPerformanceMetrics>,
    suite_start_time: Instant,
}

impl BatchMetricsCollector {
    pub fn new() -> Self {
        Self {
            test_metrics: Vec::new(),
            suite_start_time: Instant::now(),
        }
    }
    
    pub fn add_test_metrics(&mut self, metrics: TestPerformanceMetrics) {
        self.test_metrics.push(metrics);
    }
    
    pub fn get_suite_duration(&self) -> Duration {
        self.suite_start_time.elapsed()
    }
    
    pub fn get_all_metrics(&self) -> &[TestPerformanceMetrics] {
        &self.test_metrics
    }
    
    pub fn get_slowest_tests(&self, count: usize) -> Vec<&TestPerformanceMetrics> {
        let mut sorted = self.test_metrics.iter().collect::<Vec<_>>();
        sorted.sort_by(|a, b| b.execution_time.cmp(&a.execution_time));
        sorted.into_iter().take(count).collect()
    }
    
    pub fn get_average_execution_time(&self) -> Duration {
        if self.test_metrics.is_empty() {
            return Duration::new(0, 0);
        }
        
        let total: Duration = self.test_metrics.iter().map(|m| m.execution_time).sum();
        total / self.test_metrics.len() as u32
    }
    
    pub fn get_total_setup_time(&self) -> Duration {
        self.test_metrics.iter().map(|m| m.setup_time).sum()
    }
    
    pub fn get_total_validation_time(&self) -> Duration {
        self.test_metrics.iter().map(|m| m.validation_time).sum()
    }
    
    pub fn get_total_teardown_time(&self) -> Duration {
        self.test_metrics.iter().map(|m| m.teardown_time).sum()
    }
    
    /// Generate summary statistics
    pub fn generate_summary(&self) -> MetricsSummary {
        MetricsSummary {
            total_tests: self.test_metrics.len(),
            total_execution_time: self.test_metrics.iter().map(|m| m.execution_time).sum(),
            average_execution_time: self.get_average_execution_time(),
            slowest_test_time: self.test_metrics.iter().map(|m| m.execution_time).max().unwrap_or_default(),
            fastest_test_time: self.test_metrics.iter().map(|m| m.execution_time).min().unwrap_or_default(),
            total_setup_time: self.get_total_setup_time(),
            total_validation_time: self.get_total_validation_time(),
            total_teardown_time: self.get_total_teardown_time(),
            suite_duration: self.get_suite_duration(),
            average_memory_usage: self.get_average_memory_usage(),
            peak_memory_usage: self.get_peak_memory_usage(),
            average_cpu_usage: self.get_average_cpu_usage(),
            peak_cpu_usage: self.get_peak_cpu_usage(),
        }
    }
    
    fn get_average_memory_usage(&self) -> f64 {
        let samples: Vec<f64> = self.test_metrics.iter()
            .filter_map(|m| m.memory_usage_mb)
            .collect();
        
        if samples.is_empty() {
            0.0
        } else {
            samples.iter().sum::<f64>() / samples.len() as f64
        }
    }
    
    fn get_peak_memory_usage(&self) -> f64 {
        self.test_metrics.iter()
            .filter_map(|m| m.memory_usage_mb)
            .fold(0.0, |max, usage| if usage > max { usage } else { max })
    }
    
    fn get_average_cpu_usage(&self) -> f64 {
        let samples: Vec<f64> = self.test_metrics.iter()
            .filter_map(|m| m.cpu_usage_percent)
            .collect();
        
        if samples.is_empty() {
            0.0
        } else {
            samples.iter().sum::<f64>() / samples.len() as f64
        }
    }
    
    fn get_peak_cpu_usage(&self) -> f64 {
        self.test_metrics.iter()
            .filter_map(|m| m.cpu_usage_percent)
            .fold(0.0, |max, usage| if usage > max { usage } else { max })
    }
}

/// Summary of performance metrics across tests
#[derive(Debug, Clone)]
pub struct MetricsSummary {
    pub total_tests: usize,
    pub total_execution_time: Duration,
    pub average_execution_time: Duration,
    pub slowest_test_time: Duration,
    pub fastest_test_time: Duration,
    pub total_setup_time: Duration,
    pub total_validation_time: Duration,
    pub total_teardown_time: Duration,
    pub suite_duration: Duration,
    pub average_memory_usage: f64,
    pub peak_memory_usage: f64,
    pub average_cpu_usage: f64,
    pub peak_cpu_usage: f64,
}

impl MetricsSummary {
    /// Generate human-readable performance report
    pub fn generate_report(&self) -> String {
        format!(
            "Performance Metrics Summary\n\
             ===========================\n\
             Total Tests: {}\n\
             Suite Duration: {:.2}s\n\
             Total Execution Time: {:.2}s\n\
             Average Test Time: {:.2}s\n\
             Slowest Test: {:.2}s\n\
             Fastest Test: {:.2}s\n\
             \n\
             Phase Breakdown:\n\
             - Setup Time: {:.2}s ({:.1}%)\n\
             - Validation Time: {:.2}s ({:.1}%)\n\
             - Teardown Time: {:.2}s ({:.1}%)\n\
             \n\
             Resource Usage:\n\
             - Average Memory: {:.1} MB\n\
             - Peak Memory: {:.1} MB\n\
             - Average CPU: {:.1}%\n\
             - Peak CPU: {:.1}%\n",
            self.total_tests,
            self.suite_duration.as_secs_f64(),
            self.total_execution_time.as_secs_f64(),
            self.average_execution_time.as_secs_f64(),
            self.slowest_test_time.as_secs_f64(),
            self.fastest_test_time.as_secs_f64(),
            self.total_setup_time.as_secs_f64(),
            self.calculate_percentage(self.total_setup_time, self.total_execution_time),
            self.total_validation_time.as_secs_f64(),
            self.calculate_percentage(self.total_validation_time, self.total_execution_time),
            self.total_teardown_time.as_secs_f64(),
            self.calculate_percentage(self.total_teardown_time, self.total_execution_time),
            self.average_memory_usage,
            self.peak_memory_usage,
            self.average_cpu_usage,
            self.peak_cpu_usage,
        )
    }
    
    fn calculate_percentage(&self, part: Duration, total: Duration) -> f64 {
        if total.as_millis() == 0 {
            0.0
        } else {
            (part.as_millis() as f64 / total.as_millis() as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    #[test]
    fn test_simple_timer() {
        let timer = SimpleTestTimer::new("test_timer".to_string());
        std::thread::sleep(Duration::from_millis(10));
        let (name, duration) = timer.finish();
        
        assert_eq!(name, "test_timer");
        assert!(duration >= Duration::from_millis(10));
        assert!(duration < Duration::from_millis(50)); // Should be reasonably close
    }
    
    #[test]
    fn test_batch_metrics_collector() {
        let mut collector = BatchMetricsCollector::new();
        
        let metrics1 = TestPerformanceMetrics {
            test_name: "test1".to_string(),
            execution_time: Duration::from_millis(100),
            memory_usage_mb: Some(50.0),
            cpu_usage_percent: Some(25.0),
            setup_time: Duration::from_millis(10),
            teardown_time: Duration::from_millis(5),
            validation_time: Duration::from_millis(85),
        };
        
        let metrics2 = TestPerformanceMetrics {
            test_name: "test2".to_string(),
            execution_time: Duration::from_millis(200),
            memory_usage_mb: Some(75.0),
            cpu_usage_percent: Some(35.0),
            setup_time: Duration::from_millis(15),
            teardown_time: Duration::from_millis(10),
            validation_time: Duration::from_millis(175),
        };
        
        collector.add_test_metrics(metrics1);
        collector.add_test_metrics(metrics2);
        
        let summary = collector.generate_summary();
        assert_eq!(summary.total_tests, 2);
        assert_eq!(summary.total_execution_time, Duration::from_millis(300));
        assert_eq!(summary.average_execution_time, Duration::from_millis(150));
        assert_eq!(summary.slowest_test_time, Duration::from_millis(200));
        assert_eq!(summary.fastest_test_time, Duration::from_millis(100));
        assert_eq!(summary.average_memory_usage, 62.5);
        assert_eq!(summary.peak_memory_usage, 75.0);
        assert_eq!(summary.average_cpu_usage, 30.0);
        assert_eq!(summary.peak_cpu_usage, 35.0);
    }
    
    #[test]
    fn test_metrics_summary_report() {
        let summary = MetricsSummary {
            total_tests: 3,
            total_execution_time: Duration::from_secs(6),
            average_execution_time: Duration::from_secs(2),
            slowest_test_time: Duration::from_secs(3),
            fastest_test_time: Duration::from_secs(1),
            total_setup_time: Duration::from_millis(600),
            total_validation_time: Duration::from_millis(5100),
            total_teardown_time: Duration::from_millis(300),
            suite_duration: Duration::from_secs(10),
            average_memory_usage: 45.5,
            peak_memory_usage: 67.8,
            average_cpu_usage: 22.3,
            peak_cpu_usage: 45.6,
        };
        
        let report = summary.generate_report();
        assert!(report.contains("Total Tests: 3"));
        assert!(report.contains("Suite Duration: 10.00s"));
        assert!(report.contains("Average Memory: 45.5 MB"));
        assert!(report.contains("Peak CPU: 45.6%"));
    }
}