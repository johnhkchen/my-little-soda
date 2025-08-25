# My Little Soda

**Turn your GitHub Issues into an autonomous coding queue - scale your productivity with an AI agent that works while you focus elsewhere.** 

My Little Soda enables a single autonomous AI coding assistant to work on your GitHub Issues continuously while you focus elsewhere. It provides unattended operation and multiplicative productivity through the one-agent-per-repo architecture.

[![Property-Based Tests](https://github.com/johnhkchen/my-little-soda/actions/workflows/property-tests.yml/badge.svg)](https://github.com/johnhkchen/my-little-soda/actions/workflows/property-tests.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Version](https://img.shields.io/badge/version-0.1.0-blue.svg)](https://github.com/johnhkchen/my-little-soda)
[![Status](https://img.shields.io/badge/status-Early%20Alpha-red.svg)](https://github.com/johnhkchen/my-little-soda)

## What My Little Soda Does

- **🤖 Autonomous operation** - Single AI agent works continuously on GitHub Issues while you focus elsewhere
- **🔄 Multiplicative productivity** - 8 hours human work + 3 autonomous repos = 32 repo-hours of progress
- **⚡ Seamless workflow** through a 3-phase cycle: Work → Review → Merge
- **👁️ GitHub native** - All coordination visible through labels and PRs

**In simple terms:** Scale your productivity with an autonomous AI assistant that works unattended on your repository.

## Quick Start

**Already have it installed?** Here's the 30-second workflow:

```bash
# Get your next task
./target/debug/my-little-soda pop

# Work on it (make changes, commit)
git add . && git commit -m "Fix the bug"

# Submit your work
./target/debug/my-little-soda bottle

# System automatically gives you the next task
./target/debug/my-little-soda pop
```

**New to My Little Soda?** → [Installation Guide](#installation)


## See It In Action

### GitHub Issue Management

```bash
# Your repository has labeled issues ready for work
$ gh issue list --label="route:ready"
278	Improve Table of Contents organization and navigation	route:ready
277	Improve platform support visibility and Windows-specific guidance	route:ready  
275	Move detailed configuration and autonomous features to separate documentation	route:ready
```

### Complete Workflow Example

```bash
# Get assigned to the highest priority task
# Linux/macOS:
$ ./target/debug/my-little-soda pop
# Windows:
$ .\target\debug\my-little-soda.exe pop
🎯 Popping next available task...
🔄 Connecting to GitHub... ✅
📋 Searching for available tasks... 
🤖 Attempting atomic assignment: agent agent001 -> issue #271
✅ Reserved assignment: agent agent001 -> issue #271 (capacity: 1/1)
✅ Issue #271 assigned to GitHub user: johnhkchen
🏷️  Adding agent label: agent001
✅ Added agent label: agent001
🌿 Creating branch 'agent001/271-add-visual-demonstrations-to-r' from 'main'
✅ Branch 'agent001/271-add-visual-demonstrations-to-r' created successfully

✅ Successfully popped task:
  📋 Issue #271: Add visual demonstrations to README (screenshots/GIFs)
  👤 Assigned to: agent001
  🌿 Branch: agent001/271-add-visual-demonstrations-to-r
  🔗 URL: https://github.com/johnhkchen/my-little-soda/issues/271

🚀 Ready to work! Issue assigned and branch created/targeted.
   Next: git checkout agent001/271-add-visual-demonstrations-to-r
```

```bash
# Work on it, then submit
$ git add . && git commit -m "Add visual demonstrations to README"
# Linux/macOS:
$ ./target/debug/my-little-soda bottle
# Windows:
$ .\target\debug\my-little-soda.exe bottle
✅ Pull request created: Add visual demonstrations to README (screenshots/GIFs)
✅ Work submitted for review - ready for next task!
```

**Result**: Your repository gets continuous development while you focus on other work.

## Table of Contents

🚀 **Getting Started**
- [Quick Start](#quick-start) - See it working in 30 seconds
- [Installation](#installation) - Get it running on your system (includes platform support)

⚙️ **Using My Little Soda**  
- [Basic Workflow](#basic-agent-workflow) - pop → work → bottle cycle
- [Command Reference](#usage-examples) - All available commands

📚 **Documentation & Help**
- [Troubleshooting](#troubleshooting-autonomous-operation) - Common issues and solutions
- [Complete Documentation](docs/README.md) - Comprehensive guides
- [Contributing](#contributing) - How to help improve the project

🏗️ **Advanced**
- [System Architecture](spec.md) - Technical specifications
- [Autonomous Features](#autonomous-system-features) - Advanced AI capabilities

## Installation

### Platform Support

✅ **Linux** (x86_64, aarch64) | ✅ **macOS** (Intel, Apple Silicon) | ✅ **Windows** (Windows 10/11)

**Windows Users:** Use `.\target\release\my-little-soda.exe` instead of `./target/release/my-little-soda`  
**All Platforms:** PowerShell, Command Prompt, and terminal applications supported

### Quick Install

**Get running in 60 seconds:**

```bash
git clone https://github.com/johnhkchen/my-little-soda.git
cd my-little-soda
cargo build --release

# Test it works
./target/release/my-little-soda --help
# Windows: .\target\release\my-little-soda.exe --help
```

**Executable location:**
- **Linux/macOS**: `./target/release/my-little-soda`
- **Windows**: `.\target\release\my-little-soda.exe`

### Prerequisites

**Essential requirements:**
- **Git** - Standard git installation
- **Rust** - Version 1.75+ (for building from source)
- **GitHub CLI** - `gh auth login` (install from https://cli.github.com/)
- **GitHub Personal Access Token** - With `repo` scope ([create here](https://github.com/settings/tokens))

**Repository permissions needed:**
- Write access to your target repository
- Issues and Pull requests permissions

### Pre-built Binaries
Pre-built binaries are planned for future releases.

### Optional: Build with Features

**Default build (recommended):**
```bash
cargo build --release  # ~15MB, all core functionality
```

**With additional features:**
```bash
cargo build --release --features metrics      # Performance tracking
cargo build --release --features autonomous   # Advanced recovery
cargo build --release --all-features          # Everything (~17MB)
```

Available features: `autonomous`, `metrics`, `observability`, `database`

### Basic Configuration

**Set these environment variables:**
```bash
export GITHUB_TOKEN="ghp_xxxxxxxxxxxxx"
export MY_LITTLE_SODA_GITHUB_OWNER="your-username"
export MY_LITTLE_SODA_GITHUB_REPO="your-repo"
```

**Create a few required labels:**
```bash
gh label create "route:ready" --color "0052cc" --description "Available for agent assignment"
gh label create "route:priority-high" --color "ff6b6b" --description "Priority: 3"
```

**Test it works:**
```bash
./target/release/my-little-soda status
# Windows: .\target\release\my-little-soda.exe status
```

**That's it!** You're ready to start using My Little Soda.

**Need more setup details?** → See [complete installation guide](docs/README.md#installation) for advanced configuration, all labels, and troubleshooting.

## Project Status

**Early Alpha** - Not recommended for production use. See [detailed status information](docs/README.md#project-status) for current capabilities and limitations.

## Usage Examples

### Basic Agent Workflow

Start your development session by claiming work:

```bash
# Get your next assigned task (primary command)
$ ./target/debug/my-little-soda pop
🎯 Popping next available task...
🔄 Connecting to GitHub... ✅
📋 Searching for available tasks... 
✅ Reserved assignment: agent agent001 -> issue #278 (capacity: 1/1)
✅ Issue #278 assigned to GitHub user: johnhkchen
🏷️  Adding agent label: agent001
✅ Added agent label: agent001
🌿 Creating branch 'agent001/278-improve-table-of-contents' from 'main'

✅ Successfully popped task:
  📋 Issue #278: Improve Table of Contents organization and navigation
  👤 Assigned to: agent001
  🌿 Branch: agent001/278-improve-table-of-contents
  🔗 URL: https://github.com/johnhkchen/my-little-soda/issues/278

🚀 Ready to work! Issue assigned and branch created/targeted.
   Next: git checkout agent001/278-improve-table-of-contents
```

**What this does:**
- Assigns you the highest priority issue
- Creates a dedicated branch (e.g., `agent001/278-improve-table-of-contents`)
- Switches you to that branch automatically

### Working on Your Task

Once you have a task, implement your solution:

```bash
# Work in your assigned branch
$ git add .
$ git commit -m "Improve table of contents organization"

# Complete your work and create PR
$ ./target/debug/my-little-soda bottle
✅ Creating pull request for branch: agent001/278-improve-table-of-contents
✅ Pull request created: Improve Table of Contents organization and navigation
✅ Added route:review label to issue #278
✅ Removed agent001 label from issue #278
✅ Work submitted for review - ready for next task!
```

**What `bottle` does:**
- Creates a pull request from your branch
- Marks your work ready for review  
- Frees you to work on the next task

### System Monitoring

Check what's happening in your repository:

```bash
# View agent status and task queue
$ ./target/debug/my-little-soda status
🤖 MY LITTLE SODA STATUS - Repository: my-little-soda
==========================================

🔧 AGENT STATUS:
────────────────
🔴 Busy - Currently working on assigned task
📍 Current branch: agent001/271-add-visual-demonstrations-to-r
🚀 Mode: Manual (use 'my-little-soda spawn --autonomous' for unattended)

📋 ISSUE QUEUE (7 waiting):
────────────────────────────
🟢 #278 Improve Table of Contents organization and navigation
   📝 Priority: Normal | Labels: none

🟢 #277 Improve platform support visibility and Windows-specific guidance
   📝 Priority: Normal | Labels: none

🟢 #275 Move detailed configuration and autonomous features to separate documentation
   📝 Priority: Normal | Labels: none

   ... and 4 more tasks

🎯 NEXT ACTIONS:
   → my-little-soda pop       # Get highest priority task
   → my-little-soda peek      # Preview task details
   → my-little-soda spawn --autonomous  # Start unattended mode
```

### Preview Next Task

See what work is available without claiming it:

```bash
# Preview the next task you would get
$ ./target/debug/my-little-soda peek
🔍 Peeking at next available task...

📋 NEXT TASK:
  Issue #278: Improve Table of Contents organization and navigation
  🏷️  Labels: route:ready
  📍 Priority: Normal
  🔗 URL: https://github.com/johnhkchen/my-little-soda/issues/278

💡 To claim this task, run: my-little-soda pop
```

### Complete Daily Workflow Example

Here's a typical development session:

```bash
# 1. Start your day - get first task
$ ./target/debug/my-little-soda pop
✅ Successfully popped task:
  📋 Issue #278: Improve Table of Contents organization and navigation
  👤 Assigned to: agent001
  🌿 Branch: agent001/278-improve-table-of-contents

# 2. Work on the issue (implement your solution)
# ... write code, update documentation, etc ...
$ git add .
$ git commit -m "Reorganize README table of contents for better navigation"

# 3. Submit your work
$ ./target/debug/my-little-soda bottle
✅ Pull request created: Improve Table of Contents organization and navigation
✅ Work submitted for review - ready for next task!

# 4. Get next task immediately
$ ./target/debug/my-little-soda pop  
✅ Successfully popped task:
  📋 Issue #277: Improve platform support visibility and Windows-specific guidance
  👤 Assigned to: agent001

# 5. Continue the cycle...
```

### Administrative Commands

```bash
# Initialize a new repository (run once per repo)
# Linux/macOS: ./target/debug/my-little-soda init
# Windows: .\target\debug\my-little-soda.exe init

# Reset agent state (admin only)
# Linux/macOS: ./target/debug/my-little-soda reset
# Windows: .\target\debug\my-little-soda.exe reset

# Bundle multiple PRs for review
# Linux/macOS: ./target/debug/my-little-soda bundle
# Windows: .\target\debug\my-little-soda.exe bundle
```

### Getting Help

```bash
# See all available commands
# Linux/macOS:
$ ./target/debug/my-little-soda --help
# Windows:
$ .\target\debug\my-little-soda.exe --help
My Little Soda - Autonomous AI agent orchestration for GitHub repositories

Usage: my-little-soda <COMMAND>

Commands:
  pop     Get assigned to the highest priority available task
  bottle  Create pull request and mark work ready for review
  status  View agent status and task queue
  peek    Preview next available task without claiming it
  init    Initialize repository for My Little Soda
  reset   Reset agent state and assignments
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version

# Get help for specific command
# Linux/macOS: ./target/debug/my-little-soda pop --help
# Windows: .\target\debug\my-little-soda.exe pop --help
Get assigned to the highest priority available task

Usage: my-little-soda pop [OPTIONS]

Options:
      --force-resync  Force resync with GitHub state before assignment
  -h, --help         Print help
```

## Documentation

Comprehensive documentation is organized for different audiences and use cases:

### 📚 User Documentation
- **[Complete User Guide](docs/README.md)** - Installation, configuration, workflows, and troubleshooting
- **[Command Reference](docs/README.md#commands-commandsmd)** - All CLI commands with examples  
- **[Configuration Guide](docs/README.md#configuration-configurationmd)** - Setup and customization options

### 🏗️ Architecture & Specifications  
- **[System Specification](spec.md)** - Complete system architecture and design principles
- **[Domain Specifications](specs/README.md)** - Detailed technical specifications by domain
- **API Documentation** - Auto-generated Rust API docs (available after crate publication)

### 🤖 Agent Integration
- **[Agent Lifecycle](docs/agent_lifecycle.md)** - How autonomous agent operates and processes issues
- **[System Analysis](docs/system_analysis_and_opportunities.md)** - Autonomous agent operation patterns
- **[Autonomous System Features](#autonomous-system-features)** - State drift detection, error recovery, and work continuity
- **[Troubleshooting Guide](#troubleshooting-autonomous-operation)** - Common issues and solutions for autonomous operation

## Autonomous System Features

My Little Soda provides advanced autonomous operation capabilities designed for unattended, long-running development workflows. These features ensure reliable operation and maintain work continuity even when issues arise.

### 🔍 State Drift Detection

The autonomous system continuously monitors for **state drift** - discrepancies between expected system state and actual GitHub/workspace state that can occur during long-running operations.

**What State Drift Detection Monitors:**
- **Issue assignments** - Detects if issues are unexpectedly reassigned or closed
- **Branch state** - Monitors for deleted branches or unexpected commits  
- **Pull request status** - Tracks unexpected merges, closes, or review changes
- **Workspace consistency** - Validates local git state matches expectations

**Automatic Correction Strategies:**
```bash
# Minor drifts: Update local expectations to match GitHub
# Moderate drifts: Synchronize state and continue autonomously  
# Critical drifts: Create issue for manual intervention, preserve work
```

**Configuration Example:**
```bash
# Enable drift detection with custom thresholds
export MY_LITTLE_SODA_DRIFT_DETECTION_ENABLED=true
export MY_LITTLE_SODA_DRIFT_VALIDATION_INTERVAL=5  # minutes
export MY_LITTLE_SODA_MAX_COMMITS_BEHIND=10
```

### ⚡ Error Recovery System

Autonomous error recovery handles various failure scenarios without human intervention:

**Supported Error Types:**
- **Git operations** - Push failures, merge conflicts, authentication issues
- **Build failures** - Compilation errors, dependency issues, test failures  
- **CI/CD failures** - Test timeouts, deployment issues, security scans
- **GitHub API** - Rate limits, connectivity issues, permission changes

**Recovery Strategies:**
- **Automated fixes** - Syntax errors, simple merge conflicts, dependency updates
- **Retry with backoff** - Network timeouts, temporary API failures
- **Escalation** - Complex issues requiring human review

**Example Recovery Scenarios:**
```bash
# Network timeout during git push
# → Automatic retry with exponential backoff

# Simple merge conflict in documentation  
# → Automatic resolution and re-attempt

# Critical security vulnerability detected
# → Create tracking issue, preserve work, escalate
```

### 💾 Work Continuity & Persistence

Ensures work continues seamlessly across agent restarts and system interruptions:

**State Persistence Features:**
- **Automatic checkpoints** - Regular saves of workflow state and progress
- **Crash recovery** - Resume work after unexpected shutdowns  
- **State validation** - Verify workspace consistency after restart
- **Work preservation** - Never lose progress due to system issues

**Checkpoint Configuration:**
```toml
# my-little-soda.toml
[autonomous.persistence]
enable_persistence = true
auto_save_interval_minutes = 5
max_state_history_entries = 100
backup_retention_days = 7
enable_integrity_checks = true
```

**Recovery Actions After Restart:**
- **Continue work** - Resume from exactly where you left off
- **Validate and resync** - Check state consistency before continuing
- **Start fresh** - Begin new work if previous state is too old/invalid

### 🛠️ Configuration Options

**Full Autonomous System Configuration:**
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

**Environment Variable Overrides:**
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

### 📊 Monitoring & Observability

Track autonomous operation health and performance:

**Status Commands:**
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

**Key Metrics Monitored:**
- **Drift detection** - Validation frequency, detected drifts, correction success rate
- **Error recovery** - Recovery attempts, success rate, average resolution time
- **Work continuity** - Checkpoint frequency, restart recovery success, state integrity
- **Performance** - Operation throughput, memory usage, processing times

## Troubleshooting Autonomous Operation

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

**Need More Help?**
- **[GitHub Issues](https://github.com/johnhkchen/my-little-soda/issues)** - Report problems or ask questions
- **[System Specification](spec.md)** - Deep dive into autonomous system architecture  
- **[Agent Lifecycle Documentation](docs/agent_lifecycle.md)** - Understanding agent behavior

## Support & Community

**Need help? Start with:**
- **[Complete Documentation](docs/README.md)** - User guides, troubleshooting, and configuration
- **[GitHub Issues](https://github.com/johnhkchen/my-little-soda/issues)** - Bug reports, feature requests, and questions
- **[System Specification](spec.md)** - Architecture and design principles

## Contributing

We welcome contributions! See the [comprehensive contributing guide](docs/README.md#contributing) for:

- Development setup and guidelines  
- Code quality standards
- Testing approach
- Pull request process

## License

MIT License - see [LICENSE](LICENSE) file for details.

**Copyright © 2025 John Chen**