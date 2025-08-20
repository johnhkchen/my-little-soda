# Domain Specifications

> **Isolated requirements. Focused scope. Agent-optimized.**

## Domain Structure

Each domain contains exactly three files:
- `requirements.md` - What must be built
- `contracts.md` - API contracts and interfaces  
- `invariants.md` - Rules that cannot be violated

This structure ensures:
1. **No information overload** - Agents only load relevant domains
2. **Clear boundaries** - No cross-domain pollution
3. **Consistent format** - Same structure everywhere
4. **DRY principle** - Each requirement stated once

## Available Domains

### GitHub Integration (`github-integration/`)
**Purpose**: Orchestrate all GitHub API operations
**Key Invariant**: GitHub is the single source of truth
**Agent Context**: Loaded for issue routing, PR creation, project board updates

### Agent Coordination (`agent-coordination/`)
**Purpose**: Manage multi-agent lifecycle and assignments
**Key Invariant**: No race conditions or duplicate assignments
**Agent Context**: Loaded for routing decisions, conflict detection, capacity management

### Work Integration (`work-integration/`)
**Purpose**: Safely land completed work to main branch
**Key Invariant**: Work is never lost, even during failures
**Agent Context**: Loaded for PR creation, merge coordination, conflict resolution

### Observability (`observability/`)
**Purpose**: Phoenix integration and tracing
**Key Invariant**: Every coordination decision is observable
**Agent Context**: Loaded for metric reporting, trace creation, performance analysis

### Testing (`testing/`)
**Purpose**: Comprehensive test framework with chaos engineering
**Key Invariant**: 100% critical path coverage
**Agent Context**: Loaded for test generation, property verification, chaos scenarios

### CLI (`cli/`)
**Purpose**: Command-line interface and user experience
**Key Invariant**: Safety mechanisms cannot be bypassed
**Agent Context**: Loaded for command implementation, argument parsing, output formatting

## Using Domain Specs

### For Implementation
```rust
// Load domain requirements during development
let domain = DomainSpec::load("github-integration")?;
let requirements = domain.requirements();
let contracts = domain.contracts();
let invariants = domain.invariants();

// Validate implementation against spec
domain.validate_implementation(&my_code)?;
```

### For Agents
```rust
// Agents automatically get domain context
pub fn spawn_agent(ticket: &Issue) -> Result<Agent> {
    let domain = classify_ticket_domain(ticket)?;
    let context = DomainContext::from_spec(domain)?;
    
    Agent::new()
        .with_context(context)
        .with_invariant_checking(true)
        .spawn()
}
```

### For Testing
```rust
// Generate tests from domain specs
generate_tests! {
    domain: "agent-coordination",
    test_all_requirements: true,
    test_all_invariants: true,
    chaos_test_contracts: true,
}
```

## Domain Independence

Domains are intentionally isolated:
- No cross-references between domains
- No shared state or configuration
- No circular dependencies
- Each domain is self-contained

This ensures:
- Agents aren't confused by irrelevant specs
- Changes are localized
- Testing is focused
- Cognitive load is minimized

## Adding New Domains

### Requirements for New Domains
1. Clear, single responsibility
2. No overlap with existing domains
3. Complete specification (all 3 files)
4. Agent-loadable context
5. Testable contracts

### Domain Template
```markdown
# requirements.md
## Functional Requirements
- REQ-001: [Specific requirement]
- REQ-002: [Another requirement]

# contracts.md  
## API Contracts
- CONTRACT-001: [Input/Output specification]
- CONTRACT-002: [Error handling contract]

# invariants.md
## Safety Invariants
- INV-001: [Rule that must always hold]
- INV-002: [Another invariant]
```

## Domain Validation

All domains are continuously validated:
```bash
# Validate domain completeness
clambake validate-domain github-integration

# Check for requirement coverage
clambake coverage-check --domain agent-coordination

# Verify invariant enforcement
clambake invariant-check --all-domains
```

---

**Domains define the law. Code must comply.**