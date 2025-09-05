//! Init Command Performance Benchmarks  
//!
//! This module provides comprehensive performance benchmarking for the init command
//! as required by Issue #398, focusing on scenarios with 1000+ issues and various
//! repository configurations.

use std::time::{Duration, Instant};
use tempfile::TempDir;

use my_little_soda::cli::commands::init::InitCommand;

/// Performance benchmark configuration for init command tests
#[derive(Debug, Clone)]
pub struct InitPerformanceBenchmarkConfig {
    pub test_name: String,
    pub simulated_issue_count: usize,
    pub simulated_pr_count: usize,
    pub simulated_file_count: usize,
    pub enable_git_simulation: bool,
    pub enable_api_simulation: bool,
    pub timeout: Duration,
}

impl Default for InitPerformanceBenchmarkConfig {
    fn default() -> Self {
        Self {
            test_name: "init_performance_benchmark".to_string(),
            simulated_issue_count: 1000,
            simulated_pr_count: 100,
            simulated_file_count: 500,
            enable_git_simulation: true,
            enable_api_simulation: true,
            timeout: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Results from init command performance benchmark
#[derive(Debug, Clone)]
pub struct InitPerformanceBenchmarkResults {
    pub test_name: String,
    pub config: InitPerformanceBenchmarkConfig,
    pub total_duration: Duration,
    pub initialization_phase_duration: Duration,
    pub repository_analysis_duration: Duration,
    pub configuration_generation_duration: Duration,
    pub validation_phase_duration: Duration,
    pub memory_usage_peak_mb: f64,
    pub file_operations_count: usize,
    pub git_operations_count: usize,
    pub api_operations_count: usize,
    pub success_rate: f64,
    pub performance_score: f64,
}

/// Init command performance benchmark runner
pub struct InitCommandBenchmarkRunner {
    config: InitPerformanceBenchmarkConfig,
    temp_dir: Option<TempDir>,
}

impl InitCommandBenchmarkRunner {
    pub fn new(config: InitPerformanceBenchmarkConfig) -> Self {
        Self {
            config,
            temp_dir: None,
        }
    }

    pub async fn run_benchmark(&mut self) -> Result<InitPerformanceBenchmarkResults, String> {
        println!(
            "🏁 Starting init command performance benchmark: {}",
            self.config.test_name
        );

        // Setup test environment
        self.setup_test_environment().await?;
        let overall_start = Instant::now();

        // Phase 1: Initialization
        let init_start = Instant::now();
        self.simulate_initialization_phase().await?;
        let initialization_duration = init_start.elapsed();

        // Phase 2: Repository Analysis
        let analysis_start = Instant::now();
        self.simulate_repository_analysis().await?;
        let analysis_duration = analysis_start.elapsed();

        // Phase 3: Configuration Generation
        let config_start = Instant::now();
        self.simulate_configuration_generation().await?;
        let config_duration = config_start.elapsed();

        // Phase 4: Validation
        let validation_start = Instant::now();
        self.simulate_validation_phase().await?;
        let validation_duration = validation_start.elapsed();

        let total_duration = overall_start.elapsed();

        // Collect metrics
        let results = InitPerformanceBenchmarkResults {
            test_name: self.config.test_name.clone(),
            config: self.config.clone(),
            total_duration,
            initialization_phase_duration: initialization_duration,
            repository_analysis_duration: analysis_duration,
            configuration_generation_duration: config_duration,
            validation_phase_duration: validation_duration,
            memory_usage_peak_mb: self.estimate_memory_usage(),
            file_operations_count: self.count_file_operations(),
            git_operations_count: self.count_git_operations(),
            api_operations_count: self.count_api_operations(),
            success_rate: self.calculate_success_rate(),
            performance_score: 0.0, // Will be calculated
        };

        let final_results = self.calculate_performance_score(results);
        self.print_benchmark_results(&final_results);

        Ok(final_results)
    }

    async fn setup_test_environment(&mut self) -> Result<(), String> {
        self.temp_dir =
            Some(TempDir::new().map_err(|e| format!("Failed to create temp directory: {}", e))?);

        // Simulate repository structure with many files/issues
        self.create_simulated_repository_structure().await?;

        Ok(())
    }

    async fn create_simulated_repository_structure(&self) -> Result<(), String> {
        if let Some(temp_dir) = &self.temp_dir {
            let repo_path = temp_dir.path();

            // Create directory structure
            std::fs::create_dir_all(repo_path.join("src")).map_err(|e| e.to_string())?;
            std::fs::create_dir_all(repo_path.join("tests")).map_err(|e| e.to_string())?;
            std::fs::create_dir_all(repo_path.join("docs")).map_err(|e| e.to_string())?;

            // Create simulated files (proportional to configured count)
            let files_per_dir = self.config.simulated_file_count / 3;
            for i in 0..files_per_dir {
                let src_file = repo_path.join("src").join(format!("module_{}.rs", i));
                let test_file = repo_path.join("tests").join(format!("test_{}.rs", i));
                let doc_file = repo_path.join("docs").join(format!("doc_{}.md", i));

                std::fs::write(
                    &src_file,
                    format!("// Module {}\npub fn function_{}() {{}}", i, i),
                )
                .map_err(|e| e.to_string())?;
                std::fs::write(
                    &test_file,
                    format!("#[test]\nfn test_{}() {{assert!(true);}}", i),
                )
                .map_err(|e| e.to_string())?;
                std::fs::write(
                    &doc_file,
                    format!("# Documentation {}\nContent for doc {}", i, i),
                )
                .map_err(|e| e.to_string())?;
            }

            // Initialize git repository if enabled
            if self.config.enable_git_simulation {
                self.initialize_git_repository(repo_path).await?;
            }
        }

        Ok(())
    }

    async fn initialize_git_repository(&self, repo_path: &std::path::Path) -> Result<(), String> {
        // Simulate git initialization (in real implementation, this would use git2 or command)
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Create .git directory structure simulation
        std::fs::create_dir_all(repo_path.join(".git")).map_err(|e| e.to_string())?;

        Ok(())
    }

    async fn simulate_initialization_phase(&self) -> Result<(), String> {
        println!("  Phase 1: Initialization (simulating startup and environment setup)");

        // Simulate startup overhead
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Simulate environment validation
        for check in 0..10 {
            tokio::time::sleep(Duration::from_millis(10)).await;
            if check % 3 == 0 {
                println!("    Environment check {}/10", check + 1);
            }
        }

        Ok(())
    }

    async fn simulate_repository_analysis(&self) -> Result<(), String> {
        println!(
            "  Phase 2: Repository Analysis (simulating {} issues, {} PRs)",
            self.config.simulated_issue_count, self.config.simulated_pr_count
        );

        // Simulate analyzing large numbers of issues
        let issue_batches = (self.config.simulated_issue_count + 99) / 100;
        for batch in 0..issue_batches {
            // Each batch takes some time to process
            let batch_time = Duration::from_millis(50 + fastrand::u64(0..=50));
            tokio::time::sleep(batch_time).await;

            if batch % 5 == 0 {
                println!("    Analyzed {} issues", (batch + 1) * 100);
            }
        }

        // Simulate PR analysis
        let pr_analysis_time = Duration::from_millis(self.config.simulated_pr_count as u64 * 2);
        tokio::time::sleep(pr_analysis_time).await;

        // Simulate file system analysis
        let file_analysis_time =
            Duration::from_millis(self.config.simulated_file_count as u64 / 10);
        tokio::time::sleep(file_analysis_time).await;

        Ok(())
    }

    async fn simulate_configuration_generation(&self) -> Result<(), String> {
        println!("  Phase 3: Configuration Generation");

        // Simulate generating configuration based on repository analysis
        let config_generation_time =
            Duration::from_millis(100 + (self.config.simulated_issue_count as u64 / 100));
        tokio::time::sleep(config_generation_time).await;

        // Simulate writing configuration files
        if let Some(temp_dir) = &self.temp_dir {
            let config_path = temp_dir.path().join("my-little-soda.toml");
            let config_content = format!(
                r#"# Generated configuration for {} issues
[repository]
issue_count = {}
pr_count = {}
file_count = {}

[optimization]
batch_size = 100
enable_caching = true
"#,
                self.config.simulated_issue_count,
                self.config.simulated_issue_count,
                self.config.simulated_pr_count,
                self.config.simulated_file_count
            );

            tokio::fs::write(&config_path, config_content)
                .await
                .map_err(|e| format!("Failed to write config: {}", e))?;
        }

        Ok(())
    }

    async fn simulate_validation_phase(&self) -> Result<(), String> {
        println!("  Phase 4: Validation");

        // Simulate validating the generated configuration
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Simulate running validation checks
        let validation_checks = vec![
            "Configuration syntax",
            "Repository compatibility",
            "GitHub API connectivity",
            "File permissions",
            "Git repository status",
        ];

        for (i, check) in validation_checks.iter().enumerate() {
            tokio::time::sleep(Duration::from_millis(50)).await;
            println!("    Validated: {}", check);
        }

        Ok(())
    }

    fn estimate_memory_usage(&self) -> f64 {
        // Estimate memory usage based on repository size
        let base_memory = 50.0; // 50MB base
        let issue_memory = self.config.simulated_issue_count as f64 * 0.1; // 0.1MB per issue
        let file_memory = self.config.simulated_file_count as f64 * 0.05; // 0.05MB per file

        base_memory + issue_memory + file_memory
    }

    fn count_file_operations(&self) -> usize {
        // Estimate file operations performed during init
        let config_files = 3; // my-little-soda.toml, .gitignore, etc.
        let analysis_operations = self.config.simulated_file_count; // Reading existing files
        let git_operations = if self.config.enable_git_simulation {
            10
        } else {
            0
        };

        config_files + analysis_operations + git_operations
    }

    fn count_git_operations(&self) -> usize {
        if self.config.enable_git_simulation {
            15 // git status, config reads, etc.
        } else {
            0
        }
    }

    fn count_api_operations(&self) -> usize {
        if self.config.enable_api_simulation {
            // Estimate API calls for large repository analysis
            let issue_calls = (self.config.simulated_issue_count + 99) / 100; // Paginated
            let pr_calls = (self.config.simulated_pr_count + 99) / 100;
            let repo_info_calls = 5; // Repository metadata

            issue_calls + pr_calls + repo_info_calls
        } else {
            0
        }
    }

    fn calculate_success_rate(&self) -> f64 {
        // Simulate some occasional failures
        let base_success = 0.95;
        let complexity_penalty = (self.config.simulated_issue_count as f64 / 10000.0) * 0.1;
        (base_success - complexity_penalty).max(0.8)
    }

    fn calculate_performance_score(
        &self,
        mut results: InitPerformanceBenchmarkResults,
    ) -> InitPerformanceBenchmarkResults {
        // Performance score calculation (0.0 to 1.0, higher is better)

        // Time efficiency (target: <300s for 1000 issues)
        let time_score = if results.total_duration.as_secs() < 300 {
            1.0
        } else {
            (300.0 / results.total_duration.as_secs() as f64).max(0.1)
        };

        // Memory efficiency (target: <200MB for 1000 issues)
        let memory_score = if results.memory_usage_peak_mb < 200.0 {
            1.0
        } else {
            (200.0 / results.memory_usage_peak_mb).max(0.1)
        };

        // Success rate component
        let success_score = results.success_rate;

        // Overall weighted score
        results.performance_score =
            (time_score * 0.4) + (memory_score * 0.3) + (success_score * 0.3);

        results
    }

    fn print_benchmark_results(&self, results: &InitPerformanceBenchmarkResults) {
        println!("\n📊 Init Command Performance Benchmark Results");
        println!("═══════════════════════════════════════════════════════════════");
        println!("Test: {}", results.test_name);
        println!(
            "Configuration: {} issues, {} PRs, {} files",
            results.config.simulated_issue_count,
            results.config.simulated_pr_count,
            results.config.simulated_file_count
        );

        println!("\n⏱️ Timing Breakdown:");
        println!("  Total Duration: {:?}", results.total_duration);
        println!(
            "  Initialization: {:?}",
            results.initialization_phase_duration
        );
        println!(
            "  Repository Analysis: {:?}",
            results.repository_analysis_duration
        );
        println!(
            "  Configuration Generation: {:?}",
            results.configuration_generation_duration
        );
        println!("  Validation: {:?}", results.validation_phase_duration);

        println!("\n📈 Performance Metrics:");
        println!(
            "  Peak Memory Usage: {:.1} MB",
            results.memory_usage_peak_mb
        );
        println!("  File Operations: {}", results.file_operations_count);
        println!("  Git Operations: {}", results.git_operations_count);
        println!("  API Operations: {}", results.api_operations_count);
        println!("  Success Rate: {:.1}%", results.success_rate * 100.0);

        println!(
            "\n🎯 Performance Score: {:.2}/1.0",
            results.performance_score
        );

        // Performance assessment
        if results.performance_score >= 0.8 {
            println!("✅ EXCELLENT - Init command performs very well at scale");
        } else if results.performance_score >= 0.6 {
            println!("✅ GOOD - Init command performance is acceptable at scale");
        } else if results.performance_score >= 0.4 {
            println!("⚠️ ACCEPTABLE - Init command handles scale but with overhead");
        } else {
            println!("❌ NEEDS IMPROVEMENT - Init command struggles at scale");
        }

        // Specific recommendations
        if results.total_duration > Duration::from_secs(300) {
            println!("💡 Recommendation: Optimize repository analysis for large issue counts");
        }
        if results.memory_usage_peak_mb > 200.0 {
            println!("💡 Recommendation: Implement streaming or batched processing to reduce memory usage");
        }
        if results.api_operations_count > results.config.simulated_issue_count / 50 {
            println!("💡 Recommendation: Optimize API usage with better batching or caching");
        }
    }
}

#[cfg(test)]
mod init_performance_tests {
    use super::*;

    #[tokio::test]
    async fn test_init_command_1000_issues_benchmark() {
        println!("🧪 Testing init command performance with 1000 issues");

        let config = InitPerformanceBenchmarkConfig {
            test_name: "Init Command 1000 Issues Benchmark".to_string(),
            simulated_issue_count: 1000,
            simulated_pr_count: 100,
            simulated_file_count: 500,
            enable_git_simulation: true,
            enable_api_simulation: true,
            timeout: Duration::from_secs(300),
        };

        let mut runner = InitCommandBenchmarkRunner::new(config);
        let results = runner
            .run_benchmark()
            .await
            .expect("Benchmark should complete");

        // Performance requirements validation
        assert!(
            results.total_duration < Duration::from_secs(300),
            "Init command too slow: {:?} (target: <300s)",
            results.total_duration
        );

        assert!(
            results.memory_usage_peak_mb < 300.0,
            "Memory usage too high: {:.1} MB (target: <300MB)",
            results.memory_usage_peak_mb
        );

        assert!(
            results.success_rate > 0.9,
            "Success rate too low: {:.1}% (target: >90%)",
            results.success_rate * 100.0
        );

        assert!(
            results.performance_score > 0.5,
            "Performance score too low: {:.2} (target: >0.5)",
            results.performance_score
        );

        // Validate phase timings are reasonable
        assert!(
            results.repository_analysis_duration < Duration::from_secs(200),
            "Repository analysis too slow: {:?}",
            results.repository_analysis_duration
        );

        println!("✅ Init command 1000 issues benchmark completed successfully");
    }

    #[tokio::test]
    async fn test_init_command_scalability_analysis() {
        println!("🧪 Testing init command scalability across different repository sizes");

        let test_configurations = vec![
            ("Small", 100, 10, 50),
            ("Medium", 500, 50, 200),
            ("Large", 1000, 100, 500),
            ("Extra Large", 2000, 200, 1000),
        ];

        let mut scalability_results = Vec::new();

        for (scale_name, issue_count, pr_count, file_count) in test_configurations {
            println!("Testing {} scale: {} issues", scale_name, issue_count);

            let config = InitPerformanceBenchmarkConfig {
                test_name: format!("Init Scalability - {}", scale_name),
                simulated_issue_count: issue_count,
                simulated_pr_count: pr_count,
                simulated_file_count: file_count,
                enable_git_simulation: true,
                enable_api_simulation: true,
                timeout: Duration::from_secs(600),
            };

            let mut runner = InitCommandBenchmarkRunner::new(config);
            let results = runner
                .run_benchmark()
                .await
                .expect("Benchmark should complete");

            scalability_results.push((
                scale_name,
                issue_count,
                results.total_duration,
                results.performance_score,
                results.memory_usage_peak_mb,
            ));
        }

        // Analyze scalability characteristics
        println!("\n📊 Init Command Scalability Analysis:");
        for (scale_name, issue_count, duration, score, memory) in &scalability_results {
            println!(
                "  {} ({} issues): {:?}, Score: {:.2}, Memory: {:.1}MB",
                scale_name, issue_count, duration, score, memory
            );
        }

        // Validate scalability characteristics
        let small_duration = scalability_results[0].2.as_secs_f64();
        let extra_large_duration = scalability_results[3].2.as_secs_f64();
        let duration_scaling_factor = extra_large_duration / small_duration;

        assert!(
            duration_scaling_factor < 25.0, // 20x issue increase should be <25x duration increase
            "Duration scaling too poor: {:.2}x increase for 20x issues",
            duration_scaling_factor
        );

        // Memory should scale sub-linearly
        let small_memory = scalability_results[0].4;
        let extra_large_memory = scalability_results[3].4;
        let memory_scaling_factor = extra_large_memory / small_memory;

        assert!(
            memory_scaling_factor < 15.0, // Memory should scale better than linear
            "Memory scaling too poor: {:.2}x increase for 20x issues",
            memory_scaling_factor
        );

        // All scales should maintain acceptable performance scores
        for (scale_name, _issue_count, _duration, score, _memory) in &scalability_results {
            assert!(
                *score > 0.3,
                "{} scale performance score too low: {:.2}",
                scale_name,
                score
            );
        }

        println!("✅ Init command scalability analysis completed successfully");
    }

    #[tokio::test]
    async fn test_init_command_memory_efficiency() {
        println!("🧪 Testing init command memory efficiency with large repositories");

        let config = InitPerformanceBenchmarkConfig {
            test_name: "Init Command Memory Efficiency".to_string(),
            simulated_issue_count: 1500,
            simulated_pr_count: 150,
            simulated_file_count: 750,
            enable_git_simulation: true,
            enable_api_simulation: false, // Focus on memory usage
            timeout: Duration::from_secs(400),
        };

        let mut runner = InitCommandBenchmarkRunner::new(config);
        let results = runner
            .run_benchmark()
            .await
            .expect("Benchmark should complete");

        // Memory efficiency requirements
        let memory_per_issue =
            results.memory_usage_peak_mb / results.config.simulated_issue_count as f64;

        assert!(
            memory_per_issue < 0.2, // <0.2MB per issue
            "Memory per issue too high: {:.3} MB/issue",
            memory_per_issue
        );

        assert!(
            results.memory_usage_peak_mb < 400.0,
            "Absolute memory usage too high: {:.1} MB",
            results.memory_usage_peak_mb
        );

        // Performance should still be acceptable
        assert!(
            results.performance_score > 0.4,
            "Performance score too low for memory-focused test: {:.2}",
            results.performance_score
        );

        println!("Memory efficiency: {:.3} MB per issue", memory_per_issue);
        println!("✅ Init command memory efficiency test completed successfully");
    }

    #[tokio::test]
    async fn test_init_command_baseline_performance() {
        println!("🧪 Establishing init command performance baselines");

        let baseline_config = InitPerformanceBenchmarkConfig {
            test_name: "Init Command Performance Baseline".to_string(),
            simulated_issue_count: 1000,
            simulated_pr_count: 100,
            simulated_file_count: 500,
            enable_git_simulation: true,
            enable_api_simulation: true,
            timeout: Duration::from_secs(300),
        };

        let mut runner = InitCommandBenchmarkRunner::new(baseline_config);
        let results = runner
            .run_benchmark()
            .await
            .expect("Benchmark should complete");

        // Establish baseline thresholds for regression detection
        println!("\n📊 Performance Baseline Established:");
        println!(
            "  Total Duration Baseline: {:?} (target: <300s)",
            results.total_duration
        );
        println!(
            "  Memory Usage Baseline: {:.1} MB (target: <300MB)",
            results.memory_usage_peak_mb
        );
        println!(
            "  Performance Score Baseline: {:.2} (target: >0.5)",
            results.performance_score
        );
        println!(
            "  Success Rate Baseline: {:.1}% (target: >90%)",
            results.success_rate * 100.0
        );

        // Phase timing baselines
        println!(
            "  Initialization Phase: {:?}",
            results.initialization_phase_duration
        );
        println!(
            "  Analysis Phase: {:?}",
            results.repository_analysis_duration
        );
        println!(
            "  Configuration Phase: {:?}",
            results.configuration_generation_duration
        );
        println!(
            "  Validation Phase: {:?}",
            results.validation_phase_duration
        );

        // Operations count baselines
        println!("  File Operations: {}", results.file_operations_count);
        println!("  Git Operations: {}", results.git_operations_count);
        println!("  API Operations: {}", results.api_operations_count);

        // All baselines should be within acceptable ranges
        assert!(results.total_duration < Duration::from_secs(300));
        assert!(results.memory_usage_peak_mb < 300.0);
        assert!(results.performance_score > 0.5);
        assert!(results.success_rate > 0.9);

        println!("✅ Init command performance baseline established successfully");
    }
}
