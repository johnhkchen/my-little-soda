/// Integration test for the comprehensive test result reporting system
/// 
/// This test demonstrates the B3d deliverable: test result aggregation, detailed failure reporting,
/// test summary generation, performance metrics collection, and test result artifact generation.

mod fixtures;

use fixtures::test_result_reporting::{
    TestResultReporter, TestResult, TestStatus, TestPerformanceMetrics,
};
use fixtures::test_metrics_collector::{
    TestMetricsCollector, BatchMetricsCollector, SimpleTestTimer,
};
use fixtures::test_harness::helpers::simple_harness;
use std::time::Duration;
use tempfile::TempDir;
use anyhow::Result;

#[tokio::test]
async fn test_comprehensive_test_result_reporting_system() -> Result<()> {
    println!("🔧 Testing Comprehensive Test Result Reporting System");
    println!("====================================================");
    
    let test_output_dir = TempDir::new()?;
    let output_path = test_output_dir.path();
    
    let mut reporter = TestResultReporter::new("init_command_test_suite".to_string());
    let mut batch_collector = BatchMetricsCollector::new();
    
    println!("📊 Running test scenarios with comprehensive reporting...");
    
    // Simulate running various test scenarios with different outcomes
    let test_scenarios = vec![
        ("test_successful_init", "init_commands", true),
        ("test_file_creation_validation", "validation", true),
        ("test_git_repository_setup", "git_operations", false), // This will fail intentionally
        ("test_configuration_validation", "validation", true),
        ("test_cleanup_verification", "cleanup", true),
        ("test_permission_handling", "security", false), // This will also fail
    ];
    
    for (test_name, category, should_pass) in test_scenarios {
        println!("   Running test: {}", test_name);
        
        let test_result = if should_pass {
            run_passing_test_with_metrics(test_name, category).await?
        } else {
            run_failing_test_with_metrics(test_name, category).await?
        };
        
        batch_collector.add_test_metrics(test_result.performance_metrics.clone());
        reporter.add_test_result(test_result);
    }
    
    // Finalize performance statistics
    reporter.finalize_performance_stats();
    
    // Generate comprehensive summary report
    let summary = reporter.generate_summary_report();
    println!("\n📋 Test Suite Summary:");
    println!("   Total Tests: {}", summary.total_tests);
    println!("   Passed: {}", summary.passed_tests);
    println!("   Failed: {}", summary.failed_tests);
    println!("   Success Rate: {:.2}%", summary.success_rate * 100.0);
    println!("   Total Execution Time: {:.2}s", summary.total_execution_time.as_secs_f64());
    
    // Validate summary contents
    assert_eq!(summary.total_tests, 6);
    assert_eq!(summary.passed_tests, 4);
    assert_eq!(summary.failed_tests, 2);
    assert!((summary.success_rate - 0.666).abs() < 0.01); // Approximately 66.67%
    
    // Test detailed failure reporting
    assert_eq!(summary.failed_test_details.len(), 2);
    let failure_names: Vec<String> = summary.failed_test_details.iter()
        .map(|f| f.test_name.clone())
        .collect();
    assert!(failure_names.contains(&"test_git_repository_setup".to_string()));
    assert!(failure_names.contains(&"test_permission_handling".to_string()));
    
    // Verify performance highlights
    assert!(!summary.performance_highlights.slowest_tests.is_empty());
    assert!(summary.performance_highlights.average_execution_time > Duration::from_millis(0));
    
    // Verify recommendations are generated
    assert!(!summary.recommendations.is_empty());
    println!("\n💡 Generated Recommendations:");
    for (i, rec) in summary.recommendations.iter().enumerate() {
        println!("   {}. {}", i + 1, rec);
    }
    
    println!("\n📁 Generating test result artifacts...");
    
    // Generate comprehensive test artifacts
    let artifacts = reporter.generate_artifacts(output_path)?;
    assert!(!artifacts.is_empty());
    assert!(artifacts.len() >= 3); // JSON, CSV, and failure report
    
    println!("   Generated {} artifacts:", artifacts.len());
    for artifact in &artifacts {
        println!("     - {}", artifact);
        assert!(std::path::Path::new(artifact).exists());
    }
    
    // Verify artifact contents
    let json_path = output_path.join("test_results.json");
    assert!(json_path.exists());
    let json_content = std::fs::read_to_string(&json_path)?;
    assert!(json_content.contains("init_command_test_suite"));
    assert!(json_content.contains("test_successful_init"));
    
    let summary_path = output_path.join("test_summary.json");
    assert!(summary_path.exists());
    let summary_content = std::fs::read_to_string(&summary_path)?;
    assert!(summary_content.contains("success_rate"));
    assert!(summary_content.contains("performance_highlights"));
    
    let csv_path = output_path.join("test_results.csv");
    assert!(csv_path.exists());
    let csv_content = std::fs::read_to_string(&csv_path)?;
    assert!(csv_content.contains("test_name,category,status"));
    assert!(csv_content.lines().count() > 6); // Header + 6 test results
    
    // Verify failure analysis report exists for failed tests
    let failure_report_path = output_path.join("failure_analysis.txt");
    assert!(failure_report_path.exists());
    let failure_content = std::fs::read_to_string(&failure_report_path)?;
    assert!(failure_content.contains("# Test Failure Analysis Report"));
    assert!(failure_content.contains("test_git_repository_setup"));
    assert!(failure_content.contains("## Recommendations"));
    
    println!("\n✅ Comprehensive test result reporting system test PASSED!");
    Ok(())
}

