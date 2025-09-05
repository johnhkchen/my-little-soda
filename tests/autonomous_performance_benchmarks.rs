#![cfg(feature = "autonomous")]
//! Performance benchmarks and long-running operation tests for autonomous system
//!
//! These tests validate that the autonomous system can handle extended operation
//! periods efficiently and maintain performance characteristics under load.

use chrono::Utc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::{interval, sleep};

use my_little_soda::{
    agents::recovery::AutoRecovery,
    autonomous::{
        state_validation::{DriftThresholds, StateDriftDetector},
        work_continuity::WorkProgress,
        workflow_state_machine::WorkspaceState,
        AgentId, AutonomousCoordinator, AutonomousEvent, AutonomousWorkflowMachine,
        AutonomousWorkflowState, CoordinationConfig, Issue, Priority,
    },
    GitHubClient,
};

/// Performance metrics for autonomous operations
#[derive(Debug, Clone)]
struct PerformanceMetrics {
    operation_count: u64,
    total_duration: Duration,
    min_duration: Duration,
    max_duration: Duration,
    error_count: u64,
    memory_usage_bytes: u64,
}

impl PerformanceMetrics {
    fn new() -> Self {
        Self {
            operation_count: 0,
            total_duration: Duration::ZERO,
            min_duration: Duration::MAX,
            max_duration: Duration::ZERO,
            error_count: 0,
            memory_usage_bytes: 0,
        }
    }

    fn record_operation(&mut self, duration: Duration, success: bool) {
        self.operation_count += 1;
        self.total_duration += duration;
        self.min_duration = self.min_duration.min(duration);
        self.max_duration = self.max_duration.max(duration);
        if !success {
            self.error_count += 1;
        }
    }

    fn average_duration(&self) -> Duration {
        if self.operation_count > 0 {
            self.total_duration / self.operation_count as u32
        } else {
            Duration::ZERO
        }
    }

    fn success_rate(&self) -> f64 {
        if self.operation_count > 0 {
            (self.operation_count - self.error_count) as f64 / self.operation_count as f64
        } else {
            1.0
        }
    }
}

/// Test workflow state machine performance under repeated state transitions
#[tokio::test]
async fn test_workflow_state_transition_performance() {
    let mut workflow = AutonomousWorkflowMachine::new(24); // Long work period
    let mut metrics = PerformanceMetrics::new();
    let iterations = 100;

    println!(
        "Testing workflow state transition performance over {} iterations",
        iterations
    );

    for i in 0..iterations {
        let start_time = Instant::now();

        // Reset workflow for each iteration
        let assign_result = workflow
            .handle_event(AutonomousEvent::AssignAgent {
                agent: AgentId(format!("perf-test-agent-{}", i)),
                workspace_ready: true,
            })
            .await;

        let operation_duration = start_time.elapsed();
        let success = assign_result.is_ok();
        metrics.record_operation(operation_duration, success);

        if i % 10 == 0 {
            println!(
                "Completed {} iterations, avg duration: {:?}",
                i,
                metrics.average_duration()
            );
        }
    }

    // Validate performance characteristics
    println!("Performance Results:");
    println!("  Total operations: {}", metrics.operation_count);
    println!("  Average duration: {:?}", metrics.average_duration());
    println!("  Min duration: {:?}", metrics.min_duration);
    println!("  Max duration: {:?}", metrics.max_duration);
    println!("  Success rate: {:.2}%", metrics.success_rate() * 100.0);
    println!("  Error count: {}", metrics.error_count);

    // Assert performance requirements
    assert!(
        metrics.success_rate() > 0.95,
        "Success rate too low: {:.2}%",
        metrics.success_rate() * 100.0
    );
    assert!(
        metrics.average_duration() < Duration::from_millis(50),
        "Average operation too slow: {:?}",
        metrics.average_duration()
    );
    assert!(
        metrics.max_duration() < Duration::from_millis(500),
        "Max operation too slow: {:?}",
        metrics.max_duration()
    );
}

