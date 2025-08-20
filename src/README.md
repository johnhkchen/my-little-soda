# Clambake Source Code

> **Single source of truth. Atomic operations. Zero manual sync.**

## Architecture Overview

Clambake's source code is organized around domain-driven design with compile-time safety guarantees that make coordination disasters impossible.

## Module Structure

### Core (`core/`)
- `orchestrator.rs` - Main coordination engine (single source of truth)
- `config.rs` - Configuration management (no env var overrides)
- `container.rs` - Dependency injection for testability

### GitHub Integration (`github/`)
- `client.rs` - Octocrab wrapper with atomic operations
- `issues.rs` - Issue management (tickets)
- `projects.rs` - Project board operations (agent state)
- `pulls.rs` - Pull request handling (work integration)
- `branches.rs` - Branch management (isolated workspaces)

### Agent Coordination (`agents/`)
- `router.rs` - Intelligent ticket routing
- `coordinator.rs` - Agent state management
- `integrator.rs` - Work integration pipeline
- `recovery.rs` - Automatic recovery operations
- `context_loader.rs` - Domain context for agents
- `claude_integration.rs` - Claude Code sub-agent management

### Workflows (`workflows/`)
- `state_machine.rs` - Type-safe state transitions
- `atomic_operations.rs` - All-or-nothing transactions
- `retry.rs` - Consistent retry patterns
- `contracts.rs` - Formal workflow contracts

### Observability (`phoenix/`)
- `tracer.rs` - OpenTelemetry integration
- `metrics.rs` - Coordination metrics
- `dashboard.rs` - Real-time monitoring
- `evaluator.rs` - Agent performance analysis

### Commands (`commands/`)
- `init.rs` - Project initialization
- `route.rs` - Ticket routing
- `land.rs` - Work integration
- `sync.rs` - State synchronization
- `recover.rs` - Work recovery
- `status.rs` - System status
- `cleanup.rs` - Safe cleanup

### Error Handling (`error/`)
- `types.rs` - Exhaustive error types
- `recovery.rs` - Uniform recovery strategies

### White Magic (`macros/`)
- `single_source_of_truth.rs` - Prevents dual state
- `atomic_coordination.rs` - Prevents race conditions
- `safety_invariants.rs` - Compile-time safety
- `coordination_dsl.rs` - Type-safe workflows
- `zero_trust_operations.rs` - No silent failures
- `observability_by_design.rs` - Automatic tracing

## Safety Guarantees

### Compile-Time Protection
```rust
#[deny(unsafe_code)]           // No unsafe code
#[deny(dead_code)]            // No unused code
#[deny(missing_docs)]         // All public APIs documented
#[forbid(dual_state_stores)]  // No state duplication
#[forbid(manual_sync)]        // No manual synchronization
#[forbid(environment_overrides)] // No env var bypasses
```

### Runtime Invariants
- GitHub is the ONLY source of truth
- All operations are atomic
- Work is preserved before any destructive operation
- Every coordination decision is traced
- Failures are explicit, never silent

## Development Guidelines

### Adding New Features
1. Write test first (TDD mandatory)
2. Use existing macros for safety
3. No custom state files
4. All GitHub operations through coordination layer
5. Trace every decision point

### Code Review Checklist
Every change must answer NO to:
- [ ] Creates dual state stores?
- [ ] Allows manual synchronization?
- [ ] Creates race conditions?
- [ ] Allows silent failures?
- [ ] Bypasses safety mechanisms?
- [ ] Uses environment variables for behavior?
- [ ] Lacks test coverage?
- [ ] Creates shared mutable state?

## Building

```bash
# Development build with all checks
cargo build

# Release build with optimizations
cargo build --release

# Run all tests
cargo test

# Run with instrumentation
cargo run --features observability
```

## Architecture Decisions

See [`../specs/architecture/`](../specs/architecture/) for detailed design decisions.

---

**Remember: Every line of code here was written to prevent another event-api disaster.**