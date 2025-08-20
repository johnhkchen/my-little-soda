# Clambake Test Suite

> **Test-driven. Chaos-engineered. Property-verified.**

## Testing Philosophy

We write "testing spells" - expressive, declarative specifications that generate comprehensive test coverage automatically. Every feature is tested before it's written.

## Test Organization

### Unit Tests (`unit/`)
- Fast, focused tests for individual components
- Run on every file save during development
- Complete in <1 second

### Integration Tests (`integration/`)
- `github_api/` - Real GitHub API interaction tests
- `claude_code/` - Agent coordination tests
- `phoenix/` - Observability integration tests
- `e2e/` - Complete workflow tests

### Property Tests (`properties/`)
- `coordination/` - Multi-agent invariant testing
- `performance/` - Latency and throughput properties
- `chaos/` - Failure injection and recovery

### Benchmarks (`../benchmarks/`)
- Performance regression detection
- Scalability testing (1-12 agents)
- Resource usage profiling

## Test Infrastructure

### Fixtures (`fixtures/`)
- Realistic GitHub API responses
- Agent behavior patterns
- Test repository structures

### Generators (`generators/`)
- Random issue generation
- Agent behavior simulation
- Chaos scenario generation

### Mocks (`mocks/`)
- GitHub API mock server
- Claude Code agent simulator
- Phoenix tracing mock

## Running Tests

```bash
# Run all tests
cargo test

# Run specific test category
cargo test --test integration

# Run with chaos engineering
cargo test --features chaos

# Run benchmarks
cargo bench

# Run property tests (1000 iterations)
cargo test --features proptest --release
```

## Key Testing Patterns

### Scenario Testing
```rust
scenario! {
    name: "Multi-agent coordination",
    given: { test_repo_with_issues!() },
    when: { route_tickets_to_agents!() },
    then: { 
        no_duplicate_assignments!(),
        work_never_lost!(),
        state_always_consistent!()
    },
}
```

### Property Testing
```rust
property_test! {
    name: "Coordination invariants",
    forall: (agents: 1..=12, issues: 1..=100),
    invariants: {
        work_preservation: "No work is ever lost",
        state_consistency: "GitHub == local state",
        atomic_operations: "All or nothing",
    },
}
```

### Chaos Testing
```rust
chaos_test! {
    inject: [
        github_api_timeout,
        agent_crash,
        network_partition,
    ],
    verify: {
        system_recovers!(),
        work_preserved!(),
        no_corruption!(),
    },
}
```

## Test Coverage Requirements

- **Minimum**: 90% line coverage
- **Critical paths**: 100% coverage required
- **Error handling**: Every error path tested
- **Recovery logic**: Chaos-tested

## Continuous Testing

### Pre-commit
- Unit tests must pass
- No new code without tests

### CI Pipeline
- Full test suite
- Property tests (100 iterations)
- Integration with real GitHub API (sandbox)

### Nightly
- Chaos engineering (1000 scenarios)
- Performance benchmarks
- Memory leak detection

## Writing New Tests

1. **Test First**: Write the test before the implementation
2. **Use Helpers**: Leverage test macros and generators
3. **Test Properties**: Not just examples, test invariants
4. **Inject Chaos**: Every feature should handle failures
5. **Trace Tests**: All tests integrate with Phoenix tracing

## Test Debugging

```bash
# Run single test with output
cargo test test_name -- --nocapture

# Run with tracing
RUST_LOG=debug cargo test

# Generate test coverage report
cargo tarpaulin --out Html
```

---

**Every test here prevents a coordination disaster. No exceptions.**