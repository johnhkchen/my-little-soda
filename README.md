# My Little Soda

**Horizontal scaling for autonomous agents: Today 8→32 repo-hours, tomorrow 1→15 days of work.**

My Little Soda enables horizontal scaling across repositories to multiply autonomous agent productivity. As AI agents evolve to work unattended for full days, this architecture scales that productivity across your entire repository portfolio.

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

**Built for the future of autonomous agents.** As AI agents evolve to work unattended for full days (turning 1 day into 5 days of work), this architecture multiplies that productivity across repositories.

**Current scaling**: 1 developer + 3 agents = 32 repo-hours  
**Future scaling**: 1 day unattended + 3 repos = 15 days of work

- **No coordination complexity**: One agent per repo eliminates merge conflicts during long autonomous sessions
- **Repository-specific context**: Each agent builds deep knowledge of one codebase over extended periods
- **Clean isolation**: Long-running agents won't interfere with each other
- **Issue-driven workflow**: GitHub Issues become distributed task queues for autonomous work

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

**Designed for autonomous operation**: Issues fit in single context windows today, but the workflow scales to full-day autonomous sessions.

1. **Peek**: Preview available issues without claiming them
2. **Pop**: Agent claims a `route:ready` issue and creates branch `agent001/123-fix-login-bug`
3. **Work**: Agent implements solution autonomously (15-60 minutes today, hours tomorrow)
4. **Bottle**: Agent bundles completed work, adds `route:review` label
5. **Review**: You review PR and merge when ready

**Two-phase design**: Separates autonomous implementation from human-supervised merge decisions, enabling long unattended work sessions.

## Architecture: Horizontal Scaling Infrastructure

**Strategic positioning**: My Little Soda provides horizontal scaling infrastructure for autonomous agents. As vertical scaling improves (1 day → 5 days of work), horizontal scaling multiplies that across repositories.

**One agent per repository eliminates**:
- Merge conflicts during long autonomous sessions
- Complex coordination that derails unattended operation
- Context contamination between different codebases

**Enables**:
- Repository-specific knowledge building over extended periods
- Clean handoffs between autonomous work and human review
- Portfolio-wide productivity scaling as agents become more capable

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