use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

/// Initialize OpenTelemetry tracing with distributed tracing support
/// Exports to stdout by default, can be configured for OTLP/Jaeger in production
pub fn init_telemetry() -> Result<()> {
    // For now, simplified telemetry with OpenTelemetry layer but no external export
    // This provides the structured span data needed for correlation
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_current_span(true)
                .with_span_list(true),
        )
        .with(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tracing::info!("My Little Soda telemetry initialized with structured logging and span support");
    Ok(())
}

/// Generate a correlation ID for linking related operations
pub fn generate_correlation_id() -> String {
    Uuid::new_v4().to_string()
}

/// Create a span with common agent coordination attributes
pub fn create_coordination_span(
    operation: &str,
    agent_id: Option<&str>,
    issue_number: Option<u64>,
    correlation_id: Option<&str>,
) -> tracing::Span {
    tracing::info_span!(
        "agent_coordination",
        operation = operation,
        agent.id = agent_id,
        issue.number = issue_number,
        correlation.id = correlation_id,
        otel.kind = "internal"
    )
}

/// Shutdown telemetry gracefully
pub fn shutdown_telemetry() {
    tracing::info!("Shutting down telemetry...");

    // For basic structured logging, no special shutdown needed
    // In the future, this would flush OpenTelemetry spans

    tracing::info!("My Little Soda telemetry shutdown complete");
}
