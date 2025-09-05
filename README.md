# My Little Soda

[![Development Status](https://img.shields.io/badge/status-early%20alpha-orange)](https://github.com/johnhkchen/my-little-soda)
[![Stability](https://img.shields.io/badge/stability-experimental-red)](https://github.com/johnhkchen/my-little-soda)
[![License](https://img.shields.io/badge/license-open%20source-blue)](LICENSE)

**Multiply your development capacity across repositories.**

## What You Get

🚀 **3 repositories = 3x development capacity**  
⏱️ **Agents work 15-60 minutes per issue unattended**  
🧠 **You focus on architecture while agents handle implementation**  
🔄 **One agent per repo prevents merge conflicts**

## How It Works

**Your workflow:**
1. Open Claude Code in multiple terminals (one per repository)
2. Paste `/prompts/initial-system-prompt.md` into each session  
3. Agents work autonomously on issues labeled `route:ready`
4. When done, paste `/prompts/finishing-prompt.md` and type `/clear`

**Agent workflow:** `pop` → work → `bottle`  
**Your role:** Review PR → merge

## Setup

**Prerequisites:**
- GitHub personal access token with repo permissions

**Installation:**
```bash
# Build the binary
cargo build --release

# Set up GitHub authentication
export MY_LITTLE_SODA_GITHUB_TOKEN=your_github_token_here

# Initialize in your repository
./target/release/my-little-soda init

# Validate setup
./target/release/my-little-soda doctor
```

**For development:**
```bash
# Alternative: Install globally
cargo install --path .
# Then use: my-little-soda init
# Validate: my-little-soda doctor
```

## Commands

**Core workflow:**
```bash
my-little-soda init      # Initialize repository
my-little-soda peek      # Preview available issues  
my-little-soda pop       # Claim and start work on issue
my-little-soda bottle    # Bundle completed work for review
my-little-soda status    # Check agent status
```

**System diagnostics:**
```bash
my-little-soda doctor    # Run comprehensive system health checks
my-little-soda doctor --verbose  # Show detailed diagnostic information
my-little-soda doctor --format json  # Machine-readable JSON output
my-little-soda doctor --ci-mode  # Optimized for CI/CD environments
```

**Common troubleshooting scenarios:**
- **Setup validation:** Run `my-little-soda doctor` after installation to verify everything is configured correctly
- **Authentication issues:** Doctor command checks GitHub authentication and repository permissions  
- **Missing labels:** Identifies required labels that need to be created with `my-little-soda init`
- **Agent state problems:** Detects stuck agents, orphaned assignments, and workflow issues
- **CI/CD debugging:** Use `my-little-soda doctor --ci-mode --format json` for automated health checks

Run `my-little-soda --help` to see all available commands.

## Development

```bash
cargo build && cargo test
```

## License

Open source. See LICENSE file for details.