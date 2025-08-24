// Integration Success Tracking and Metrics
// Provides tracking for work integration success rates and performance

pub mod analysis;
pub mod bottleneck;
pub mod performance;
pub mod reports;
pub mod storage;
pub mod tracking;
pub mod types;

// Re-export public types and main interfaces
pub use analysis::MetricsAnalyzer;
pub use tracking::MetricsTracker;
pub use types::*;

use crate::github::GitHubError;
use std::collections::HashMap;

// Main public interface - MetricsTracker maintains backward compatibility
impl MetricsTracker {
    // Keep existing public API for backward compatibility
    pub async fn load_attempts(&self) -> Result<Vec<IntegrationAttempt>, GitHubError> {
        self.storage.load_integration_attempts().await
    }

    pub async fn calculate_metrics(
        &self,
        lookback_hours: Option<u64>,
    ) -> Result<IntegrationMetrics, GitHubError> {
        let analyzer = MetricsAnalyzer::new();
        analyzer.calculate_metrics(lookback_hours).await
    }

    pub fn format_metrics_report(&self, metrics: &IntegrationMetrics, detailed: bool) -> String {
        let analyzer = MetricsAnalyzer::new();
        analyzer.format_metrics_report(metrics, detailed)
    }

    pub async fn format_performance_report(
        &self,
        lookback_hours: Option<u64>,
    ) -> Result<String, GitHubError> {
        let analyzer = MetricsAnalyzer::new();
        analyzer.format_performance_report(lookback_hours).await
    }

    pub async fn export_metrics_for_monitoring(
        &self,
        lookback_hours: Option<u64>,
    ) -> Result<HashMap<String, serde_json::Value>, GitHubError> {
        let analyzer = MetricsAnalyzer::new();
        analyzer.export_metrics_for_monitoring(lookback_hours).await
    }
}
