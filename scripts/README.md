# Clambake Scripts

> **Automation that follows the rules. No shortcuts. No bypasses.**

## Script Organization

Every script here follows VERBOTEN principles: no state files, no manual sync, no environment variable bypasses.

## Available Scripts

### Setup (`setup.sh`)
```bash
# Complete development environment setup
./scripts/setup.sh
```
- Installs Rust toolchain
- Sets up git hooks
- Configures GitHub CLI
- No environment variables used

### Phoenix Installation (`install-phoenix.sh`)
```bash
# Deploy Phoenix observability stack
./scripts/install-phoenix.sh
```
- Docker-based deployment
- Automatic configuration
- Health check verification
- No manual configuration needed

### Test Data Generation (`generate-test-data.sh`)
```bash
# Generate realistic test scenarios
./scripts/generate-test-data.sh --agents 8 --issues 50
```
- Creates test GitHub issues
- Generates agent scenarios
- Builds chaos test cases
- Deterministic and reproducible

### Migration (`migrate-from-kanban.sh`)
```bash
# Migrate from kanban.yaml disaster to Clambake
./scripts/migrate-from-kanban.sh --dry-run
```
- Analyzes existing kanban.yaml
- Creates GitHub issues
- Sets up project board
- Preserves all work (no data loss)

### Coverage Reporting (`run-coverage.sh`)
```bash
# Generate HTML coverage report for library tests
./scripts/run-coverage.sh

# Generate all formats for comprehensive coverage analysis
./scripts/run-coverage.sh --scope all --format all

# Generate LCOV report with threshold enforcement
./scripts/run-coverage.sh --format lcov --fail-on-threshold

# Open coverage report in browser
./scripts/run-coverage.sh --open
```
- Generates code coverage reports using cargo-llvm-cov
- Supports multiple output formats: HTML, LCOV, JSON, Cobertura XML
- Configurable coverage scopes: lib, integration, all targets
- Automatic LLVM toolchain detection and configuration
- Coverage threshold enforcement (70% lines, 75% functions, 65% regions)
- CI/CD integration with artifact upload
- Browser integration for HTML reports

## Script Safety Rules

### VERBOTEN in Scripts
- ❌ No reading environment variables for configuration
- ❌ No creating state files
- ❌ No manual synchronization
- ❌ No silent failures
- ❌ No destructive operations without confirmation
- ❌ No bypassing safety checks

### Required Patterns
- ✅ All operations are idempotent
- ✅ Explicit error handling
- ✅ Dry-run mode for dangerous operations
- ✅ Atomic operations (all or nothing)
- ✅ Progress reporting
- ✅ Rollback capability

## Writing New Scripts

### Template
```bash
#!/usr/bin/env bash
set -euo pipefail  # Exit on error, undefined vars are errors, pipe failures are errors

# No environment variable configuration
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Explicit configuration only
readonly CONFIG_FILE="${PROJECT_ROOT}/.clambake/config.toml"

# Progress reporting
log() { echo "[$(date +'%Y-%m-%d %H:%M:%S')] $*" >&2; }
error() { log "ERROR: $*"; exit 1; }

# Dry run support
DRY_RUN=${1:-false}
if [[ "$DRY_RUN" == "--dry-run" ]]; then
    log "DRY RUN MODE - No changes will be made"
fi

# Atomic operations
trap 'error "Script failed. Rolling back..."' ERR

# Main logic here
log "Starting operation..."
# ...
log "Operation completed successfully"
```

### Testing Scripts
```bash
# Test in dry-run mode first
./scripts/my-script.sh --dry-run

# Run shellcheck for safety
shellcheck scripts/*.sh

# Test idempotency (run twice, same result)
./scripts/my-script.sh
./scripts/my-script.sh  # Should be safe to run again
```

## Script Dependencies

Required tools (checked by setup.sh):
- Bash 4.0+
- Git 2.30+
- Docker 20.10+
- GitHub CLI 2.0+
- Rust 1.75+
- cargo-edit
- cargo-watch

## Continuous Integration

All scripts are:
- Linted with shellcheck
- Tested in CI pipeline
- Run in isolated environments
- Verified for idempotency

## Common Functions

Shared functions in `common.sh`:
```bash
source "${SCRIPT_DIR}/common.sh"

# Available functions:
# - check_requirements()    # Verify system requirements
# - confirm_operation()     # Get user confirmation
# - atomic_operation()      # Run with rollback on failure
# - report_progress()       # Standardized progress reporting
```

## Debugging Scripts

```bash
# Run with trace output
bash -x ./scripts/my-script.sh

# Run with verbose logging
DEBUG=1 ./scripts/my-script.sh

# Check script syntax without running
bash -n ./scripts/my-script.sh
```

---

**These scripts encode our hard-won lessons. No exceptions to safety rules.**