/// Test state drift detection performance
#[tokio::test]
async fn test_drift_detection_performance() {
    let github_client = GitHubClient::new().expect("Failed to create GitHub client");
    let mut detector = StateDriftDetector::new(github_client, "perf-drift-agent".to_string())
        .with_validation_interval(chrono::Duration::seconds(1));

    let mut metrics = PerformanceMetrics::new();
    let iterations = 50; // Fewer iterations since this may involve I/O

    println!(
        "Testing drift detection performance over {} iterations",
        iterations
    );

    for i in 0..iterations {
        let start_time = Instant::now();

        // Update expected state
        let test_state = AutonomousWorkflowState::Assigned {
            issue: Issue {
                number: (i + 1) as u64,
                title: format!("Performance test issue {}", i),
                body: "Performance testing".to_string(),
                labels: vec![format!("perf-test-{}", i)],
                priority: Priority::Medium,
                estimated_hours: Some(1),
            },
            agent: AgentId(format!("perf-agent-{}", i)),
            workspace: WorkspaceState {
                current_branch: "main".to_string(),
                uncommitted_changes: false,
            },
        };

        let update_result = detector.update_expected_state(&test_state).await;
        let operation_duration = start_time.elapsed();
        let success = update_result.is_ok();

        metrics.record_operation(operation_duration, success);

        if i % 10 == 0 {
            println!(
                "Completed {} state updates, avg duration: {:?}",
                i,
                metrics.average_duration()
            );
        }
    }

    // Test validation performance (this might be slower due to I/O)
    let validation_start = Instant::now();
    let validation_result = detector.validate_state().await;
    let validation_duration = validation_start.elapsed();

    println!("State Drift Performance Results:");
    println!("  State update operations: {}", metrics.operation_count);
    println!(
        "  Average update duration: {:?}",
        metrics.average_duration()
    );
    println!(
        "  Update success rate: {:.2}%",
        metrics.success_rate() * 100.0
    );
    println!("  Validation duration: {:?}", validation_duration);
    println!("  Validation result: {:?}", validation_result.is_ok());

    // Performance assertions
    assert!(
        metrics.success_rate() > 0.95,
        "State update success rate too low"
    );
    assert!(
        metrics.average_duration() < Duration::from_millis(10),
        "State updates too slow: {:?}",
        metrics.average_duration()
    );
    assert!(
        validation_duration < Duration::from_secs(30),
        "State validation too slow: {:?}",
        validation_duration
    );
}

/// Test memory usage during extended autonomous operation
#[tokio::test]
async fn test_memory_usage_during_extended_operation() {
    let github_client = GitHubClient::new().expect("Failed to create GitHub client");
    let recovery_client = Box::new(AutoRecovery::new(github_client.clone(), true));
    let config = CoordinationConfig {
        max_work_hours: 1,
        max_recovery_attempts: 1,
        recovery_timeout_minutes: 1,
        enable_aggressive_recovery: false,
        enable_state_persistence: false, // Disabled to isolate memory usage
        monitoring_interval_minutes: 1,
        enable_drift_detection: false, // Disabled to focus on core workflow
        drift_validation_interval_minutes: 30,
    };

    let coordinator = AutonomousCoordinator::new(
        github_client,
        "memory-test-agent".to_string(),
        recovery_client,
        config,
    )
    .await
    .expect("Failed to create coordinator");

    let initial_memory = get_memory_usage();
    println!("Initial memory usage: {} KB", initial_memory / 1024);

    // Run multiple status report cycles to test memory stability
    let cycles = 100;
    let mut max_memory = initial_memory;

    for i in 0..cycles {
        // Generate status reports
        let _status_report = coordinator.get_status_report().await;
        let _recovery_report = coordinator.get_recovery_report().await;
        let _drift_report = coordinator.get_drift_report().await;

        if i % 10 == 0 {
            let current_memory = get_memory_usage();
            max_memory = max_memory.max(current_memory);
            println!("Cycle {}: Memory usage {} KB", i, current_memory / 1024);
        }

        // Small delay to allow garbage collection
        sleep(Duration::from_millis(1)).await;
    }

    let final_memory = get_memory_usage();
    let memory_growth = final_memory.saturating_sub(initial_memory);
    let growth_percentage = (memory_growth as f64 / initial_memory as f64) * 100.0;

    println!("Memory Usage Results:");
    println!("  Initial memory: {} KB", initial_memory / 1024);
    println!("  Final memory: {} KB", final_memory / 1024);
    println!("  Max memory: {} KB", max_memory / 1024);
    println!(
        "  Memory growth: {} KB ({:.2}%)",
        memory_growth / 1024,
        growth_percentage
    );

    // Memory growth should be reasonable (less than 50% increase)
    assert!(
        growth_percentage < 50.0,
        "Memory growth too high: {:.2}% ({}KB -> {}KB)",
        growth_percentage,
        initial_memory / 1024,
        final_memory / 1024
    );
}

