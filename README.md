# My Little Soda

**Horizontal scaling for solo developers: Turn 8 hours into 32 repo-hours.**

My Little Soda lets you run autonomous coding agents across multiple repositories simultaneously. Instead of working on one repo at a time, multiply your capacity by treating each repository as an independent work queue.

## The Problem

Solo developers maintaining multiple repositories face a capacity bottleneck: you can only actively work on one codebase at a time. Context switching between repositories is expensive and limits your effective development time.

**Early codebases hit bottlenecks fast.** PM/marketing teams want to contribute code but lack context. Developers burn out juggling multiple projects.

## The Solution

**Repository-level horizontal scaling.** Run one agent per repository, each working independently on GitHub Issues labeled `route:ready`. One developer + 3 repositories = 3x development capacity.

```bash
# Terminal 1: Web app repository  
my-little-soda pop  # Gets issue #123: Fix login bug

# Terminal 2: API repository
my-little-soda pop  # Gets issue #45: Add rate limiting  

# Terminal 3: Mobile app repository
my-little-soda peek  # Check what's available without claiming
my-little-soda pop   # Gets issue #67: Update dependencies
```

All three agents work simultaneously while you focus on architecture, planning, or other high-value tasks.

## Why This Architecture Works

**Efficiency over maximum speed.** Instead of optimizing for fastest throughput per repository (complex, merge-heavy), we optimize for **sustainable scaling across repositories** (simple, conflict-free).

- **No merge conflicts**: One agent per repo eliminates coordination complexity
- **Sustainable pace**: Faster than manual, simpler than multi-agent chaos  
- **Context preservation**: Each agent maintains repository-specific knowledge
- **PM/Marketing friendly**: Non-technical team members can create meaningful issues for agents to implement

## Quick Start

### Install
```bash
# From source
cargo install --path .

# Or download binary from releases
```

### Setup Repository
Add labels to your GitHub issues:
- `route:ready` → Available for agent pickup
- `route:priority-high` → Process first (optional)

### Run Agent
```bash
# Check available work
my-little-soda peek

# Get assigned work
my-little-soda pop

# Complete and bundle work  
my-little-soda bottle

# Check status
my-little-soda status
```

## Core Workflow

1. **Peek**: Preview available issues without claiming them
2. **Pop**: Agent claims a `route:ready` issue and creates branch `agent001/123-fix-login-bug`
3. **Work**: Agent implements solution autonomously
4. **Bottle**: Agent bundles completed work, adds `route:review` label
5. **Review**: You review PR and merge when ready

## Architecture: One Agent Per Repository

**Key insight**: Instead of complex multi-agent coordination within one repository, scale horizontally across your repository portfolio. Each repository gets one autonomous agent working independently.

**Benefits**:
- No merge conflicts between agents
- Simple, predictable operation  
- GitHub Issues become your task queue
- Native integration with existing workflows
- **Enables non-technical team members** to become effective code creators through well-written issues

## Development

```bash
# Build and test
cargo build
cargo test

# Initialize a repository for My Little Soda
my-little-soda init
```

## Contributing

1. Fork the repository
2. Add `route:ready` labels to issues you want agents to work on
3. Submit pull requests with clear descriptions

## License

This project is open source. See LICENSE file for details.