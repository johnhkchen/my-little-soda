use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

/// Initialize OpenTelemetry tracing with Phoenix/OTLP backend
/// This is a simplified version that just sets up basic tracing
/// For production, you'd want to add OpenTelemetry export when Phoenix is available
pub fn init_telemetry() -> Result<()> {
    // For now, just initialize tracing with JSON output for structured logging
    // This provides the correlation IDs and structured data needed for observability
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_current_span(true)
                .with_span_list(true)
        )
        .with(tracing_subscriber::EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    tracing::info!("Clambake telemetry initialized with structured logging");
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
    // For structured logging, no explicit shutdown needed
    tracing::info!("Clambake telemetry shutdown complete");
}