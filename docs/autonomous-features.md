# Autonomous System Features

My Little Soda provides advanced autonomous operation capabilities designed for unattended, long-running development workflows. These features ensure reliable operation and maintain work continuity even when issues arise.

For basic workflow information, see the [main README](../README.md#basic-agent-workflow). For configuration details, see the [Configuration Guide](configuration.md).

## Table of Contents
- [Overview](#overview)
- [State Drift Detection](#state-drift-detection)
- [Error Recovery System](#error-recovery-system)
- [Work Continuity & Persistence](#work-continuity--persistence)
- [Configuration Options](#configuration-options)
- [Monitoring & Observability](#monitoring--observability)
- [Troubleshooting](#troubleshooting)

## Overview

The autonomous system enables My Little Soda to operate unattended for extended periods, handling various failure scenarios and maintaining work continuity across interruptions.

**Key Capabilities:**
- **🔍 State Drift Detection** - Monitors and corrects discrepancies between expected and actual system state
- **⚡ Error Recovery** - Automatically handles various failure scenarios
- **💾 Work Continuity** - Preserves and resumes work across system interruptions
- **📊 Monitoring** - Comprehensive observability and performance tracking

## State Drift Detection

The autonomous system continuously monitors for **state drift** - discrepancies between expected system state and actual GitHub/workspace state that can occur during long-running operations.

### What State Drift Detection Monitors

- **Issue assignments** - Detects if issues are unexpectedly reassigned or closed
- **Branch state** - Monitors for deleted branches or unexpected commits  
- **Pull request status** - Tracks unexpected merges, closes, or review changes
- **Workspace consistency** - Validates local git state matches expectations

### Automatic Correction Strategies

```bash
# Minor drifts: Update local expectations to match GitHub
# Moderate drifts: Synchronize state and continue autonomously  
# Critical drifts: Create issue for manual intervention, preserve work
```

### Configuration Example

```bash
# Enable drift detection with custom thresholds
export MY_LITTLE_SODA_DRIFT_DETECTION_ENABLED=true
export MY_LITTLE_SODA_DRIFT_VALIDATION_INTERVAL=5  # minutes
export MY_LITTLE_SODA_MAX_COMMITS_BEHIND=10
```

**Configuration File:**
```toml
[autonomous]
enable_drift_detection = true
drift_validation_interval_minutes = 10
max_commits_behind = 10
```

## Error Recovery System

Autonomous error recovery handles various failure scenarios without human intervention.

### Supported Error Types

- **Git operations** - Push failures, merge conflicts, authentication issues
- **Build failures** - Compilation errors, dependency issues, test failures  
- **CI/CD failures** - Test timeouts, deployment issues, security scans
- **GitHub API** - Rate limits, connectivity issues, permission changes

### Recovery Strategies

- **Automated fixes** - Syntax errors, simple merge conflicts, dependency updates
- **Retry with backoff** - Network timeouts, temporary API failures
- **Escalation** - Complex issues requiring human review

### Example Recovery Scenarios

```bash
# Network timeout during git push
# → Automatic retry with exponential backoff

# Simple merge conflict in documentation  
# → Automatic resolution and re-attempt

# Critical security vulnerability detected
# → Create tracking issue, preserve work, escalate
```

### Configuration

**Environment Variables:**
```bash
export MY_LITTLE_SODA_MAX_RECOVERY_ATTEMPTS=5
export MY_LITTLE_SODA_RECOVERY_TIMEOUT_MINUTES=45
export MY_LITTLE_SODA_ENABLE_AGGRESSIVE_RECOVERY=false
```

**Configuration File:**
```toml
[autonomous.recovery]
max_recovery_attempts = 3
recovery_timeout_minutes = 30
enable_aggressive_recovery = false
```

## Work Continuity & Persistence

Ensures work continues seamlessly across agent restarts and system interruptions.

### State Persistence Features

- **Automatic checkpoints** - Regular saves of workflow state and progress
- **Crash recovery** - Resume work after unexpected shutdowns  
- **State validation** - Verify workspace consistency after restart
- **Work preservation** - Never lose progress due to system issues

### Recovery Actions After Restart

- **Continue work** - Resume from exactly where you left off
- **Validate and resync** - Check state consistency before continuing
- **Start fresh** - Begin new work if previous state is too old/invalid

### Configuration

**Environment Variables:**
```bash
export MY_LITTLE_SODA_ENABLE_PERSISTENCE=true
export MY_LITTLE_SODA_AUTO_SAVE_INTERVAL=3  # minutes
export MY_LITTLE_SODA_PERSISTENCE_DIRECTORY=".my-little-soda/state"
```

**Configuration File:**
```toml
[autonomous.persistence] 
enable_persistence = true
persistence_directory = ".my-little-soda/state"
auto_save_interval_minutes = 5
backup_retention_days = 7
enable_integrity_checks = true
```

## Configuration Options

### Full Autonomous System Configuration

```toml
# my-little-soda.toml
[autonomous]
max_work_hours = 8
enable_drift_detection = true
drift_validation_interval_minutes = 10

[autonomous.recovery]
max_recovery_attempts = 3
recovery_timeout_minutes = 30
enable_aggressive_recovery = false

[autonomous.persistence] 
enable_persistence = true
persistence_directory = ".my-little-soda/state"
auto_save_interval_minutes = 5

[autonomous.monitoring]
monitoring_interval_minutes = 5
enable_performance_metrics = true
```

### Environment Variable Overrides

```bash
# Core autonomous settings
export MY_LITTLE_SODA_MAX_WORK_HOURS=12
export MY_LITTLE_SODA_ENABLE_DRIFT_DETECTION=true

# Recovery settings  
export MY_LITTLE_SODA_MAX_RECOVERY_ATTEMPTS=5
export MY_LITTLE_SODA_RECOVERY_TIMEOUT_MINUTES=45

# Persistence settings
export MY_LITTLE_SODA_ENABLE_PERSISTENCE=true
export MY_LITTLE_SODA_AUTO_SAVE_INTERVAL=3
```

## Monitoring & Observability

Track autonomous operation health and performance.

### Status Commands

```bash
# Check autonomous system status
# Linux/macOS: ./target/release/my-little-soda status --autonomous
# Windows: .\target\release\my-little-soda.exe status --autonomous

# View drift detection report
# Linux/macOS: ./target/release/my-little-soda drift-report
# Windows: .\target\release\my-little-soda.exe drift-report

# Check error recovery statistics  
# Linux/macOS: ./target/release/my-little-soda recovery-report
# Windows: .\target\release\my-little-soda.exe recovery-report

# Validate work continuity state
# Linux/macOS: ./target/release/my-little-soda continuity-status
# Windows: .\target\release\my-little-soda.exe continuity-status
```

### Key Metrics Monitored

- **Drift detection** - Validation frequency, detected drifts, correction success rate
- **Error recovery** - Recovery attempts, success rate, average resolution time
- **Work continuity** - Checkpoint frequency, restart recovery success, state integrity
- **Performance** - Operation throughput, memory usage, processing times

### Observability Configuration

**Configuration File:**
```toml
[observability]
otlp_endpoint = "http://localhost:4317"
enable_tracing = true
enable_metrics = true
```

**Environment Variables:**
```bash
export MY_LITTLE_SODA_OBSERVABILITY_OTLP_ENDPOINT="http://localhost:4317"
export MY_LITTLE_SODA_ENABLE_TRACING=true
export MY_LITTLE_SODA_ENABLE_METRICS=true
```

## Troubleshooting

### Common State Drift Issues

**Issue:** "Critical drift detected requiring manual intervention"
```bash
# Check what drifts were detected
# Linux/macOS: ./target/release/my-little-soda drift-report
# Windows: .\target\release\my-little-soda.exe drift-report

# Common causes:
# - Issue was closed while agent was working
# - Work branch was deleted by another user  
# - PR was merged without agent knowledge

# Resolution:
# 1. Review drift details in created GitHub issue
# 2. Decide whether to restore state or start fresh
# 3. Use my-little-soda reset if starting fresh
```

**Issue:** "State validation failed"
```bash
# Verify workspace consistency
git status
git log --oneline -10

# Check expected vs actual state
# Linux/macOS: ./target/release/my-little-soda status --detailed
# Windows: .\target\release\my-little-soda.exe status --detailed

# Resolution:
# 1. Fix any uncommitted changes or conflicts
# 2. Ensure branch matches expected state
# 3. Run: my-little-soda pop --force-resync
```

### Error Recovery Troubleshooting

**Issue:** "Recovery attempts exhausted"
```bash
# Check recovery history
# Linux/macOS: ./target/release/my-little-soda recovery-report
# Windows: .\target\release\my-little-soda.exe recovery-report

# View detailed error logs
tail -f .my-little-soda/logs/autonomous.log

# Resolution:
# 1. Address root cause shown in recovery report
# 2. Manually fix if automation can't handle
# 3. Reset recovery state: my-little-soda reset --recovery-only
```

**Issue:** "Build failures persist after recovery"
```bash
# Test build manually
cargo build --verbose

# Check if dependencies changed
git diff HEAD~1 Cargo.toml Cargo.lock

# Resolution:
# 1. Fix build issues manually
# 2. Commit fixes: git commit -m "Fix build issues"  
# 3. Continue: my-little-soda bottle
```

### Work Continuity Issues

**Issue:** "Cannot resume work after restart"
```bash
# Check persistence state
ls -la .my-little-soda/state/  # Linux/macOS
dir .my-little-soda\state\     # Windows

# Validate state files
# Linux/macOS: ./target/release/my-little-soda continuity-status
# Windows: .\target\release\my-little-soda.exe continuity-status

# Resolution:
# 1. Check state file permissions
# 2. Verify disk space availability
# 3. If corrupted: my-little-soda reset --state-only
```

**Issue:** "Workspace inconsistencies after restart"
```bash
# Validate workspace state
git status --porcelain
git branch -vv

# Check for uncommitted changes
git diff HEAD

# Resolution:
# 1. Stash uncommitted changes: git stash
# 2. Sync to expected branch: git checkout <expected-branch>
# 3. Resume: my-little-soda pop --validate-workspace
```

### Performance Issues

**Issue:** "Autonomous operations running slowly"
```bash
# Check system resources
df -h .my-little-soda/  # Disk space
ps aux | grep my-little-soda  # CPU usage

# Review performance metrics
# Linux/macOS: ./target/release/my-little-soda status --performance
# Windows: .\target\release\my-little-soda.exe status --performance

# Resolution:
# 1. Clean old state files: my-little-soda cleanup --old-states
# 2. Reduce monitoring frequency in config
# 3. Disable non-essential features temporarily
```

### Advanced Troubleshooting

**Debug Mode:**
```bash
# Enable verbose logging
export RUST_LOG=debug

# Run with detailed output
./target/debug/my-little-soda status --verbose
```

**State Inspection:**
```bash
# View state files
ls -la .my-little-soda/state/
cat .my-little-soda/state/current_state.json

# Check log files
tail -f .my-little-soda/logs/autonomous.log
tail -f .my-little-soda/logs/drift_detection.log
```

**Manual State Reset:**
```bash
# Reset specific components
my-little-soda reset --drift-detection
my-little-soda reset --recovery-state
my-little-soda reset --persistence

# Full reset (use with caution)
my-little-soda reset --all
```

## Getting Help

**Need More Help?**
- **[GitHub Issues](https://github.com/johnhkchen/my-little-soda/issues)** - Report problems or ask questions
- **[System Specification](../spec.md)** - Deep dive into autonomous system architecture  
- **[Agent Lifecycle Documentation](agent_lifecycle.md)** - Understanding agent behavior
- **[Configuration Guide](configuration.md)** - Detailed configuration options