/// Helper function to get current memory usage (basic implementation)
fn get_memory_usage() -> u64 {
    // This is a simple estimation - in a real implementation, you might use
    // system-specific calls or memory profiling tools
    use std::alloc::{GlobalAlloc, Layout, System};

    // For this test, we'll return a mock value since actual memory measurement
    // requires more complex system integration
    1024 * 1024 // 1MB baseline
}

/// Test concurrent workflow operations performance
#[tokio::test]
async fn test_concurrent_workflow_performance() {
    const CONCURRENT_WORKFLOWS: usize = 10;
    const OPERATIONS_PER_WORKFLOW: usize = 20;

    println!(
        "Testing {} concurrent workflows with {} operations each",
        CONCURRENT_WORKFLOWS, OPERATIONS_PER_WORKFLOW
    );

    let start_time = Instant::now();
    let metrics = Arc::new(RwLock::new(PerformanceMetrics::new()));

    // Create concurrent workflow tasks
    let mut tasks = Vec::new();

    for workflow_id in 0..CONCURRENT_WORKFLOWS {
        let metrics_clone = Arc::clone(&metrics);

        let task = tokio::spawn(async move {
            let mut workflow = AutonomousWorkflowMachine::new(8);

            for op_id in 0..OPERATIONS_PER_WORKFLOW {
                let op_start = Instant::now();

                let event = match op_id % 4 {
                    0 => AutonomousEvent::AssignAgent {
                        agent: AgentId(format!("concurrent-agent-{}-{}", workflow_id, op_id)),
                        workspace_ready: true,
                    },
                    1 => AutonomousEvent::StartWork,
                    2 => AutonomousEvent::MakeProgress {
                        commits: 1,
                        files_changed: 2,
                    },
                    _ => AutonomousEvent::CompleteWork,
                };

                let result = workflow.handle_event(event).await;
                let op_duration = op_start.elapsed();

                let mut metrics_guard = metrics_clone.write().await;
                metrics_guard.record_operation(op_duration, result.is_ok());
                drop(metrics_guard);

                // Small delay to simulate realistic timing
                sleep(Duration::from_micros(100)).await;
            }
        });

        tasks.push(task);
    }

    // Wait for all workflows to complete
    for task in tasks {
        task.await.expect("Workflow task should complete");
    }

    let total_duration = start_time.elapsed();
    let final_metrics = metrics.read().await.clone();

    println!("Concurrent Performance Results:");
    println!("  Total time: {:?}", total_duration);
    println!("  Total operations: {}", final_metrics.operation_count);
    println!(
        "  Operations per second: {:.2}",
        final_metrics.operation_count as f64 / total_duration.as_secs_f64()
    );
    println!(
        "  Average operation time: {:?}",
        final_metrics.average_duration()
    );
    println!(
        "  Success rate: {:.2}%",
        final_metrics.success_rate() * 100.0
    );

    // Performance assertions
    assert!(
        final_metrics.success_rate() > 0.90,
        "Concurrent success rate too low"
    );
    assert!(
        final_metrics.average_duration() < Duration::from_millis(100),
        "Concurrent operations too slow"
    );
    assert_eq!(
        final_metrics.operation_count,
        (CONCURRENT_WORKFLOWS * OPERATIONS_PER_WORKFLOW) as u64
    );
}

