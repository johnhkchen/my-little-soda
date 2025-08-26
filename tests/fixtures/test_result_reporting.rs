/// Test result reporting system for init command testing infrastructure
/// 
/// This module provides comprehensive test result aggregation, detailed failure reporting,
/// test summary generation, and performance metrics collection for the init command test suite.

use super::automated_validators::{ValidationSummaryReport, FailureAnalysisReport};
use anyhow::Result;
use std::time::{SystemTime, Duration, Instant};
use std::collections::HashMap;
use std::path::Path;
use serde::{Serialize, Deserialize};

/// Test execution timing and performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestPerformanceMetrics {
    pub test_name: String,
    pub execution_time: Duration,
    pub memory_usage_mb: Option<f64>,
    pub cpu_usage_percent: Option<f64>,
    pub setup_time: Duration,
    pub teardown_time: Duration,
    pub validation_time: Duration,
}

impl TestPerformanceMetrics {
    pub fn new(test_name: String) -> Self {
        Self {
            test_name,
            execution_time: Duration::new(0, 0),
            memory_usage_mb: None,
            cpu_usage_percent: None,
            setup_time: Duration::new(0, 0),
            teardown_time: Duration::new(0, 0),
            validation_time: Duration::new(0, 0),
        }
    }
}

/// Individual test result with comprehensive details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_name: String,
    pub test_category: String,
    pub status: TestStatus,
    pub execution_time: Duration,
    pub error_message: Option<String>,
    pub failure_details: Vec<String>,
    pub fixture_name: Option<String>,
    pub scenario_name: Option<String>,
    pub validation_results: Option<ValidationSummaryReport>,
    pub performance_metrics: TestPerformanceMetrics,
    pub timestamp: SystemTime,
    pub artifacts_generated: Vec<String>,
}

impl TestResult {
    pub fn new(test_name: String, test_category: String) -> Self {
        Self {
            test_name: test_name.clone(),
            test_category,
            status: TestStatus::NotStarted,
            execution_time: Duration::new(0, 0),
            error_message: None,
            failure_details: Vec::new(),
            fixture_name: None,
            scenario_name: None,
            validation_results: None,
            performance_metrics: TestPerformanceMetrics::new(test_name),
            timestamp: SystemTime::now(),
            artifacts_generated: Vec::new(),
        }
    }
    
    pub fn mark_passed(&mut self, execution_time: Duration) {
        self.status = TestStatus::Passed;
        self.execution_time = execution_time;
        self.timestamp = SystemTime::now();
    }
    
    pub fn mark_failed(&mut self, execution_time: Duration, error: String, details: Vec<String>) {
        self.status = TestStatus::Failed;
        self.execution_time = execution_time;
        self.error_message = Some(error);
        self.failure_details = details;
        self.timestamp = SystemTime::now();
    }
    
    pub fn mark_skipped(&mut self, reason: String) {
        self.status = TestStatus::Skipped;
        self.error_message = Some(reason);
        self.timestamp = SystemTime::now();
    }
}

/// Test execution status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TestStatus {
    NotStarted,
    Running,
    Passed,
    Failed,
    Skipped,
    Timeout,
}

/// Aggregated test results across multiple test runs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResultAggregation {
    pub suite_name: String,
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub skipped_tests: usize,
    pub timeout_tests: usize,
    pub total_execution_time: Duration,
    pub test_results: Vec<TestResult>,
    pub failure_analysis: Option<FailureAnalysisReport>,
    pub performance_summary: TestPerformanceSummary,
    pub created_at: SystemTime,
}

impl TestResultAggregation {
    pub fn new(suite_name: String) -> Self {
        Self {
            suite_name,
            total_tests: 0,
            passed_tests: 0,
            failed_tests: 0,
            skipped_tests: 0,
            timeout_tests: 0,
            total_execution_time: Duration::new(0, 0),
            test_results: Vec::new(),
            failure_analysis: None,
            performance_summary: TestPerformanceSummary::new(),
            created_at: SystemTime::now(),
        }
    }
    
    pub fn add_test_result(&mut self, test_result: TestResult) {
        self.total_tests += 1;
        self.total_execution_time += test_result.execution_time;
        
        match test_result.status {
            TestStatus::Passed => self.passed_tests += 1,
            TestStatus::Failed => self.failed_tests += 1,
            TestStatus::Skipped => self.skipped_tests += 1,
            TestStatus::Timeout => self.timeout_tests += 1,
            _ => {}
        }
        
        self.performance_summary.add_metrics(&test_result.performance_metrics);
        self.test_results.push(test_result);
    }
    
    pub fn success_rate(&self) -> f64 {
        if self.total_tests == 0 {
            return 0.0;
        }
        self.passed_tests as f64 / self.total_tests as f64
    }
    
