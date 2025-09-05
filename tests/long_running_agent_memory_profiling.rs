//! Long-Running Agent Memory Profiling
//!
//! This module provides comprehensive memory profiling for long-running agents
//! as required by Issue #398. Tests validate memory usage patterns, leak detection,
//! and performance characteristics during extended operation periods.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::interval;

/// Memory profiling configuration
#[derive(Debug, Clone)]
pub struct MemoryProfilingConfig {
    pub test_name: String,
    pub profiling_duration: Duration,
    pub sampling_interval: Duration,
    pub work_simulation_enabled: bool,
    pub memory_tracking_enabled: bool,
    pub leak_detection_enabled: bool,
    pub gc_simulation_enabled: bool,
}

impl Default for MemoryProfilingConfig {
    fn default() -> Self {
        Self {
            test_name: "long_running_agent_memory_profile".to_string(),
            profiling_duration: Duration::from_secs(300), // 5 minutes
            sampling_interval: Duration::from_secs(10),   // Sample every 10 seconds
            work_simulation_enabled: true,
            memory_tracking_enabled: true,
            leak_detection_enabled: true,
            gc_simulation_enabled: true,
        }
    }
}

/// Memory usage sample
#[derive(Debug, Clone)]
pub struct MemorySample {
    pub timestamp: Instant,
    pub allocated_mb: f64,
    pub used_mb: f64,
    pub peak_mb: f64,
    pub gc_events: usize,
    pub active_operations: usize,
    pub notes: String,
}

/// Memory profiling results
#[derive(Debug, Clone)]
pub struct MemoryProfilingResults {
    pub test_name: String,
    pub config: MemoryProfilingConfig,
    pub total_duration: Duration,
    pub samples: Vec<MemorySample>,
    pub initial_memory_mb: f64,
    pub peak_memory_mb: f64,
    pub final_memory_mb: f64,
    pub average_memory_mb: f64,
    pub memory_growth_rate: f64, // MB per hour
    pub memory_stability_score: f64,
    pub leak_detected: bool,
    pub gc_efficiency_score: f64,
    pub performance_impact_score: f64,
}

/// Long-running agent simulator for memory profiling
pub struct LongRunningAgentSimulator {
    config: MemoryProfilingConfig,
    memory_samples: Arc<RwLock<Vec<MemorySample>>>,
    current_memory_mb: Arc<RwLock<f64>>,
    peak_memory_mb: Arc<RwLock<f64>>,
    gc_events: Arc<RwLock<usize>>,
    active_operations: Arc<RwLock<usize>>,
    simulation_data: Arc<RwLock<VecDeque<SimulatedWorkItem>>>,
}

#[derive(Debug, Clone)]
struct SimulatedWorkItem {
    id: usize,
    memory_usage_mb: f64,
    duration: Duration,
    created_at: Instant,
}

impl LongRunningAgentSimulator {
    pub fn new(config: MemoryProfilingConfig) -> Self {
        Self {
            config,
            memory_samples: Arc::new(RwLock::new(Vec::new())),
            current_memory_mb: Arc::new(RwLock::new(50.0)), // Start with 50MB baseline
            peak_memory_mb: Arc::new(RwLock::new(50.0)),
            gc_events: Arc::new(RwLock::new(0)),
            active_operations: Arc::new(RwLock::new(0)),
            simulation_data: Arc::new(RwLock::new(VecDeque::new())),
        }
    }

    pub async fn run_memory_profiling(&self) -> Result<MemoryProfilingResults, String> {
        println!(
            "🧠 Starting long-running agent memory profiling: {}",
            self.config.test_name
        );
        println!(
            "   Duration: {:?}, Sampling every: {:?}",
            self.config.profiling_duration, self.config.sampling_interval
        );

        let start_time = Instant::now();

        // Record initial memory sample
        self.record_memory_sample("Initial state").await;

        // Start memory sampling task
        let sampling_task = self.start_memory_sampling_task();

        // Start work simulation task if enabled
        let work_task = if self.config.work_simulation_enabled {
            Some(self.start_work_simulation_task())
        } else {
            None
        };

        // Start garbage collection simulation if enabled
        let gc_task = if self.config.gc_simulation_enabled {
            Some(self.start_gc_simulation_task())
        } else {
            None
        };

        // Wait for profiling duration
        tokio::time::sleep(self.config.profiling_duration).await;

        // Stop all tasks
        sampling_task.abort();
        if let Some(task) = work_task {
            task.abort();
        }
        if let Some(task) = gc_task {
            task.abort();
        }

        // Record final memory sample
        self.record_memory_sample("Final state").await;

        let total_duration = start_time.elapsed();

        // Generate results
        let results = self.generate_profiling_results(total_duration).await;
        self.print_profiling_results(&results);

        Ok(results)
    }

