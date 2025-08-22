# Issue #126: Migrate Bundling to GitHub Actions & Implement Real Agent Integration

> **Replace prototype agent mocks with real Claude Code integration and leverage GitHub Actions for bundling workflows**

## Problem Statement

### Current State
- **Mock-Only System**: Comprehensive agent mocks (`src/agent_lifecycle/mocks.rs`) but no real Claude Code process spawning
- **Manual Bundle Execution**: Bundling requires local Rust execution rather than cloud-native scheduling
- **Process Management Gap**: No lifecycle management for real agent processes (spawn, monitor, cleanup)
- **Scalability Bottleneck**: Single-machine bundling limits concurrent agent capacity

### Context
- System architecture assumes real agents but currently only simulates work completion
- GitHub Actions provides native scheduling, observability, and scaling that eliminates local resource constraints
- Agent integration is prerequisite for validating multi-agent coordination assumptions
- Cloud-native bundling enables repository-level automation without local dependencies

### Strategic Value
Real agent integration with GitHub Actions bundling provides:
- **Authentic Validation**: Test multi-agent coordination with real Claude Code behavior patterns
- **Operational Reliability**: Eliminate single points of failure from local bundling execution  
- **Resource Scaling**: GitHub Actions scales bundling capacity without infrastructure management
- **Native Integration**: Leverage GitHub's scheduling, retry, and observability capabilities

## Target State

### Vision
A production-ready system where:
1. **Real Claude Code agents** execute work on assigned issues with full process lifecycle management
2. **GitHub Actions workflows** handle bundling scheduling, execution, and monitoring
3. **Hybrid architecture** maintains local coordination with cloud-native bundle execution
4. **Process isolation** ensures agent failures don't cascade to system stability

### Success Metrics
- **Agent Integration**: Successfully spawn, monitor, and cleanup real Claude Code processes
- **GitHub Actions Migration**: 100% of bundling operations execute via GitHub Actions
- **Process Reliability**: <5% agent process failure rate with automatic recovery
- **Workflow Automation**: Zero manual intervention required for standard bundling operations
- **Resource Efficiency**: Support 5+ concurrent agents without local resource exhaustion

### Non-Goals
- Custom GitHub Actions runners or infrastructure
- Agent work quality assessment or code review automation
- Multi-repository bundling coordination
- Advanced process monitoring beyond basic health checks

## Interfaces & Contracts

### Agent Process Management
```rust
pub trait AgentProcessManager {
    /// Spawn Claude Code agent for specific issue
    async fn spawn_agent(&self, agent_id: &str, issue: u64) -> Result<AgentProcess>;
    
    /// Monitor agent process health and progress
    async fn monitor_agent(&self, process_id: &str) -> Result<AgentStatus>;
    
    /// Gracefully terminate agent process
    async fn terminate_agent(&self, process_id: &str) -> Result<()>;
    
    /// Get all active agent processes
    fn list_active_agents(&self) -> Vec<AgentProcess>;
}

pub struct AgentProcess {
    pub process_id: String,
    pub agent_id: String,
    pub issue_number: u64,
    pub branch_name: String,
    pub status: ProcessStatus,
    pub started_at: DateTime<Utc>,
}

pub enum ProcessStatus {
    Starting,
    Working { last_activity: DateTime<Utc> },
    Completed { exit_code: i32 },
    Failed { error: String },
    Terminated,
}
```

### GitHub Actions Integration
```yaml
# .github/workflows/clambake-bundling.yml
name: Clambake Bundle Management
on:
  schedule:
    - cron: '*/10 * * * *'  # Every 10 minutes
  workflow_dispatch:        # Manual trigger
    inputs:
      force_bundle:
        description: 'Force bundling outside schedule'
        required: false
        default: 'false'

jobs:
  bundle-agent-work:
    runs-on: ubuntu-latest
    steps:
      - name: Check for bundling eligibility
        id: check
        run: |
          # Query for route:review issues
          # Set output: should_bundle=true/false
      
      - name: Execute bundling
        if: steps.check.outputs.should_bundle == 'true'
        run: |
          # Execute bundling workflow
          # Create bundle PR or individual PRs
```

### Command-Line Interface
```bash
# Enhanced agent management
clambake spawn --agent agent001 --issue 123   # Spawn real Claude Code agent
clambake monitor --agent agent001             # Monitor agent progress  
clambake terminate --agent agent001           # Gracefully stop agent

# GitHub Actions integration
clambake actions --trigger-bundle             # Manually trigger bundling workflow
clambake actions --status                     # Check GitHub Actions workflow status
clambake actions --logs --run-id 12345        # Fetch workflow logs
```