    pub fn get_failed_tests(&self) -> Vec<&TestResult> {
        self.test_results.iter().filter(|t| t.status == TestStatus::Failed).collect()
    }
    
    pub fn get_slowest_tests(&self, count: usize) -> Vec<&TestResult> {
        let mut sorted_tests = self.test_results.iter().collect::<Vec<_>>();
        sorted_tests.sort_by(|a, b| b.execution_time.cmp(&a.execution_time));
        sorted_tests.into_iter().take(count).collect()
    }
}

/// Performance summary across all tests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestPerformanceSummary {
    pub average_execution_time: Duration,
    pub median_execution_time: Duration,
    pub max_execution_time: Duration,
    pub min_execution_time: Duration,
    pub total_setup_time: Duration,
    pub total_teardown_time: Duration,
    pub total_validation_time: Duration,
    pub memory_usage_stats: MemoryUsageStats,
    pub cpu_usage_stats: CpuUsageStats,
}

impl TestPerformanceSummary {
    pub fn new() -> Self {
        Self {
            average_execution_time: Duration::new(0, 0),
            median_execution_time: Duration::new(0, 0),
            max_execution_time: Duration::new(0, 0),
            min_execution_time: Duration::new(0, 0),
            total_setup_time: Duration::new(0, 0),
            total_teardown_time: Duration::new(0, 0),
            total_validation_time: Duration::new(0, 0),
            memory_usage_stats: MemoryUsageStats::new(),
            cpu_usage_stats: CpuUsageStats::new(),
        }
    }
    
    pub fn add_metrics(&mut self, metrics: &TestPerformanceMetrics) {
        self.total_setup_time += metrics.setup_time;
        self.total_teardown_time += metrics.teardown_time;
        self.total_validation_time += metrics.validation_time;
        
        if metrics.execution_time > self.max_execution_time {
            self.max_execution_time = metrics.execution_time;
        }
        
        if self.min_execution_time == Duration::new(0, 0) || metrics.execution_time < self.min_execution_time {
            self.min_execution_time = metrics.execution_time;
        }
        
        if let Some(memory) = metrics.memory_usage_mb {
            self.memory_usage_stats.add_sample(memory);
        }
        
        if let Some(cpu) = metrics.cpu_usage_percent {
            self.cpu_usage_stats.add_sample(cpu);
        }
    }
    
    pub fn calculate_final_stats(&mut self, test_count: usize, execution_times: &[Duration]) {
        if test_count > 0 {
            let total_time: Duration = execution_times.iter().sum();
            self.average_execution_time = total_time / test_count as u32;
            
            let mut sorted_times = execution_times.to_vec();
            sorted_times.sort();
            if !sorted_times.is_empty() {
                self.median_execution_time = sorted_times[sorted_times.len() / 2];
            }
        }
    }
}

/// Memory usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsageStats {
    pub samples: Vec<f64>,
    pub average_mb: f64,
    pub peak_mb: f64,
    pub min_mb: f64,
}

impl MemoryUsageStats {
    pub fn new() -> Self {
        Self {
            samples: Vec::new(),
            average_mb: 0.0,
            peak_mb: 0.0,
            min_mb: 0.0,
        }
    }
    
    pub fn add_sample(&mut self, memory_mb: f64) {
        self.samples.push(memory_mb);
        
        if memory_mb > self.peak_mb {
            self.peak_mb = memory_mb;
        }
        
        if self.min_mb == 0.0 || memory_mb < self.min_mb {
            self.min_mb = memory_mb;
        }
        
        self.average_mb = self.samples.iter().sum::<f64>() / self.samples.len() as f64;
    }
}

/// CPU usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuUsageStats {
    pub samples: Vec<f64>,
    pub average_percent: f64,
    pub peak_percent: f64,
    pub min_percent: f64,
}

impl CpuUsageStats {
    pub fn new() -> Self {
        Self {
            samples: Vec::new(),
            average_percent: 0.0,
            peak_percent: 0.0,
            min_percent: 0.0,
        }
    }
    
    pub fn add_sample(&mut self, cpu_percent: f64) {
        self.samples.push(cpu_percent);
        
        if cpu_percent > self.peak_percent {
            self.peak_percent = cpu_percent;
        }
        
        if self.min_percent == 0.0 || cpu_percent < self.min_percent {
            self.min_percent = cpu_percent;
        }
        
        self.average_percent = self.samples.iter().sum::<f64>() / self.samples.len() as f64;
    }
}

/// Test result reporter and artifact generator
pub struct TestResultReporter {
    aggregation: TestResultAggregation,
}

impl TestResultReporter {
    pub fn new(suite_name: String) -> Self {
        Self {
            aggregation: TestResultAggregation::new(suite_name),
        }
    }
    
