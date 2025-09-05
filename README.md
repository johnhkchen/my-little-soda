# My Little Soda

**Autonomous coding agents for multiple repositories simultaneously.**

Run one Claude Code agent per repository to multiply development capacity. Agents work unattended on GitHub issues while you focus on high-level tasks.

## Usage

**Setup:**
```bash
cargo install --path .
my-little-soda init
```

**Run agents:**
1. Open Claude Code in multiple terminals (one per repository)
2. Paste `/prompts/initial-system-prompt.md` into each session
3. Agents work autonomously on issues labeled `route:ready`
4. When done, paste `/prompts/finishing-prompt.md` and type `/clear`

**Result:** 3 repositories = 3x development capacity

## Commands

```bash
my-little-soda init      # Initialize repository
my-little-soda peek      # Preview available issues  
my-little-soda pop       # Claim and start work on issue
my-little-soda bottle    # Bundle completed work for review
my-little-soda status    # Check agent status
```

## Workflow

**Agent (autonomous):** `pop` → work → `bottle`  
**Human (supervised):** Review PR → merge

Agents work 15-60 minutes per issue. One agent per repository prevents conflicts.

## Development

```bash
cargo build && cargo test
```

## License

Open source. See LICENSE file for details.