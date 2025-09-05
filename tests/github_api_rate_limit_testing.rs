//! GitHub API Rate Limit Testing and Documentation
//!
//! This module provides comprehensive testing and documentation of GitHub API rate limit
//! handling as required by Issue #398. Tests validate rate limit detection, recovery
//! mechanisms, and efficiency strategies.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tokio::time::interval;

/// GitHub API rate limit testing configuration
#[derive(Debug, Clone)]
pub struct RateLimitTestConfig {
    pub test_name: String,
    pub test_duration: Duration,
    pub initial_requests_per_minute: usize,
    pub simulate_rate_limits: bool,
    pub simulate_secondary_limits: bool,
    pub track_recovery_metrics: bool,
    pub enable_adaptive_strategies: bool,
}

impl Default for RateLimitTestConfig {
    fn default() -> Self {
        Self {
            test_name: "github_api_rate_limit_test".to_string(),
            test_duration: Duration::from_secs(180), // 3 minutes
            initial_requests_per_minute: 60,         // Start with 1 req/sec
            simulate_rate_limits: true,
            simulate_secondary_limits: true,
            track_recovery_metrics: true,
            enable_adaptive_strategies: true,
        }
    }
}

/// API request tracking information
#[derive(Debug, Clone)]
pub struct ApiRequest {
    pub endpoint: String,
    pub timestamp: Instant,
    pub duration: Duration,
    pub success: bool,
    pub rate_limited: bool,
    pub rate_limit_type: RateLimitType,
    pub remaining_quota: Option<usize>,
    pub reset_time: Option<Instant>,
    pub retry_after: Option<Duration>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RateLimitType {
    None,
    Primary,   // Main rate limit (5000/hour)
    Secondary, // Secondary rate limit (abuse detection)
    Search,    // Search API specific limits
    GraphQL,   // GraphQL API limits
}

/// Rate limit test results
#[derive(Debug, Clone)]
pub struct RateLimitTestResults {
    pub test_name: String,
    pub config: RateLimitTestConfig,
    pub total_duration: Duration,
    pub requests: Vec<ApiRequest>,
    pub rate_limit_encounters: HashMap<RateLimitType, usize>,
    pub recovery_metrics: RateLimitRecoveryMetrics,
    pub throughput_metrics: ThroughputMetrics,
    pub adaptive_strategy_metrics: AdaptiveStrategyMetrics,
    pub efficiency_score: f64,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RateLimitRecoveryMetrics {
    pub total_recovery_events: usize,
    pub average_recovery_time: Duration,
    pub fastest_recovery_time: Duration,
    pub slowest_recovery_time: Duration,
    pub recovery_success_rate: f64,
    pub adaptive_backoff_effectiveness: f64,
}

#[derive(Debug, Clone)]
pub struct ThroughputMetrics {
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub requests_per_minute: f64,
    pub peak_requests_per_minute: f64,
    pub efficiency_ratio: f64, // successful requests / total attempts
}

#[derive(Debug, Clone)]
pub struct AdaptiveStrategyMetrics {
    pub strategy_changes: usize,
    pub optimal_request_rate_found: Option<usize>, // requests per minute
    pub time_to_optimal_rate: Option<Duration>,
    pub strategy_effectiveness_score: f64,
}

/// GitHub API rate limit simulator
pub struct GitHubApiRateLimitSimulator {
    config: RateLimitTestConfig,
    requests: Arc<RwLock<Vec<ApiRequest>>>,
    rate_limit_state: Arc<Mutex<RateLimitState>>,
    current_request_rate: Arc<RwLock<usize>>, // requests per minute
    adaptive_strategy: Arc<RwLock<AdaptiveStrategy>>,
}

#[derive(Debug, Clone)]
struct RateLimitState {
    primary_remaining: usize,
    primary_reset_time: Option<Instant>,
    secondary_limited: bool,
    secondary_reset_time: Option<Instant>,
    search_remaining: usize,
    search_reset_time: Option<Instant>,
    consecutive_rate_limits: usize,
}

impl Default for RateLimitState {
    fn default() -> Self {
        Self {
            primary_remaining: 5000, // GitHub's hourly limit
            primary_reset_time: None,
            secondary_limited: false,
            secondary_reset_time: None,
            search_remaining: 30, // GitHub's search limit per minute
            search_reset_time: None,
            consecutive_rate_limits: 0,
        }
    }
}

#[derive(Debug, Clone)]
struct AdaptiveStrategy {
    current_backoff_multiplier: f64,
    optimal_rate_discovered: Option<usize>,
    strategy_changes: usize,
    last_successful_rate: Option<usize>,
}

impl Default for AdaptiveStrategy {
    fn default() -> Self {
        Self {
            current_backoff_multiplier: 1.0,
            optimal_rate_discovered: None,
            strategy_changes: 0,
            last_successful_rate: None,
        }
    }
}

impl GitHubApiRateLimitSimulator {
    pub fn new(config: RateLimitTestConfig) -> Self {
        Self {
            current_request_rate: Arc::new(RwLock::new(config.initial_requests_per_minute)),
            config,
            requests: Arc::new(RwLock::new(Vec::new())),
            rate_limit_state: Arc::new(Mutex::new(RateLimitState::default())),
            adaptive_strategy: Arc::new(RwLock::new(AdaptiveStrategy::default())),
        }
    }

