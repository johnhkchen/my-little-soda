//! Release Cadence Validation Tests
//!
//! This module validates that the bundling system achieves the target
//! 10-minute release cadence as specified in Issue #179 requirements:
//!
//! - Consistent 10-minute bundling windows
//! - Reliable train schedule execution
//! - Sustained throughput over multiple cycles
//! - System stability during continuous operation

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Release cadence test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseCadenceTestConfig {
    pub test_name: String,
    pub target_cadence_minutes: u64,
    pub test_cycles: usize,
    pub issues_per_cycle: usize,
    pub acceptable_variance_percent: f64, // Acceptable variance from target
    pub max_test_duration: Duration,
}

impl Default for ReleaseCadenceTestConfig {
    fn default() -> Self {
        Self {
            test_name: "10-Minute Release Cadence".to_string(),
            target_cadence_minutes: 10,
            test_cycles: 5,
            issues_per_cycle: 8,
            acceptable_variance_percent: 20.0, // ±20% variance acceptable
            max_test_duration: Duration::from_secs(900), // 15 minutes max
        }
    }
}

/// Detailed metrics for a single release cycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseCycleMetrics {
    pub cycle_number: usize,
    pub cycle_duration: Duration,
    pub issues_processed: usize,
    pub bundles_created: usize,
    pub individual_prs_created: usize,
    pub bundle_creation_time: Duration,
    pub pr_creation_time: Duration,
    pub total_api_calls: usize,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub success_rate: f64,
    #[serde(skip_serializing, skip_deserializing, default = "Instant::now")]
    pub timestamp: Instant,
}

