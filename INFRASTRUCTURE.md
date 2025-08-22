# Clambake Infrastructure

This document describes the production-ready observability, configuration, and shutdown infrastructure added to Clambake.

## Features

### Enhanced Observability

- **Structured Logging**: JSON-formatted logs with span correlation
- **GitHub API Metrics**: Automatic tracking of rate limits, errors, and cache performance
- **Correlation IDs**: End-to-end tracing across agent operations
- **Operation Timing**: Built-in performance measurement macros

```rust
// Example usage
use clambake::time_operation;

async fn process_issue() {
    time_operation!("process_issue");
    // Your code here - timing is automatic
}
```

### Configuration Management

Layered configuration system with precedence:
1. Default values (in code)
2. Configuration files (clambake.toml, .clambakerc)
3. Environment variables (CLAMBAKE_* prefix)

```bash
# Environment override examples
export CLAMBAKE_GITHUB_TOKEN="ghp_your_token"
export CLAMBAKE_AGENTS_MAX_AGENTS=8
export CLAMBAKE_OBSERVABILITY_LOG_LEVEL=debug
```

Configuration file example:
```toml
[github]
owner = "your-org"
repo = "your-repo"

[observability]
tracing_enabled = true
log_level = "info"

[agents]
max_agents = 4
```

### Graceful Shutdown

Infrastructure for clean shutdown handling:
- Cancels ongoing git operations
- Waits for agents to finish current tasks
- Completes bundle processing
- Closes connections gracefully

### Optional Database Integration

SQLite-based persistent state storage (enable with `--features database`):
- Agent coordination state tracking
- Bundle processing state persistence  
- Automatic migrations
- Configurable connection pooling

```rust
// Example usage (when database feature is enabled)
use clambake::{init_database, database};

// Initialize during startup
init_database().await?;

// Use in your code
if let Some(db) = database().await {
    // Store/retrieve state
}
```

## Usage

### Basic Setup

1. Copy configuration template:
   ```bash
   cp clambake.example.toml clambake.toml
   ```

2. Set required environment variables:
   ```bash
   export GITHUB_TOKEN="ghp_your_token_here"
   ```

3. Run with enhanced infrastructure:
   ```bash
   # Basic usage
   cargo run -- pop
   
   # With database support
   cargo run --features database -- pop
   ```

### Development

The infrastructure is designed to be non-intrusive:
- Zero configuration required for basic operation
- Graceful degradation when features are disabled
- Compatible with existing Clambake workflows

### Monitoring

All operations emit structured logs suitable for aggregation:

```json
{
  "timestamp": "2024-01-01T12:00:00Z",
  "level": "INFO",
  "fields": {
    "message": "Agent coordination complete",
    "agent.id": "agent001",
    "issue.number": 123,
    "correlation.id": "uuid-here",
    "duration_ms": 1500
  }
}
```

GitHub API metrics are logged periodically:
```json
{
  "message": "GitHub API metrics: requests=45, rate_limits=0, errors=2, cache_hits=12, cache_misses=3"
}
```

## Architecture

The infrastructure follows these principles:
- **Optional**: All features can be disabled/enabled independently
- **Layered**: Configuration, observability, and persistence work together
- **Non-blocking**: Never interferes with core Clambake functionality
- **Production-ready**: Designed for monitoring and operational visibility

### Key Components

- `observability.rs`: Metrics collection and structured logging
- `config.rs`: Layered configuration management
- `shutdown.rs`: Graceful shutdown coordination
- `database.rs`: Optional persistent state storage
- `telemetry.rs`: OpenTelemetry integration foundation

This infrastructure provides the foundation for production deployment while maintaining Clambake's developer-focused design.