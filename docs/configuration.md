# My Little Soda Configuration Guide

This guide provides comprehensive configuration options for My Little Soda. For quick setup, see the [main README](../README.md#configuration).

## Table of Contents
- [Configuration Methods](#configuration-methods)
- [Environment Variables](#environment-variables)
- [Configuration File](#configuration-file)
- [Feature Flags](#feature-flags)
- [Repository Setup](#repository-setup)
- [Troubleshooting Configuration](#troubleshooting-configuration)

## Configuration Methods

My Little Soda supports multiple configuration methods in order of precedence:

1. **Environment Variables** (highest precedence)
2. **Configuration File** (`my-little-soda.toml`)
3. **Default Values** (lowest precedence)

### Option 1: Environment Variables (Recommended for CI/CD)

```bash
export GITHUB_TOKEN="ghp_xxxxxxxxxxxxx"
export MY_LITTLE_SODA_GITHUB_OWNER="your-username"
export MY_LITTLE_SODA_GITHUB_REPO="your-repo"
```

### Option 2: Configuration File (Recommended for local development)

Copy the example configuration and customize:
```bash
cp my-little-soda.example.toml my-little-soda.toml
# Edit my-little-soda.toml with your repository details
```

### Option 3: .env File

Create a `.env` file in your project root:
```bash
GITHUB_TOKEN=ghp_xxxxxxxxxxxxx
MY_LITTLE_SODA_GITHUB_OWNER=your-username
MY_LITTLE_SODA_GITHUB_REPO=your-repo
```

## Environment Variables

### Core Configuration

| Variable | Description | Required | Example |
|----------|-------------|----------|---------|
| `GITHUB_TOKEN` | GitHub Personal Access Token | Yes | `ghp_xxxxxxxxxxxxx` |
| `MY_LITTLE_SODA_GITHUB_OWNER` | Repository owner/username | Yes | `your-username` |
| `MY_LITTLE_SODA_GITHUB_REPO` | Repository name | Yes | `your-repo` |

### Optional Configuration

| Variable | Description | Default | Example |
|----------|-------------|---------|---------|
| `MY_LITTLE_SODA_GITHUB_TOKEN` | Alternative to GITHUB_TOKEN | None | `ghp_xxxxxxxxxxxxx` |
| `MY_LITTLE_SODA_DATABASE_URL` | SQLite database path | None | `.my-little-soda/my-little-soda.db` |
| `MY_LITTLE_SODA_OBSERVABILITY_OTLP_ENDPOINT` | OpenTelemetry endpoint | None | `http://localhost:4317` |

### Autonomous Operation Environment Variables

```bash
# Core autonomous settings
export MY_LITTLE_SODA_MAX_WORK_HOURS=12
export MY_LITTLE_SODA_ENABLE_DRIFT_DETECTION=true
export MY_LITTLE_SODA_DRIFT_VALIDATION_INTERVAL=10  # minutes
export MY_LITTLE_SODA_MAX_COMMITS_BEHIND=10

# Recovery settings  
export MY_LITTLE_SODA_MAX_RECOVERY_ATTEMPTS=5
export MY_LITTLE_SODA_RECOVERY_TIMEOUT_MINUTES=45
export MY_LITTLE_SODA_ENABLE_AGGRESSIVE_RECOVERY=false

# Persistence settings
export MY_LITTLE_SODA_ENABLE_PERSISTENCE=true
export MY_LITTLE_SODA_AUTO_SAVE_INTERVAL=3  # minutes
export MY_LITTLE_SODA_PERSISTENCE_DIRECTORY=".my-little-soda/state"

# Monitoring settings
export MY_LITTLE_SODA_MONITORING_INTERVAL=5  # minutes
export MY_LITTLE_SODA_ENABLE_PERFORMANCE_METRICS=true
```

## Configuration File

### Basic Configuration File

Example `my-little-soda.toml`:

```toml
[github]
owner = "your-username"
repo = "your-repo"
token = "ghp_xxxxxxxxxxxxx"  # Optional if GITHUB_TOKEN is set

[agent]
id = "agent001"
capacity = 1

[features]
autonomous = false
metrics = false
observability = false
database = false
```

### Full Configuration File with Autonomous Features

```toml
[github]
owner = "your-username"
repo = "your-repo"
token = "ghp_xxxxxxxxxxxxx"

[agent]
id = "agent001"
capacity = 1

[features]
autonomous = true
metrics = true
observability = true
database = true

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
backup_retention_days = 7
enable_integrity_checks = true

[autonomous.monitoring]
monitoring_interval_minutes = 5
enable_performance_metrics = true

[database]
url = ".my-little-soda/my-little-soda.db"
auto_create = true

[observability]
otlp_endpoint = "http://localhost:4317"
enable_tracing = true
enable_metrics = true
```

## Feature Flags

My Little Soda supports optional features that can be enabled during compilation:

### Available Features

- `autonomous` - Work continuity and recovery capabilities for resuming interrupted tasks
- `metrics` - Performance tracking and routing metrics collection
- `observability` - Enhanced telemetry and tracing capabilities
- `database` - SQLite database support for persistent storage

### Build Examples

**Default (minimal):**
```bash
cargo build --release
# Builds with basic functionality only (~15MB)
```

**With specific features:**
```bash
# Build with metrics tracking
cargo build --release --features metrics

# Build with work continuity
cargo build --release --features autonomous

# Build with observability and metrics
cargo build --release --features "observability,metrics"

# Build with all features
cargo build --release --all-features
```

### Binary Size Comparison
- **Default build**: ~15MB (core functionality only)  
- **All features**: ~17MB (includes all modules)
- **Individual features**: Add ~0.5-1MB each

### Recommendations
- **Production**: Use default build for minimal footprint
- **Development**: Use `--features metrics` for performance insights
- **CI/CD environments**: Use `--features autonomous` for recovery capabilities

## Repository Setup

### Prerequisites

#### Required
- **GitHub CLI**: `gh auth login` (for GitHub API access)
  - Install: https://cli.github.com/
  - Authenticate: `gh auth login` (required for repository operations)
  - Verify: `gh auth status`
- **Git**: Standard git installation
- **Rust**: 1.75+ (for building from source)
- **GitHub Personal Access Token**: Required for API operations
  - Create at: https://github.com/settings/tokens
  - Required scopes: `repo`, `read:org` (for private repos)
  - Can be set via `GITHUB_TOKEN` or `MY_LITTLE_SODA_GITHUB_TOKEN` environment variable

#### Repository Permissions
- **Write access** to the target repository (for creating branches, PRs, and labels)
- **Issues permission** (to read, create, and modify issues)
- **Pull requests permission** (to create and manage PRs)

#### Optional Dependencies
- **Database** (SQLite): For persistent state storage and metrics
  - Auto-created at `.my-little-soda/my-little-soda.db` if enabled
  - Enable in `my-little-soda.toml` or via `MY_LITTLE_SODA_DATABASE_URL`
- **OpenTelemetry Endpoint**: For distributed tracing and observability
  - Defaults to stdout export if not configured
  - Set via `MY_LITTLE_SODA_OBSERVABILITY_OTLP_ENDPOINT`

### Automated Setup (Coming Soon)

The `my-little-soda init` command will automate repository setup in a future release:

```bash
# Future: One-command setup (WIP)
./target/release/my-little-soda init
```

**What this will do:**
- ✅ Validate GitHub authentication and permissions
- 🏷️  Create required routing labels (`route:ready`, `route:priority-high`, etc.)
- ⚙️  Generate `my-little-soda.toml` configuration 
- 🤖 Initialize autonomous agent configuration
- 📁 Create `.my-little-soda/` directory structure
- ✅ Verify setup and test connectivity

### Manual Setup (Current Required Process)

Until `my-little-soda init` is implemented, set up your repository manually:

**1. Create Required GitHub Labels:**
```bash
# Core routing labels
gh label create "route:ready" --color "0052cc" --description "Available for agent assignment"
gh label create "route:ready_to_merge" --color "5319e7" --description "Completed work ready for merge"
gh label create "route:unblocker" --color "d73a4a" --description "Critical system issues"
gh label create "route:review" --color "fbca04" --description "Under review"
gh label create "route:human-only" --color "7057ff" --description "Requires human attention"

# Priority labels  
gh label create "route:priority-low" --color "c2e0c6" --description "Priority: 1"
gh label create "route:priority-medium" --color "f9d71c" --description "Priority: 2"
gh label create "route:priority-high" --color "ff6b6b" --description "Priority: 3"  
gh label create "route:priority-very-high" --color "d73a4a" --description "Priority: 4"
```

**2. Verify Configuration:**
```bash
# Test that my-little-soda can connect to your repository
$ ./target/debug/my-little-soda status
🤖 MY LITTLE SODA STATUS - Repository: my-little-soda
==========================================
🔄 Gathering system information... ✅

🔧 AGENT STATUS:
────────────────
🟢 Available - Ready for new assignments
🚀 Mode: Manual (use 'my-little-soda spawn --autonomous' for unattended)

📋 ISSUE QUEUE (7 waiting):
────────────────────────────
🟢 #278 Improve Table of Contents organization and navigation
   📝 Priority: Normal | Labels: route:ready
...

🎯 NEXT ACTIONS:
   → my-little-soda pop       # Get highest priority task
```

**3. Start Using My Little Soda:**
```bash
# Label some issues as ready for the agent
$ gh issue edit 278 --add-label "route:ready"
✓ Labeled issue #278 in johnhkchen/my-little-soda

# Begin agent workflow
$ ./target/debug/my-little-soda pop
```

## Troubleshooting Configuration

### Common Issues

**Issue: "GitHub authentication failed"**
```bash
# Check GitHub CLI authentication
gh auth status

# Re-authenticate if needed
gh auth login

# Verify token permissions
gh api user
```

**Issue: "Repository not found or access denied"**
```bash
# Verify repository exists and you have access
gh repo view YOUR_OWNER/YOUR_REPO

# Check configuration
echo $MY_LITTLE_SODA_GITHUB_OWNER
echo $MY_LITTLE_SODA_GITHUB_REPO
```

**Issue: "Missing required labels"**
```bash
# Check existing labels
gh label list | grep "route:"

# Create missing labels using the setup commands above
```

**Issue: "Configuration file not found"**
```bash
# Copy example configuration
cp my-little-soda.example.toml my-little-soda.toml

# Or use environment variables instead
export GITHUB_TOKEN="ghp_xxxxxxxxxxxxx"
export MY_LITTLE_SODA_GITHUB_OWNER="your-username"
export MY_LITTLE_SODA_GITHUB_REPO="your-repo"
```

**Issue: "Database connection failed"**
```bash
# Check database directory exists and is writable
ls -la .my-little-soda/
mkdir -p .my-little-soda/

# Verify database URL is correct
echo $MY_LITTLE_SODA_DATABASE_URL

# Test with disabled database
unset MY_LITTLE_SODA_DATABASE_URL
./target/debug/my-little-soda status
```

### Configuration Validation

**Test your configuration:**
```bash
# Basic connectivity test
./target/debug/my-little-soda status

# Detailed configuration check
./target/debug/my-little-soda status --verbose

# Test GitHub operations
gh api repos/$MY_LITTLE_SODA_GITHUB_OWNER/$MY_LITTLE_SODA_GITHUB_REPO
```

**Verify all components:**
```bash
# Check GitHub CLI
gh --version

# Check git
git --version

# Check Rust (if building from source)
rustc --version

# Check repository access
gh repo view $MY_LITTLE_SODA_GITHUB_OWNER/$MY_LITTLE_SODA_GITHUB_REPO
```

### Getting Help

If you're still experiencing configuration issues:

1. Check the [main troubleshooting guide](../README.md#troubleshooting-autonomous-operation)
2. Review the [system specification](../spec.md) for architecture details
3. Create an issue on [GitHub](https://github.com/johnhkchen/my-little-soda/issues) with:
   - Your configuration (sanitized - remove tokens)
   - Error messages
   - Output of `./target/debug/my-little-soda status`