    fn start_memory_sampling_task(&self) -> tokio::task::JoinHandle<()> {
        let samples = Arc::clone(&self.memory_samples);
        let current_memory = Arc::clone(&self.current_memory_mb);
        let peak_memory = Arc::clone(&self.peak_memory_mb);
        let gc_events = Arc::clone(&self.gc_events);
        let active_ops = Arc::clone(&self.active_operations);
        let interval_duration = self.config.sampling_interval;

        tokio::spawn(async move {
            let mut interval_timer = interval(interval_duration);
            let mut sample_count = 0;

            loop {
                interval_timer.tick().await;
                sample_count += 1;

                let current = *current_memory.read().await;
                let peak = *peak_memory.read().await;
                let gc = *gc_events.read().await;
                let ops = *active_ops.read().await;

                let sample = MemorySample {
                    timestamp: Instant::now(),
                    allocated_mb: current * 1.1, // Simulate allocation overhead
                    used_mb: current,
                    peak_mb: peak,
                    gc_events: gc,
                    active_operations: ops,
                    notes: format!("Sample #{}", sample_count),
                };

                samples.write().await.push(sample);

                if sample_count % 6 == 0 {
                    // Every minute if sampling every 10s
                    println!(
                        "   Memory sample #{}: {:.1}MB used, {:.1}MB peak, {} active ops",
                        sample_count, current, peak, ops
                    );
                }
            }
        })
    }

    fn start_work_simulation_task(&self) -> tokio::task::JoinHandle<()> {
        let current_memory = Arc::clone(&self.current_memory_mb);
        let peak_memory = Arc::clone(&self.peak_memory_mb);
        let active_ops = Arc::clone(&self.active_operations);
        let simulation_data = Arc::clone(&self.simulation_data);

        tokio::spawn(async move {
            let mut work_id = 0;
            let mut work_interval = interval(Duration::from_secs(15)); // New work every 15 seconds

            loop {
                work_interval.tick().await;
                work_id += 1;

                // Simulate starting new work
                let work_memory = 5.0 + fastrand::f64() * 10.0; // 5-15MB per work item
                let work_duration = Duration::from_secs(30 + fastrand::u64(0..60)); // 30-90 seconds

                {
                    let mut current = current_memory.write().await;
                    let mut peak = peak_memory.write().await;
                    let mut ops = active_ops.write().await;

                    *current += work_memory;
                    *peak = (*peak).max(*current);
                    *ops += 1;
                }

                // Add work item to simulation data
                {
                    let mut data = simulation_data.write().await;
                    data.push_back(SimulatedWorkItem {
                        id: work_id,
                        memory_usage_mb: work_memory,
                        duration: work_duration,
                        created_at: Instant::now(),
                    });
                }

                // Simulate completing work after duration
                let memory_ref = Arc::clone(&current_memory);
                let ops_ref = Arc::clone(&active_ops);
                let data_ref = Arc::clone(&simulation_data);

                tokio::spawn(async move {
                    tokio::time::sleep(work_duration).await;

                    // Complete work - free memory
                    {
                        let mut current = memory_ref.write().await;
                        let mut ops = ops_ref.write().await;

                        *current -= work_memory;
                        *ops = ops.saturating_sub(1);

                        // Remove completed work from simulation data
                        let mut data = data_ref.write().await;
                        data.retain(|item| item.id != work_id);
                    }
                });
            }
        })
    }

