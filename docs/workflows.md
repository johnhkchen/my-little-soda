# My Little Soda Workflows

> **Common multi-agent patterns and development workflows.**

This document covers established workflows and processes for My Little Soda development and maintenance.

## Dependency Management

### Cargo-udeps Analysis Workflow

When analyzing dependencies for cleanup, cargo-udeps may flag dependencies as "unused" even when they are legitimately needed. This section documents verified findings to prevent accidental removal of required dependencies.

#### Dependencies Flagged but Verified as Used

The following dependencies have been analyzed and confirmed as necessary:

**Testing Dependencies:**
- `tokio-test`: Used extensively in test code with `tokio_test::block_on` calls
- Used in async test scenarios where tokio runtime control is needed

**System Integration:**
- `hostname`: Used in production code:
  - `src/autonomous/work_continuity.rs`
  - `src/autonomous/persistence.rs`
  - Required for system identification and work continuity features

**Utilities:**
- `rand`: Used in multiple source files:
  - `src/autonomous/error_recovery.rs`
  - Required for backoff strategies and randomization

**HTTP Client (Conditional):**
- `reqwest`: Used in tests and commented code
  - May be re-enabled for future features
  - Present in test scenarios
- `reqwest-middleware` & `reqwest-retry`: 
  - Have commented usage that may be restored
  - Part of robust HTTP client configuration

**Observability Stack:**
- `opentelemetry` & related crates:
  - Part of observability and metrics features
  - May be feature-gated but essential for monitoring
- `tracing-opentelemetry`:
  - Bridges tracing with OpenTelemetry
  - Part of telemetry system architecture

#### Running cargo-udeps

When performing dependency analysis:

1. **Install cargo-udeps:**
   ```bash
   cargo install cargo-udeps
   ```

2. **Run analysis:**
   ```bash
   cargo udeps
   ```

3. **Review findings against this documentation:**
   - Check each flagged dependency against the verified list above
   - Only remove dependencies not documented here
   - When in doubt, grep the codebase for usage

4. **Update this documentation:**
   - Document any newly verified dependencies
   - Update usage patterns and rationale
   - Include file locations where dependencies are used

#### Guidelines for Future Audits

**Before removing any dependency:**
1. Check if it's listed in the "verified as used" section above
2. Search the entire codebase including:
   - Source code (`src/`)
   - Tests (`tests/`)
   - Examples and commented code
   - Feature-gated code
3. Consider if the dependency is part of a larger subsystem
4. Test that removal doesn't break any features

**When adding new dependencies:**
- Document the usage rationale
- Note if the dependency is feature-gated
- Include in integration tests if appropriate

**Maintenance:**
- Review this document during major version updates
- Update when new subsystems are added
- Keep usage locations current

### Expected Benefits

Following this workflow prevents:
- Accidental removal of actually-used dependencies
- Breaking feature-gated or conditionally compiled code
- Disruption of testing infrastructure
- Loss of observability capabilities

## Other Workflows

*Additional workflows will be documented here as they are established.*

---

**Remember: Dependencies serve the system. Document their purpose to prevent coordination disasters.**