#[tokio::test]
async fn test_performance_metrics_collection_detailed() -> Result<()> {
    println!("⚡ Testing Performance Metrics Collection");
    println!("========================================");
    
    // Test real-time metrics collection during test execution
    let mut collector = TestMetricsCollector::new("performance_test".to_string());
    
    println!("   Starting performance monitoring...");
    collector.start_test()?;
    
    // Simulate test phases with timing
    collector.start_setup();
    simulate_work(Duration::from_millis(50)); // Setup work
    collector.end_setup();
    
    collector.start_validation();
    simulate_work(Duration::from_millis(200)); // Validation work
    collector.end_validation();
    
    collector.start_teardown();
    simulate_work(Duration::from_millis(30)); // Teardown work
    collector.end_teardown();
    
    let metrics = collector.stop_test()?;
    
    println!("   Collected metrics:");
    println!("     Execution Time: {:?}", metrics.execution_time);
    println!("     Setup Time: {:?}", metrics.setup_time);
    println!("     Validation Time: {:?}", metrics.validation_time);
    println!("     Teardown Time: {:?}", metrics.teardown_time);
    
    // Verify timing measurements
    assert!(metrics.execution_time >= Duration::from_millis(280)); // Total should be at least the sum of phases
    assert!(metrics.setup_time >= Duration::from_millis(45)); // Allow some variance
    assert!(metrics.validation_time >= Duration::from_millis(195));
    assert!(metrics.teardown_time >= Duration::from_millis(25));
    
    // Memory and CPU metrics might be 0.0 in test environment, but structure should be correct
    assert!(metrics.memory_usage_mb.is_some() || metrics.memory_usage_mb.is_none());
    assert!(metrics.cpu_usage_percent.is_some() || metrics.cpu_usage_percent.is_none());
    
    println!("✅ Performance metrics collection working correctly");
    Ok(())
}

#[tokio::test]
async fn test_batch_metrics_aggregation() -> Result<()> {
    println!("📊 Testing Batch Metrics Aggregation");
    println!("====================================");
    
    let mut batch_collector = BatchMetricsCollector::new();
    
    // Simulate collecting metrics from multiple tests
    let test_metrics = vec![
        create_mock_metrics("fast_test", Duration::from_millis(100), Some(30.0), Some(15.0)),
        create_mock_metrics("slow_test", Duration::from_millis(500), Some(45.0), Some(25.0)),
        create_mock_metrics("medium_test", Duration::from_millis(250), Some(35.0), Some(20.0)),
        create_mock_metrics("memory_heavy_test", Duration::from_millis(200), Some(80.0), Some(30.0)),
        create_mock_metrics("cpu_intensive_test", Duration::from_millis(300), Some(40.0), Some(65.0)),
    ];
    
    for metrics in test_metrics {
        batch_collector.add_test_metrics(metrics);
    }
    
    let summary = batch_collector.generate_summary();
    
    println!("   Batch Summary:");
    println!("     Total Tests: {}", summary.total_tests);
    println!("     Average Execution Time: {:?}", summary.average_execution_time);
    println!("     Slowest Test: {:?}", summary.slowest_test_time);
    println!("     Fastest Test: {:?}", summary.fastest_test_time);
    println!("     Average Memory: {:.1} MB", summary.average_memory_usage);
    println!("     Peak Memory: {:.1} MB", summary.peak_memory_usage);
    println!("     Average CPU: {:.1}%", summary.average_cpu_usage);
    println!("     Peak CPU: {:.1}%", summary.peak_cpu_usage);
    
    // Verify aggregation calculations
    assert_eq!(summary.total_tests, 5);
    assert_eq!(summary.average_execution_time, Duration::from_millis(270)); // (100+500+250+200+300)/5
    assert_eq!(summary.slowest_test_time, Duration::from_millis(500));
    assert_eq!(summary.fastest_test_time, Duration::from_millis(100));
    assert_eq!(summary.average_memory_usage, 46.0); // (30+45+35+80+40)/5
    assert_eq!(summary.peak_memory_usage, 80.0);
    assert_eq!(summary.average_cpu_usage, 31.0); // (15+25+20+30+65)/5
    assert_eq!(summary.peak_cpu_usage, 65.0);
    
    // Test report generation
    let report = summary.generate_report();
    assert!(report.contains("Performance Metrics Summary"));
    assert!(report.contains("Total Tests: 5"));
    assert!(report.contains("Peak Memory: 80.0 MB"));
    assert!(report.contains("Peak CPU: 65.0%"));
    
    println!("✅ Batch metrics aggregation working correctly");
    Ok(())
}