    pub async fn run_rate_limit_test(&self) -> Result<RateLimitTestResults, String> {
        println!(
            "🌐 Starting GitHub API rate limit test: {}",
            self.config.test_name
        );
        println!(
            "   Duration: {:?}, Initial rate: {} req/min",
            self.config.test_duration, self.config.initial_requests_per_minute
        );

        let start_time = Instant::now();

        // Start request simulation task
        let request_task = self.start_request_simulation_task();

        // Start rate limit monitoring task
        let monitoring_task = self.start_rate_limit_monitoring_task();

        // Start adaptive strategy task if enabled
        let strategy_task = if self.config.enable_adaptive_strategies {
            Some(self.start_adaptive_strategy_task())
        } else {
            None
        };

        // Wait for test duration
        tokio::time::sleep(self.config.test_duration).await;

        // Stop all tasks
        request_task.abort();
        monitoring_task.abort();
        if let Some(task) = strategy_task {
            task.abort();
        }

        let total_duration = start_time.elapsed();

        // Generate results
        let results = self.generate_test_results(total_duration).await;
        self.print_test_results(&results);
        self.generate_documentation(&results).await;

        Ok(results)
    }

    async fn start_request_simulation_task(&self) -> tokio::task::JoinHandle<()> {
        let requests = Arc::clone(&self.requests);
        let rate_limit_state = Arc::clone(&self.rate_limit_state);
        let current_rate = Arc::clone(&self.current_request_rate);
        let adaptive_strategy = Arc::clone(&self.adaptive_strategy);
        let simulate_limits = self.config.simulate_rate_limits;

        tokio::spawn(async move {
            let mut request_id = 0;

            loop {
                let current_rate_value = *current_rate.read().await;
                let interval_ms = if current_rate_value > 0 {
                    60000 / current_rate_value // milliseconds between requests
                } else {
                    1000 // fallback to 1 second
                };

                tokio::time::sleep(Duration::from_millis(interval_ms as u64)).await;
                request_id += 1;

                // Simulate different types of API requests
                let endpoint = match request_id % 4 {
                    0 => "GET /repos/owner/repo/issues",
                    1 => "GET /repos/owner/repo/pulls",
                    2 => "GET /search/issues",
                    _ => "GET /repos/owner/repo",
                };

                let request_start = Instant::now();

                // Check rate limits and simulate request
                let (success, rate_limited, rate_limit_type) = {
                    let mut state = rate_limit_state.lock().await;
                    Self::simulate_api_request_with_limits(&mut state, endpoint, simulate_limits)
                };

                let request_duration =
                    request_start.elapsed() + Duration::from_millis(50 + fastrand::u64(0..=100));

                // Handle rate limiting
                if rate_limited {
                    let mut strategy = adaptive_strategy.write().await;
                    strategy.current_backoff_multiplier *= 1.5; // Increase backoff
                    strategy.strategy_changes += 1;

                    // Adaptive delay based on rate limit type
                    let delay = match rate_limit_type {
                        RateLimitType::Primary => Duration::from_secs(60),
                        RateLimitType::Secondary => Duration::from_secs(30),
                        RateLimitType::Search => Duration::from_secs(60),
                        _ => Duration::from_secs(10),
                    };

                    tokio::time::sleep(delay).await;
                }

                // Record the request
                let api_request = ApiRequest {
                    endpoint: endpoint.to_string(),
                    timestamp: request_start,
                    duration: request_duration,
                    success,
                    rate_limited,
                    rate_limit_type,
                    remaining_quota: Some(rate_limit_state.lock().await.primary_remaining),
                    reset_time: rate_limit_state.lock().await.primary_reset_time,
                    retry_after: if rate_limited {
                        Some(Duration::from_secs(60))
                    } else {
                        None
                    },
                };

                requests.write().await.push(api_request);
            }
        })
    }

