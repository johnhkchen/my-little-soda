# Clambake Project Templates

> **Start right. Scale right. No coordination disasters.**

## Available Templates

Each template provides a complete multi-agent development setup with GitHub integration and Phoenix observability from day one.

## Template Structure

Every template includes:
- `.clambake/` - Pre-configured for multi-agent coordination
- `.github/` - GitHub Actions and issue templates
- `docker-compose.phoenix.yml` - Phoenix observability stack
- `README.md` - Project-specific documentation
- **NEVER**: State files, manual sync, or environment configs

## Templates

### Web Application (`webapp/`)
```bash
clambake init --template webapp --agents 8
```
- Next.js/React frontend setup
- Node.js backend configuration
- GitHub Actions for JS/TS
- Agent routing for frontend/backend tickets
- Dependency management built-in

### API Service (`api/`)
```bash
clambake init --template api --agents 6
```
- REST/GraphQL API structure
- Database migration coordination
- API testing workflow
- Agent specialization for endpoints
- Automatic OpenAPI generation

### CLI Tool (`cli/`)
```bash
clambake init --template cli --agents 4
```
- Rust/Go CLI structure
- Command organization
- Cross-platform building
- Agent routing for commands
- Release automation

### Microservices (`microservices/`)
```bash
clambake init --template microservices --agents 12
```
- Service isolation patterns
- Inter-service coordination
- Distributed tracing setup
- Agent-per-service allocation
- Kubernetes deployment configs

### Library (`library/`)
```bash
clambake init --template library --agents 3
```
- Package structure
- Documentation generation
- Version management
- Agent routing for features
- Publishing workflow

## Template Configuration

### Default Settings (`.clambake/config.toml`)
```toml
[github]
# Templates never hardcode values
owner = "${GITHUB_OWNER}"  # Set during init
repo = "${GITHUB_REPO}"    # Set during init
project_id = "${PROJECT_ID}" # Created during init

[routing]
max_agents = 8
routing_label = "route:ready"
# No environment variable overrides

[integration]
auto_merge = true
require_reviews = true
# All safety checks mandatory
```

### GitHub Integration (`.github/`)
```yaml
# Workflows are Phoenix-integrated from day one
name: CI
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Run Tests
        run: |
          # Tests with Phoenix tracing
          cargo test --features observability
      - name: Report to Phoenix
        run: |
          # Automatic observability
          clambake report-metrics
```

### Phoenix Stack (`docker-compose.phoenix.yml`)
```yaml
version: '3.8'
services:
  phoenix:
    image: arizephoenix/phoenix:latest
    ports:
      - "6006:6006"  # Phoenix UI
      - "4317:4317"  # OTLP gRPC
      - "4318:4318"  # OTLP HTTP
    environment:
      # No custom configuration needed
      - PHOENIX_WORKING_DIR=/data
    volumes:
      - phoenix-data:/data
```

## Using Templates

### Quick Start
```bash
# Create new project from template
clambake init --template webapp --name my-app --agents 8

# Navigate to project
cd my-app

# Start Phoenix observability
docker-compose -f docker-compose.phoenix.yml up -d

# Create initial issues
gh issue create --title "Setup authentication" --label "route:ready"
gh issue create --title "Add user dashboard" --label "route:ready"

# Route to agents
clambake route

# Monitor progress
clambake dashboard
```

### Customizing Templates

1. **Never add**:
   - State files (YAML/JSON/TOML)
   - Manual synchronization
   - Environment variable configs
   - Bypass mechanisms

2. **Always include**:
   - GitHub as source of truth
   - Phoenix observability
   - Atomic operations
   - Work preservation

## Creating New Templates

### Requirements
- Must follow all VERBOTEN rules
- Must include Phoenix integration
- Must use GitHub Projects V2
- Must have isolated agent workspaces
- Must preserve work always

### Structure
```
my-template/
├── .clambake/
│   ├── config.toml.template
│   └── agents/
│       └── default.toml
├── .github/
│   ├── workflows/
│   │   └── ci.yml
│   └── ISSUE_TEMPLATE/
│       └── feature.yml
├── docker-compose.phoenix.yml
├── README.md.template
└── template.json  # Template metadata
```

### Validation
```bash
# Validate template follows rules
clambake validate-template ./my-template

# Test template initialization
clambake init --template ./my-template --dry-run
```

## Template Principles

### Start Right
- GitHub integration from commit zero
- Phoenix observability from day one
- Multi-agent ready immediately
- No technical debt

### Scale Right
- 1 agent or 12 agents, same workflow
- Atomic operations at any scale
- Work preserved always
- Coordination guaranteed

### No Disasters
- No state files to corrupt
- No manual sync to break
- No environment variables to drift
- No silent failures to hide

---

**Templates encode best practices. Start every project the right way.**