### Configuration Extensions
```toml
# clambake.toml additions
[agents]
claude_code_path = "claude-code"
max_concurrent = 5
timeout_minutes = 30
cleanup_on_failure = true

[github_actions]
bundling_workflow = ".github/workflows/clambake-bundling.yml"
manual_trigger_enabled = true
workflow_timeout_minutes = 15
```

## Technical Implementation Strategy

### Phase 1: Real Agent Process Integration (3 hours)
**Replace mocks with actual Claude Code process spawning and management**

1. **Agent Process Spawning** (90 minutes)
   - Implement `tokio::process::Command` for Claude Code execution
   - Add process ID tracking and agent-to-process mapping
   - Create agent working directory isolation
   - Add environment variable passing for GitHub tokens and configuration

2. **Process Lifecycle Management** (90 minutes)
   - Implement process monitoring with heartbeat detection
   - Add graceful termination with timeout-based force kill
   - Create automatic cleanup for failed or abandoned agents
   - Add process status reporting and health checks

### Phase 2: GitHub Actions Workflow Creation (2 hours)
**Migrate bundling execution from local to GitHub Actions**

1. **Workflow Definition** (60 minutes)
   - Create `.github/workflows/clambake-bundling.yml` with proper triggers
   - Add job for querying route:review issues via GitHub API
   - Implement bundling logic as workflow steps
   - Add proper error handling and status reporting

2. **Local-to-Actions Integration** (60 minutes)
   - Add GitHub Actions API client for workflow triggering
   - Implement workflow status monitoring from local system
   - Create bridge between local agent coordination and cloud bundling
   - Add workflow log retrieval for debugging

### Phase 3: System Integration & Testing (1 hour)
**Ensure real agents work with GitHub Actions bundling**

1. **End-to-End Integration** (30 minutes)
   - Connect real agent completion to GitHub Actions bundling triggers
   - Validate agent lifecycle with cloud-native bundling
   - Test multi-agent scenarios with GitHub Actions coordination

2. **Error Handling & Recovery** (30 minutes)
   - Handle GitHub Actions failures gracefully
   - Add fallback to local bundling for Actions outages
   - Implement agent process recovery from system restarts

## Implementation Tasks (1-Hour Chunks)

### Task 1: Implement Claude Code Process Spawning (1 hour)
- **Objective**: Replace agent mocks with real Claude Code process execution  
- **Acceptance**: Successfully spawn Claude Code agent for assigned issue
- **Files**: `src/agents/process_manager.rs`, agent lifecycle integration
- **Test**: Spawn agent, verify process running, confirm issue assignment

### Task 2: Add Agent Process Monitoring (1 hour)
- **Objective**: Track agent health and detect failures or completion
- **Acceptance**: Detect agent completion, failure, and timeout scenarios
- **Files**: Process monitoring, status tracking, cleanup logic
- **Test**: Monitor agent through full work lifecycle

### Task 3: Create GitHub Actions Bundling Workflow (1 hour)
- **Objective**: Implement bundling as GitHub Actions workflow
- **Acceptance**: GitHub Actions successfully executes bundling on schedule
- **Files**: `.github/workflows/clambake-bundling.yml`, workflow definition
- **Test**: Trigger workflow, verify bundling execution, check PR creation

### Task 4: Integrate GitHub Actions API Client (1 hour)  
- **Objective**: Enable local system to trigger and monitor GitHub Actions
- **Acceptance**: Local commands can trigger Actions and get status/logs
- **Files**: GitHub Actions client, workflow integration
- **Test**: Trigger bundling from CLI, monitor workflow progress

### Task 5: Connect Real Agents to Actions Bundling (1 hour)
- **Objective**: Complete end-to-end flow with real agents and cloud bundling
- **Acceptance**: Real agent work completion triggers GitHub Actions bundling
- **Files**: Agent completion handlers, Actions integration
- **Test**: Full workflow from agent assignment to bundle PR creation

### Task 6: Add Error Recovery and Fallback (1 hour)
- **Objective**: Handle failures gracefully with appropriate fallbacks  
- **Acceptance**: System continues operating despite agent or Actions failures
- **Files**: Error handling, fallback logic, recovery procedures
- **Test**: Inject failures, verify system recovery and continued operation