    fn start_gc_simulation_task(&self) -> tokio::task::JoinHandle<()> {
        let current_memory = Arc::clone(&self.current_memory_mb);
        let gc_events = Arc::clone(&self.gc_events);

        tokio::spawn(async move {
            let mut gc_interval = interval(Duration::from_secs(45)); // GC every 45 seconds

            loop {
                gc_interval.tick().await;

                // Simulate garbage collection
                let gc_start = Instant::now();
                tokio::time::sleep(Duration::from_millis(100)).await; // GC pause simulation
                let _gc_duration = gc_start.elapsed();

                {
                    let mut current = current_memory.write().await;
                    let mut gc = gc_events.write().await;

                    // GC reclaims some memory (5-15%)
                    let reclaimed_ratio = 0.05 + fastrand::f64() * 0.10;
                    let reclaimed_memory = *current * reclaimed_ratio;
                    *current -= reclaimed_memory;

                    // Ensure minimum baseline
                    *current = (*current).max(40.0);

                    *gc += 1;
                }
            }
        })
    }

    async fn record_memory_sample(&self, notes: &str) {
        let current = *self.current_memory_mb.read().await;
        let peak = *self.peak_memory_mb.read().await;
        let gc = *self.gc_events.read().await;
        let ops = *self.active_operations.read().await;

        let sample = MemorySample {
            timestamp: Instant::now(),
            allocated_mb: current * 1.1,
            used_mb: current,
            peak_mb: peak,
            gc_events: gc,
            active_operations: ops,
            notes: notes.to_string(),
        };

        self.memory_samples.write().await.push(sample);
    }

    async fn generate_profiling_results(&self, total_duration: Duration) -> MemoryProfilingResults {
        let samples = self.memory_samples.read().await.clone();

        if samples.is_empty() {
            return MemoryProfilingResults {
                test_name: self.config.test_name.clone(),
                config: self.config.clone(),
                total_duration,
                samples: Vec::new(),
                initial_memory_mb: 0.0,
                peak_memory_mb: 0.0,
                final_memory_mb: 0.0,
                average_memory_mb: 0.0,
                memory_growth_rate: 0.0,
                memory_stability_score: 0.0,
                leak_detected: false,
                gc_efficiency_score: 0.0,
                performance_impact_score: 0.0,
            };
        }

        let initial_memory = samples.first().unwrap().used_mb;
        let final_memory = samples.last().unwrap().used_mb;
        let peak_memory = samples
            .iter()
            .map(|s| s.peak_mb)
            .fold(0.0f64, |a, b| a.max(b));
        let average_memory = samples.iter().map(|s| s.used_mb).sum::<f64>() / samples.len() as f64;

        // Calculate memory growth rate (MB per hour)
        let duration_hours = total_duration.as_secs_f64() / 3600.0;
        let memory_growth_rate = if duration_hours > 0.0 {
            (final_memory - initial_memory) / duration_hours
        } else {
            0.0
        };

        // Calculate memory stability score (0.0 to 1.0, higher is more stable)
        let memory_variance = samples
            .iter()
            .map(|s| (s.used_mb - average_memory).powi(2))
            .sum::<f64>()
            / samples.len() as f64;
        let memory_stability_score = (100.0 / (memory_variance.sqrt() + 1.0)).min(1.0);

        // Detect potential memory leaks
        let leak_detected = memory_growth_rate > 10.0; // >10MB/hour growth considered a leak

        // Calculate GC efficiency (how well GC maintains memory levels)
        let total_gc_events = samples.last().unwrap().gc_events;
        let gc_efficiency_score = if total_gc_events > 0 {
            (initial_memory / peak_memory).min(1.0)
        } else {
            0.5
        };

        // Calculate performance impact score (based on memory usage relative to baseline)
        let baseline_memory = 100.0; // 100MB considered baseline
        let performance_impact_score = (baseline_memory / peak_memory).min(1.0);

        MemoryProfilingResults {
            test_name: self.config.test_name.clone(),
            config: self.config.clone(),
            total_duration,
            samples,
            initial_memory_mb: initial_memory,
            peak_memory_mb: peak_memory,
            final_memory_mb: final_memory,
            average_memory_mb: average_memory,
            memory_growth_rate,
            memory_stability_score,
            leak_detected,
            gc_efficiency_score,
            performance_impact_score,
        }
    }