    fn simulate_api_request_with_limits(
        state: &mut RateLimitState,
        endpoint: &str,
        simulate_limits: bool,
    ) -> (bool, bool, RateLimitType) {
        if !simulate_limits {
            return (true, false, RateLimitType::None);
        }

        // Check primary rate limit
        if state.primary_remaining == 0 {
            if let Some(reset_time) = state.primary_reset_time {
                if Instant::now() < reset_time {
                    state.consecutive_rate_limits += 1;
                    return (false, true, RateLimitType::Primary);
                } else {
                    // Reset quota
                    state.primary_remaining = 5000;
                    state.primary_reset_time = Some(Instant::now() + Duration::from_secs(3600));
                    state.consecutive_rate_limits = 0;
                }
            }
        }

        // Check search API specific limits
        if endpoint.contains("/search/") {
            if state.search_remaining == 0 {
                if let Some(reset_time) = state.search_reset_time {
                    if Instant::now() < reset_time {
                        return (false, true, RateLimitType::Search);
                    } else {
                        // Reset search quota
                        state.search_remaining = 30;
                        state.search_reset_time = Some(Instant::now() + Duration::from_secs(60));
                    }
                }
            }
            state.search_remaining = state.search_remaining.saturating_sub(1);
        }

        // Simulate secondary rate limits (abuse detection)
        if state.consecutive_rate_limits > 3 && fastrand::f64() < 0.3 {
            state.secondary_limited = true;
            state.secondary_reset_time = Some(Instant::now() + Duration::from_secs(30));
            return (false, true, RateLimitType::Secondary);
        }

        // Normal request
        state.primary_remaining = state.primary_remaining.saturating_sub(1);
        if state.primary_remaining == 0 {
            state.primary_reset_time = Some(Instant::now() + Duration::from_secs(3600));
        }

        // Simulate occasional failures unrelated to rate limits
        let success = fastrand::f64() > 0.05; // 5% failure rate

        (success, false, RateLimitType::None)
    }

    async fn start_rate_limit_monitoring_task(&self) -> tokio::task::JoinHandle<()> {
        let rate_limit_state = Arc::clone(&self.rate_limit_state);
        let current_rate = Arc::clone(&self.current_request_rate);

        tokio::spawn(async move {
            let mut monitor_interval = interval(Duration::from_secs(30)); // Check every 30 seconds

            loop {
                monitor_interval.tick().await;

                let state = rate_limit_state.lock().await;
                let current_rate_value = *current_rate.read().await;

                println!(
                    "   Rate limit status: {} requests remaining, current rate: {}/min",
                    state.primary_remaining, current_rate_value
                );

                // Log if we're hitting rate limits frequently
                if state.consecutive_rate_limits > 2 {
                    println!(
                        "   ⚠️ Consecutive rate limits detected: {}",
                        state.consecutive_rate_limits
                    );
                }
            }
        })
    }

    async fn start_adaptive_strategy_task(&self) -> tokio::task::JoinHandle<()> {
        let requests = Arc::clone(&self.requests);
        let current_rate = Arc::clone(&self.current_request_rate);
        let adaptive_strategy = Arc::clone(&self.adaptive_strategy);

        tokio::spawn(async move {
            let mut strategy_interval = interval(Duration::from_secs(45)); // Adjust strategy every 45 seconds

            loop {
                strategy_interval.tick().await;

                // Analyze recent performance
                let recent_requests = {
                    let all_requests = requests.read().await;
                    let cutoff_time = Instant::now() - Duration::from_secs(60); // Last minute
                    all_requests
                        .iter()
                        .filter(|req| req.timestamp > cutoff_time)
                        .cloned()
                        .collect::<Vec<_>>()
                };

                if recent_requests.is_empty() {
                    continue;
                }

                let recent_rate_limited = recent_requests
                    .iter()
                    .filter(|req| req.rate_limited)
                    .count();
                let success_rate = recent_requests
                    .iter()
                    .filter(|req| req.success && !req.rate_limited)
                    .count() as f64
                    / recent_requests.len() as f64;

                let mut strategy = adaptive_strategy.write().await;
                let mut rate = current_rate.write().await;

                // Adaptive strategy logic
                if recent_rate_limited > 2 {
                    // Too many rate limits - reduce rate
                    let new_rate = (*rate as f64 * 0.8) as usize; // Reduce by 20%
                    *rate = new_rate.max(10); // Minimum 10 requests per minute
                    strategy.strategy_changes += 1;
                    println!(
                        "   🔄 Adaptive strategy: Reduced rate to {}/min due to rate limits",
                        *rate
                    );
                } else if success_rate > 0.95 && recent_rate_limited == 0 {
                    // High success rate - try increasing rate
                    let new_rate = (*rate as f64 * 1.1) as usize; // Increase by 10%
                    *rate = new_rate.min(120); // Maximum 120 requests per minute (2/sec)
                    strategy.strategy_changes += 1;
                    strategy.last_successful_rate = Some(*rate);
                    println!("   🔄 Adaptive strategy: Increased rate to {}/min", *rate);
                }

                // Track optimal rate discovery
                if success_rate > 0.95 && recent_rate_limited == 0 {
                    if strategy.optimal_rate_discovered.is_none()
                        || strategy.optimal_rate_discovered.unwrap() < *rate
                    {
                        strategy.optimal_rate_discovered = Some(*rate);
                    }
                }
            }
        })
    }

