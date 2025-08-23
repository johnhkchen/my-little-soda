# Autonomous System Configuration Guide

This document provides comprehensive configuration options for My Little Soda's autonomous operation system, including state drift detection, error recovery, work continuity, and performance tuning.

## Table of Contents

1. [Configuration Overview](#configuration-overview)
2. [Core Autonomous Settings](#core-autonomous-settings)
3. [State Drift Detection](#state-drift-detection)
4. [Error Recovery System](#error-recovery-system)
5. [Work Continuity & Persistence](#work-continuity--persistence)
6. [Performance & Monitoring](#performance--monitoring)
7. [Environment Variables](#environment-variables)
8. [Configuration Examples](#configuration-examples)
9. [Validation & Testing](#validation--testing)

## Configuration Overview

My Little Soda supports multiple configuration methods in order of precedence:

1. **Environment Variables** (highest priority)
2. **Configuration File** (`my-little-soda.toml`)
3. **Command Line Arguments**
4. **Default Values** (lowest priority)

### Configuration File Structure

```toml
# my-little-soda.toml

[autonomous]
# Core autonomous operation settings

[autonomous.drift_detection]
# State drift detection and correction

[autonomous.recovery] 
# Error recovery strategies and limits

[autonomous.persistence]
# Work continuity and state persistence

[autonomous.monitoring]
# Performance monitoring and observability
```

## Core Autonomous Settings

### Basic Operation Configuration

```toml
[autonomous]
# Maximum continuous work hours before requiring break
max_work_hours = 8

# Enable/disable autonomous operation entirely
enable_autonomous_operation = true

# Agent identification for multi-agent scenarios
agent_id = "agent001"

# Workspace validation on startup
validate_workspace_on_start = true

# Maximum issues to work on simultaneously  
max_concurrent_issues = 1

# Work assignment timeout (minutes)
assignment_timeout_minutes = 30
```

**Environment Variables**:
```bash
export MY_LITTLE_SODA_MAX_WORK_HOURS=12
export MY_LITTLE_SODA_ENABLE_AUTONOMOUS_OPERATION=true
export MY_LITTLE_SODA_AGENT_ID="production-agent"
export MY_LITTLE_SODA_VALIDATE_WORKSPACE_ON_START=true
```

### Agent Lifecycle Configuration

```toml
[autonomous.lifecycle]
# Startup behavior
auto_start_on_init = false
resume_on_restart = true
validate_state_on_resume = true

# Shutdown behavior
graceful_shutdown_timeout_seconds = 60
preserve_state_on_shutdown = true
cleanup_temp_files_on_exit = true

# Health checks
enable_health_monitoring = true
health_check_interval_minutes = 5
max_unhealthy_periods = 3
```

## State Drift Detection

### Core Drift Detection Settings

```toml
[autonomous.drift_detection]
# Enable drift detection system
enable_drift_detection = true

# Validation frequency
validation_interval_minutes = 5
max_validation_interval_idle = 30
max_validation_interval_active = 2

# Drift thresholds
max_commits_behind = 10
max_commits_ahead = 50
max_drift_age_hours = 24

# Critical drift types that always require attention
critical_drift_types = [
    "IssueUnexpectedlyClosed",
    "BranchDeleted",
    "PRUnexpectedlyMerged", 
    "GitStateInconsistent"
]
```

### Drift Correction Configuration

```toml
[autonomous.drift_detection.correction]
# Correction strategy priority order
correction_strategies = [
    "WorkPreserving",           # Always try to preserve work first
    "GitHubAuthoritative",      # Trust GitHub state over local expectations
    "EscalateAndContinue",      # Create issues but continue working
    "RequireManualIntervention" # Stop and wait for human intervention
]

# Work preservation settings
preserve_partial_work = true
create_backup_branches = true
backup_branch_prefix = "backup-"
max_backup_retention_days = 30

# Escalation behavior
create_drift_issues = true
drift_issue_labels = ["drift-detection", "autonomous-agent", "requires-attention"]
notify_on_critical_drift = true
pause_on_critical_drift = true

# Auto-correction limits
max_auto_corrections_per_hour = 10
max_consecutive_corrections = 3
correction_cooldown_minutes = 5
```

### State Validation Configuration

```toml
[autonomous.drift_detection.validation]
# GitHub API validation
validate_issue_assignments = true
validate_issue_states = true
validate_branch_existence = true
validate_pr_status = true

# Workspace validation  
validate_git_state = true
validate_uncommitted_changes = true
validate_branch_tracking = true
validate_remote_sync = true

# Performance settings
validation_timeout_seconds = 30
max_parallel_validations = 5
cache_validation_results_minutes = 2

# Retry settings for transient failures
max_validation_retries = 3
validation_retry_delay_seconds = 5
```

**Environment Variables**:
```bash
# Core drift detection
export MY_LITTLE_SODA_ENABLE_DRIFT_DETECTION=true
export MY_LITTLE_SODA_DRIFT_VALIDATION_INTERVAL=3
export MY_LITTLE_SODA_MAX_COMMITS_BEHIND=15

# Correction behavior
export MY_LITTLE_SODA_PRESERVE_WORK_ON_DRIFT=true
export MY_LITTLE_SODA_CREATE_DRIFT_ISSUES=true
export MY_LITTLE_SODA_PAUSE_ON_CRITICAL_DRIFT=false
```

## Error Recovery System

### Core Recovery Configuration

```toml
[autonomous.recovery]
# Enable error recovery system
enable_error_recovery = true

# Recovery attempt limits
max_recovery_attempts = 3
max_recovery_attempts_per_error_type = 5
recovery_timeout_minutes = 30

# Recovery strategies
enable_aggressive_recovery = false
enable_experimental_fixes = false
prefer_safe_recovery = true

# Recovery cooldown periods  
recovery_cooldown_minutes = 10
error_type_cooldown_minutes = 60
global_recovery_cooldown_minutes = 5
```

### Recovery Strategy Configuration

```toml
[autonomous.recovery.strategies]
# Automated fix configuration
[autonomous.recovery.strategies.automated_fix]
enable_syntax_fixes = true
enable_merge_conflict_resolution = true
enable_dependency_fixes = true
enable_test_fixes = false  # More risky
max_fix_confidence_threshold = "High"

# Retry configuration  
[autonomous.recovery.strategies.retry]
enable_retry_with_backoff = true
initial_delay_seconds = 5
max_delay_seconds = 300
backoff_multiplier = 2.0
max_retry_attempts = 5

# Escalation configuration
[autonomous.recovery.strategies.escalation] 
create_recovery_issues = true
escalation_labels = ["recovery-needed", "autonomous-agent"]
escalate_after_attempts = 2
escalate_critical_errors_immediately = true
```

### Error Type Handling

```toml
[autonomous.recovery.error_types]
# Git operation failures
[autonomous.recovery.error_types.git]
push_failures = "RetryWithBackoff"
pull_failures = "RetryWithBackoff" 
clone_failures = "Escalate"
merge_conflicts = "AutomatedFix"

# Build failures
[autonomous.recovery.error_types.build]
compilation_errors = "AutomatedFix"
dependency_issues = "AutomatedFix"
test_failures = "RetryWithBackoff"
lint_failures = "AutomatedFix"

# CI/CD failures
[autonomous.recovery.error_types.ci]
test_timeouts = "RetryWithBackoff"
deployment_failures = "Escalate"
security_scans = "Escalate"
performance_regressions = "Escalate"

# GitHub API failures  
[autonomous.recovery.error_types.github]
rate_limit_exceeded = "RetryWithBackoff"
authentication_failures = "Escalate"
permission_denied = "Escalate"
api_timeouts = "RetryWithBackoff"
```

**Environment Variables**:
```bash
# Core recovery settings
export MY_LITTLE_SODA_ENABLE_ERROR_RECOVERY=true
export MY_LITTLE_SODA_MAX_RECOVERY_ATTEMPTS=5
export MY_LITTLE_SODA_RECOVERY_TIMEOUT_MINUTES=45

# Recovery behavior
export MY_LITTLE_SODA_ENABLE_AGGRESSIVE_RECOVERY=false
export MY_LITTLE_SODA_PREFER_SAFE_RECOVERY=true
export MY_LITTLE_SODA_CREATE_RECOVERY_ISSUES=true
```

## Work Continuity & Persistence

### Core Persistence Configuration

```toml
[autonomous.persistence]
# Enable state persistence
enable_persistence = true

# Persistence directory and files
persistence_directory = ".my-little-soda/state"
state_file_name = "autonomous-state.json"
backup_directory = ".my-little-soda/backups"

# Checkpoint frequency
auto_save_interval_minutes = 5
checkpoint_on_state_change = true
checkpoint_on_progress = true
checkpoint_before_risky_operations = true

# State history
max_state_history_entries = 100
max_recovery_history_entries = 50
compress_old_states = true
cleanup_old_states_days = 30
```

### Checkpoint Configuration

```toml
[autonomous.persistence.checkpoints]
# Automatic checkpoint triggers
checkpoint_on_issue_assignment = true
checkpoint_on_work_completion = true
checkpoint_on_pr_submission = true  
checkpoint_on_error_recovery = true
checkpoint_on_drift_correction = true

# Checkpoint retention
max_checkpoints_per_agent = 10
checkpoint_retention_days = 14
compress_old_checkpoints = true

# Checkpoint validation
enable_integrity_checks = true
validate_on_load = true
backup_corrupted_checkpoints = true
```

### Work Continuity Configuration

```toml
[autonomous.persistence.continuity]
# Resume behavior
enable_work_continuity = true
auto_resume_on_restart = true
validate_workspace_on_resume = true
max_resume_age_hours = 48

# Recovery actions
force_fresh_start_after_hours = 72
preserve_partial_work_always = true
create_recovery_branches = true
max_recovery_attempts = 3

# Validation settings
validation_timeout_seconds = 30
workspace_validation_level = "Full"  # "Basic", "Standard", "Full"
```

**Environment Variables**:
```bash
# Core persistence
export MY_LITTLE_SODA_ENABLE_PERSISTENCE=true
export MY_LITTLE_SODA_PERSISTENCE_DIRECTORY="/custom/path/state"
export MY_LITTLE_SODA_AUTO_SAVE_INTERVAL=3

# Continuity settings
export MY_LITTLE_SODA_ENABLE_WORK_CONTINUITY=true
export MY_LITTLE_SODA_AUTO_RESUME_ON_RESTART=true
export MY_LITTLE_SODA_MAX_RESUME_AGE_HOURS=24
```

## Performance & Monitoring

### Core Monitoring Configuration

```toml
[autonomous.monitoring]
# Enable performance monitoring
enable_performance_monitoring = true
enable_detailed_metrics = true
enable_tracing = false  # Can impact performance

# Monitoring intervals
monitoring_interval_minutes = 5
metrics_collection_interval_seconds = 60
health_check_interval_minutes = 2

# Resource monitoring
monitor_memory_usage = true
monitor_cpu_usage = true
monitor_disk_usage = true
monitor_network_usage = false

# Performance thresholds
max_memory_usage_mb = 1024
max_cpu_usage_percent = 80
max_operation_duration_seconds = 300
```

### Observability Configuration

```toml
[autonomous.monitoring.observability]
# Logging configuration
log_level = "INFO"  # "TRACE", "DEBUG", "INFO", "WARN", "ERROR"
log_to_file = true
log_file_path = ".my-little-soda/logs/autonomous.log"
max_log_file_size_mb = 100
max_log_files = 10

# Structured logging
enable_structured_logging = true
log_format = "json"  # "json", "text", "compact"
include_timestamps = true
include_trace_ids = true

# Metrics export
enable_metrics_export = true
metrics_export_interval_seconds = 60
metrics_export_format = "prometheus"  # "prometheus", "json", "statsd"

# Tracing (optional)
enable_distributed_tracing = false
tracing_endpoint = ""  # OTLP endpoint
tracing_sample_rate = 0.1
```

### Alert Configuration

```toml
[autonomous.monitoring.alerts]
# Enable alerting
enable_alerts = true
alert_on_critical_drift = true
alert_on_recovery_failures = true
alert_on_performance_issues = true

# Alert thresholds
critical_drift_threshold = 1
recovery_failure_threshold = 3
performance_degradation_threshold = 50  # percent

# Alert destinations
slack_webhook_url = ""
email_recipients = []
webhook_endpoints = []

# Alert frequency limits
max_alerts_per_hour = 10
alert_cooldown_minutes = 15
duplicate_alert_suppression_minutes = 60
```

**Environment Variables**:
```bash
# Core monitoring
export MY_LITTLE_SODA_ENABLE_PERFORMANCE_MONITORING=true
export MY_LITTLE_SODA_MONITORING_INTERVAL=3
export MY_LITTLE_SODA_LOG_LEVEL=DEBUG

# Resource limits
export MY_LITTLE_SODA_MAX_MEMORY_USAGE_MB=2048
export MY_LITTLE_SODA_MAX_CPU_USAGE_PERCENT=70

# Alerting
export MY_LITTLE_SODA_SLACK_WEBHOOK="https://hooks.slack.com/..."
export MY_LITTLE_SODA_EMAIL_ALERTS="admin@company.com"
```

## Environment Variables

### Complete Environment Variable Reference

```bash
# === Core Autonomous Settings ===
export MY_LITTLE_SODA_MAX_WORK_HOURS=8
export MY_LITTLE_SODA_ENABLE_AUTONOMOUS_OPERATION=true
export MY_LITTLE_SODA_AGENT_ID="agent001"
export MY_LITTLE_SODA_VALIDATE_WORKSPACE_ON_START=true

# === Drift Detection ===
export MY_LITTLE_SODA_ENABLE_DRIFT_DETECTION=true
export MY_LITTLE_SODA_DRIFT_VALIDATION_INTERVAL=5
export MY_LITTLE_SODA_MAX_COMMITS_BEHIND=10
export MY_LITTLE_SODA_PRESERVE_WORK_ON_DRIFT=true
export MY_LITTLE_SODA_CREATE_DRIFT_ISSUES=true
export MY_LITTLE_SODA_PAUSE_ON_CRITICAL_DRIFT=true

# === Error Recovery ===
export MY_LITTLE_SODA_ENABLE_ERROR_RECOVERY=true
export MY_LITTLE_SODA_MAX_RECOVERY_ATTEMPTS=3
export MY_LITTLE_SODA_RECOVERY_TIMEOUT_MINUTES=30
export MY_LITTLE_SODA_ENABLE_AGGRESSIVE_RECOVERY=false
export MY_LITTLE_SODA_CREATE_RECOVERY_ISSUES=true

# === Persistence & Continuity ===
export MY_LITTLE_SODA_ENABLE_PERSISTENCE=true
export MY_LITTLE_SODA_PERSISTENCE_DIRECTORY=".my-little-soda/state"
export MY_LITTLE_SODA_AUTO_SAVE_INTERVAL=5
export MY_LITTLE_SODA_ENABLE_WORK_CONTINUITY=true
export MY_LITTLE_SODA_AUTO_RESUME_ON_RESTART=true
export MY_LITTLE_SODA_MAX_RESUME_AGE_HOURS=48

# === Monitoring & Performance ===
export MY_LITTLE_SODA_ENABLE_PERFORMANCE_MONITORING=true
export MY_LITTLE_SODA_MONITORING_INTERVAL=5
export MY_LITTLE_SODA_LOG_LEVEL=INFO
export MY_LITTLE_SODA_MAX_MEMORY_USAGE_MB=1024

# === GitHub Integration ===
export MY_LITTLE_SODA_GITHUB_TOKEN="ghp_xxxxxxxxxxxx"
export MY_LITTLE_SODA_GITHUB_OWNER="your-username"
export MY_LITTLE_SODA_GITHUB_REPO="your-repo"

# === Database (Optional) ===
export MY_LITTLE_SODA_DATABASE_URL="sqlite:.my-little-soda/my-little-soda.db"
export MY_LITTLE_SODA_ENABLE_DATABASE_METRICS=true

# === Observability (Optional) ===
export MY_LITTLE_SODA_OBSERVABILITY_OTLP_ENDPOINT="http://localhost:4317"
export MY_LITTLE_SODA_ENABLE_TRACING=false
```

## Configuration Examples

### Development Configuration

```toml
# my-little-soda.dev.toml - Development environment
[autonomous]
max_work_hours = 2  # Shorter for testing
enable_autonomous_operation = true
agent_id = "dev-agent"

[autonomous.drift_detection]
enable_drift_detection = true
validation_interval_minutes = 2  # More frequent validation
max_commits_behind = 5

[autonomous.recovery]
enable_error_recovery = true
max_recovery_attempts = 2
recovery_timeout_minutes = 10
enable_aggressive_recovery = false

[autonomous.persistence]
enable_persistence = true
auto_save_interval_minutes = 2  # More frequent saves
persistence_directory = ".dev-soda/state"

[autonomous.monitoring]
enable_performance_monitoring = true
log_level = "DEBUG"
monitoring_interval_minutes = 1
```

### Production Configuration

```toml
# my-little-soda.prod.toml - Production environment  
[autonomous]
max_work_hours = 12  # Longer production runs
enable_autonomous_operation = true
agent_id = "prod-agent-001"
validate_workspace_on_start = true

[autonomous.drift_detection]
enable_drift_detection = true
validation_interval_minutes = 10  # Less frequent, more efficient
max_commits_behind = 20
pause_on_critical_drift = true
create_drift_issues = true

[autonomous.recovery]
enable_error_recovery = true
max_recovery_attempts = 5
recovery_timeout_minutes = 60
enable_aggressive_recovery = false
prefer_safe_recovery = true

[autonomous.persistence]
enable_persistence = true
auto_save_interval_minutes = 10
backup_directory = "/backup/soda/state"
max_state_history_entries = 500
cleanup_old_states_days = 90

[autonomous.monitoring]
enable_performance_monitoring = true
log_level = "INFO"
monitoring_interval_minutes = 10
enable_metrics_export = true
metrics_export_format = "prometheus"

[autonomous.monitoring.alerts]
enable_alerts = true
slack_webhook_url = "https://hooks.slack.com/services/YOUR/WEBHOOK/URL"
alert_on_critical_drift = true
alert_on_recovery_failures = true
```

### High-Performance Configuration

```toml
# my-little-soda.perf.toml - High-performance environment
[autonomous]
max_work_hours = 8
max_concurrent_issues = 1  # Keep single-threaded for safety

[autonomous.drift_detection]
enable_drift_detection = true
validation_interval_minutes = 15  # Less frequent validation
cache_validation_results_minutes = 5  # Cache results
max_parallel_validations = 3

[autonomous.recovery]
enable_error_recovery = true
max_recovery_attempts = 3
recovery_cooldown_minutes = 5  # Shorter cooldown

[autonomous.persistence]
enable_persistence = true
auto_save_interval_minutes = 15  # Less frequent saves
compress_old_states = true  # Save disk space
enable_integrity_checks = false  # Disable for performance

[autonomous.monitoring]
enable_performance_monitoring = true
enable_detailed_metrics = false  # Reduce overhead
enable_tracing = false  # Disable tracing
monitoring_interval_minutes = 15
```

## Validation & Testing

### Configuration Validation

```bash
# Validate configuration file
./target/release/my-little-soda config validate

# Test configuration with dry run
./target/release/my-little-soda config test --dry-run

# Show effective configuration (after env var overrides)
./target/release/my-little-soda config show --effective
```

### Configuration Testing

```bash
# Test drift detection with current config
./target/release/my-little-soda test drift-detection

# Test error recovery scenarios
./target/release/my-little-soda test error-recovery

# Test persistence and continuity
./target/release/my-little-soda test work-continuity

# Performance test with current config
./target/release/my-little-soda test performance --duration 300
```

### Configuration Migration

```bash
# Migrate configuration from older version
./target/release/my-little-soda config migrate --from-version 0.1.0

# Export configuration to different format
./target/release/my-little-soda config export --format env > .env
./target/release/my-little-soda config export --format yaml > config.yaml

# Import configuration from environment
./target/release/my-little-soda config import --from-env
```

### Best Practices

1. **Start Conservative**: Begin with default settings and tune based on your needs
2. **Monitor Performance**: Watch for configuration changes that impact performance
3. **Test Changes**: Always validate configuration changes in development first
4. **Document Customizations**: Keep track of why you changed specific settings
5. **Regular Review**: Periodically review and optimize your configuration
6. **Environment Consistency**: Keep configurations consistent across environments
7. **Security**: Never commit sensitive tokens or credentials to configuration files

For more information about using these configuration options, see the [main documentation](../README.md#autonomous-system-features) and [troubleshooting guide](../README.md#troubleshooting-autonomous-operation).