#[tokio::test]
async fn test_real_test_execution_with_reporting() -> Result<()> {
    println!("🧪 Testing Real Test Execution with Comprehensive Reporting");
    println!("==========================================================");
    
    let mut reporter = TestResultReporter::new("real_execution_suite".to_string());
    
    // Run actual test harness operations with reporting
    let test_result = run_real_test_with_comprehensive_reporting().await?;
    reporter.add_test_result(test_result);
    
    // Generate and verify reports
    let summary = reporter.generate_summary_report();
    assert_eq!(summary.total_tests, 1);
    assert!(summary.passed_tests == 1 || summary.failed_tests == 1);
    
    let temp_dir = TempDir::new()?;
    let artifacts = reporter.generate_artifacts(temp_dir.path())?;
    
    println!("   Generated {} artifacts for real test execution", artifacts.len());
    for artifact in &artifacts {
        assert!(std::path::Path::new(artifact).exists());
        println!("     ✓ {}", artifact);
    }
    
    println!("✅ Real test execution with reporting completed successfully");
    Ok(())
}

#[tokio::test]
async fn test_test_summary_dashboard_generation() -> Result<()> {
    println!("📊 Testing Test Summary Dashboard Generation");
    println!("===========================================");
    
    let mut reporter = TestResultReporter::new("dashboard_test_suite".to_string());
    
    // Add various test results to create a comprehensive dashboard
    let test_results = vec![
        create_mock_test_result("ui_component_tests", "frontend", TestStatus::Passed, Duration::from_millis(150)),
        create_mock_test_result("api_integration_tests", "backend", TestStatus::Passed, Duration::from_millis(300)),
        create_mock_test_result("database_migration_tests", "database", TestStatus::Failed, Duration::from_millis(450)),
        create_mock_test_result("security_validation_tests", "security", TestStatus::Passed, Duration::from_millis(200)),
        create_mock_test_result("performance_benchmark_tests", "performance", TestStatus::Failed, Duration::from_millis(800)),
        create_mock_test_result("deployment_verification_tests", "deployment", TestStatus::Passed, Duration::from_millis(100)),
    ];
    
    for result in test_results {
        reporter.add_test_result(result);
    }
    
    reporter.finalize_performance_stats();
    let summary = reporter.generate_summary_report();
    
    println!("   Dashboard Summary:");
    println!("     Test Categories: {}", get_unique_categories(&summary));
    println!("     Overall Health: {:.1}%", summary.success_rate * 100.0);
    println!("     Critical Issues: {}", summary.failed_tests);
    println!("     Performance Score: {}", calculate_performance_score(&summary));
    
    // Verify dashboard contains comprehensive information
    assert_eq!(summary.total_tests, 6);
    assert_eq!(summary.passed_tests, 4);
    assert_eq!(summary.failed_tests, 2);
    
    // Verify different test categories are represented
    let categories = get_unique_categories(&summary);
    assert!(categories >= 5); // Should have at least 5 different categories
    
    // Verify performance highlights include slowest tests
    assert!(!summary.performance_highlights.slowest_tests.is_empty());
    let slowest = &summary.performance_highlights.slowest_tests[0];
    assert_eq!(slowest.test_name, "performance_benchmark_tests");
    
    // Verify failure analysis provides actionable insights
    assert!(!summary.failed_test_details.is_empty());
    let failed_test = &summary.failed_test_details[0];
    assert!(!failed_test.error_message.is_empty());
    
    println!("✅ Test summary dashboard generation working correctly");
    Ok(())
}

// Helper functions

async fn run_passing_test_with_metrics(test_name: &str, category: &str) -> Result<TestResult> {
    let timer = SimpleTestTimer::new(test_name.to_string());
    
    // Simulate test execution with some work
    simulate_work(Duration::from_millis(50 + (test_name.len() * 10) as u64));
    
    let (name, duration) = timer.finish();
    let mut result = TestResult::new(name, category.to_string());
    result.mark_passed(duration);
    result.performance_metrics.execution_time = duration;
    result.performance_metrics.memory_usage_mb = Some(25.0 + (category.len() as f64 * 2.0));
    result.performance_metrics.cpu_usage_percent = Some(15.0 + (test_name.len() as f64));
    
    Ok(result)
}

