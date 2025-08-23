# Claude Development Notes

## Documentation Maintenance

⚠️ **README Maintenance Reminder**: The README is a living document that must be kept current.

### When to Update README:
- **Command changes**: Update examples when CLI interface changes
- **Feature additions**: Document new functionality and workflows  
- **Version releases**: Update version badges and feature availability
- **Link changes**: Verify external links during releases
- **Setup process changes**: Update installation/setup instructions

### Pre-Release Checklist:
- [ ] Verify all command examples work with current build
- [ ] Check that version numbers match Cargo.toml
- [ ] Test all external links are accessible
- [ ] Ensure feature descriptions match actual functionality
- [ ] Validate setup instructions with fresh repository

### Quick README Verification:
```bash
# Test that key commands work as documented
./target/release/my-little-soda --help
./target/release/my-little-soda pop --help
./target/release/my-little-soda bottle --help
./target/release/my-little-soda init --help

# Verify repository functionality
./target/release/my-little-soda status
```

**Remember**: Documentation debt is technical debt. Fix it promptly when discovered.

## Architectural Constraints

🏗️ **CRITICAL ARCHITECTURAL CONSTRAINT** - This section defines the fundamental architecture that MUST be followed throughout development.

### One-Agent-Per-Repository Architecture

**NEVER IMPLEMENT MULTI-AGENT COORDINATION IN THE SAME REPOSITORY**

My Little Soda follows a strict **ONE AGENT PER REPOSITORY** architecture:

✅ **Correct Architecture:**
- Single autonomous agent processes issues sequentially within one repository
- Scale productivity by running multiple My Little Soda instances across different repositories
- Agent operates unattended while human focuses on other work
- Multiplicative productivity: 8 hours human + 3 autonomous agents = 32 repo-hours

❌ **Never Implement:**
- Multiple concurrent agents in the same repository
- Agent-to-agent coordination or communication
- Resource sharing between agents in the same repo
- Complex multi-agent merge conflict resolution

### Why This Architecture?

**Productivity Focus:** The goal is multiplicative productivity through horizontal scaling across repositories, not complex coordination within a single repository.

**Simplicity:** Single-agent operation eliminates:
- Merge conflicts between agents
- Complex coordination logic
- Resource contention issues
- Agent-to-agent communication overhead

**Autonomous Operation:** Enables true unattended operation where the agent works continuously while the human developer focuses elsewhere.

### Implementation Guidelines

**When Building Features:**
- Design for single-agent sequential operation
- Focus on autonomous operation capabilities
- Optimize for unattended continuous processing
- Enable horizontal scaling across repositories

**When Writing Specifications:**
- Never assume multiple concurrent agents per repository
- Use "autonomous agent" instead of "multi-agent" language
- Focus on productivity multiplication through horizontal scaling
- Emphasize unattended operation as key value proposition

**When Reviewing Code:**
- Reject any implementation that assumes multiple agents per repo
- Ensure agent lifecycle is designed for single-agent operation
- Verify that coordination is through GitHub labels/issues, not inter-agent communication

### Success Metrics Alignment

**Measure This:**
- Agent uptime and continuous operation
- Issues processed per hour by single agent
- Time from issue assignment to completion
- Success rate of autonomous operation periods

**Don't Measure:**
- Concurrent agent coordination efficiency
- Multi-agent resource utilization
- Inter-agent communication latency

### Future Development Warning

⚠️ **If you find yourself implementing any of the following, STOP:**
- Agent ID management for multiple concurrent agents
- Agent-to-agent communication protocols
- Multi-agent resource locks or semaphores
- Complex agent coordination state machines

**Instead, implement:**
- Single agent lifecycle management
- Issue queue processing for sequential work
- Autonomous operation monitoring
- Horizontal scaling documentation

This architectural constraint is fundamental to My Little Soda's value proposition and must never be violated.