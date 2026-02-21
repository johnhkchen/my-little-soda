# My Little Soda

[![Archived](https://img.shields.io/badge/status-archived-lightgrey)](https://github.com/johnhkchen/my-little-soda)
[![Successor](https://img.shields.io/badge/successor-lisa-blue)](https://github.com/johnhkchen/lisa)

> **This project has been archived.** Its successor, [**lisa**](https://github.com/johnhkchen/lisa), takes the ideas explored here and rebuilds them into a more capable system. If you're interested in AI-assisted development orchestration, head there instead.

## What Was My Little Soda?

My Little Soda was a Rust CLI tool for multiplying development capacity by running autonomous coding agents across repositories. The core idea: while you focus on architecture and code review, agents claim GitHub issues, implement solutions unattended, and bundle their work into pull requests for you to review.

### The Pop-Bottle Workflow

The agent workflow followed a simple cycle:

1. **Pop** an issue from the queue (claim the next available GitHub issue)
2. **Work** on it autonomously (implement the solution on an agent branch)
3. **Bottle** the result (push the branch, mark it ready for review, free the agent)
4. **Bundle** completed branches into consolidated PRs at scheduled intervals

The agent was freed immediately after bottling, not when the PR merged, so it could continuously pick up new work.

### One Agent Per Repository

The central architectural constraint was **one agent per repository**. Rather than coordinating multiple agents in a single repo (which introduces merge conflicts, resource contention, and complex synchronization), My Little Soda scaled horizontally: run separate instances across different repositories. Three repos with one agent each gives you 3x development capacity with zero coordination overhead.

### GitHub as Single Source of Truth

All state lived in GitHub: issues for tasks, labels for routing (`route:ready`, `route:review`), branches for work-in-progress, and PRs for review. No local state files, no dual state stores, no manual synchronization. This was a hard lesson learned from earlier projects where split state caused coordination disasters.

### Key Features

- **Priority routing** with label-based priority levels from normal to unblocker
- **Train schedule bundling** at 10-minute intervals for predictable PR creation
- **Merge conflict analysis** before bundling, with fallback to individual PRs
- **Agent lifecycle management** with stuck-agent detection and recovery
- **Structured observability** with OpenTelemetry integration and correlation IDs

## Why Archived?

My Little Soda validated the core concept of AI-assisted development orchestration, but the approach of one-agent-per-repo with manual Claude Code session management had limits. The ideas here evolved into something better.

## What Comes Next: Lisa

[**Lisa**](https://github.com/johnhkchen/lisa) is the successor to My Little Soda, rebuilt from the ground up as a DAG-driven concurrent task scheduler for AI-assisted development.

Where My Little Soda ran one agent per repo processing issues sequentially, Lisa computes a dependency graph across your tickets and runs multiple Claude Code sessions in parallel on the same codebase. Key differences:

- **DAG-based scheduling**: Tickets declare dependencies via YAML frontmatter. Lisa topologically sorts them and launches work as prerequisites complete, rather than processing a flat issue queue.
- **Parallel execution**: Multiple Claude Code sessions work simultaneously, with file locking handling commit serialization. No more one-at-a-time bottleneck.
- **Five-phase workflow**: Each ticket progresses through Research, Design, Structure, Plan, and Implement phases, producing ~200-line artifacts at each step that serve as both review checkpoints and crash recovery points.
- **Zellij integration**: Lisa runs as a terminal multiplexer plugin with a real-time dashboard showing execution status across all active sessions.
- **Local-first**: Tickets are markdown files in your repo rather than GitHub issues, giving you version-controlled task definitions that travel with the code.

If the ideas in My Little Soda interest you, [**lisa**](https://github.com/johnhkchen/lisa) is where they landed.

## License

Open source. See LICENSE file for details.
