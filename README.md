# My Little Soda

**Horizontal scaling for autonomous agents: Today 8→32 repo-hours, tomorrow 1→15 days of work.**

Run autonomous agents across multiple repositories simultaneously. As AI agents evolve to work unattended for full days, this architecture multiplies that productivity across your repository portfolio.

## The Problem

Solo developers can only work one repository at a time. Context switching is expensive. PM/marketing teams want to contribute code but lack context.

## The Solution

Run one agent per repository on GitHub Issues labeled `route:ready`. One developer + 3 repositories = 3x development capacity.

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

**Current**: 1 developer + 3 agents = 32 repo-hours  
**Future**: 1 day unattended + 3 repos = 15 days of work

One agent per repository eliminates merge conflicts, builds repository-specific context, and prevents interference during long autonomous sessions.

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

1. **Peek**: Preview available issues
2. **Pop**: Claim issue, create branch `agent001/123-fix-login-bug`
3. **Work**: Implement solution autonomously (15-60 minutes today, hours tomorrow)
4. **Bottle**: Bundle work, add `route:review` label
5. **Review**: Review PR and merge

Two-phase design separates autonomous implementation from human-supervised merges.

## Architecture

Horizontal scaling infrastructure for autonomous agents. As vertical scaling improves (1 day → 5 days of work), horizontal scaling multiplies that across repositories.

One agent per repository eliminates merge conflicts, coordination complexity, and context contamination while enabling repository-specific knowledge and clean autonomous-to-human handoffs.

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