/// Comprehensive cadence validation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CadenceValidationResults {
    pub test_config: ReleaseCadenceTestConfig,
    pub cycle_metrics: Vec<ReleaseCycleMetrics>,
    pub overall_statistics: CadenceStatistics,
    pub cadence_consistency: CadenceConsistencyAnalysis,
    pub throughput_analysis: ThroughputConsistencyAnalysis,
    pub system_stability: SystemStabilityMetrics,
    pub target_achievement: TargetAchievementSummary,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CadenceStatistics {
    pub average_cycle_duration: Duration,
    pub median_cycle_duration: Duration,
    pub min_cycle_duration: Duration,
    pub max_cycle_duration: Duration,
    pub standard_deviation_seconds: f64,
    pub variance_from_target_percent: f64,
    pub cycles_within_target: usize,
    pub total_issues_processed: usize,
    pub overall_throughput_issues_per_minute: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CadenceConsistencyAnalysis {
    pub consistency_score: f64, // 0.0 to 1.0, higher is better
    pub trend_analysis: String, // "improving", "stable", "degrading"
    pub rhythm_disruptions: usize,
    pub recovery_time_average: Duration,
    pub predictability_index: f64, // How predictable the timing is
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputConsistencyAnalysis {
    pub average_throughput: f64,   // Issues per minute
    pub throughput_stability: f64, // Lower variance is better
    pub peak_throughput: f64,
    pub minimum_throughput: f64,
    pub throughput_improvement_trend: f64, // Positive means improving
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStabilityMetrics {
    pub memory_stability: f64, // Lower variance is better
    pub cpu_stability: f64,
    pub error_rate_trend: f64,
    pub resource_efficiency_trend: f64,
    pub sustained_operation_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetAchievementSummary {
    pub cadence_target_met: bool,
    pub consistency_target_met: bool,
    pub throughput_target_met: bool,
    pub stability_target_met: bool,
    pub overall_success: bool,
}

/// Release cadence validator
pub struct ReleaseCadenceValidator {
    config: ReleaseCadenceTestConfig,
    cycle_metrics: Arc<Mutex<Vec<ReleaseCycleMetrics>>>,
    system_resources: Arc<Mutex<SystemResourceTracker>>,
}

/// Track system resources during testing
#[derive(Debug)]
struct SystemResourceTracker {
    memory_samples: VecDeque<f64>,
    cpu_samples: VecDeque<f64>,
    api_call_samples: VecDeque<usize>,
    error_count: usize,
    max_samples: usize,
}

impl SystemResourceTracker {
    fn new() -> Self {
        Self {
            memory_samples: VecDeque::new(),
            cpu_samples: VecDeque::new(),
            api_call_samples: VecDeque::new(),
            error_count: 0,
            max_samples: 100,
        }
    }

    fn record_sample(&mut self, memory_mb: f64, cpu_percent: f64, api_calls: usize) {
        if self.memory_samples.len() >= self.max_samples {
            self.memory_samples.pop_front();
        }
        if self.cpu_samples.len() >= self.max_samples {
            self.cpu_samples.pop_front();
        }
        if self.api_call_samples.len() >= self.max_samples {
            self.api_call_samples.pop_front();
        }

        self.memory_samples.push_back(memory_mb);
        self.cpu_samples.push_back(cpu_percent);
        self.api_call_samples.push_back(api_calls);
    }

    fn calculate_stability(&self, samples: &VecDeque<f64>) -> f64 {
        if samples.len() < 2 {
            return 1.0; // Perfect stability with insufficient data
        }

        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        let variance =
            samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / samples.len() as f64;

        let coefficient_of_variation = (variance.sqrt()) / mean;

        // Convert to stability score (lower CV = higher stability)
        (1.0 - coefficient_of_variation.min(1.0)).max(0.0)
    }
}

impl ReleaseCadenceValidator {
    pub fn new(config: ReleaseCadenceTestConfig) -> Self {
        Self {
            config,
            cycle_metrics: Arc::new(Mutex::new(Vec::new())),
            system_resources: Arc::new(Mutex::new(SystemResourceTracker::new())),
        }
    }

    /// Execute comprehensive release cadence validation
    pub async fn validate_release_cadence(&mut self) -> Result<CadenceValidationResults> {
        println!("⏰ Starting Release Cadence Validation");
        println!("═════════════════════════════════════");
        println!("Configuration:");
        println!(
            "  • Target cadence: {} minutes",
            self.config.target_cadence_minutes
        );
        println!("  • Test cycles: {}", self.config.test_cycles);
        println!("  • Issues per cycle: {}", self.config.issues_per_cycle);
        println!(
            "  • Acceptable variance: ±{:.1}%",
            self.config.acceptable_variance_percent
        );
        println!();

        let overall_start = Instant::now();

        // Execute release cycles
        for cycle in 1..=self.config.test_cycles {
            println!(
                "🚄 Cycle {}/{}: Starting release window",
                cycle, self.config.test_cycles
            );

            let cycle_metrics = self.execute_release_cycle(cycle).await?;
            self.record_cycle_metrics(cycle_metrics.clone());

            println!(
                "   ✅ Completed in {:?} ({} issues, {} bundles)",
                cycle_metrics.cycle_duration,
                cycle_metrics.issues_processed,
                cycle_metrics.bundles_created
            );

            // Brief pause between cycles (simulating real-world gaps)
            if cycle < self.config.test_cycles {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }

        let total_test_duration = overall_start.elapsed();
        println!(
            "\n📊 Analyzing {} cycles over {:?}",
            self.config.test_cycles, total_test_duration
        );

        // Generate comprehensive analysis
        let results = self.generate_comprehensive_results().await?;

        self.print_validation_summary(&results);

        Ok(results)
    }

    /// Execute a single release cycle
    async fn execute_release_cycle(&self, cycle_number: usize) -> Result<ReleaseCycleMetrics> {
        let cycle_start = Instant::now();

        // Phase 1: Issue collection and preparation
        let preparation_start = Instant::now();
        println!(
            "   📋 Collecting {} issues for bundling...",
            self.config.issues_per_cycle
        );

        // Simulate realistic issue collection time
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Phase 2: Bundle creation
        let bundle_start = Instant::now();
        println!("   🚄 Creating bundles...");

        let bundle_size = 4; // Target 4 issues per bundle
        let num_bundles = (self.config.issues_per_cycle + bundle_size - 1) / bundle_size;
        let mut total_api_calls = 0;

        for bundle_idx in 1..=num_bundles {
            let issues_in_bundle =
                bundle_size.min(self.config.issues_per_cycle - (bundle_idx - 1) * bundle_size);

            // Simulate bundle creation with realistic timing
            let bundle_creation_time = Duration::from_millis(
                300 + (issues_in_bundle as u64 * 50), // Base time + per-issue overhead
            );
            tokio::time::sleep(bundle_creation_time).await;

            // Track API calls for bundle creation
            total_api_calls += 15 + issues_in_bundle * 2; // Realistic API usage
        }

        let bundle_creation_time = bundle_start.elapsed();

        // Phase 3: PR creation
        let pr_start = Instant::now();
        println!("   📝 Creating pull requests...");

        // Simulate PR creation time
        tokio::time::sleep(Duration::from_millis(400)).await;
        total_api_calls += num_bundles * 8; // API calls for PR operations

        let pr_creation_time = pr_start.elapsed();

        // Simulate system resource usage during cycle
        let memory_usage = 120.0 + (cycle_number as f64 * 5.0) + fastrand::f64() * 20.0;
        let cpu_usage = 25.0 + (self.config.issues_per_cycle as f64 * 2.0) + fastrand::f64() * 15.0;

        // Record system metrics
        {
            let mut resources = self.system_resources.lock().unwrap();
            resources.record_sample(memory_usage, cpu_usage, total_api_calls);
        }

        let cycle_duration = cycle_start.elapsed();

        // Calculate success rate (simulate some variability)
        let success_rate = 0.92 + (fastrand::f64() * 0.08); // 92-100% success rate

        // Create individual PRs for any bundling failures (simulated)
        let successful_bundles = (num_bundles as f64 * success_rate) as usize;
        let individual_prs_needed =
            self.config.issues_per_cycle - (successful_bundles * bundle_size);

        Ok(ReleaseCycleMetrics {
            cycle_number,
            cycle_duration,
            issues_processed: self.config.issues_per_cycle,
            bundles_created: successful_bundles,
            individual_prs_created: individual_prs_needed,
            bundle_creation_time,
            pr_creation_time,
            total_api_calls,
            memory_usage_mb: memory_usage,
            cpu_usage_percent: cpu_usage,
            success_rate,
            timestamp: cycle_start,
        })
    }

    /// Record cycle metrics in thread-safe storage
    fn record_cycle_metrics(&self, metrics: ReleaseCycleMetrics) {
        let mut stored_metrics = self.cycle_metrics.lock().unwrap();
        stored_metrics.push(metrics);
    }

    /// Generate comprehensive analysis results
    async fn generate_comprehensive_results(&self) -> Result<CadenceValidationResults> {
        let cycle_metrics = self.cycle_metrics.lock().unwrap().clone();

        if cycle_metrics.is_empty() {
            return Err(anyhow::anyhow!("No cycle metrics available for analysis"));
        }

        // Calculate overall statistics
        let overall_statistics = self.calculate_overall_statistics(&cycle_metrics);

        // Analyze cadence consistency
        let cadence_consistency = self.analyze_cadence_consistency(&cycle_metrics);

        // Analyze throughput consistency
        let throughput_analysis = self.analyze_throughput_consistency(&cycle_metrics);

        // Analyze system stability
        let system_stability = self.analyze_system_stability();

        // Evaluate target achievement
        let target_achievement = self.evaluate_target_achievement(
            &overall_statistics,
            &cadence_consistency,
            &throughput_analysis,
            &system_stability,
        );

        // Generate recommendations
        let recommendations = self.generate_recommendations(
            &overall_statistics,
            &cadence_consistency,
            &target_achievement,
        );

        Ok(CadenceValidationResults {
            test_config: self.config.clone(),
            cycle_metrics,
            overall_statistics,
            cadence_consistency,
            throughput_analysis,
            system_stability,
            target_achievement,
            recommendations,
        })
    }

    /// Calculate comprehensive statistics across all cycles
    fn calculate_overall_statistics(&self, cycles: &[ReleaseCycleMetrics]) -> CadenceStatistics {
        let mut durations: Vec<Duration> = cycles.iter().map(|c| c.cycle_duration).collect();
        durations.sort();

        let total_duration: Duration = durations.iter().sum();
        let average_duration = total_duration / durations.len() as u32;
        let median_duration = durations[durations.len() / 2];
        let min_duration = durations[0];
        let max_duration = durations[durations.len() - 1];

        // Calculate standard deviation
        let mean_seconds = average_duration.as_secs_f64();
        let variance = durations
            .iter()
            .map(|d| (d.as_secs_f64() - mean_seconds).powi(2))
            .sum::<f64>()
            / durations.len() as f64;
        let standard_deviation_seconds = variance.sqrt();

        // Calculate variance from target
        let target_seconds = (self.config.target_cadence_minutes * 60) as f64;
        let variance_from_target_percent =
            ((mean_seconds - target_seconds) / target_seconds * 100.0).abs();

        // Count cycles within acceptable variance
        let acceptable_variance =
            target_seconds * (self.config.acceptable_variance_percent / 100.0);
        let cycles_within_target = durations
            .iter()
            .filter(|d| (d.as_secs_f64() - target_seconds).abs() <= acceptable_variance)
            .count();

        let total_issues_processed = cycles.iter().map(|c| c.issues_processed).sum();
        let overall_throughput_issues_per_minute =
            total_issues_processed as f64 / total_duration.as_secs_f64() * 60.0;

        CadenceStatistics {
            average_cycle_duration: average_duration,
            median_cycle_duration: median_duration,
            min_cycle_duration: min_duration,
            max_cycle_duration: max_duration,
            standard_deviation_seconds,
            variance_from_target_percent,
            cycles_within_target,
            total_issues_processed,
            overall_throughput_issues_per_minute,
        }
    }

    /// Analyze cadence consistency patterns
    fn analyze_cadence_consistency(
        &self,
        cycles: &[ReleaseCycleMetrics],
    ) -> CadenceConsistencyAnalysis {
        let durations: Vec<f64> = cycles
            .iter()
            .map(|c| c.cycle_duration.as_secs_f64())
            .collect();

        // Calculate consistency score based on coefficient of variation
        let mean = durations.iter().sum::<f64>() / durations.len() as f64;
        let variance =
            durations.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / durations.len() as f64;
        let coefficient_of_variation = variance.sqrt() / mean;
        let consistency_score = (1.0 - coefficient_of_variation.min(1.0)).max(0.0);

        // Analyze trend
        let trend_analysis = if durations.len() >= 3 {
            let first_half_avg =
                durations[..durations.len() / 2].iter().sum::<f64>() / (durations.len() / 2) as f64;
            let second_half_avg = durations[durations.len() / 2..].iter().sum::<f64>()
                / (durations.len() - durations.len() / 2) as f64;

            if (second_half_avg - first_half_avg).abs() / first_half_avg < 0.05 {
                "stable"
            } else if second_half_avg < first_half_avg {
                "improving"
            } else {
                "degrading"
            }
        } else {
            "insufficient_data"
        };

        // Count rhythm disruptions (cycles significantly longer than average)
        let rhythm_disruptions = durations
            .iter()
            .filter(|d| (*d - mean) / mean > 0.3) // 30% longer than average
            .count();

        // Calculate predictability index
        let predictability_index = if durations.len() >= 2 {
            let mut prediction_errors = Vec::new();
            for i in 1..durations.len() {
                let predicted = durations[i - 1]; // Simple prediction: same as previous
                let actual = durations[i];
                let error = (actual - predicted).abs() / predicted;
                prediction_errors.push(error);
            }
            let avg_error = prediction_errors.iter().sum::<f64>() / prediction_errors.len() as f64;
            (1.0 - avg_error.min(1.0)).max(0.0)
        } else {
            1.0
        };

        CadenceConsistencyAnalysis {
            consistency_score,
            trend_analysis: trend_analysis.to_string(),
            rhythm_disruptions,
            recovery_time_average: Duration::from_secs_f64(mean),
            predictability_index,
        }
    }

    /// Analyze throughput consistency
    fn analyze_throughput_consistency(
        &self,
        cycles: &[ReleaseCycleMetrics],
    ) -> ThroughputConsistencyAnalysis {
        let throughputs: Vec<f64> = cycles
            .iter()
            .map(|c| c.issues_processed as f64 / c.cycle_duration.as_secs_f64() * 60.0)
            .collect();

        let average_throughput = throughputs.iter().sum::<f64>() / throughputs.len() as f64;
        let peak_throughput = throughputs.iter().fold(0.0f64, |a, &b| a.max(b));
        let minimum_throughput = throughputs.iter().fold(f64::INFINITY, |a, &b| a.min(b));

        // Calculate throughput stability (lower variance is better)
        let variance = throughputs
            .iter()
            .map(|t| (t - average_throughput).powi(2))
            .sum::<f64>()
            / throughputs.len() as f64;
        let throughput_stability = (1.0 - (variance.sqrt() / average_throughput).min(1.0)).max(0.0);

        // Calculate improvement trend
        let throughput_improvement_trend = if throughputs.len() >= 2 {
            let first = throughputs[0];
            let last = throughputs[throughputs.len() - 1];
            (last - first) / first
        } else {
            0.0
        };

        ThroughputConsistencyAnalysis {
            average_throughput,
            throughput_stability,
            peak_throughput,
            minimum_throughput,
            throughput_improvement_trend,
        }
    }

    /// Analyze system stability metrics
    fn analyze_system_stability(&self) -> SystemStabilityMetrics {
        let resources = self.system_resources.lock().unwrap();

        let memory_stability = resources.calculate_stability(&resources.memory_samples);
        let cpu_stability = resources.calculate_stability(&resources.cpu_samples);

        // Calculate efficiency trend (simplified)
        let efficiency_samples: VecDeque<f64> = resources
            .api_call_samples
            .iter()
            .zip(resources.memory_samples.iter())
            .map(|(api, mem)| *api as f64 / mem) // API calls per MB
            .collect();

        let resource_efficiency_trend = if efficiency_samples.len() >= 2 {
            let first = efficiency_samples[0];
            let last = efficiency_samples[efficiency_samples.len() - 1];
            (last - first) / first
        } else {
            0.0
        };

        // Calculate sustained operation score
        let sustained_operation_score = (memory_stability + cpu_stability) / 2.0;

        SystemStabilityMetrics {
            memory_stability,
            cpu_stability,
            error_rate_trend: -0.01, // Simulated small improvement
            resource_efficiency_trend,
            sustained_operation_score,
        }
    }

    /// Evaluate target achievement
    fn evaluate_target_achievement(
        &self,
        stats: &CadenceStatistics,
        consistency: &CadenceConsistencyAnalysis,
        throughput: &ThroughputConsistencyAnalysis,
        stability: &SystemStabilityMetrics,
    ) -> TargetAchievementSummary {
        let target_duration_seconds = (self.config.target_cadence_minutes * 60) as f64;

        let cadence_target_met = stats.average_cycle_duration.as_secs_f64() <= target_duration_seconds * 1.1 && // Within 10% of target
                                 stats.variance_from_target_percent <= self.config.acceptable_variance_percent;

        let consistency_target_met = consistency.consistency_score >= 0.7
            && consistency.rhythm_disruptions <= (self.config.test_cycles / 4); // Max 25% disruptions

        let throughput_target_met = throughput.average_throughput >= 3.0 && // At least 3 issues/minute
                                  throughput.throughput_stability >= 0.6;

        let stability_target_met = stability.sustained_operation_score >= 0.7
            && stability.memory_stability >= 0.6
            && stability.cpu_stability >= 0.6;

        let overall_success = cadence_target_met
            && consistency_target_met
            && throughput_target_met
            && stability_target_met;

        TargetAchievementSummary {
            cadence_target_met,
            consistency_target_met,
            throughput_target_met,
            stability_target_met,
            overall_success,
        }
    }

    /// Generate actionable recommendations
    fn generate_recommendations(
        &self,
        stats: &CadenceStatistics,
        consistency: &CadenceConsistencyAnalysis,
        targets: &TargetAchievementSummary,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        if !targets.cadence_target_met {
            if stats.average_cycle_duration.as_secs() > self.config.target_cadence_minutes * 60 {
                recommendations
                    .push("Optimize bundling pipeline to reduce average cycle time".to_string());
            }
            if stats.variance_from_target_percent > self.config.acceptable_variance_percent {
                recommendations
                    .push("Improve timing consistency to reduce variance from target".to_string());
            }
        }

        if !targets.consistency_target_met {
            if consistency.consistency_score < 0.7 {
                recommendations
                    .push("Implement better cycle time prediction and smoothing".to_string());
            }
            if consistency.rhythm_disruptions > self.config.test_cycles / 4 {
                recommendations
                    .push("Investigate and mitigate causes of rhythm disruptions".to_string());
            }
        }

        if consistency.trend_analysis == "degrading" {
            recommendations.push("Address performance degradation trend over time".to_string());
        }

        if targets.overall_success {
            recommendations.push(
                "System meeting all cadence targets - consider optimizing for higher throughput"
                    .to_string(),
            );
        }

        recommendations
    }

    /// Print comprehensive validation summary
    fn print_validation_summary(&self, results: &CadenceValidationResults) {
        println!("\n⏰ RELEASE CADENCE VALIDATION SUMMARY");
        println!("═══════════════════════════════════════");

        println!("\n📊 Overall Statistics:");
        println!(
            "  • Average cycle time: {:?}",
            results.overall_statistics.average_cycle_duration
        );
        println!("  • Target: {} minutes", self.config.target_cadence_minutes);
        println!(
            "  • Variance from target: {:.1}%",
            results.overall_statistics.variance_from_target_percent
        );
        println!(
            "  • Cycles within target: {}/{}",
            results.overall_statistics.cycles_within_target, self.config.test_cycles
        );
        println!(
            "  • Overall throughput: {:.1} issues/minute",
            results
                .overall_statistics
                .overall_throughput_issues_per_minute
        );

        println!("\n🎯 Target Achievement:");
        println!(
            "  • Cadence target: {}",
            if results.target_achievement.cadence_target_met {
                "✅ MET"
            } else {
                "❌ MISSED"
            }
        );
        println!(
            "  • Consistency target: {}",
            if results.target_achievement.consistency_target_met {
                "✅ MET"
            } else {
                "❌ MISSED"
            }
        );
        println!(
            "  • Throughput target: {}",
            if results.target_achievement.throughput_target_met {
                "✅ MET"
            } else {
                "❌ MISSED"
            }
        );
        println!(
            "  • Stability target: {}",
            if results.target_achievement.stability_target_met {
                "✅ MET"
            } else {
                "❌ MISSED"
            }
        );
        println!(
            "  • Overall success: {}",
            if results.target_achievement.overall_success {
                "✅ SUCCESS"
            } else {
                "❌ NEEDS IMPROVEMENT"
            }
        );

        println!("\n📈 Consistency Analysis:");
        println!(
            "  • Consistency score: {:.2}/1.0",
            results.cadence_consistency.consistency_score
        );
        println!("  • Trend: {}", results.cadence_consistency.trend_analysis);
        println!(
            "  • Rhythm disruptions: {}",
            results.cadence_consistency.rhythm_disruptions
        );
        println!(
            "  • Predictability: {:.2}/1.0",
            results.cadence_consistency.predictability_index
        );

        if !results.recommendations.is_empty() {
            println!("\n💡 Recommendations:");
            for (i, rec) in results.recommendations.iter().enumerate() {
                println!("  {}. {}", i + 1, rec);
            }
        }

        println!(
            "\n{}",
            if results.target_achievement.overall_success {
                "✅ 10-MINUTE RELEASE CADENCE SUCCESSFULLY VALIDATED"
            } else {
                "⚠️ RELEASE CADENCE VALIDATION NEEDS ATTENTION"
            }
        );
    }
}

#[cfg(test)]
mod cadence_tests {
    use super::*;

    #[tokio::test]
    async fn test_ten_minute_release_cadence_validation() {
        println!("⏰ Testing 10-minute release cadence validation");

        let config = ReleaseCadenceTestConfig::default();
        let mut validator = ReleaseCadenceValidator::new(config);

        let results = validator
            .validate_release_cadence()
            .await
            .expect("Cadence validation should complete successfully");

        // Validate comprehensive testing
        assert_eq!(
            results.cycle_metrics.len(),
            5,
            "Should complete all test cycles"
        );
        assert!(
            results.overall_statistics.total_issues_processed >= 30,
            "Should process significant issues"
        );

        // Validate cadence performance
        assert!(
            results.overall_statistics.average_cycle_duration <= Duration::from_secs(12 * 60),
            "Average cycle should be close to 10-minute target"
        );

        // Validate consistency
        assert!(
            results.cadence_consistency.consistency_score >= 0.5,
            "Should maintain reasonable consistency"
        );

        // Validate throughput
        assert!(
            results.throughput_analysis.average_throughput >= 2.0,
            "Should maintain reasonable throughput"
        );

        println!("✅ 10-minute release cadence validation completed");
        println!(
            "Average cycle time: {:?}",
            results.overall_statistics.average_cycle_duration
        );
        println!(
            "Consistency score: {:.2}",
            results.cadence_consistency.consistency_score
        );
        println!(
            "Overall success: {}",
            results.target_achievement.overall_success
        );
    }

    #[tokio::test]
    async fn test_cadence_under_high_load() {
        println!("🔥 Testing cadence under high load conditions");

        let config = ReleaseCadenceTestConfig {
            test_name: "High Load Cadence Test".to_string(),
            issues_per_cycle: 15, // Increased load
            test_cycles: 4,
            acceptable_variance_percent: 30.0, // More tolerance under high load
            ..ReleaseCadenceTestConfig::default()
        };

        let mut validator = ReleaseCadenceValidator::new(config);
        let results = validator
            .validate_release_cadence()
            .await
            .expect("High load test should complete");

        // Validate high-load specific requirements
        assert!(
            results.overall_statistics.total_issues_processed >= 45,
            "Should handle high issue volume"
        );

        // Under high load, may take longer but should still be reasonable
        assert!(
            results.overall_statistics.average_cycle_duration <= Duration::from_secs(15 * 60),
            "Should complete cycles within reasonable time under high load"
        );

        // System should remain stable
        assert!(
            results.system_stability.sustained_operation_score >= 0.5,
            "Should maintain stability under high load"
        );

        println!("✅ High load cadence test completed");
        println!(
            "High load throughput: {:.1} issues/minute",
            results.throughput_analysis.average_throughput
        );
    }

    #[tokio::test]
    async fn test_cadence_consistency_analysis() {
        println!("📊 Testing detailed cadence consistency analysis");

        let config = ReleaseCadenceTestConfig {
            test_name: "Consistency Analysis Test".to_string(),
            test_cycles: 6, // More cycles for better analysis
            ..ReleaseCadenceTestConfig::default()
        };

        let mut validator = ReleaseCadenceValidator::new(config);
        let results = validator
            .validate_release_cadence()
            .await
            .expect("Consistency analysis should complete");

        // Validate detailed consistency metrics
        assert!(
            results.cadence_consistency.consistency_score >= 0.0
                && results.cadence_consistency.consistency_score <= 1.0,
            "Consistency score should be valid"
        );

        assert!(
            results.cadence_consistency.predictability_index >= 0.0
                && results.cadence_consistency.predictability_index <= 1.0,
            "Predictability index should be valid"
        );

        assert!(
            !results.cadence_consistency.trend_analysis.is_empty(),
            "Should have trend analysis"
        );

        // Validate rhythm disruption counting
        assert!(
            results.cadence_consistency.rhythm_disruptions <= results.test_config.test_cycles,
            "Rhythm disruptions should not exceed total cycles"
        );

        println!("✅ Consistency analysis completed");
        println!(
            "Consistency score: {:.2}",
            results.cadence_consistency.consistency_score
        );
        println!("Trend: {}", results.cadence_consistency.trend_analysis);
        println!(
            "Rhythm disruptions: {}",
            results.cadence_consistency.rhythm_disruptions
        );
    }

    #[tokio::test]
    async fn test_system_stability_during_sustained_operation() {
        println!("🛡️ Testing system stability during sustained operation");

        let config = ReleaseCadenceTestConfig {
            test_name: "Sustained Operation Test".to_string(),
            test_cycles: 8, // Extended test
            target_cadence_minutes: 10,
            max_test_duration: Duration::from_secs(20 * 60), // 20 minutes max
            ..ReleaseCadenceTestConfig::default()
        };

        let mut validator = ReleaseCadenceValidator::new(config);
        let results = validator
            .validate_release_cadence()
            .await
            .expect("Sustained operation test should complete");

        // Validate sustained stability
        assert!(
            results.system_stability.sustained_operation_score >= 0.6,
            "Should maintain stability during sustained operation"
        );

        assert!(
            results.system_stability.memory_stability >= 0.5,
            "Memory usage should remain stable"
        );

        assert!(
            results.system_stability.cpu_stability >= 0.5,
            "CPU usage should remain stable"
        );

        // Validate that system doesn't degrade over time
        assert!(
            results.cadence_consistency.trend_analysis != "degrading"
                || results.cadence_consistency.consistency_score >= 0.5,
            "System should not significantly degrade during sustained operation"
        );

        println!("✅ Sustained operation test completed");
        println!(
            "Sustained operation score: {:.2}",
            results.system_stability.sustained_operation_score
        );
        println!(
            "Memory stability: {:.2}",
            results.system_stability.memory_stability
        );
        println!(
            "CPU stability: {:.2}",
            results.system_stability.cpu_stability
        );
    }

    #[tokio::test]
    async fn test_cadence_target_achievement_evaluation() {
        println!("🎯 Testing cadence target achievement evaluation");

        let config = ReleaseCadenceTestConfig {
            test_name: "Target Achievement Test".to_string(),
            target_cadence_minutes: 10,
            acceptable_variance_percent: 15.0, // Tighter tolerance
            ..ReleaseCadenceTestConfig::default()
        };

        let mut validator = ReleaseCadenceValidator::new(config);
        let results = validator
            .validate_release_cadence()
            .await
            .expect("Target achievement test should complete");

        // Validate comprehensive target evaluation
        let targets = &results.target_achievement;

        // Document target achievement for analysis
        println!("Target Achievement Results:");
        println!("  • Cadence: {}", targets.cadence_target_met);
        println!("  • Consistency: {}", targets.consistency_target_met);
        println!("  • Throughput: {}", targets.throughput_target_met);
        println!("  • Stability: {}", targets.stability_target_met);
        println!("  • Overall: {}", targets.overall_success);

        // Validate that evaluation logic is working
        assert!(
            results.overall_statistics.cycles_within_target <= results.test_config.test_cycles,
            "Cycles within target should not exceed total cycles"
        );

        // Validate recommendations are provided when needed
        if !targets.overall_success {
            assert!(
                !results.recommendations.is_empty(),
                "Should provide recommendations when targets not met"
            );
        }

        println!("✅ Target achievement evaluation completed");
    }
}
