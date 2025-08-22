# Clambake

**Multi-agent AI orchestration for GitHub repositories.** 

Clambake coordinates multiple AI coding assistants working on your GitHub Issues simultaneously. It prevents conflicts and manages their progress through proper development workflows.

[![Property-Based Tests](https://github.com/johnhkchen/clambake/actions/workflows/property-tests.yml/badge.svg)](https://github.com/johnhkchen/clambake/actions/workflows/property-tests.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Version](https://img.shields.io/badge/version-0.1.0-blue.svg)](https://github.com/johnhkchen/clambake)
[![Status](https://img.shields.io/badge/status-Early%20Alpha-red.svg)](https://github.com/johnhkchen/clambake)

## What Clambake Does

- **🤖 Coordinates AI agents** working on different GitHub Issues simultaneously
- **🔀 Prevents conflicts** with automatic branch isolation for each agent
- **⚡ Manages workflow** through a 3-phase cycle: Work → Review → Merge
- **👁️ Uses GitHub natively** - all coordination visible through labels and PRs

**In simple terms:** Scale your development team with AI assistants that work together like human developers.

## Table of Contents
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Usage Examples](#usage-examples)
- [Documentation](#documentation)
- [Contributing](#contributing)

## Installation

### Requirements
- **GitHub CLI**: `gh auth login` (for GitHub API access)
- **Git**: Standard git installation
- **Rust**: 1.75+ (for building from source)

### Platform Support

**Supported Operating Systems:**
- **Linux** (x86_64, aarch64)
- **macOS** (Intel and Apple Silicon)  
- **Windows** (Windows 10/11, PowerShell/Command Prompt)

**Platform-Specific Notes:**
- **Windows users:** Use `.\target\release\clambake.exe` instead of `./target/release/clambake`
- **Windows paths:** Commands use backslashes (`\`) for path separators
- **Git behavior:** Cross-platform - same commands work on all operating systems
- **GitHub CLI:** Available on all platforms via package managers

### Option 1: Build from Source

**Unix/Linux/macOS:**
```bash
git clone https://github.com/johnhkchen/clambake.git
cd clambake
cargo build --release
```

**Windows (PowerShell/Command Prompt):**
```cmd
git clone https://github.com/johnhkchen/clambake.git
cd clambake
cargo build --release
```

**Executable location:**
- **Unix/Linux/macOS:** `./target/release/clambake`
- **Windows:** `.\target\release\clambake.exe`

### Option 2: Pre-built Binaries
Pre-built binaries are planned for future releases.

### Setup Your Repository
After installation, set up the required GitHub labels:

**Unix/Linux/macOS:**
```bash
./target/release/clambake setup-labels
```

**Windows:**
```cmd
.\target\release\clambake.exe setup-labels
```

> 📖 **Need help?** See the [complete installation guide](docs/README.md#installation) for troubleshooting and advanced configuration.

## Project Status

**Early Alpha** - Not recommended for production use. See [detailed status information](docs/README.md#project-status) for current capabilities and limitations.

## Quick Start

**Already installed?** Here's the essential workflow:

1. **Get a task:** `./target/release/clambake pop`
2. **Work on it:** Make your changes and commit
3. **Submit work:** `./target/release/clambake land`
4. **Repeat:** System automatically assigns next task

See [Usage Examples](#usage-examples) for detailed commands.

## Usage Examples

### Basic Agent Workflow

Start your development session by claiming work:

**Unix/Linux/macOS:**
```bash
# Get your next assigned task (primary command)
./target/release/clambake pop
```

**Windows:**
```cmd
# Get your next assigned task (primary command)
.\target\release\clambake.exe pop
```

**What this does:**
- Assigns you the highest priority issue
- Creates a dedicated branch (e.g., `agent001/42-fix-bug`)
- Switches you to that branch automatically

### Working on Your Task

Once you have a task, implement your solution:

```bash
# Work in your assigned branch (same on all platforms)
git add .
git commit -m "Implement feature X"
```

**Complete your work and create PR:**

**Unix/Linux/macOS:**
```bash
./target/release/clambake land
```

**Windows:**
```cmd
.\target\release\clambake.exe land
```

**What `land` does:**
- Creates a pull request from your branch
- Marks your work ready for review
- Frees you to work on the next task

### System Monitoring

Check what's happening in your repository:

**Unix/Linux/macOS:**
```bash
# View agent status and task queue
./target/release/clambake status
```

**Windows:**
```cmd
# View agent status and task queue
.\target\release\clambake.exe status
```

Example output:
```
🤖 Agent Status:
  agent001: Working on issue #42 (branch: agent001/42-fix-bug)
  
📋 Task Queue: 3 issues available
  #45: Add user authentication [priority-high]
  #48: Update documentation [priority-medium]  
  #51: Refactor API client [priority-low]
```

### Preview Next Task

See what work is available without claiming it:

**Unix/Linux/macOS:**
```bash
# Preview the next task you would get
./target/release/clambake peek
```

**Windows:**
```cmd
# Preview the next task you would get
.\target\release\clambake.exe peek
```

### Complete Daily Workflow Example

Here's a typical development session:

**Unix/Linux/macOS:**
```bash
# 1. Start your day - get first task
./target/release/clambake pop
# ✅ Assigned issue #42: Fix login bug

# 2. Work on the issue (implement your solution)
# ... write code, tests, etc ...
git add .
git commit -m "Fix login validation bug"

# 3. Submit your work
./target/release/clambake land
# ✅ PR created, work submitted for review

# 4. Get next task immediately
./target/release/clambake pop  
# ✅ Assigned issue #45: Add user authentication

# 5. Continue the cycle...
```

**Windows:**
```cmd
REM 1. Start your day - get first task
.\target\release\clambake.exe pop
REM ✅ Assigned issue #42: Fix login bug

REM 2. Work on the issue (implement your solution)
REM ... write code, tests, etc ...
git add .
git commit -m "Fix login validation bug"

REM 3. Submit your work
.\target\release\clambake.exe land
REM ✅ PR created, work submitted for review

REM 4. Get next task immediately
.\target\release\clambake.exe pop  
REM ✅ Assigned issue #45: Add user authentication

REM 5. Continue the cycle...
```

### Administrative Commands

**Initialize a new repository:**

**Unix/Linux/macOS:**
```bash
# Set up labels and configuration (run once per repo)
./target/release/clambake init
```

**Windows:**
```cmd
REM Set up labels and configuration (run once per repo)
.\target\release\clambake.exe init
```

**Reset all agents (admin only):**

**Unix/Linux/macOS:**
```bash
# Clear all agent assignments
./target/release/clambake reset
```

**Windows:**
```cmd
REM Clear all agent assignments
.\target\release\clambake.exe reset
```

**Bundle multiple PRs for review:**

**Unix/Linux/macOS:**
```bash
# Combine completed work into single review bundle
./target/release/clambake bundle
```

**Windows:**
```cmd
REM Combine completed work into single review bundle
.\target\release\clambake.exe bundle
```

### Getting Help

**Unix/Linux/macOS:**
```bash
# See all available commands
./target/release/clambake --help

# Get help for specific command
./target/release/clambake pop --help
```

**Windows:**
```cmd
REM See all available commands
.\target\release\clambake.exe --help

REM Get help for specific command
.\target\release\clambake.exe pop --help
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
- **[API Documentation](https://docs.rs/clambake)** - Auto-generated Rust API docs

### 🤖 Agent Integration
- **[Agent Lifecycle](docs/agent_lifecycle.md)** - How agents coordinate and work together
- **[System Analysis](docs/system_analysis_and_opportunities.md)** - Agent coordination patterns

## Support & Community

**Need help? Start with:**
- **[Complete Documentation](docs/README.md)** - User guides, troubleshooting, and configuration
- **[GitHub Issues](https://github.com/johnhkchen/clambake/issues)** - Bug reports, feature requests, and questions
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