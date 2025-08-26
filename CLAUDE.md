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

## Testing Infrastructure Guidelines

### B2c - Enhanced Cleanup and Isolation Mechanisms

The init command testing infrastructure includes comprehensive cleanup and isolation mechanisms to prevent resource leaks and test interference.

#### Enhanced Test Harness Features:
- **Resource Tracking**: Monitors temp directories, files, and processes
- **Leak Detection**: Automatically detects resource leaks outside system temp directories  
- **Cross-Test Isolation**: Verifies test environments don't interfere with each other
- **Cleanup Strategies**: Multiple cleanup approaches (immediate, deferred, force, graceful retry)
- **Error Recovery**: Handles cleanup failures and provides detailed error reports

#### Running Enhanced Cleanup Tests:
```bash
# Run enhanced cleanup and isolation tests
cargo test --test enhanced_cleanup_isolation_tests -- --test-threads=1

# Generate coverage for enhanced cleanup tests  
cargo llvm-cov --test enhanced_cleanup_isolation_tests --lcov --output-path target/coverage-enhanced-cleanup.lcov
```

#### Test Guidelines:
- **Isolation Required**: All init tests must use enhanced test harnesses
- **Resource Cleanup**: Tests must properly clean up temporary resources
- **Thread Safety**: Use `--test-threads=1` for isolation tests to prevent race conditions
- **Leak Detection**: Tests automatically detect and report resource leaks
- **CI Integration**: Enhanced cleanup tests run in all CI environments

### File System Integration Test Requirements:

#### Test Structure:
- **Real File Operations**: Tests create actual files and directories
- **Git Integration**: Tests validate Git repository operations
- **Content Verification**: Tests verify file contents and metadata
- **Cleanup Verification**: Tests ensure proper cleanup after completion

#### Test Patterns:
```rust
// Use mutable harnesses for file operations
let mut harness = simple_harness().unwrap();

// Create files with proper error handling
let file_path = harness.create_file("test.txt", "content").unwrap();
assert!(file_path.exists());

// Verify isolation
harness.verify_isolation().unwrap();
```

### CI/CD Integration:

The CI pipeline includes comprehensive testing across multiple environments:
- **Unit Tests**: Core functionality with coverage reporting
- **Integration Tests**: File system, Git, and workflow integration
- **Enhanced Cleanup Tests**: Resource management and isolation verification  
- **Property-Based Tests**: Configuration validation
- **Cross-Platform Testing**: Windows, macOS, Linux compatibility

#### Test Execution Order:
1. Unit tests with coverage generation
2. Enhanced cleanup and isolation tests (thread-isolated)
3. File system integration tests (thread-isolated) 
4. General workflow integration tests
5. Property-based tests

#### Coverage Requirements:
- **Library Coverage**: 70% line coverage minimum
- **Test Coverage**: Enhanced cleanup tests included in coverage reports
- **Artifact Upload**: Coverage reports uploaded for analysis

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

## CI/CD and Development Workflow

- All CI workflows must pass before code changes may be reviewed.
- The existing code structure must not be changed without a strong reason.
- Every bug must be reproduced by a unit test before being fixed.
- Every new feature must be covered by a unit test before it is implemented.
- Minor inconsistencies and typos in the existing code may be fixed.

## Documentation

- The README.md file must explain the purpose of the repository.
- The README.md file must be free of typos, grammar mistakes, and broken English.
- The README.md file must be as short as possible and must not duplicate code documentation.
- Every struct and enum must have a supplementary doc comment (`///`) preceding it.
- Doc comments must explain the purpose and provide usage examples.
- Every function and method must have a supplementary doc comment preceding it.
- Doc comments must be written in English only, using UTF-8 encoding.
- Use `cargo doc` to generate and verify documentation.

## Code Style and Structure

- Function bodies may not contain blank lines.
- Function and method bodies may not contain comments (use self-documenting code).
- Variable names should be descriptive nouns following `snake_case` convention.
- Function names should be descriptive verbs following `snake_case` convention.
- Respect Rust’s bracket placement conventions (opening brace on same line).
- Error messages should not end with a period.
- Error messages must always be a single sentence, with no periods inside.
- Favor “fail fast” paradigm: use `panic!`, `expect()`, or return `Result` early.

## Rust-Specific Design Principles

