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

### Prerequisites

Before installing Clambake, ensure you have the following:

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
  - Can be set via `GITHUB_TOKEN` or `CLAMBAKE_GITHUB_TOKEN` environment variable

#### Repository Permissions
- **Write access** to the target repository (for creating branches, PRs, and labels)
- **Issues permission** (to read, create, and modify issues)
- **Pull requests permission** (to create and manage PRs)

#### Optional Dependencies
- **Database** (SQLite): For persistent state storage and metrics
  - Auto-created at `.clambake/clambake.db` if enabled
  - Enable in `clambake.toml` or via `CLAMBAKE_DATABASE_URL`
- **OpenTelemetry Endpoint**: For distributed tracing and observability
  - Defaults to stdout export if not configured
  - Set via `CLAMBAKE_OBSERVABILITY_OTLP_ENDPOINT`

> **Note**: Clambake is a coordination tool for GitHub repositories. It does not require API keys for AI services (OpenAI, Anthropic, etc.) as it manages workflows for external AI agents that handle their own authentication.

### Platform Support
- **Linux** (x86_64, aarch64)
- **macOS** (Intel and Apple Silicon)  
- **Windows** (Windows 10/11)

> **Windows Note:** Use `.\target\release\clambake.exe` instead of `./target/release/clambake`

### Option 1: Build from Source

```bash
git clone https://github.com/johnhkchen/clambake.git
cd clambake
cargo build --release
```

Executable location: `./target/release/clambake` (Windows: `.\target\release\clambake.exe`)

### Option 2: Pre-built Binaries
Pre-built binaries are planned for future releases.

### Configuration

Clambake supports multiple configuration methods in order of precedence:

#### Option 1: Environment Variables (Recommended for CI/CD)
```bash
export GITHUB_TOKEN="ghp_xxxxxxxxxxxxx"
export CLAMBAKE_GITHUB_OWNER="your-username"
export CLAMBAKE_GITHUB_REPO="your-repo"
```

#### Option 2: Configuration File (Recommended for local development)
Copy the example configuration and customize:
```bash
cp clambake.example.toml clambake.toml
# Edit clambake.toml with your repository details
```

#### Option 3: .env File
Create a `.env` file in your project root:
```bash
GITHUB_TOKEN=ghp_xxxxxxxxxxxxx
CLAMBAKE_GITHUB_OWNER=your-username
CLAMBAKE_GITHUB_REPO=your-repo
```

### Setup Your Repository
After installation and configuration, set up the required GitHub labels:

```bash
./target/release/clambake setup-labels
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

```bash
# Get your next assigned task (primary command)
./target/release/clambake pop
```

**What this does:**
- Assigns you the highest priority issue
- Creates a dedicated branch (e.g., `agent001/42-fix-bug`)
- Switches you to that branch automatically

### Working on Your Task

Once you have a task, implement your solution:

```bash
# Work in your assigned branch
git add .
git commit -m "Implement feature X"

# Complete your work and create PR
./target/release/clambake land
```

**What `land` does:**
- Creates a pull request from your branch
- Marks your work ready for review
- Frees you to work on the next task

### System Monitoring

Check what's happening in your repository:

```bash
# View agent status and task queue
./target/release/clambake status
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

```bash
# Preview the next task you would get
./target/release/clambake peek
```

### Complete Daily Workflow Example

Here's a typical development session:

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

### Administrative Commands

```bash
# Initialize a new repository (run once per repo)
./target/release/clambake init

# Reset all agents (admin only)
./target/release/clambake reset

# Bundle multiple PRs for review
./target/release/clambake bundle
```

### Getting Help

```bash
# See all available commands
./target/release/clambake --help

# Get help for specific command
./target/release/clambake pop --help
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