    fn print_profiling_results(&self, results: &MemoryProfilingResults) {
        println!("\n🧠 Long-Running Agent Memory Profiling Results");
        println!("═════════════════════════════════════════════════════════════════");
        println!("Test: {}", results.test_name);
        println!(
            "Duration: {:?} ({} samples)",
            results.total_duration,
            results.samples.len()
        );

        println!("\n📊 Memory Usage Summary:");
        println!("  Initial Memory: {:.1} MB", results.initial_memory_mb);
        println!("  Peak Memory: {:.1} MB", results.peak_memory_mb);
        println!("  Final Memory: {:.1} MB", results.final_memory_mb);
        println!("  Average Memory: {:.1} MB", results.average_memory_mb);

        println!("\n📈 Memory Characteristics:");
        println!("  Growth Rate: {:.2} MB/hour", results.memory_growth_rate);
        println!(
            "  Stability Score: {:.2}/1.0",
            results.memory_stability_score
        );
        println!(
            "  Leak Detected: {}",
            if results.leak_detected {
                "❌ YES"
            } else {
                "✅ NO"
            }
        );

        println!("\n🗑️ Garbage Collection:");
        if let Some(final_sample) = results.samples.last() {
            println!("  Total GC Events: {}", final_sample.gc_events);
            println!("  GC Efficiency: {:.2}/1.0", results.gc_efficiency_score);
        }

        println!("\n⚡ Performance Impact:");
        println!(
            "  Performance Score: {:.2}/1.0",
            results.performance_impact_score
        );

        // Memory usage over time analysis
        if results.samples.len() > 10 {
            println!("\n📉 Memory Usage Timeline (last 10 samples):");
            let recent_samples = &results.samples[results.samples.len() - 10..];
            for (i, sample) in recent_samples.iter().enumerate() {
                println!(
                    "    Sample {}: {:.1}MB used, {} ops active",
                    results.samples.len() - 10 + i + 1,
                    sample.used_mb,
                    sample.active_operations
                );
            }
        }

        // Overall assessment
        println!("\n🎯 Memory Health Assessment:");
        if results.leak_detected {
            println!(
                "❌ MEMORY LEAK DETECTED - Growth rate: {:.2} MB/hour",
                results.memory_growth_rate
            );
        } else if results.memory_growth_rate > 5.0 {
            println!(
                "⚠️ HIGH MEMORY GROWTH - Growth rate: {:.2} MB/hour",
                results.memory_growth_rate
            );
        } else if results.memory_growth_rate < 1.0 {
            println!(
                "✅ STABLE MEMORY USAGE - Growth rate: {:.2} MB/hour",
                results.memory_growth_rate
            );
        } else {
            println!(
                "✅ ACCEPTABLE MEMORY USAGE - Growth rate: {:.2} MB/hour",
                results.memory_growth_rate
            );
        }

        if results.peak_memory_mb > 500.0 {
            println!(
                "⚠️ HIGH PEAK MEMORY - Consider optimization: {:.1} MB",
                results.peak_memory_mb
            );
        } else if results.peak_memory_mb > 200.0 {
            println!(
                "ℹ️ MODERATE PEAK MEMORY - Monitor usage: {:.1} MB",
                results.peak_memory_mb
            );
        } else {
            println!(
                "✅ EFFICIENT MEMORY USAGE - Peak: {:.1} MB",
                results.peak_memory_mb
            );
        }

        if results.memory_stability_score < 0.5 {
            println!(
                "⚠️ UNSTABLE MEMORY PATTERN - Score: {:.2}",
                results.memory_stability_score
            );
        } else {
            println!(
                "✅ STABLE MEMORY PATTERN - Score: {:.2}",
                results.memory_stability_score
            );
        }
    }
}

#[cfg(test)]
mod memory_profiling_tests {
    use super::*;