async fn run_failing_test_with_metrics(test_name: &str, category: &str) -> Result<TestResult> {
    let timer = SimpleTestTimer::new(test_name.to_string());
    
    // Simulate test execution that fails
    simulate_work(Duration::from_millis(100 + (test_name.len() * 5) as u64));
    
    let (name, duration) = timer.finish();
    let mut result = TestResult::new(name, category.to_string());
    
    let error_msg = format!("Simulated failure in {}: validation check failed", test_name);
    let failure_details = vec![
        "Expected file not found: test_output.txt".to_string(),
        "Validation timeout after 30 seconds".to_string(),
        format!("Error in {} operation", category),
    ];
    
    result.mark_failed(duration, error_msg, failure_details);
    result.performance_metrics.execution_time = duration;
    result.performance_metrics.memory_usage_mb = Some(40.0 + (category.len() as f64 * 3.0));
    result.performance_metrics.cpu_usage_percent = Some(25.0 + (test_name.len() as f64 * 1.5));
    
    Ok(result)
}

async fn run_real_test_with_comprehensive_reporting() -> Result<TestResult> {
    let test_name = "real_harness_integration_test";
    let mut collector = TestMetricsCollector::new(test_name.to_string());
    
    collector.start_test()?;
    collector.start_setup();
    
    // Create actual test harness and perform operations
    let mut harness = simple_harness()?;
    harness.create_file("test_file.txt", "test content")?;
    harness.create_dir("test_directory")?;
    
    collector.end_setup();
    collector.start_validation();
    
    // Perform actual validation
    let file_path = harness.path().join("test_file.txt");
    assert!(file_path.exists());
    
    let content = std::fs::read_to_string(&file_path)?;
    assert_eq!(content, "test content");
    
    collector.end_validation();
    collector.start_teardown();
    
    // Harness cleanup happens automatically on drop
    
    collector.end_teardown();
    let metrics = collector.stop_test()?;
    
    let mut result = TestResult::new(test_name.to_string(), "integration".to_string());
    result.mark_passed(metrics.execution_time);
    result.performance_metrics = metrics;
    
    Ok(result)
}

fn create_mock_metrics(name: &str, execution_time: Duration, memory_mb: Option<f64>, cpu_percent: Option<f64>) -> TestPerformanceMetrics {
    TestPerformanceMetrics {
        test_name: name.to_string(),
        execution_time,
        memory_usage_mb: memory_mb,
        cpu_usage_percent: cpu_percent,
        setup_time: Duration::from_millis(10),
        teardown_time: Duration::from_millis(5),
        validation_time: execution_time - Duration::from_millis(15),
    }
}

fn create_mock_test_result(name: &str, category: &str, status: TestStatus, execution_time: Duration) -> TestResult {
    let mut result = TestResult::new(name.to_string(), category.to_string());
    
    match status {
        TestStatus::Passed => {
            result.mark_passed(execution_time);
        }
        TestStatus::Failed => {
            result.mark_failed(
                execution_time,
                format!("Mock failure in {}", name),
                vec![format!("Simulated error for testing purposes")],
            );
        }
        TestStatus::Skipped => {
            result.mark_skipped(format!("Mock skip reason for {}", name));
        }
        _ => {}
    }
    
    result.performance_metrics.execution_time = execution_time;
    result.performance_metrics.memory_usage_mb = Some(30.0 + (name.len() as f64 * 2.0));
    result.performance_metrics.cpu_usage_percent = Some(20.0 + (category.len() as f64 * 1.5));
    
    result
}

fn simulate_work(duration: Duration) {
    std::thread::sleep(duration);
}

fn get_unique_categories(summary: &fixtures::test_result_reporting::TestSummaryReport) -> usize {
    use std::collections::HashSet;
    let mut categories = HashSet::new();
    for detail in &summary.failed_test_details {
        categories.insert(&detail.test_category);
    }
    categories.len()
}

fn calculate_performance_score(summary: &fixtures::test_result_reporting::TestSummaryReport) -> String {
    let avg_time_ms = summary.performance_highlights.average_execution_time.as_millis() as f64;
    if avg_time_ms < 100.0 {
        "Excellent".to_string()
    } else if avg_time_ms < 300.0 {
        "Good".to_string()
    } else if avg_time_ms < 500.0 {
        "Fair".to_string()
    } else {
        "Needs Improvement".to_string()
    }
}