    pub fn add_test_result(&mut self, test_result: TestResult) {
        self.aggregation.add_test_result(test_result);
    }
    
    pub fn finalize_performance_stats(&mut self) {
        let execution_times: Vec<Duration> = self.aggregation.test_results
            .iter()
            .map(|t| t.execution_time)
            .collect();
        
        self.aggregation.performance_summary.calculate_final_stats(
            self.aggregation.total_tests,
            &execution_times,
        );
    }
    
    /// Generate comprehensive test summary report
    pub fn generate_summary_report(&self) -> TestSummaryReport {
        TestSummaryReport {
            suite_name: self.aggregation.suite_name.clone(),
            total_tests: self.aggregation.total_tests,
            passed_tests: self.aggregation.passed_tests,
            failed_tests: self.aggregation.failed_tests,
            skipped_tests: self.aggregation.skipped_tests,
            success_rate: self.aggregation.success_rate(),
            total_execution_time: self.aggregation.total_execution_time,
            failed_test_details: self.get_failure_details(),
            performance_highlights: self.get_performance_highlights(),
            recommendations: self.generate_recommendations(),
            timestamp: SystemTime::now(),
        }
    }
    
    fn get_failure_details(&self) -> Vec<TestFailureDetail> {
        self.aggregation.get_failed_tests()
            .into_iter()
            .map(|test| TestFailureDetail {
                test_name: test.test_name.clone(),
                test_category: test.test_category.clone(),
                error_message: test.error_message.clone().unwrap_or_default(),
                failure_details: test.failure_details.clone(),
                execution_time: test.execution_time,
                fixture_name: test.fixture_name.clone(),
            })
            .collect()
    }
    
    fn get_performance_highlights(&self) -> PerformanceHighlights {
        let slowest_tests = self.aggregation.get_slowest_tests(5);
        
        PerformanceHighlights {
            slowest_tests: slowest_tests.into_iter().map(|test| {
                SlowTestInfo {
                    test_name: test.test_name.clone(),
                    execution_time: test.execution_time,
                    category: test.test_category.clone(),
                }
            }).collect(),
            average_execution_time: self.aggregation.performance_summary.average_execution_time,
            total_setup_time: self.aggregation.performance_summary.total_setup_time,
            total_validation_time: self.aggregation.performance_summary.total_validation_time,
            memory_peak: self.aggregation.performance_summary.memory_usage_stats.peak_mb,
            cpu_peak: self.aggregation.performance_summary.cpu_usage_stats.peak_percent,
        }
    }
    
    fn generate_recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        let success_rate = self.aggregation.success_rate();
        if success_rate < 0.8 {
            recommendations.push("Consider reviewing test reliability - success rate is below 80%".to_string());
        }
        
        let slow_tests = self.aggregation.get_slowest_tests(3);
        if slow_tests.iter().any(|t| t.execution_time > Duration::from_secs(30)) {
            recommendations.push("Some tests are taking longer than 30 seconds - consider optimization".to_string());
        }
        
        if self.aggregation.performance_summary.memory_usage_stats.peak_mb > 500.0 {
            recommendations.push("Peak memory usage exceeds 500MB - monitor for memory leaks".to_string());
        }
        
        if self.aggregation.failed_tests > 0 {
            let common_failures = self.analyze_common_failure_patterns();
            for pattern in common_failures {
                recommendations.push(format!("Common failure pattern detected: {}", pattern));
            }
        }
        
