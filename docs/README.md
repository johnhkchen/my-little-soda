# My Little Soda Documentation

> **From quickstart to production. For humans and agents.**

## Documentation Structure

Our documentation follows a progressive disclosure pattern - start simple, dive deep when needed.

## Getting Started (`getting-started/`)

### Quickstart (`quickstart.md`)
```bash
# 5 minutes to multi-agent development
cargo install my-little-soda
my-little-soda init
my-little-soda route
my-little-soda dashboard
```

### Installation (`installation.md`)
- System requirements
- Installation methods
- Verification steps
- Troubleshooting

### First Project (`first-project.md`)
- Initialize a project
- Create GitHub issues
- Route to agents
- Monitor progress
- Land completed work

## User Guide (`user-guide/`)

### Commands (`commands.md`)
Complete reference for all CLI commands:
- `init` - Project setup
- `route` - Ticket assignment
- `land` - Work integration
- `status` - System monitoring
- `recover` - Disaster recovery
- `dashboard` - Phoenix observability

### Configuration (`configuration.md`)
- `my-little-soda.toml` structure
- GitHub integration setup
- Agent configuration
- Phoenix deployment

### Workflows (`workflows.md`)
Common multi-agent patterns:
- Sprint planning workflow
- Continuous integration
- Hotfix coordination
- Dependency management

### Troubleshooting (`troubleshooting.md`)
- Common issues and solutions
- Debugging techniques
- Recovery procedures
- Performance tuning

## Architecture (`architecture/`)

### Design Decisions (`design-decisions.md`)
Why we built it this way:
- GitHub as single source of truth
- Atomic operations only
- Compile-time safety
- Observable by design

### Domain Model (`domain-model.md`)
- Core concepts
- State transitions
- Invariants
- Recovery model

### Integration Points (`integration-points.md`)
- GitHub API usage
- Claude Code integration
- Phoenix observability
- Git worktree management

## API Documentation

Generated from source code:
```bash
# Generate API docs
cargo doc --open

# Online at
https://docs.rs/my-little-soda
```

## Documentation Principles

### DRY Documentation
- Single source of truth for each concept
- Generated from code where possible
- Cross-referenced, not duplicated
- Automated freshness checks

### Accessibility
- Examples for every feature
- Progressive complexity
- Agent-readable formats
- Searchable and indexed

### Maintenance
- Docs reviewed with code
- Automated link checking
- Version-specific docs
- Migration guides

## Finding Information

### Quick Reference
| Need | Location |
|------|----------|
| Install My Little Soda | `getting-started/installation.md` |
| Command reference | `user-guide/commands.md` |
| Architecture | `architecture/design-decisions.md` |
| API docs | `cargo doc` or docs.rs |
| Specifications | [`../specs/`](../specs/) |
| Contributing | [`../CONTRIBUTING.md`](../CONTRIBUTING.md) |
| Historical documentation | [`../archive/`](../archive/) |

### Search
```bash
# Search all documentation
grep -r "your term" docs/ specs/

# Find examples
find docs -name "*.md" -exec grep -l "example" {} \;
```

## Contributing to Docs

1. **Accuracy**: Test all examples
2. **Clarity**: Write for your past self
3. **Completeness**: Cover edge cases
4. **Currency**: Update with code changes
5. **Accessibility**: Consider all readers

## Documentation Tools

```bash
# Check for broken links
./scripts/check-links.sh

# Generate command references
./scripts/generate-cli-docs.sh

# Build full documentation site
mdbook build
```

---

**Good documentation prevents coordination disasters. Keep it current.**