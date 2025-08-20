# Cross-Cutting Concerns

> **System-wide rules. No exceptions. Always enforced.**

## Overview

These specifications apply to ALL domains and ALL code. They represent hard-learned lessons from the event-api disaster and are non-negotiable.

## Files

### Anti-Patterns (`anti-patterns.md`)
**Source**: VERBOTEN.md
**Purpose**: Document what we must NEVER do
**Enforcement**: Compile-time and runtime checks

Key prohibitions:
- No dual state stores
- No manual synchronization  
- No silent failures
- No environment variable configs
- No destructive operations without preservation

### Defensive Patterns (`defensive-patterns.md`)
**Source**: White Magic macros
**Purpose**: Proactive prevention of disasters
**Enforcement**: Macro system and type safety

Key patterns:
- Single source of truth enforcement
- Atomic operation guarantees
- Immutable state management
- Exhaustive error handling
- Observable by design

### Safety Invariants (`safety-invariants.md`)
**Purpose**: System-wide guarantees that must always hold
**Enforcement**: Continuous runtime validation

Core invariants:
1. **Work is never lost** - Even during catastrophic failures
2. **State is always consistent** - GitHub == local state
3. **Operations are atomic** - All succeed or all fail
4. **Failures are explicit** - No silent errors
5. **Coordination is observable** - Every decision traced

### Performance Targets (`performance-targets.md`)
**Purpose**: Concrete performance requirements
**Enforcement**: Benchmark tests and monitoring

Targets:
- Routing latency: <2 seconds
- Integration time: <5 minutes  
- Concurrent agents: 8-12
- Memory usage: <500MB
- API calls per operation: <50

### Error Recovery (`error-recovery.md`)
**Purpose**: Uniform recovery strategies
**Enforcement**: Recovery trait implementation

Strategies:
1. **Exponential backoff** - For transient failures
2. **Work preservation** - Before any retry
3. **Automatic recovery** - For known patterns
4. **Human escalation** - For unknown failures
5. **Audit trail** - For all recovery attempts

## How Cross-Cutting Specs Work

### Automatic Application
```rust
// These specs are automatically enforced
#[enforce_cross_cutting_specs]
impl AnyClambakeComponent {
    // All methods automatically get:
    // - Anti-pattern detection
    // - Defensive patterns applied
    // - Invariant checking
    // - Performance monitoring
    // - Error recovery
}
```

### Compile-Time Enforcement
```rust
// Won't compile - violates anti-patterns
struct BadDesign {
    local_state: StateFile,     // ERROR: Dual state store
    github_state: GitHubAPI,    // ERROR: Must be single source
}

// Won't compile - missing defensive patterns  
fn bad_operation() {
    update_state();  // ERROR: Not atomic
    // ERROR: No error handling
    // ERROR: Not observable
}
```

### Runtime Enforcement
```rust
// Invariants checked continuously
fn any_operation() -> Result<()> {
    // Automatic pre-condition check
    assert_invariants!();
    
    // Operation logic
    let result = do_work()?;
    
    // Automatic post-condition check
    assert_invariants!();
    
    Ok(result)
}
```

## Hierarchy of Enforcement

1. **Type System** (Strongest)
   - Impossible to express violations
   - Caught at compile time
   - Zero runtime cost

2. **Macro System**
   - Generates safe code
   - Validates at expansion time
   - Minimal runtime cost

3. **Trait Bounds**
   - Enforces patterns
   - Caught at compile time
   - Interface guarantees

4. **Runtime Checks**
   - Continuous validation
   - Performance monitored
   - Recovery triggered

5. **Observability** (Weakest but pervasive)
   - Everything traced
   - Anomalies detected
   - Alerts triggered

## Updating Cross-Cutting Specs

### Requirements
1. Architecture review required
2. Must strengthen, never weaken
3. Migration path required
4. All domains must comply
5. Tests must be updated

### Process
```bash
# Propose change
clambake propose-spec-change --file safety-invariants.md

# Validate impact
clambake analyze-spec-impact

# Generate migration
clambake generate-migration --spec safety-invariants.md

# Apply with validation
clambake apply-spec-change --validate
```

## Non-Negotiable Rules

These specs are **NEVER** optional:
- Apply to all code, no exceptions
- Cannot be disabled or bypassed
- Enforced in development and production
- Violations break the build
- No "temporary" exemptions

---

**These are the laws of physics for Clambake. They cannot be violated.**