- Constructors should be simple associated functions (typically `new()`) containing only initialization.
- Favor composition over inheritance (Rust doesn’t have classical inheritance).
- Avoid unnecessary getter methods; prefer direct field access or explicit methods.
- Follow domain-driven design principles where applicable.
- Struct names may not end with the `-er` suffix unless they represent actors (like `Iterator`).
- Favor immutable data structures; use `mut` sparingly and deliberately.
- Every struct should have one primary constructor (`new()`); additional constructors should delegate.
- Every struct should encapsulate no more than four fields for simplicity.
- Every struct must encapsulate at least one field or be a unit struct with clear purpose.
- Utility modules are preferred over utility structs with associated functions.
- Free functions are acceptable in modules; avoid complex associated functions on empty structs.
- Function names must respect command-query separation: either return data or cause side effects.
- Avoid public constants in structs; use module-level constants instead.
- Define behavior in traits and implement them for structs.
- Public methods should implement traits when possible.
- Functions must return `Option<T>` or `Result<T, E>` instead of potentially panicking.
- Use Rust’s type system to validate arguments at compile time when possible.
- `None` and error values should be handled explicitly, never ignored.
- Avoid `as` casting; use `From`/`Into` traits or explicit conversion methods.
- Avoid reflection-like behavior; use compile-time generics and traits.
- Structs should be the default; use inheritance-like patterns only when necessary via traits.
- Error types should include comprehensive context using `thiserror` or similar.

## Testing Guidelines

- Every change must be covered by a unit test to guarantee repeatability.
- Every test case may contain only one assertion.
- In every test, the assertion must be the last statement.
- Test cases must be as short as possible.
- Every test must assert at least once.
- Each test file should have a clear relationship with the module it tests.
- Every assertion should include a descriptive failure message.
- Tests must use diverse inputs, including non-ASCII strings and edge cases.
- Tests may not share state between test cases.
- Tests may not use setup/teardown; each test should be self-contained.
- Tests may not use shared constants; generate values within each test.
- Test function names must be descriptive sentences using `snake_case`.
- Tests may not test functionality irrelevant to their stated purpose.
- Tests must properly handle resource cleanup using RAII or explicit cleanup.
- Code must not provide functionality used only by tests.
- Tests may not assert on logging output; test behavior, not side effects.
- Tests should not test simple field access or basic constructors.
- Tests must prepare clean state at the start, not clean up afterward.
- Prefer real objects over mocks; use test doubles sparingly.
- Aim for single-statement tests when possible.
- Use standard Rust assertion macros (`assert!`, `assert_eq!`, `assert_ne!`).
- Each test must verify only one specific behavioral pattern.
- Tests should use generated random values as inputs where appropriate.
- Tests should use `tempfile` crate for temporary files and directories.
- Tests must not produce log output during normal execution.
- Configure test logging appropriately using `env_logger` or similar.
- Tests must not wait indefinitely; use timeouts with `tokio::time::timeout` or similar.
- Tests must verify behavior in concurrent environments using appropriate tools.
- Tests should retry flaky operations with exponential backoff.
- Tests must not assume network connectivity; mock external services.
- Tests may not assert on specific error messages, only error types and behavior.
- Tests must provide explicit configuration rather than relying on defaults.
- Tests should not mock fundamental system resources like filesystem or memory.
- Tests must use ephemeral ports from `std::net::TcpListener::bind("127.0.0.1:0")`.
- Tests should inline small test data rather than loading from files.
- Tests should generate large fixtures at runtime using appropriate libraries.
- Tests may create helper functions to reduce duplication while maintaining clarity.
- Test function names should spell “cannot” and “dont” without apostrophes, following Rust naming conventions.

## Additional Rust-Specific Recommendations

- Use `clippy` for additional code quality checks.
- Follow `rustfmt` formatting standards.
- Leverage Rust’s ownership system to prevent common bugs.
- Use `Result<T, E>` for recoverable errors and `panic!` for unrecoverable ones.
- Prefer `&str` over `String` for function parameters when possible.
- Use appropriate collection types (`Vec`, `HashMap`, `BTreeMap`) for specific use cases.
- Implement `Debug`, `Clone`, and other standard traits as needed.
- Use feature flags for optional functionality.
- Follow semantic versioning for public APIs.​​​​​​​​​​​​​​​​