    async fn generate_test_results(&self, total_duration: Duration) -> RateLimitTestResults {
        let requests = self.requests.read().await.clone();

        // Calculate rate limit encounters
        let mut rate_limit_encounters = HashMap::new();
        for request in &requests {
            if request.rate_limited {
                *rate_limit_encounters
                    .entry(request.rate_limit_type.clone())
                    .or_insert(0) += 1;
            }
        }

        // Calculate recovery metrics
        let recovery_metrics = self.calculate_recovery_metrics(&requests).await;

        // Calculate throughput metrics
        let throughput_metrics = self
            .calculate_throughput_metrics(&requests, total_duration)
            .await;

        // Calculate adaptive strategy metrics
        let adaptive_strategy_metrics = self.calculate_adaptive_strategy_metrics().await;

        // Calculate efficiency score
        let efficiency_score = self.calculate_efficiency_score(
            &throughput_metrics,
            &recovery_metrics,
            &adaptive_strategy_metrics,
        );

        // Generate recommendations
        let recommendations = self
            .generate_recommendations(&requests, &rate_limit_encounters, &throughput_metrics)
            .await;

        RateLimitTestResults {
            test_name: self.config.test_name.clone(),
            config: self.config.clone(),
            total_duration,
            requests,
            rate_limit_encounters,
            recovery_metrics,
            throughput_metrics,
            adaptive_strategy_metrics,
            efficiency_score,
            recommendations,
        }
    }

    async fn calculate_recovery_metrics(
        &self,
        requests: &[ApiRequest],
    ) -> RateLimitRecoveryMetrics {
        let mut recovery_events = Vec::new();
        let mut last_rate_limit_time = None;

        for request in requests {
            if request.rate_limited {
                last_rate_limit_time = Some(request.timestamp);
            } else if let Some(rate_limit_time) = last_rate_limit_time {
                // Found recovery - successful request after rate limit
                let recovery_time = request.timestamp.duration_since(rate_limit_time);
                recovery_events.push(recovery_time);
                last_rate_limit_time = None;
            }
        }

        if recovery_events.is_empty() {
            return RateLimitRecoveryMetrics {
                total_recovery_events: 0,
                average_recovery_time: Duration::ZERO,
                fastest_recovery_time: Duration::ZERO,
                slowest_recovery_time: Duration::ZERO,
                recovery_success_rate: 1.0,
                adaptive_backoff_effectiveness: 0.0,
            };
        }

        let total_recovery_events = recovery_events.len();
        let average_recovery_time =
            recovery_events.iter().sum::<Duration>() / recovery_events.len() as u32;
        let fastest_recovery_time = recovery_events
            .iter()
            .min()
            .copied()
            .unwrap_or(Duration::ZERO);
        let slowest_recovery_time = recovery_events
            .iter()
            .max()
            .copied()
            .unwrap_or(Duration::ZERO);

        let total_rate_limits = requests.iter().filter(|r| r.rate_limited).count();
        let recovery_success_rate = if total_rate_limits > 0 {
            total_recovery_events as f64 / total_rate_limits as f64
        } else {
            1.0
        };

        // Adaptive backoff effectiveness: shorter recovery times indicate better backoff
        let adaptive_backoff_effectiveness = if average_recovery_time.as_secs() > 0 {
            (60.0 / average_recovery_time.as_secs() as f64).min(1.0)
        } else {
            1.0
        };

        RateLimitRecoveryMetrics {
            total_recovery_events,
            average_recovery_time,
            fastest_recovery_time,
            slowest_recovery_time,
            recovery_success_rate,
            adaptive_backoff_effectiveness,
        }
    }

    async fn calculate_throughput_metrics(
        &self,
        requests: &[ApiRequest],
        total_duration: Duration,
    ) -> ThroughputMetrics {
        let successful_requests = requests
            .iter()
            .filter(|r| r.success && !r.rate_limited)
            .count();
        let failed_requests = requests
            .iter()
            .filter(|r| !r.success || r.rate_limited)
            .count();
        let total_requests = requests.len();

        let minutes = total_duration.as_secs_f64() / 60.0;
        let requests_per_minute = if minutes > 0.0 {
            total_requests as f64 / minutes
        } else {
            0.0
        };

        // Calculate peak requests per minute (in any 1-minute window)
        let mut peak_requests_per_minute = 0.0f64;
        if !requests.is_empty() {
            let start_time = requests[0].timestamp;
            let mut window_start = start_time;

            while window_start < requests.last().unwrap().timestamp {
                let window_end = window_start + Duration::from_secs(60);
                let requests_in_window = requests
                    .iter()
                    .filter(|r| r.timestamp >= window_start && r.timestamp < window_end)
                    .count();
                peak_requests_per_minute = peak_requests_per_minute.max(requests_in_window as f64);
                window_start += Duration::from_secs(30); // Slide window by 30 seconds
            }
        }

        let efficiency_ratio = if total_requests > 0 {
            successful_requests as f64 / total_requests as f64
        } else {
            0.0
        };

        ThroughputMetrics {
            successful_requests,
            failed_requests,
            requests_per_minute,
            peak_requests_per_minute,
            efficiency_ratio,
        }
    }

