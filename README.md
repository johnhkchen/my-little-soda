# My Little Soda

**Multiply your development capacity: Today 1→4 developers, tomorrow 1→15 days of work.**

Set up autonomous coding agents across multiple repositories. You paste two prompts into Claude Code, and agents work unattended while you focus on architecture, planning, or other high-value tasks.

## The Problem

Solo developers can only work one repository at a time. Context switching is expensive. You want to make progress on multiple codebases but lack the capacity.

## The Solution for Humans

**Simple Setup:**
1. Open Claude Code in 3 terminals for 3 repositories
2. Paste the same two prompts into each Claude session
3. Agents work autonomously on GitHub Issues labeled `route:ready`
4. You get 3x development capacity while focusing elsewhere

**The Two Prompts You Need:**
- Copy `/prompts/initial-system-prompt.md` to start an agent session
- Copy `/prompts/finishing-prompt.md` when ready to wrap up
- Type `/clear` to end the session

**Your Experience:**
```bash
# Terminal 1: Web app repository  
# [Paste initial prompt] → Agent fixes login bug autonomously

# Terminal 2: API repository  
# [Paste initial prompt] → Agent adds rate limiting autonomously

# Terminal 3: Mobile app repository
# [Paste initial prompt] → Agent updates dependencies autonomously
```

While agents work (15-60 minutes each today, hours tomorrow), you focus on high-value tasks like architecture decisions, code reviews, or planning.

## Why This Architecture Works

**Current**: 1 developer + 3 agents = 32 repo-hours  
**Future**: 1 day unattended + 3 repos = 15 days of work

One agent per repository eliminates merge conflicts, builds repository-specific context, and prevents interference during long autonomous sessions.

## Quick Start for Humans

### 1. Install My Little Soda
```bash
# From source
cargo install --path .

# Or download binary from releases
```

### 2. Setup Your Repository
Add labels to your GitHub issues:
- `route:ready` → Available for agent pickup
- `route:priority-high` → Process first (optional)

Initialize repository:
```bash
my-little-soda init
```

### 3. Start Your Autonomous Agent

**Copy this exact text into Claude Code:**
```
[Content of /prompts/initial-system-prompt.md - the agent workflow]
```

**The agent will automatically:**
- Run `my-little-soda pop` to get work
- Implement solutions autonomously  
- Run `my-little-soda bottle` to bundle completed work

**When ready to finish, paste:**
```
Add issue covering undone work. merge/commit these updates to main, then clean up branches such that main is the only one left. If 'my-little-soda status' reports the agent as busy, you may have to close the issue and de-assign the agent001 label.
```

**Then type:** `/clear`

### 4. Scale Across Repositories
Repeat steps 2-3 in different terminals for each repository you want agents to work on simultaneously.

## How It Works (Agent Workflow)

**For the Agent (Autonomous):**
1. **Pop**: Claim issue, create branch `agent001/123-fix-login-bug`
2. **Work**: Implement solution (15-60 minutes today, hours tomorrow)
3. **Bottle**: Bundle work, add `route:review` label

**For You (Human-Supervised):**
4. **Review**: Review PR and merge when ready

Two-phase design: agents work autonomously, humans review and merge. You stay in control while multiplying development capacity.

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