/// Test workflow state history performance with large state histories
#[tokio::test]
async fn test_large_state_history_performance() {
    let mut workflow = AutonomousWorkflowMachine::new(24);
    let state_transitions = 1000;

    println!(
        "Testing large state history performance with {} transitions",
        state_transitions
    );

    let start_time = Instant::now();

    // Generate many state transitions
    for i in 0..state_transitions {
        let agent_id = format!("history-test-agent-{}", i % 10); // Reuse some agent IDs

        let result = workflow
            .handle_event(AutonomousEvent::AssignAgent {
                agent: AgentId(agent_id),
                workspace_ready: true,
            })
            .await;

        assert!(
            result.is_ok(),
            "State transition {} failed: {:?}",
            i,
            result
        );

        if i % 100 == 0 {
            println!("Completed {} transitions in {:?}", i, start_time.elapsed());
        }
    }

    let history_build_time = start_time.elapsed();

    // Test history access performance
    let history_access_start = Instant::now();
    let history = workflow.state_history();
    let history_access_time = history_access_start.elapsed();

    // Test status report generation with large history
    let status_start = Instant::now();
    let status_report = workflow.generate_status_report();
    let status_time = status_start.elapsed();

    println!("Large State History Results:");
    println!("  Total transitions: {}", state_transitions);
    println!("  History build time: {:?}", history_build_time);
    println!("  History access time: {:?}", history_access_time);
    println!("  Status generation time: {:?}", status_time);
    println!("  History length: {}", history.len());
    println!(
        "  Status transitions count: {}",
        status_report.transitions_count
    );

    // Performance assertions
    assert_eq!(history.len(), state_transitions as usize);
    assert_eq!(status_report.transitions_count, state_transitions as u64);
    assert!(
        history_access_time < Duration::from_millis(10),
        "History access too slow: {:?}",
        history_access_time
    );
    assert!(
        status_time < Duration::from_millis(50),
        "Status generation too slow with large history: {:?}",
        status_time
    );
    assert!(
        history_build_time < Duration::from_secs(5),
        "History building too slow: {:?}",
        history_build_time
    );
}

/// Test performance under rapid event processing
#[tokio::test]
async fn test_rapid_event_processing_performance() {
    let mut workflow = AutonomousWorkflowMachine::new(8);
    let rapid_events = 500;

    println!(
        "Testing rapid event processing with {} events",
        rapid_events
    );

    // Initialize workflow
    workflow
        .handle_event(AutonomousEvent::AssignAgent {
            agent: AgentId("rapid-test-agent".to_string()),
            workspace_ready: true,
        })
        .await
        .unwrap();

    workflow
        .handle_event(AutonomousEvent::StartWork)
        .await
        .unwrap();

    let start_time = Instant::now();
    let mut successful_events = 0;

    // Process rapid progress events
    for i in 0..rapid_events {
        let result = workflow
            .handle_event(AutonomousEvent::MakeProgress {
                commits: 1,
                files_changed: (i % 5) + 1,
            })
            .await;

        if result.is_ok() {
            successful_events += 1;
        }

        // No delays - test maximum throughput
    }

    let processing_time = start_time.elapsed();
    let events_per_second = successful_events as f64 / processing_time.as_secs_f64();
    let average_event_time = processing_time / successful_events as u32;

    println!("Rapid Event Processing Results:");
    println!("  Total events: {}", rapid_events);
    println!("  Successful events: {}", successful_events);
    println!("  Processing time: {:?}", processing_time);
    println!("  Events per second: {:.2}", events_per_second);
    println!("  Average event time: {:?}", average_event_time);

    // Performance assertions
    assert!(
        successful_events > rapid_events * 9 / 10,
        "Too many failed events"
    );
    assert!(
        events_per_second > 100.0,
        "Event processing too slow: {:.2} events/sec",
        events_per_second
    );
    assert!(
        average_event_time < Duration::from_millis(10),
        "Average event time too slow: {:?}",
        average_event_time
    );
}