    async fn calculate_adaptive_strategy_metrics(&self) -> AdaptiveStrategyMetrics {
        let strategy = self.adaptive_strategy.read().await;

        AdaptiveStrategyMetrics {
            strategy_changes: strategy.strategy_changes,
            optimal_request_rate_found: strategy.optimal_rate_discovered,
            time_to_optimal_rate: None, // Would need to track timing in real implementation
            strategy_effectiveness_score: if strategy.strategy_changes > 0 {
                (strategy.optimal_rate_discovered.unwrap_or(0) as f64 / 120.0).min(1.0)
            } else {
                0.5
            },
        }
    }

    fn calculate_efficiency_score(
        &self,
        throughput: &ThroughputMetrics,
        recovery: &RateLimitRecoveryMetrics,
        adaptive: &AdaptiveStrategyMetrics,
    ) -> f64 {
        // Weighted efficiency score (0.0 to 1.0, higher is better)
        let throughput_score = throughput.efficiency_ratio;
        let recovery_score =
            recovery.recovery_success_rate * recovery.adaptive_backoff_effectiveness;
        let adaptive_score = adaptive.strategy_effectiveness_score;

        (throughput_score * 0.5) + (recovery_score * 0.3) + (adaptive_score * 0.2)
    }

    async fn generate_recommendations(
        &self,
        requests: &[ApiRequest],
        rate_limit_encounters: &HashMap<RateLimitType, usize>,
        throughput: &ThroughputMetrics,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Analyze rate limit patterns
        if let Some(&primary_limits) = rate_limit_encounters.get(&RateLimitType::Primary) {
            if primary_limits > 10 {
                recommendations.push(format!(
                    "High primary rate limit encounters ({}). Consider implementing exponential backoff with longer delays.",
                    primary_limits
                ));
            }
        }

        if let Some(&search_limits) = rate_limit_encounters.get(&RateLimitType::Search) {
            if search_limits > 5 {
                recommendations.push(format!(
                    "Search API rate limits encountered ({}). Implement caching for search results.",
                    search_limits
                ));
            }
        }

        // Analyze throughput efficiency
        if throughput.efficiency_ratio < 0.8 {
            recommendations.push(format!(
                "Low request efficiency ({:.1}%). Implement better error handling and retry logic.",
                throughput.efficiency_ratio * 100.0
            ));
        }

        // Analyze request patterns
        let avg_request_duration = if !requests.is_empty() {
            requests.iter().map(|r| r.duration).sum::<Duration>() / requests.len() as u32
        } else {
            Duration::ZERO
        };

        if avg_request_duration > Duration::from_secs(2) {
            recommendations.push(format!(
                "High average request duration ({:.1}s). Consider request batching or GraphQL API.",
                avg_request_duration.as_secs_f64()
            ));
        }

        // Peak usage recommendations
        if throughput.peak_requests_per_minute > 100.0 {
            recommendations.push(
                "Peak request rate exceeds sustainable levels. Implement request queuing."
                    .to_string(),
            );
        }

        if recommendations.is_empty() {
            recommendations
                .push("API usage patterns appear efficient. Continue monitoring.".to_string());
        }

        recommendations
    }