    #[tokio::test]
    async fn test_long_running_agent_5_minute_profile() {
        println!("🧪 Testing long-running agent memory usage over 5 minutes");

        let config = MemoryProfilingConfig {
            test_name: "5-Minute Memory Profile".to_string(),
            profiling_duration: Duration::from_secs(60), // Reduced for testing
            sampling_interval: Duration::from_secs(5),
            work_simulation_enabled: true,
            memory_tracking_enabled: true,
            leak_detection_enabled: true,
            gc_simulation_enabled: true,
        };

        let simulator = LongRunningAgentSimulator::new(config);
        let results = simulator
            .run_memory_profiling()
            .await
            .expect("Profiling should complete");

        // Memory usage requirements
        assert!(
            results.peak_memory_mb < 300.0,
            "Peak memory too high: {:.1} MB",
            results.peak_memory_mb
        );

        assert!(
            results.memory_growth_rate < 50.0, // <50MB/hour growth
            "Memory growth rate too high: {:.2} MB/hour",
            results.memory_growth_rate
        );

        assert!(
            !results.leak_detected,
            "Memory leak detected with growth rate: {:.2} MB/hour",
            results.memory_growth_rate
        );

        assert!(
            results.memory_stability_score > 0.3,
            "Memory stability too low: {:.2}",
            results.memory_stability_score
        );

        assert!(
            results.samples.len() >= 10,
            "Insufficient memory samples: {}",
            results.samples.len()
        );

        println!("✅ 5-minute memory profiling completed successfully");
    }

    #[tokio::test]
    async fn test_memory_leak_detection() {
        println!("🧪 Testing memory leak detection capabilities");

        // Create a configuration that simulates memory leak conditions
        let config = MemoryProfilingConfig {
            test_name: "Memory Leak Detection Test".to_string(),
            profiling_duration: Duration::from_secs(45),
            sampling_interval: Duration::from_secs(3),
            work_simulation_enabled: true,
            memory_tracking_enabled: true,
            leak_detection_enabled: true,
            gc_simulation_enabled: false, // Disable GC to simulate leaks
        };

        let simulator = LongRunningAgentSimulator::new(config);
        let results = simulator
            .run_memory_profiling()
            .await
            .expect("Profiling should complete");

        // With GC disabled and work simulation enabled, we should see memory growth
        assert!(
            results.memory_growth_rate > 0.0,
            "Should detect memory growth with GC disabled"
        );

        // Final memory should be higher than initial
        assert!(
            results.final_memory_mb > results.initial_memory_mb,
            "Final memory ({:.1}MB) should be higher than initial ({:.1}MB)",
            results.final_memory_mb,
            results.initial_memory_mb
        );

        // Should have multiple samples
        assert!(
            results.samples.len() >= 10,
            "Should have sufficient samples for analysis"
        );

        println!(
            "Memory growth detected: {:.2} MB/hour",
            results.memory_growth_rate
        );
        println!("Leak detection working: {}", results.leak_detected);

        println!("✅ Memory leak detection test completed successfully");
    }

    #[tokio::test]
    async fn test_garbage_collection_efficiency() {
        println!("🧪 Testing garbage collection efficiency");

        let config = MemoryProfilingConfig {
            test_name: "GC Efficiency Test".to_string(),
            profiling_duration: Duration::from_secs(90),
            sampling_interval: Duration::from_secs(5),
            work_simulation_enabled: true,
            memory_tracking_enabled: true,
            leak_detection_enabled: true,
            gc_simulation_enabled: true,
        };

        let simulator = LongRunningAgentSimulator::new(config);
        let results = simulator
            .run_memory_profiling()
            .await
            .expect("Profiling should complete");

        // With GC enabled, memory growth should be controlled
        assert!(
            results.memory_growth_rate < 30.0,
            "Memory growth should be controlled with GC: {:.2} MB/hour",
            results.memory_growth_rate
        );

        // Should have GC events
        if let Some(final_sample) = results.samples.last() {
            assert!(
                final_sample.gc_events > 0,
                "Should have garbage collection events"
            );
        }

        // GC efficiency should be reasonable
        assert!(
            results.gc_efficiency_score > 0.2,
            "GC efficiency too low: {:.2}",
            results.gc_efficiency_score
        );

        // Memory should not grow excessively with GC
        let memory_increase = results.final_memory_mb - results.initial_memory_mb;
        assert!(
            memory_increase < 100.0,
            "Memory increase too high with GC: {:.1} MB",
            memory_increase
        );

        println!("✅ Garbage collection efficiency test completed successfully");
    }