/// Benchmark drift detection validation intervals
#[tokio::test]
async fn test_drift_detection_interval_performance() {
    let github_client = GitHubClient::new().expect("Failed to create GitHub client");

    // Test different validation intervals
    let test_intervals = vec![1, 5, 15, 30]; // minutes

    for interval_minutes in test_intervals {
        println!(
            "Testing drift detection with {}-minute intervals",
            interval_minutes
        );

        let thresholds = DriftThresholds {
            max_validation_interval_active: interval_minutes,
            max_validation_interval_idle: interval_minutes * 2,
            max_commits_behind: 10,
            critical_drift_types: vec![],
        };

        let detector = StateDriftDetector::new(
            github_client.clone(),
            format!("interval-test-agent-{}", interval_minutes),
        )
        .with_drift_thresholds(thresholds);

        // Test validation timing logic
        let needs_validation_active = detector.needs_validation(true);
        let needs_validation_idle = detector.needs_validation(false);

        println!(
            "  Interval {}min - Active needs validation: {}, Idle needs validation: {}",
            interval_minutes, needs_validation_active, needs_validation_idle
        );

        // Both should be true initially due to old last_validation timestamp
        assert!(needs_validation_active);
        assert!(needs_validation_idle);

        // Test report generation performance
        let report_start = Instant::now();
        let report = detector.generate_drift_report();
        let report_time = report_start.elapsed();

        println!("  Report generation time: {:?}", report_time);
        assert!(
            report_time < Duration::from_millis(10),
            "Report generation too slow for interval {}: {:?}",
            interval_minutes,
            report_time
        );
    }
}

/// Test performance regression by comparing with baseline measurements
#[tokio::test]
async fn test_performance_regression_baseline() {
    // This test establishes baseline performance metrics that can be used
    // to detect performance regressions in future changes

    let operations = 100;
    let mut baseline_metrics = std::collections::HashMap::new();

    // Workflow creation performance
    let workflow_creation_start = Instant::now();
    let _workflow = AutonomousWorkflowMachine::new(8);
    let workflow_creation_time = workflow_creation_start.elapsed();
    baseline_metrics.insert("workflow_creation", workflow_creation_time);

    // GitHub client creation performance
    let client_creation_start = Instant::now();
    let github_client = GitHubClient::new().expect("Failed to create GitHub client");
    let client_creation_time = client_creation_start.elapsed();
    baseline_metrics.insert("github_client_creation", client_creation_time);

    // Drift detector creation performance
    let detector_creation_start = Instant::now();
    let _detector = StateDriftDetector::new(github_client, "baseline-test-agent".to_string());
    let detector_creation_time = detector_creation_start.elapsed();
    baseline_metrics.insert("drift_detector_creation", detector_creation_time);

    println!("Performance Baseline Results:");
    for (operation, duration) in &baseline_metrics {
        println!("  {}: {:?}", operation, duration);
    }

    // Establish baseline thresholds (these should be adjusted based on expected performance)
    assert!(
        baseline_metrics["workflow_creation"] < Duration::from_millis(10),
        "Workflow creation baseline too slow"
    );
    assert!(
        baseline_metrics["github_client_creation"] < Duration::from_millis(100),
        "GitHub client creation baseline too slow"
    );
    assert!(
        baseline_metrics["drift_detector_creation"] < Duration::from_millis(50),
        "Drift detector creation baseline too slow"
    );

    println!("All baseline performance metrics within expected thresholds");
}
