# Clambake

**Multi-agent AI orchestration for GitHub repositories.** Clambake coordinates multiple AI coding assistants working on your GitHub Issues simultaneously, preventing conflicts and managing their progress through proper development workflows.

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

## Project Status

**Early Alpha** - Not recommended for production use. See [detailed status information](docs/README.md#project-status) for current capabilities and limitations.

## Quick Start

### Prerequisites
- GitHub repository with Issues enabled
- GitHub CLI (gh) authenticated: `gh auth login`
- Rust 1.75+ for building from source

### Install
```bash
git clone https://github.com/johnhkchen/clambake.git
cd clambake
cargo build --release
```

### Setup Repository Labels
```bash
./target/release/clambake setup-labels
```

### Start Using
```bash
# Get next task
./target/release/clambake pop

# Check system status  
./target/release/clambake status

# Complete work cycle
./target/release/clambake land
```

> 📖 **Need more detail?** See the [complete installation guide](docs/README.md#installation) for platform-specific requirements, authentication setup, and troubleshooting.

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

Need help? Start with:
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