        recommendations
    }
    
    fn analyze_common_failure_patterns(&self) -> Vec<String> {
        let mut patterns = Vec::new();
        let failed_tests = self.aggregation.get_failed_tests();
        
        let mut error_counts: HashMap<String, usize> = HashMap::new();
        for test in &failed_tests {
            if let Some(error) = &test.error_message {
                let simplified_error = self.simplify_error_message(error);
                *error_counts.entry(simplified_error).or_insert(0) += 1;
            }
        }
        
        for (error_pattern, count) in error_counts {
            if count > 1 {
                patterns.push(format!("{} (occurred {} times)", error_pattern, count));
            }
        }
        
        patterns
    }
    
    fn simplify_error_message(&self, error: &str) -> String {
        if error.contains("timeout") {
            "Test timeout".to_string()
        } else if error.contains("permission") {
            "Permission error".to_string()
        } else if error.contains("file not found") || error.contains("No such file") {
            "File not found".to_string()
        } else if error.contains("validation") {
            "Validation failure".to_string()
        } else {
            error.split_whitespace().take(5).collect::<Vec<_>>().join(" ")
        }
    }
    
    /// Generate detailed test result artifacts
    pub fn generate_artifacts(&self, output_dir: &Path) -> Result<Vec<String>> {
        let mut artifacts = Vec::new();
        
        std::fs::create_dir_all(output_dir)?;
        
        let json_path = output_dir.join("test_results.json");
        let json_content = serde_json::to_string_pretty(&self.aggregation)?;
        std::fs::write(&json_path, json_content)?;
        artifacts.push(json_path.to_string_lossy().to_string());
        
        let summary_path = output_dir.join("test_summary.json");
        let summary = self.generate_summary_report();
        let summary_content = serde_json::to_string_pretty(&summary)?;
        std::fs::write(&summary_path, summary_content)?;
        artifacts.push(summary_path.to_string_lossy().to_string());
        
        let csv_path = output_dir.join("test_results.csv");
        let csv_content = self.generate_csv_report();
        std::fs::write(&csv_path, csv_content)?;
        artifacts.push(csv_path.to_string_lossy().to_string());
        
        if !self.aggregation.get_failed_tests().is_empty() {
            let failure_report_path = output_dir.join("failure_analysis.txt");
            let failure_content = self.generate_failure_analysis_report();
            std::fs::write(&failure_report_path, failure_content)?;
            artifacts.push(failure_report_path.to_string_lossy().to_string());
        }
        
        Ok(artifacts)
    }
    
    fn generate_csv_report(&self) -> String {
        let mut csv = String::from("test_name,category,status,execution_time_ms,fixture,scenario,error_message\n");
        
        for test in &self.aggregation.test_results {
            csv.push_str(&format!(
                "{},{},{:?},{},{},{},{}\n",
                test.test_name,
                test.test_category,
                test.status,
                test.execution_time.as_millis(),
                test.fixture_name.as_deref().unwrap_or(""),
                test.scenario_name.as_deref().unwrap_or(""),
                test.error_message.as_deref().unwrap_or("").replace(',', ";")
            ));
        }
        
        csv
    }
    
    fn generate_failure_analysis_report(&self) -> String {
        let mut report = String::new();
        report.push_str("# Test Failure Analysis Report\n\n");
        
        let failed_tests = self.aggregation.get_failed_tests();
        report.push_str(&format!("## Summary\n- Total Failed Tests: {}\n", failed_tests.len()));
        report.push_str(&format!("- Success Rate: {:.2}%\n\n", self.aggregation.success_rate() * 100.0));
        
        report.push_str("## Failed Tests Details\n\n");
        for (i, test) in failed_tests.iter().enumerate() {
            report.push_str(&format!("### {}. {}\n", i + 1, test.test_name));
            report.push_str(&format!("- **Category**: {}\n", test.test_category));
            report.push_str(&format!("- **Execution Time**: {:?}\n", test.execution_time));
            if let Some(fixture) = &test.fixture_name {
                report.push_str(&format!("- **Fixture**: {}\n", fixture));
            }
            if let Some(error) = &test.error_message {
                report.push_str(&format!("- **Error**: {}\n", error));
            }
            if !test.failure_details.is_empty() {
                report.push_str("- **Failure Details**:\n");
                for detail in &test.failure_details {
                    report.push_str(&format!("  - {}\n", detail));
                }
            }
            report.push_str("\n");
        }
        
        let recommendations = self.generate_recommendations();
        if !recommendations.is_empty() {
            report.push_str("## Recommendations\n\n");
            for (i, rec) in recommendations.iter().enumerate() {
                report.push_str(&format!("{}. {}\n", i + 1, rec));
            }
        }
        
        report
    }
}

/// Comprehensive test summary report
#[derive(Debug, Serialize, Deserialize)]
pub struct TestSummaryReport {
    pub suite_name: String,
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub skipped_tests: usize,
    pub success_rate: f64,
    pub total_execution_time: Duration,
    pub failed_test_details: Vec<TestFailureDetail>,
    pub performance_highlights: PerformanceHighlights,
    pub recommendations: Vec<String>,
    pub timestamp: SystemTime,
}

/// Details for a failed test
#[derive(Debug, Serialize, Deserialize)]
pub struct TestFailureDetail {
    pub test_name: String,
    pub test_category: String,
    pub error_message: String,
    pub failure_details: Vec<String>,
    pub execution_time: Duration,
    pub fixture_name: Option<String>,
}

/// Performance highlights summary
#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceHighlights {
    pub slowest_tests: Vec<SlowTestInfo>,
    pub average_execution_time: Duration,
    pub total_setup_time: Duration,
    pub total_validation_time: Duration,
    pub memory_peak: f64,
    pub cpu_peak: f64,
}

/// Information about slow tests
#[derive(Debug, Serialize, Deserialize)]
pub struct SlowTestInfo {
    pub test_name: String,
    pub execution_time: Duration,
    pub category: String,
}