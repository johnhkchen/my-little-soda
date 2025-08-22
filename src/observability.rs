use tracing::{info, warn};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// GitHub API usage metrics
#[derive(Debug, Default)]
pub struct GitHubApiMetrics {
    pub total_requests: AtomicU64,
    pub rate_limit_hits: AtomicU64,
    pub errors: AtomicU64,
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
}

impl GitHubApiMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_request(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_rate_limit_hit(&self) {
        self.rate_limit_hits.fetch_add(1, Ordering::Relaxed);
        warn!("GitHub API rate limit hit");
    }

    pub fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> GitHubApiStats {
        GitHubApiStats {
            total_requests: self.total_requests.load(Ordering::Relaxed),
            rate_limit_hits: self.rate_limit_hits.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
        }
    }

    pub fn log_stats(&self) {
        let stats = self.get_stats();
        info!(
            "GitHub API metrics: requests={}, rate_limits={}, errors={}, cache_hits={}, cache_misses={}",
            stats.total_requests,
            stats.rate_limit_hits,
            stats.errors,
            stats.cache_hits,
            stats.cache_misses
        );
    }
}

#[derive(Debug, Clone)]
pub struct GitHubApiStats {
    pub total_requests: u64,
    pub rate_limit_hits: u64,
    pub errors: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

/// Global metrics instance
static GITHUB_METRICS: std::sync::LazyLock<GitHubApiMetrics> = 
    std::sync::LazyLock::new(|| GitHubApiMetrics::new());

pub fn github_metrics() -> &'static GitHubApiMetrics {
    &GITHUB_METRICS
}

/// Create correlated spans for agent coordination workflows
pub fn create_workflow_span(workflow: &str, correlation_id: &str) -> tracing::Span {
    tracing::info_span!(
        "workflow",
        workflow.name = workflow,
        correlation.id = correlation_id,
        otel.kind = "internal"
    )
}

/// Time an operation and record metrics
pub struct OperationTimer {
    operation: String,
    start: Instant,
}

impl OperationTimer {
    pub fn new(operation: &str) -> Self {
        Self {
            operation: operation.to_string(),
            start: Instant::now(),
        }
    }

    pub fn finish(self) {
        let duration = self.start.elapsed();
        info!(
            operation = %self.operation,
            duration_ms = duration.as_millis(),
            "Operation completed"
        );
    }
}

#[macro_export]
macro_rules! time_operation {
    ($operation:expr) => {
        let _timer = $crate::observability::OperationTimer::new($operation);
    };
}