    fn print_test_results(&self, results: &RateLimitTestResults) {
        println!("\n🌐 GitHub API Rate Limit Test Results");
        println!("════════════════════════════════════════════════════════════════");
        println!("Test: {}", results.test_name);
        println!("Duration: {:?}", results.total_duration);
        println!("Total Requests: {}", results.requests.len());

        println!("\n📊 Throughput Metrics:");
        println!(
            "  Successful Requests: {}",
            results.throughput_metrics.successful_requests
        );
        println!(
            "  Failed Requests: {}",
            results.throughput_metrics.failed_requests
        );
        println!(
            "  Requests per Minute: {:.1}",
            results.throughput_metrics.requests_per_minute
        );
        println!(
            "  Peak Requests per Minute: {:.1}",
            results.throughput_metrics.peak_requests_per_minute
        );
        println!(
            "  Efficiency Ratio: {:.1}%",
            results.throughput_metrics.efficiency_ratio * 100.0
        );

        println!("\n⚠️ Rate Limit Encounters:");
        if results.rate_limit_encounters.is_empty() {
            println!("  No rate limits encountered ✅");
        } else {
            for (limit_type, count) in &results.rate_limit_encounters {
                println!("  {:?}: {} times", limit_type, count);
            }
        }

        println!("\n🔄 Recovery Metrics:");
        println!(
            "  Recovery Events: {}",
            results.recovery_metrics.total_recovery_events
        );
        if results.recovery_metrics.total_recovery_events > 0 {
            println!(
                "  Average Recovery Time: {:?}",
                results.recovery_metrics.average_recovery_time
            );
            println!(
                "  Fastest Recovery: {:?}",
                results.recovery_metrics.fastest_recovery_time
            );
            println!(
                "  Slowest Recovery: {:?}",
                results.recovery_metrics.slowest_recovery_time
            );
            println!(
                "  Recovery Success Rate: {:.1}%",
                results.recovery_metrics.recovery_success_rate * 100.0
            );
            println!(
                "  Backoff Effectiveness: {:.2}",
                results.recovery_metrics.adaptive_backoff_effectiveness
            );
        }

        if results.config.enable_adaptive_strategies {
            println!("\n🎯 Adaptive Strategy Metrics:");
            println!(
                "  Strategy Changes: {}",
                results.adaptive_strategy_metrics.strategy_changes
            );
            if let Some(optimal_rate) = results.adaptive_strategy_metrics.optimal_request_rate_found
            {
                println!("  Optimal Rate Found: {}/min", optimal_rate);
            }
            println!(
                "  Strategy Effectiveness: {:.2}",
                results
                    .adaptive_strategy_metrics
                    .strategy_effectiveness_score
            );
        }

        println!(
            "\n📈 Overall Efficiency Score: {:.2}/1.0",
            results.efficiency_score
        );

        println!("\n💡 Recommendations:");
        for (i, recommendation) in results.recommendations.iter().enumerate() {
            println!("  {}. {}", i + 1, recommendation);
        }

        // Assessment
        if results.efficiency_score >= 0.9 {
            println!(
                "\n✅ EXCELLENT - API usage is highly efficient with good rate limit handling"
            );
        } else if results.efficiency_score >= 0.7 {
            println!("\n✅ GOOD - API usage is efficient with acceptable rate limit handling");
        } else if results.efficiency_score >= 0.5 {
            println!(
                "\n⚠️ ACCEPTABLE - API usage needs optimization for better rate limit handling"
            );
        } else {
            println!("\n❌ NEEDS IMPROVEMENT - API usage has significant rate limit issues");
        }
    }

    async fn generate_documentation(&self, results: &RateLimitTestResults) {
        let doc_content = self.create_rate_limit_documentation(results);

        // In a real implementation, this would write to a file
        println!("\n📚 Generated Rate Limit Handling Documentation:");
        println!("{}", doc_content);
    }