## Data Model & Storage

### Process Tracking
```rust
// In-memory process registry (no persistent storage needed)
pub struct ProcessRegistry {
    processes: HashMap<String, AgentProcess>,
    agent_to_process: HashMap<String, String>,
}
```

### GitHub Actions State
- **Workflow Runs**: Tracked via GitHub Actions API
- **Bundle Status**: Stored in workflow outputs and PR descriptions  
- **Coordination**: GitHub issues serve as communication mechanism between local system and Actions

### Configuration Management
- **Agent Paths**: Configurable Claude Code binary location
- **Resource Limits**: Maximum concurrent agents, timeout values
- **GitHub Integration**: Actions workflow paths, API endpoints

## Observability & Monitoring

### Agent Process Metrics
- Active agent count and resource usage
- Agent completion rate and average duration
- Process failure rates and common failure modes
- Resource utilization (CPU, memory, file handles)

### GitHub Actions Integration Metrics  
- Workflow trigger success rate
- Bundling execution time and success rate
- Actions API rate limit usage
- Workflow failure recovery effectiveness

### Structured Logging
```rust
// Example log events for agent processes
info!(
    agent_id = %agent_id,
    issue_number = %issue,
    process_id = %process_id,
    "Agent process spawned successfully"
);

warn!(
    agent_id = %agent_id,
    duration_minutes = %duration,
    last_activity = %last_seen,
    "Agent process appears inactive"
);
```

## Rollback Strategy

### Immediate Rollback (< 5 minutes)
```bash
# Disable real agent integration
clambake config set agents.enabled false

# Disable GitHub Actions bundling  
clambake config set github_actions.enabled false
```

### Process Cleanup
- Terminate all active agent processes gracefully
- Cancel running GitHub Actions workflows
- Fall back to mock agents and local bundling
- Maintain data integrity through rollback

### Recovery Approach
- Real agents are additive to existing mock system
- GitHub Actions bundling has local bundling fallback
- System designed to degrade gracefully
- No data loss during rollback operations

## Definition of Done

### Functional Requirements
- [ ] Real Claude Code agents successfully spawn and execute work
- [ ] Agent processes are monitored and cleaned up appropriately
- [ ] GitHub Actions bundling executes on schedule and via triggers
- [ ] Local system integrates seamlessly with GitHub Actions workflows
- [ ] Multi-agent scenarios work with cloud-native bundling

### Quality Requirements
- [ ] Process spawning succeeds >95% of the time
- [ ] Agent process monitoring detects completion within 30 seconds
- [ ] GitHub Actions bundling has <10 second trigger latency
- [ ] System handles agent process failures without cascading issues
- [ ] End-to-end latency from agent completion to bundle PR <5 minutes

### Documentation Requirements  
- [ ] Agent process management runbook
- [ ] GitHub Actions workflow troubleshooting guide
- [ ] Configuration guide for agent resource limits
- [ ] Security considerations for agent process isolation

## Risks & Mitigation

### Technical Risks
| Risk | Impact | Probability | Mitigation |
|------|---------|-------------|------------|
| Claude Code process instability | High | Medium | Process monitoring, automatic restart, timeout handling |
| GitHub Actions rate limits | Medium | Low | Exponential backoff, fallback to local bundling |
| Process resource exhaustion | Medium | Medium | Resource limits, concurrent agent caps, cleanup procedures |

### Operational Risks
| Risk | Impact | Probability | Mitigation |
|------|---------|-------------|------------|
| Agent process zombie accumulation | Medium | Medium | Automatic cleanup, process lifecycle monitoring |
| GitHub Actions workflow failures | Medium | Low | Comprehensive error handling, local fallback |

## Success Measurement

### Agent Integration Metrics
- **Process Success Rate**: >95% of agent spawns succeed
- **Work Completion Rate**: Comparable to mock agent success rate
- **Resource Efficiency**: <20% CPU usage per concurrent agent
- **Process Lifecycle**: Zero zombie processes after 24-hour operation

### GitHub Actions Migration Metrics
- **Workflow Reliability**: >98% successful workflow executions
- **Bundling Latency**: <2 minutes from trigger to bundle PR creation
- **Operational Autonomy**: Zero manual interventions required for 48-hour period

This specification transforms Clambake from a sophisticated mock system into a production-ready multi-agent orchestration platform with cloud-native bundling capabilities.