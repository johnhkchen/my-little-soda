# Clambake Specifications

> **Living documentation. Domain-driven. Agent-accessible.**

## Specification Structure

Our specifications are organized to be both human-readable and programmatically queryable by agents. Each domain has isolated requirements to prevent information overload.

## Domain Specifications (`domains/`)

Each domain follows the same structure:
- `requirements.md` - What must be built
- `contracts.md` - API contracts and interfaces
- `invariants.md` - Safety rules that cannot be violated

### Available Domains

| Domain | Purpose | Key Invariants |
|--------|---------|----------------|
| `github-integration/` | GitHub API orchestration | Single source of truth |
| `agent-coordination/` | Multi-agent management | No race conditions |
| `work-integration/` | PR/merge workflows | Work preservation |
| `observability/` | Phoenix tracing | Complete visibility |
| `testing/` | Test framework | 100% critical path coverage |
| `cli/` | Command interface | Safety cannot be bypassed |

## Cross-Cutting Concerns (`cross-cutting/`)

### Anti-Patterns (`anti-patterns.md`)
What we must NEVER do (from VERBOTEN):
- No dual state stores
- No manual synchronization
- No silent failures
- No environment variable overrides

### Defensive Patterns (`defensive-patterns.md`)
How we prevent disasters (from White Magic):
- Compile-time safety
- Atomic operations
- Immutable state
- Type-safe workflows

### Safety Invariants (`safety-invariants.md`)
System-wide guarantees:
- Work is never lost
- State is always consistent
- Operations are atomic
- Failures are explicit

### Performance Targets (`performance-targets.md`)
- Routing: <2 seconds
- Integration: <5 minutes
- 8-12 concurrent agents
- <500MB memory usage

### Error Recovery (`error-recovery.md`)
Uniform recovery strategies:
- Exponential backoff
- Work preservation
- Automatic recovery
- Human escalation

## Architecture Documentation (`architecture/`)

- `overview.md` - System architecture
- `mvp.md` - MVP implementation plan
- `test-strategy.md` - Testing approach

## Using Specifications

### For Developers
```bash
# Search for requirements
grep -r "work preservation" specs/

# Check domain contracts
cat specs/domains/github-integration/contracts.md

# Review invariants before changes
cat specs/cross-cutting/safety-invariants.md
```

### For Agents
```rust
// Agents automatically load relevant specs
let context = DomainContext::load_for_domain("github-integration")?;
let requirements = context.requirements;
let invariants = context.invariants;
```

### For Code Review
Every PR must:
1. Comply with domain requirements
2. Respect all invariants
3. Follow defensive patterns
4. Avoid all anti-patterns

## Specification Validation

Specs are validated at:
- **Compile time**: Through macros and type system
- **Test time**: Property tests verify invariants
- **Runtime**: Invariants checked continuously
- **Review time**: Automated PR checks

## Keeping Specs Current

### DRY Principles
- Single source for each requirement
- Generated code from specifications
- Automated validation against implementation
- Living documentation from code

### Version Control
- Specs versioned with code
- Changes require same review as code
- Breaking changes documented
- Migration guides provided

## Querying Specifications

```rust
// Programmatic access to specs
let spec_server = SpecificationServer::new();

// Natural language queries
let results = spec_server.query("how to preserve work");

// Domain-specific lookups
let github_spec = spec_server.get_domain_spec(Domain::GitHubIntegration);

// Validation
let valid = spec_server.validate_against_spec(implementation);
```

## Contributing to Specs

1. Specs are code - they need tests
2. Changes must maintain consistency
3. Update all affected domains
4. Provide migration notes
5. Get architecture review

---

**These specifications are the law. The implementation must comply.**