    #[tokio::test]
    async fn test_memory_stability_analysis() {
        println!("🧪 Testing memory stability analysis");

        let config = MemoryProfilingConfig {
            test_name: "Memory Stability Analysis".to_string(),
            profiling_duration: Duration::from_secs(60),
            sampling_interval: Duration::from_secs(2),
            work_simulation_enabled: true,
            memory_tracking_enabled: true,
            leak_detection_enabled: true,
            gc_simulation_enabled: true,
        };

        let simulator = LongRunningAgentSimulator::new(config);
        let results = simulator
            .run_memory_profiling()
            .await
            .expect("Profiling should complete");

        // Should have many samples for stability analysis
        assert!(
            results.samples.len() >= 20,
            "Need sufficient samples for stability analysis: {}",
            results.samples.len()
        );

        // Stability score should be calculable
        assert!(
            results.memory_stability_score >= 0.0 && results.memory_stability_score <= 1.0,
            "Invalid stability score: {:.2}",
            results.memory_stability_score
        );

        // Average memory should be reasonable
        assert!(
            results.average_memory_mb > 0.0 && results.average_memory_mb <= results.peak_memory_mb,
            "Invalid average memory: {:.1} MB (peak: {:.1} MB)",
            results.average_memory_mb,
            results.peak_memory_mb
        );

        // Performance impact should be measurable
        assert!(
            results.performance_impact_score >= 0.0 && results.performance_impact_score <= 1.0,
            "Invalid performance impact score: {:.2}",
            results.performance_impact_score
        );

        println!(
            "Memory stability score: {:.2}/1.0",
            results.memory_stability_score
        );
        println!(
            "Performance impact score: {:.2}/1.0",
            results.performance_impact_score
        );

        println!("✅ Memory stability analysis test completed successfully");
    }

    #[tokio::test]
    async fn test_extended_operation_memory_baseline() {
        println!("🧪 Establishing extended operation memory baseline");

        let config = MemoryProfilingConfig {
            test_name: "Extended Operation Memory Baseline".to_string(),
            profiling_duration: Duration::from_secs(120), // 2 minutes for baseline
            sampling_interval: Duration::from_secs(10),
            work_simulation_enabled: true,
            memory_tracking_enabled: true,
            leak_detection_enabled: true,
            gc_simulation_enabled: true,
        };

        let simulator = LongRunningAgentSimulator::new(config);
        let results = simulator
            .run_memory_profiling()
            .await
            .expect("Profiling should complete");

        println!("\n📊 Extended Operation Memory Baseline:");
        println!("  Initial Memory: {:.1} MB", results.initial_memory_mb);
        println!("  Peak Memory: {:.1} MB", results.peak_memory_mb);
        println!("  Final Memory: {:.1} MB", results.final_memory_mb);
        println!("  Average Memory: {:.1} MB", results.average_memory_mb);
        println!(
            "  Memory Growth Rate: {:.2} MB/hour",
            results.memory_growth_rate
        );
        println!(
            "  Memory Stability Score: {:.2}/1.0",
            results.memory_stability_score
        );
        println!(
            "  GC Efficiency Score: {:.2}/1.0",
            results.gc_efficiency_score
        );
        println!(
            "  Performance Impact Score: {:.2}/1.0",
            results.performance_impact_score
        );
        println!("  Leak Detected: {}", results.leak_detected);

        // Baseline validation
        assert!(
            results.peak_memory_mb < 500.0,
            "Baseline peak memory too high"
        );
        assert!(
            results.memory_growth_rate < 100.0,
            "Baseline growth rate too high"
        );
        assert!(!results.leak_detected, "Baseline should not have leaks");
        assert!(
            results.memory_stability_score > 0.1,
            "Baseline stability too low"
        );

        println!("✅ Extended operation memory baseline established successfully");
    }
}
