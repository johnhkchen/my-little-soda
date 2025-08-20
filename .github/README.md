# GitHub Integration

> **GitHub-native CI/CD. No custom workflows. No state files.**

## GitHub Configuration

This directory contains GitHub-specific configuration that makes Clambake work seamlessly with GitHub's native features.

## Workflows (`.github/workflows/`)

All workflows follow VERBOTEN principles: no environment variables for config, no state files, no manual steps.

### Core Workflows

#### CI Pipeline (`ci.yml`)
```yaml
name: CI
on:
  push:
    branches: [main]
  pull_request:
    types: [opened, synchronize, reopened]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      
      # No environment variables for configuration
      - name: Run tests
        run: cargo test --all-features
      
      # Phoenix integration for CI observability  
      - name: Report to Phoenix
        run: |
          cargo run -- report-ci-metrics
      
      # Enforce VERBOTEN rules
      - name: Check safety rules
        run: |
          cargo clippy -- -D warnings \
            -F dual_state_stores \
            -F manual_sync \
            -F environment_overrides
```

#### Release Automation (`release.yml`)
```yaml
name: Release
on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      # Automated release, no manual steps
      - uses: actions/checkout@v4
      - name: Build release
        run: cargo build --release
      
      # Work preservation even in CI
      - name: Create release
        uses: softprops/action-gh-release@v1
        with:
          files: target/release/clambake
          # Releases tracked in GitHub, not files
```

#### Nightly Chaos Testing (`nightly-chaos.yml`)
```yaml
name: Chaos Engineering
on:
  schedule:
    - cron: '0 2 * * *'  # 2 AM UTC daily

jobs:
  chaos:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run chaos tests
        run: |
          cargo test --features chaos --release
      
      # Report chaos results to Phoenix
      - name: Analyze failures
        run: |
          cargo run -- analyze-chaos-results
```

#### Performance Regression (`performance-regression.yml`)
```yaml
name: Performance
on:
  pull_request:
    paths:
      - 'src/**'
      - 'Cargo.toml'

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run benchmarks
        run: cargo bench
      
      # Compare with baseline
      - name: Check regression
        run: |
          cargo run -- check-performance-regression
```

## Issue Templates (`ISSUE_TEMPLATE/`)

Templates designed for agent routing, not human convenience.

### Bug Report (`bug_report.yml`)
```yaml
name: Bug Report
description: Report coordination failures or work loss
labels: ["bug", "route:ready"]
body:
  - type: textarea
    id: description
    attributes:
      label: Bug Description
      description: What coordination failed?
    validations:
      required: true
  
  - type: dropdown
    id: severity
    attributes:
      label: Severity
      options:
        - Work was lost (CRITICAL)
        - State corrupted (CRITICAL)
        - Coordination failed (HIGH)
        - Performance degraded (MEDIUM)
        - Minor issue (LOW)
  
  - type: textarea
    id: phoenix-trace
    attributes:
      label: Phoenix Trace ID
      description: From the observability dashboard
      placeholder: trace-id-here
```

### Feature Request (`feature_request.yml`)
```yaml
name: Feature Request
description: Request new coordination capabilities
labels: ["enhancement", "route:ready"]
body:
  - type: textarea
    id: problem
    attributes:
      label: Problem to Solve
      description: What coordination challenge?
    validations:
      required: true
  
  - type: textarea
    id: solution
    attributes:
      label: Proposed Solution
      description: How should it work?
  
  - type: checklist
    id: safety
    attributes:
      label: Safety Checklist
      options:
        - label: No new state files
        - label: GitHub remains source of truth
        - label: Operations are atomic
        - label: Work is preserved
```

### Agent Routing (`agent-routing.yml`)
```yaml
name: Agent-Ready Task
description: Task ready for agent assignment
labels: ["route:ready"]
body:
  - type: input
    id: title
    attributes:
      label: Task Title
      description: Clear, actionable title
    validations:
      required: true
  
  - type: dropdown
    id: complexity
    attributes:
      label: Estimated Complexity
      options:
        - Low (< 2 hours)
        - Medium (2-8 hours)
        - High (1-2 days)
        - Complex (> 2 days)
  
  - type: textarea
    id: requirements
    attributes:
      label: Requirements
      description: What must be built?
  
  - type: textarea
    id: acceptance
    attributes:
      label: Acceptance Criteria
      description: How do we know it's done?
```

## Required GitHub Settings

### Repository Settings
```yaml
General:
  - Issues: Enabled
  - Projects: Enabled
  - Wiki: Disabled  # Docs in repo, not wiki

Branches:
  - Protected branch: main
  - Required reviews: 1
  - Dismiss stale reviews: Yes
  - Required status checks:
    - ci
    - safety-check
  - Include administrators: Yes  # No bypasses

Actions:
  - Actions permissions: Selected
  - Required workflows:
    - ci.yml
    - safety-check.yml
```

### Project Board Setup
```yaml
Project Type: Projects V2
Columns:
  - Backlog (route:ready)
  - Assigned (agent:assigned)
  - In Progress (agent:in-progress)
  - Review (agent:review)
  - Done (Closed issues)

Automation:
  - Issue closed -> Move to Done
  - PR merged -> Close issue
  - Label added -> Update column
```

### Labels Configuration
```yaml
Routing:
  - route:ready (green)
  - route:blocked (gray)
  - route:priority-high (red)
  - route:human-only (purple)

Agent Status:
  - agent:assigned (blue)
  - agent:in-progress (yellow)
  - agent:review (purple)
  - agent:completed (green)

Work Type:
  - bug (red)
  - enhancement (cyan)
  - documentation (light blue)
  - testing (orange)
```

## GitHub Integration Principles

### Use Native Features
- Projects V2 for coordination
- Issues for tickets
- PRs for integration
- Actions for automation
- No custom solutions

### No State Files
- No workflow artifacts for state
- No environment secrets for config
- No manual deployment steps
- Everything through GitHub API

### Observable Workflows
- All workflows report to Phoenix
- Trace IDs in PR comments
- Metrics on every run
- Failure analysis automated

---

**GitHub is the platform. Use it natively. No custom abstractions.**