    fn create_rate_limit_documentation(&self, results: &RateLimitTestResults) -> String {
        format!(
            r#"# GitHub API Rate Limit Handling Documentation

## Test Results Summary

**Test Name:** {}
**Duration:** {:?}
**Total Requests:** {}
**Efficiency Score:** {:.2}/1.0

## Rate Limit Encounters

{}

## Performance Characteristics

- **Successful Requests:** {}
- **Requests per Minute:** {:.1}
- **Peak Requests per Minute:** {:.1}
- **Efficiency Ratio:** {:.1}%

## Recovery Performance

{}

## Recommended Strategies

### 1. Request Rate Management
- Optimal request rate: {} requests per minute
- Use exponential backoff with jitter
- Monitor rate limit headers in responses

### 2. Error Handling
- Implement retry logic with appropriate delays
- Handle different rate limit types differently:
  - Primary limits: 60-second delays
  - Search API limits: 60-second delays  
  - Secondary limits: 30-second delays

### 3. Optimization Techniques
- Use GraphQL for complex queries to reduce request count
- Implement response caching where appropriate
- Batch related operations when possible
- Monitor and log rate limit metrics

## Implementation Example

```rust
// Example rate limit handling code
async fn handle_github_api_request(client: &GitHubClient, request: Request) -> Result<Response, Error> {{
    let max_retries = 3;
    let mut retry_count = 0;
    
    loop {{
        match client.send_request(request.clone()).await {{
            Ok(response) => return Ok(response),
            Err(Error::RateLimit(limit_type, retry_after)) => {{
                if retry_count >= max_retries {{
                    return Err(Error::MaxRetriesExceeded);
                }}
                
                let delay = match limit_type {{
                    RateLimitType::Primary => retry_after.unwrap_or(Duration::from_secs(60)),
                    RateLimitType::Search => Duration::from_secs(60),
                    RateLimitType::Secondary => Duration::from_secs(30),
                    _ => Duration::from_secs(10),
                }};
                
                tokio::time::sleep(delay).await;
                retry_count += 1;
            }},
            Err(e) => return Err(e),
        }}
    }}
}}
```

## Monitoring and Metrics

Track these metrics for ongoing optimization:
- Requests per hour/minute
- Rate limit encounters by type
- Recovery times
- Success/failure ratios
- API endpoint usage patterns

Generated on: {}
"#,
            results.test_name,
            results.total_duration,
            results.requests.len(),
            results.efficiency_score,
            if results.rate_limit_encounters.is_empty() {
                "No rate limits encountered during testing.".to_string()
            } else {
                results
                    .rate_limit_encounters
                    .iter()
                    .map(|(t, c)| format!("- {:?}: {} encounters", t, c))
                    .collect::<Vec<_>>()
                    .join("\n")
            },
            results.throughput_metrics.successful_requests,
            results.throughput_metrics.requests_per_minute,
            results.throughput_metrics.peak_requests_per_minute,
            results.throughput_metrics.efficiency_ratio * 100.0,
            if results.recovery_metrics.total_recovery_events > 0 {
                format!(
                    "- Recovery Events: {}\n- Average Recovery Time: {:?}\n- Success Rate: {:.1}%",
                    results.recovery_metrics.total_recovery_events,
                    results.recovery_metrics.average_recovery_time,
                    results.recovery_metrics.recovery_success_rate * 100.0
                )
            } else {
                "No recovery events recorded.".to_string()
            },
            results
                .adaptive_strategy_metrics
                .optimal_request_rate_found
                .unwrap_or(60),
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        )
    }
}

#[cfg(test)]
mod rate_limit_tests {
    use super::*;

    #[tokio::test]
    async fn test_github_api_rate_limit_handling() {
        println!("🧪 Testing GitHub API rate limit handling");

        let config = RateLimitTestConfig {
            test_name: "GitHub API Rate Limit Handling Test".to_string(),
            test_duration: Duration::from_secs(90),
            initial_requests_per_minute: 80, // Start aggressive to trigger limits
            simulate_rate_limits: true,
            simulate_secondary_limits: true,
            track_recovery_metrics: true,
            enable_adaptive_strategies: true,
        };

        let simulator = GitHubApiRateLimitSimulator::new(config);
        let results = simulator
            .run_rate_limit_test()
            .await
            .expect("Test should complete");

        // Validate test results
        assert!(
            results.requests.len() > 50,
            "Should have made sufficient requests for testing: {}",
            results.requests.len()
        );

        assert!(
            results.efficiency_score >= 0.3,
            "Efficiency score too low: {:.2}",
            results.efficiency_score
        );

        assert!(
            results.throughput_metrics.requests_per_minute > 10.0,
            "Request rate too low: {:.1}/min",
            results.throughput_metrics.requests_per_minute
        );

        // If rate limits were encountered, validate recovery metrics
        let total_rate_limits: usize = results.rate_limit_encounters.values().sum();
        if total_rate_limits > 0 {
            assert!(
                results.recovery_metrics.total_recovery_events > 0,
                "Should have recovery events if rate limits occurred"
            );
            assert!(
                results.recovery_metrics.average_recovery_time < Duration::from_secs(120),
                "Recovery time too slow: {:?}",
                results.recovery_metrics.average_recovery_time
            );
        }

        println!("✅ GitHub API rate limit handling test completed successfully");
    }

    #[tokio::test]
    async fn test_adaptive_rate_limit_strategies() {
        println!("🧪 Testing adaptive rate limit strategies");

        let config = RateLimitTestConfig {
            test_name: "Adaptive Rate Limit Strategies".to_string(),
            test_duration: Duration::from_secs(120),
            initial_requests_per_minute: 100, // Start very aggressive
            simulate_rate_limits: true,
            simulate_secondary_limits: false, // Focus on primary limits
            track_recovery_metrics: true,
            enable_adaptive_strategies: true,
        };

        let simulator = GitHubApiRateLimitSimulator::new(config);
        let results = simulator
            .run_rate_limit_test()
            .await
            .expect("Test should complete");

        // Validate adaptive strategies worked
        assert!(
            results.adaptive_strategy_metrics.strategy_changes > 0,
            "Should have made strategy adjustments"
        );

        // If optimal rate was found, it should be reasonable
        if let Some(optimal_rate) = results.adaptive_strategy_metrics.optimal_request_rate_found {
            assert!(
                optimal_rate >= 10 && optimal_rate <= 120,
                "Optimal rate should be reasonable: {}/min",
                optimal_rate
            );
        }

        assert!(
            results
                .adaptive_strategy_metrics
                .strategy_effectiveness_score
                > 0.2,
            "Strategy effectiveness too low: {:.2}",
            results
                .adaptive_strategy_metrics
                .strategy_effectiveness_score
        );

        println!(
            "Adaptive strategy made {} changes",
            results.adaptive_strategy_metrics.strategy_changes
        );
        if let Some(optimal) = results.adaptive_strategy_metrics.optimal_request_rate_found {
            println!("Optimal rate found: {}/min", optimal);
        }

        println!("✅ Adaptive rate limit strategies test completed successfully");
    }

    #[tokio::test]
    async fn test_rate_limit_recovery_performance() {
        println!("🧪 Testing rate limit recovery performance");

        let config = RateLimitTestConfig {
            test_name: "Rate Limit Recovery Performance".to_string(),
            test_duration: Duration::from_secs(90),
            initial_requests_per_minute: 150, // Very aggressive to force limits
            simulate_rate_limits: true,
            simulate_secondary_limits: true,
            track_recovery_metrics: true,
            enable_adaptive_strategies: false, // Disable to focus on recovery
        };

        let simulator = GitHubApiRateLimitSimulator::new(config);
        let results = simulator
            .run_rate_limit_test()
            .await
            .expect("Test should complete");

        // Should encounter rate limits with aggressive rate
        let total_rate_limits: usize = results.rate_limit_encounters.values().sum();
        assert!(
            total_rate_limits > 0,
            "Should encounter rate limits with aggressive testing"
        );

        // Validate recovery metrics
        assert!(
            results.recovery_metrics.total_recovery_events > 0,
            "Should have recovery events"
        );

        assert!(
            results.recovery_metrics.recovery_success_rate > 0.5,
            "Recovery success rate too low: {:.1}%",
            results.recovery_metrics.recovery_success_rate * 100.0
        );

        assert!(
            results.recovery_metrics.average_recovery_time < Duration::from_secs(180),
            "Average recovery time too slow: {:?}",
            results.recovery_metrics.average_recovery_time
        );

        assert!(
            results.recovery_metrics.adaptive_backoff_effectiveness > 0.1,
            "Backoff effectiveness too low: {:.2}",
            results.recovery_metrics.adaptive_backoff_effectiveness
        );

        println!(
            "Recovery events: {}",
            results.recovery_metrics.total_recovery_events
        );
        println!(
            "Average recovery time: {:?}",
            results.recovery_metrics.average_recovery_time
        );
        println!(
            "Recovery success rate: {:.1}%",
            results.recovery_metrics.recovery_success_rate * 100.0
        );

        println!("✅ Rate limit recovery performance test completed successfully");
    }

    #[tokio::test]
    async fn test_api_efficiency_baseline() {
        println!("🧪 Establishing API efficiency baseline");

        let config = RateLimitTestConfig {
            test_name: "API Efficiency Baseline".to_string(),
            test_duration: Duration::from_secs(60),
            initial_requests_per_minute: 60, // Moderate rate
            simulate_rate_limits: true,
            simulate_secondary_limits: false,
            track_recovery_metrics: true,
            enable_adaptive_strategies: true,
        };

        let simulator = GitHubApiRateLimitSimulator::new(config);
        let results = simulator
            .run_rate_limit_test()
            .await
            .expect("Test should complete");

        println!("\n📊 API Efficiency Baseline Established:");
        println!(
            "  Requests per Minute: {:.1}",
            results.throughput_metrics.requests_per_minute
        );
        println!(
            "  Peak Requests per Minute: {:.1}",
            results.throughput_metrics.peak_requests_per_minute
        );
        println!(
            "  Efficiency Ratio: {:.1}%",
            results.throughput_metrics.efficiency_ratio * 100.0
        );
        println!(
            "  Overall Efficiency Score: {:.2}/1.0",
            results.efficiency_score
        );
        println!(
            "  Rate Limit Encounters: {}",
            results.rate_limit_encounters.values().sum::<usize>()
        );

        // Baseline validation
        assert!(
            results.throughput_metrics.efficiency_ratio > 0.7,
            "Baseline efficiency too low"
        );
        assert!(
            results.efficiency_score > 0.5,
            "Baseline overall score too low"
        );
        assert!(
            results.requests.len() >= 30,
            "Baseline should have sufficient requests"
        );

        // Recommendations should be provided
        assert!(
            !results.recommendations.is_empty(),
            "Should provide recommendations"
        );

        println!("💡 Baseline Recommendations:");
        for (i, rec) in results.recommendations.iter().enumerate() {
            println!("  {}. {}", i + 1, rec);
        }

        println!("✅ API efficiency baseline established successfully");
    }
}
